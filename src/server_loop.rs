use crate::auth::mojang::AuthError;
use crate::auth::velocity::VelocityAuthError;
use crate::auth::{bungee, velocity, AuthConfiguration, AuthenticatedClient};
use crate::chat::Chat;
use crate::pin_fut;
use crate::pipeline::AsyncMinecraftProtocolPipeline;
use crate::protocol::handshaking::sb::{Handshake, NextState, UnlimitedAddressHandshake};
use crate::protocol::login::cb::Disconnect;
use crate::registry::RegistryError;
use crate::status::StatusBuilder;
use drax::prelude::BoxFuture;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};

#[derive(serde_derive::Deserialize, Clone)]
#[serde(untagged)]
pub enum IncomingAuthenticationOption {
    MOJANG,
    BUNGEE,
    VELOCITY { secret_key: String },
}

#[derive(serde_derive::Deserialize)]
pub struct BaseConfiguration {
    pub auth_option: IncomingAuthenticationOption,
    pub compression_threshold: isize,
    pub force_key_authentication: bool,
    pub auth_url: Option<String>,
}

pub struct ServerLoop<
    R: AsyncRead + Unpin + Sized + Send + Sync,
    W: AsyncWrite + Unpin + Sized + Send + Sync,
    Ctx,
    ClientAcceptor: (Fn(
            Ctx,
            AuthenticatedClient<R, W>,
        ) -> BoxFuture<'static, std::result::Result<(), RegistryError>>)
        + 'static,
    StatusResponder: (Fn(Handshake) -> BoxFuture<'static, StatusBuilder>) + 'static,
> {
    auth_config: Arc<AuthConfiguration>,
    auth_option: IncomingAuthenticationOption,
    client_acceptor: ClientAcceptor,
    status_responder: Arc<StatusResponder>,
    _phantom_ctx: PhantomData<Ctx>,
    _phantom_r: PhantomData<R>,
    _phantom_w: PhantomData<W>,
}

impl<
        R: AsyncRead + Unpin + Sized + Send + Sync + 'static,
        W: AsyncWrite + Unpin + Sized + Send + Sync + 'static,
        Ctx: 'static,
        ClientAcceptor: (Fn(
                Ctx,
                AuthenticatedClient<R, W>,
            ) -> BoxFuture<'static, std::result::Result<(), RegistryError>>)
            + 'static,
        StatusResponder: (Fn(Handshake) -> BoxFuture<'static, StatusBuilder>) + 'static,
    > ServerLoop<R, W, Ctx, ClientAcceptor, StatusResponder>
{
    pub fn new(
        config: BaseConfiguration,
        client_acceptor: ClientAcceptor,
        status_responder: StatusResponder,
    ) -> Self {
        Self {
            auth_config: Arc::new(AuthConfiguration {
                server_private_key: crate::crypto::new_key().expect("Key should gen."),
                compression_threshold: config.compression_threshold,
                force_key_authentication: config.force_key_authentication,
                auth_url: config.auth_url,
            }),
            auth_option: config.auth_option,
            client_acceptor,
            status_responder: Arc::new(status_responder),
            _phantom_ctx: Default::default(),
            _phantom_r: Default::default(),
            _phantom_w: Default::default(),
        }
    }

    async fn handle_handshake(_: &mut (), handshake: Handshake) -> Handshake {
        handshake
    }

    async fn handle_unlimited_address_handshake(
        _: &mut (),
        handshake: UnlimitedAddressHandshake,
    ) -> Handshake {
        handshake.into()
    }

    pub async fn accept_client(
        arc_self: Arc<Self>,
        ctx: Ctx,
        read: R,
        write: W,
    ) -> Result<(), RegistryError> {
        log::trace!("Accepting new client.");

        let mut handshake_pipeline = AsyncMinecraftProtocolPipeline::empty(read);
        log::trace!("Registering handshake.");
        if let IncomingAuthenticationOption::BUNGEE = arc_self.auth_option {
            handshake_pipeline.register(pin_fut!(Self::handle_unlimited_address_handshake));
        } else {
            handshake_pipeline.register(pin_fut!(Self::handle_handshake));
        }
        let handshake: Handshake = handshake_pipeline.execute_next_packet(&mut ()).await?;

        match handshake.next_state {
            NextState::Status => {
                log::trace!("Reading status client: {:?}", handshake);
                let res = crate::status::handle_status_client(
                    handshake_pipeline,
                    write,
                    handshake,
                    arc_self.status_responder.clone(),
                )
                .await;
                if matches!(
                    res,
                    Err(RegistryError::DraxTransportError(
                        drax::transport::Error::EOF
                    ))
                ) {
                    return Ok(());
                }
                res?;
                Ok(())
            }
            NextState::Login => match &arc_self.auth_option {
                IncomingAuthenticationOption::MOJANG => {
                    log::trace!("Logging client in.");
                    let authenticated_client = match crate::auth::mojang::auth_client(
                        handshake_pipeline,
                        write,
                        handshake,
                        arc_self.auth_config.clone(),
                    )
                    .await
                    {
                        Ok(client) => client,
                        Err(err) => {
                            log::warn!("Failed to login user for {}", err);
                            let mut error_message = Chat::text("");
                            let mut prompt = Chat::text("Failed to login: ");
                            prompt.modify_style(|style| style.color("#FF0000"));
                            error_message.push_extra(prompt);
                            error_message.push_extra(Chat::literal(format!("{}", err)));

                            (match err {
                                AuthError::InvalidState(mut writer) => writer,
                                AuthError::KeyError(mut writer, _) => writer,
                                AuthError::ValidationError(mut writer, _) => writer,
                                AuthError::TransportError(mut writer, _) => writer,
                                AuthError::RegistryError(mut writer, _) => writer,
                            })
                            .write_packet(&Disconnect {
                                reason: Some(error_message),
                                legacy_reason: None,
                            })
                            .await?;

                            return Ok(());
                        }
                    };
                    ((&arc_self.client_acceptor)(ctx, authenticated_client)).await
                }
                IncomingAuthenticationOption::BUNGEE => {
                    let client = match bungee::auth_client(
                        handshake_pipeline,
                        write,
                        handshake,
                        arc_self.auth_config.clone(),
                    )
                    .await
                    {
                        Ok(client) => client,
                        Err(mut err) => {
                            log::warn!("Encountered error during login. {}", err);
                            return err.disconnect_client_for_error().await.map_err(From::from);
                        }
                    };
                    ((&arc_self.client_acceptor)(ctx, client)).await
                }
                IncomingAuthenticationOption::VELOCITY { secret_key } => {
                    let client = match velocity::auth_client(
                        handshake_pipeline,
                        write,
                        handshake,
                        arc_self.auth_config.clone(),
                        secret_key.to_string(),
                    )
                    .await
                    {
                        Ok(client) => client,
                        Err(mut err) => {
                            log::warn!("Encountered error during login. {}", err);
                            return err.disconnect_client_for_error().await.map_err(From::from);
                        }
                    };
                    ((&arc_self.client_acceptor)(ctx, client)).await
                }
            },
            _ => Ok(()),
        }
    }
}

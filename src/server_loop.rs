use crate::auth::mojang::{AuthError, AuthenticatedClient};
use crate::auth::AuthConfiguration;
use crate::chat::Chat;
use crate::pin_fut;
use crate::pipeline::AsyncMinecraftProtocolPipeline;
use crate::protocol::handshaking::sb::{Handshake, NextState};
use crate::protocol::login::cb::Disconnect;
use crate::registry::RegistryError;
use crate::status::StatusBuilder;
use drax::prelude::BoxFuture;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};

#[derive(serde_derive::Deserialize, Clone, Copy)]
pub enum IncomingAuthenticationOption {
    MOJANG,
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

    pub async fn accept_client(
        arc_self: Arc<Self>,
        ctx: Ctx,
        read: R,
        write: W,
    ) -> Result<(), RegistryError> {
        log::trace!("Accepting new client.");

        let mut handshake_pipeline = AsyncMinecraftProtocolPipeline::empty(read);
        log::trace!("Registering handshake.");
        handshake_pipeline.register(pin_fut!(Self::handle_handshake));
        let handshake: Handshake = handshake_pipeline.execute_next_packet(&mut ()).await?;
        match handshake.next_state {
            NextState::Status => {
                log::trace!("Reading status client: {:?}", handshake);
                let res = crate::status::handle_status_client(
                    handshake_pipeline.into_inner_read(),
                    write,
                    handshake,
                    arc_self.status_responder.clone(),
                )
                .await;
                if matches!(res, Err(RegistryError::DraxTransportError(drax::transport::Error::EOF))) {
                    return Ok(());
                }
                res?;
                Ok(())
            }
            NextState::Login => match arc_self.auth_option {
                IncomingAuthenticationOption::MOJANG => {
                    log::trace!("Logging client in.");
                    let authenticated_client = match crate::auth::mojang::auth_client(
                        handshake_pipeline.into_inner_read(),
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
                            .write_packet(Disconnect {
                                reason: error_message,
                            })
                            .await?;

                            return Ok(());
                        }
                    };
                    ((&arc_self.client_acceptor)(ctx, authenticated_client)).await
                }
            },
            _ => Ok(()),
        }
    }
}

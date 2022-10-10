use crate::auth::mojang::{AuthError, AuthenticatedClient};
use crate::auth::AuthConfiguration;
use crate::chat::Chat;
use crate::pin_fut;
use crate::pipeline::AsyncMinecraftProtocolPipeline;
use crate::protocol::handshaking::sb::{Handshake, NextState};
use crate::protocol::login::cb::Disconnect;
use crate::registry::{MappedAsyncPacketRegistry, RegistryError};
use crate::status::StatusBuilder;
use drax::prelude::BoxFuture;
use drax::transport::encryption::{DecryptRead, EncryptedWriter};
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};

#[derive(serde_derive::Deserialize, Clone, Copy)]
pub enum IncomingAuthenticationOption {
    MOJANG,
}

#[derive(serde_derive::Deserialize)]
pub struct BaseConfiguration {
    auth_option: IncomingAuthenticationOption,
    compression_threshold: isize,
    force_key_authentication: bool,
    auth_url: Option<String>,
}

pub type ClientAcceptor<R, W, Ctx> =
    fn(Ctx, AuthenticatedClient<DecryptRead<R>, EncryptedWriter<W>>) -> BoxFuture<'static, ()>;
pub type StatusResponder = fn(Handshake) -> BoxFuture<'static, StatusBuilder>;

pub struct ServerLoop<
    R: AsyncRead + Unpin + Sized + Send + Sync,
    W: AsyncWrite + Unpin + Sized + Send + Sync,
    Ctx,
> {
    auth_config: Arc<AuthConfiguration>,
    auth_option: IncomingAuthenticationOption,
    client_acceptor: ClientAcceptor<R, W, Ctx>,
    status_responder: Arc<StatusResponder>,
}

impl<
        R: AsyncRead + Unpin + Sized + Send + Sync + 'static,
        W: AsyncWrite + Unpin + Sized + Send + Sync + 'static,
        Ctx: 'static,
    > ServerLoop<R, W, Ctx>
{
    pub fn new(
        config: BaseConfiguration,
        client_acceptor: ClientAcceptor<R, W, Ctx>,
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
        let mut handshake_pipeline = AsyncMinecraftProtocolPipeline::empty(read);
        handshake_pipeline.register(pin_fut!(Self::handle_handshake));
        let handshake: Handshake = handshake_pipeline.execute_next_packet(&mut ()).await?;
        match handshake.next_state {
            NextState::Status => {
                crate::status::handle_status_client(
                    handshake_pipeline.into_inner_read(),
                    write,
                    handshake,
                    arc_self.status_responder.clone(),
                )
                .await?;
            }
            NextState::Login => match arc_self.auth_option {
                IncomingAuthenticationOption::MOJANG => {
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
                            let mut error_message = Chat::literal("Failed to login: ");
                            error_message.modify_style(|style| style.color("RED"));
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
                    ((&arc_self.client_acceptor)(ctx, authenticated_client))
                        .await
                }
            },
            _ => (),
        }
        Ok(())
    }
}
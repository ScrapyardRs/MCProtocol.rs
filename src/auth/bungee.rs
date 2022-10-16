use crate::auth::AuthConfiguration;
use crate::chat::Chat;
use crate::crypto::CapturedRsaError;
use crate::pin_fut;
use crate::pipeline::{AsyncMinecraftProtocolPipeline, MinecraftProtocolWriter};
use crate::protocol::handshaking::sb::Handshake;
use crate::protocol::login::cb::{Disconnect, LoginPluginRequest};
use crate::protocol::login::sb::{LoginPluginResponse, LoginStart};
use crate::protocol::login::{IdentifiedKey, MojangIdentifiedKey, VerifyError};
use crate::protocol::{GameProfile, Property};
use crate::registry::{AsyncPacketRegistry, RegistryError};
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use uuid::{Error, Uuid};

enum AuthFunctionResponse {
    LoginStartSuccess {
        key: Option<IdentifiedKey>,
        sig_holder: Option<Uuid>,
        mojang_key: Option<MojangIdentifiedKey>,
        name: String,
    },
}

pub struct AuthClientContext {
    auth_config: Arc<AuthConfiguration>,
}

pub struct BungeeAuthErrorWithWriter<W: Send + Sync> {
    writer: MinecraftProtocolWriter<W>,
    error: BungeeAuthError,
}

impl<W: AsyncWrite + Send + Sync + Unpin + Sized> BungeeAuthErrorWithWriter<W> {
    pub async fn disconnect_client_for_error(&mut self) -> Result<(), drax::transport::Error> {
        let mut error_message = Chat::text("");
        let mut prompt = Chat::text("Failed to login: ");
        prompt.modify_style(|style| style.color("#FF0000"));
        error_message.push_extra(prompt);
        error_message.push_extra(Chat::literal(format!("{}", self.error)));
        self.writer
            .write_packet(&Disconnect {
                reason: error_message,
            })
            .await
    }
}

impl<W: Send + Sync> Debug for BungeeAuthErrorWithWriter<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.error)
    }
}

impl<W: Send + Sync> Display for BungeeAuthErrorWithWriter<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl<W: Send + Sync> std::error::Error for BungeeAuthErrorWithWriter<W> {}

#[derive(Debug)]
pub enum BungeeAuthError {
    InvalidState,
    KeyExpired,
    KeyNotFound,
    KeyHolderNotFound,
    KeyError(CapturedRsaError),
    TransportError(drax::transport::Error),
    RegistryError(RegistryError),
    MacError(sha2::digest::MacError),
    DataMismatch(String),
}

impl BungeeAuthError {
    pub fn with_ctx<W: Send + Sync>(
        self,
        ctx: MinecraftProtocolWriter<W>,
    ) -> BungeeAuthErrorWithWriter<W> {
        BungeeAuthErrorWithWriter {
            writer: ctx,
            error: self,
        }
    }
}

impl Display for BungeeAuthError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BungeeAuthError::InvalidState => write!(f, "Invalid packet sent during login."),
            BungeeAuthError::KeyExpired => write!(f, "Player key expired."),
            BungeeAuthError::KeyError(err) => write!(f, "Player key invalid. {}", err),
            BungeeAuthError::TransportError(err) => write!(f, "Transport error found! {}", err),
            BungeeAuthError::RegistryError(err) => write!(f, "Registry error found! {}", err),
            BungeeAuthError::KeyNotFound => write!(f, "Player key not found, but expected."),
            BungeeAuthError::KeyHolderNotFound => write!(f, "Player key holder not found."),
            BungeeAuthError::MacError(err) => write!(f, "Mac error encountered: {}", err),
            BungeeAuthError::DataMismatch(what) => {
                write!(f, "Data for `{}` did not match.", what)
            }
        }
    }
}

impl std::error::Error for BungeeAuthError {}

impl From<CapturedRsaError> for BungeeAuthError {
    fn from(value: CapturedRsaError) -> Self {
        Self::KeyError(value)
    }
}

impl From<drax::transport::Error> for BungeeAuthError {
    fn from(value: drax::transport::Error) -> Self {
        Self::TransportError(value)
    }
}

impl From<RegistryError> for BungeeAuthError {
    fn from(value: RegistryError) -> Self {
        Self::RegistryError(value)
    }
}

impl From<VerifyError> for BungeeAuthError {
    fn from(value: VerifyError) -> Self {
        match value {
            VerifyError::CapturedRsa(err) => Self::from(err),
            VerifyError::DraxTransport(err) => Self::from(err),
        }
    }
}

impl From<sha2::digest::MacError> for BungeeAuthError {
    fn from(value: sha2::digest::MacError) -> Self {
        Self::MacError(value)
    }
}

async fn handle_login_start(
    ctx: &mut AuthClientContext,
    login_start: LoginStart,
) -> Result<AuthFunctionResponse, BungeeAuthError> {
    let key = match (
        login_start.sig_data.as_ref(),
        login_start.sig_holder.as_ref(),
    ) {
        (Some(sig_data), Some(sig_holder)) => {
            if sig_data.has_expired() {
                return Err(BungeeAuthError::KeyExpired);
            }
            let mojang_der = crate::crypto::key_from_der(super::MOJANG_KEY)?;
            sig_data.verify_incoming_data(&mojang_der, sig_holder)?;

            Some(IdentifiedKey::new(&sig_data.public_key)?)
        }
        (Some(_), None) => {
            return Err(BungeeAuthError::KeyHolderNotFound);
        }
        (None, None) | (None, Some(_)) => {
            if ctx.auth_config.force_key_authentication {
                return Err(BungeeAuthError::KeyNotFound);
            }
            None
        }
    };

    Ok(AuthFunctionResponse::LoginStartSuccess {
        key,
        mojang_key: login_start.sig_data,
        name: login_start.name,
        sig_holder: login_start.sig_holder,
    })
}

pub async fn auth_client<
    IC: Send + Sync,
    IO: Send + Sync,
    R: AsyncRead + Unpin + Sized + Send + Sync,
    W: AsyncWrite + Unpin + Sized + Send + Sync,
    Reg: AsyncPacketRegistry<IC, IO> + Send + Sync,
>(
    auth_pipeline: AsyncMinecraftProtocolPipeline<R, IC, IO, Reg>,
    write: W,
    handshake: Handshake,
    auth_config: Arc<AuthConfiguration>,
) -> Result<super::AuthenticatedClient<R, W>, BungeeAuthErrorWithWriter<W>> {
    let writer = MinecraftProtocolWriter::from_handshake(write, &handshake);

    let mut parts = handshake.server_address.split("\0");
    let _ = parts.next(); // forced host
    macro_rules! match_parts {
        ($parts:ident, $writer:ident, $descr:literal) => {
            match $parts.next() {
                None => {
                    return Err(BungeeAuthErrorWithWriter {
                        $writer,
                        error: BungeeAuthError::DataMismatch($descr.to_string()),
                    })
                }
                Some(addr) => addr,
            }
        };
    }
    let address = match_parts!(parts, writer, "address");
    let undashed_id = match_parts!(parts, writer, "undashed_id");
    let properties_str = match_parts!(parts, writer, "properties_str");
    let id = match Uuid::parse_str(undashed_id) {
        Ok(id) => id,
        Err(_) => {
            return Err(BungeeAuthErrorWithWriter {
                writer,
                error: BungeeAuthError::DataMismatch("uuid".to_string()),
            })
        }
    };
    let properties: Vec<Property> = match serde_json::from_str(properties_str) {
        Ok(properties) => properties,
        Err(err) => {
            return Err(BungeeAuthErrorWithWriter {
                writer,
                error: BungeeAuthError::TransportError(drax::transport::Error::SerdeJsonError(err)),
            })
        }
    };

    let mut auth_pipeline = auth_pipeline.rewrite_registry(handshake.protocol_version);
    auth_pipeline.clear_data();
    auth_pipeline.register(pin_fut!(handle_login_start));

    let mut context = AuthClientContext { auth_config };

    let (key, mojang_key, name, sig_holder) =
        match auth_pipeline.execute_next_packet(&mut context).await {
            Ok(Ok(AuthFunctionResponse::LoginStartSuccess {
                key,
                mojang_key,
                name,
                sig_holder,
            })) => (key, mojang_key, name, sig_holder),
            Err(err) => {
                return Err(BungeeAuthErrorWithWriter {
                    writer,
                    error: BungeeAuthError::RegistryError(err),
                });
            }
            Ok(Err(err)) => {
                return Err(BungeeAuthErrorWithWriter { writer, error: err });
            }
        };

    let profile = GameProfile {
        id,
        name,
        properties,
    };

    Ok(super::AuthenticatedClient {
        read_write: (
            auth_pipeline.noop_decryption().clear_registry(),
            writer.noop_encryption(),
        ),
        profile,
        key,
        sig_holder,
        mojang_key,
        overridden_address: Some(address.to_string()),
    })
}

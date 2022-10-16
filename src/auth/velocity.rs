use crate::auth::AuthConfiguration;
use crate::chat::Chat;
use crate::crypto::CapturedRsaError;
use crate::pin_fut;
use crate::pipeline::{AsyncMinecraftProtocolPipeline, MinecraftProtocolWriter};
use crate::protocol::handshaking::sb::Handshake;
use crate::protocol::login::cb::{Disconnect, LoginPluginRequest};
use crate::protocol::login::sb::{LoginPluginResponse, LoginStart};
use crate::protocol::login::{IdentifiedKey, MojangIdentifiedKey, VerifyError};
use crate::protocol::GameProfile;
use crate::registry::{AsyncPacketRegistry, RegistryError};
use drax::transport::TransportProcessorContext;
use drax::VarInt;
use std::fmt::{Debug, Display, Formatter};
use std::io::Cursor;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use uuid::Uuid;

const MODERN_FORWARDING_COMPATIBILITY: u8 = 3;
type HmacSha256 = hmac::Hmac<sha2::Sha256>;

#[derive(drax_derive::DraxTransport, Debug)]
#[drax(key = {match VarInt})]
pub enum VelocityForwardingData {
    __Illegal,
    Default {
        address: String,
        profile: GameProfile,
    },
    ForwardingWithKey {
        address: String,
        profile: GameProfile,
        key: MojangIdentifiedKey,
    },
    ForwardingWithLinkedKey {
        address: String,
        profile: GameProfile,
        key: MojangIdentifiedKey,
        holder: Uuid,
    },
}

enum AuthFunctionResponse {
    LoginStartSuccess {
        key: Option<IdentifiedKey>,
        mojang_key: Option<MojangIdentifiedKey>,
    },
    PluginMessageSuccess {
        address: String,
        profile: GameProfile,
        mojang_key: Option<MojangIdentifiedKey>,
    },
}

pub enum AuthClientState {
    ExpectingLoginStart,
    ExpectingModernForwarding,
}

pub struct AuthClientContext<W: AsyncWrite + Send + Sync + Unpin + Sized> {
    state: AuthClientState,
    secret_key: String,
    auth_config: Arc<AuthConfiguration>,
    writer: MinecraftProtocolWriter<W>,
}

pub struct VelocityAuthErrorWithWriter<W: Send + Sync> {
    writer: MinecraftProtocolWriter<W>,
    error: VelocityAuthError,
}

impl<W: AsyncWrite + Send + Sync + Unpin + Sized> VelocityAuthErrorWithWriter<W> {
    pub async fn disconnect_client_for_error(&mut self) -> Result<(), drax::transport::Error> {
        let mut error_message = Chat::text("");
        let mut prompt = Chat::text("Failed to login: ");
        prompt.modify_style(|style| style.color("#FF0000"));
        error_message.push_extra(prompt);
        error_message.push_extra(Chat::literal(format!("{}", self.error)));
        self.writer
            .write_packet(Disconnect {
                reason: error_message,
            })
            .await
    }
}

impl<W: Send + Sync> Debug for VelocityAuthErrorWithWriter<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.error)
    }
}

impl<W: Send + Sync> Display for VelocityAuthErrorWithWriter<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl<W: Send + Sync> std::error::Error for VelocityAuthErrorWithWriter<W> {}

#[derive(Debug)]
pub enum VelocityAuthError {
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

impl VelocityAuthError {
    pub fn with_ctx<W: Send + Sync>(
        self,
        ctx: MinecraftProtocolWriter<W>,
    ) -> VelocityAuthErrorWithWriter<W> {
        VelocityAuthErrorWithWriter {
            writer: ctx,
            error: self,
        }
    }
}

impl Display for VelocityAuthError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VelocityAuthError::InvalidState => write!(f, "Invalid packet sent during login."),
            VelocityAuthError::KeyExpired => write!(f, "Player key expired."),
            VelocityAuthError::KeyError(err) => write!(f, "Player key invalid. {}", err),
            VelocityAuthError::TransportError(err) => write!(f, "Transport error found! {}", err),
            VelocityAuthError::RegistryError(err) => write!(f, "Registry error found! {}", err),
            VelocityAuthError::KeyNotFound => write!(f, "Player key not found, but expected."),
            VelocityAuthError::KeyHolderNotFound => write!(f, "Player key holder not found."),
            VelocityAuthError::MacError(err) => write!(f, "Mac error encountered: {}", err),
            VelocityAuthError::DataMismatch(what) => {
                write!(f, "Data for `{}` did not match.", what)
            }
        }
    }
}

impl std::error::Error for VelocityAuthError {}

impl From<CapturedRsaError> for VelocityAuthError {
    fn from(value: CapturedRsaError) -> Self {
        Self::KeyError(value)
    }
}

impl From<drax::transport::Error> for VelocityAuthError {
    fn from(value: drax::transport::Error) -> Self {
        Self::TransportError(value)
    }
}

impl From<RegistryError> for VelocityAuthError {
    fn from(value: RegistryError) -> Self {
        Self::RegistryError(value)
    }
}

impl From<VerifyError> for VelocityAuthError {
    fn from(value: VerifyError) -> Self {
        match value {
            VerifyError::CapturedRsa(err) => Self::from(err),
            VerifyError::DraxTransport(err) => Self::from(err),
        }
    }
}

impl From<sha2::digest::MacError> for VelocityAuthError {
    fn from(value: sha2::digest::MacError) -> Self {
        Self::MacError(value)
    }
}

async fn handle_login_start<W: AsyncWrite + Send + Sync + Unpin + Sized>(
    ctx: &mut AuthClientContext<W>,
    login_start: LoginStart,
) -> Result<AuthFunctionResponse, VelocityAuthError> {
    if let AuthClientState::ExpectingModernForwarding = ctx.state {
        return Err(VelocityAuthError::InvalidState);
    }

    let key = match (
        login_start.sig_data.as_ref(),
        login_start.sig_holder.as_ref(),
    ) {
        (Some(sig_data), Some(sig_holder)) => {
            if sig_data.has_expired() {
                return Err(VelocityAuthError::KeyExpired);
            }
            let mojang_der = crate::crypto::key_from_der(super::MOJANG_KEY)?;
            sig_data.verify_incoming_data(&mojang_der, sig_holder)?;

            Some(IdentifiedKey::new(&sig_data.public_key)?)
        }
        (Some(_), None) => {
            return Err(VelocityAuthError::KeyHolderNotFound);
        }
        (None, None) | (None, Some(_)) => {
            if ctx.auth_config.force_key_authentication {
                return Err(VelocityAuthError::KeyNotFound);
            }
            None
        }
    };

    ctx.writer
        .write_packet(LoginPluginRequest {
            message_id: -1,
            channel: "velocity:player_info".to_string(),
            data: vec![MODERN_FORWARDING_COMPATIBILITY],
        })
        .await?;
    ctx.state = AuthClientState::ExpectingModernForwarding;

    Ok(AuthFunctionResponse::LoginStartSuccess {
        key,
        mojang_key: login_start.sig_data,
    })
}

async fn handle_plugin_response<W: AsyncWrite + Send + Sync + Unpin + Sized>(
    ctx: &mut AuthClientContext<W>,
    login_plugin_response: LoginPluginResponse,
) -> Result<AuthFunctionResponse, VelocityAuthError> {
    if let AuthClientState::ExpectingLoginStart = ctx.state {
        return Err(VelocityAuthError::InvalidState);
    }

    if !login_plugin_response.successful || login_plugin_response.data.len() < 33 {
        return Err(VelocityAuthError::InvalidState);
    }

    use sha2::digest::Mac;
    let mut hmac: HmacSha256 =
        HmacSha256::new_from_slice(ctx.secret_key.as_bytes()).expect("Hmac keys are any length.");
    let hmac_sig = &login_plugin_response.data[0..32];
    let remaining_data = &login_plugin_response.data[32..];
    hmac.update(remaining_data);
    let mut data_cursor = Cursor::new(remaining_data);

    let data = <VelocityForwardingData as drax::transport::DraxTransport>::read_from_transport(
        &mut TransportProcessorContext::new(),
        &mut data_cursor,
    )?;

    log::info!("Got true data: {:?}", data);
    log::info!(
        "Verifying {:?} against secret {:?}",
        hmac_sig,
        ctx.secret_key.as_bytes()
    );
    log::info!(
        "Got actual slice: {:?}",
        hmac.clone().finalize().into_bytes()
    );
    hmac.verify_slice(hmac_sig)?;

    let (address, profile, mojang_key) = match data {
        VelocityForwardingData::__Illegal => return Err(VelocityAuthError::InvalidState),
        VelocityForwardingData::Default { address, profile } => (address, profile, None),
        VelocityForwardingData::ForwardingWithKey {
            address,
            profile,
            key,
        }
        | VelocityForwardingData::ForwardingWithLinkedKey {
            address,
            profile,
            key,
            ..
        } => (address, profile, Some(key)),
    };

    Ok(AuthFunctionResponse::PluginMessageSuccess {
        address,
        profile,
        mojang_key,
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
    secret_key: String,
) -> Result<super::AuthenticatedClient<R, W>, VelocityAuthErrorWithWriter<W>> {
    let mut auth_pipeline = auth_pipeline.rewrite_registry(handshake.protocol_version);
    auth_pipeline.clear_data();
    auth_pipeline.register(pin_fut!(handle_login_start::<W>));
    auth_pipeline.register(pin_fut!(handle_plugin_response::<W>));

    let mut context = AuthClientContext::<W> {
        state: AuthClientState::ExpectingLoginStart,
        secret_key,
        auth_config,
        writer: MinecraftProtocolWriter::from_handshake(write, &handshake),
    };

    let (key, mojang_key_init) = match auth_pipeline.execute_next_packet(&mut context).await {
        Ok(Ok(AuthFunctionResponse::LoginStartSuccess { key, mojang_key })) => (key, mojang_key),
        Ok(Ok(AuthFunctionResponse::PluginMessageSuccess { .. })) => {
            return Err(VelocityAuthError::InvalidState.with_ctx(context.writer))
        }
        Err(err) => {
            return Err(VelocityAuthErrorWithWriter {
                writer: context.writer,
                error: VelocityAuthError::RegistryError(err),
            });
        }
        Ok(Err(err)) => {
            return Err(VelocityAuthErrorWithWriter {
                writer: context.writer,
                error: err,
            });
        }
    };

    let (address, profile, mojang_key) = match auth_pipeline.execute_next_packet(&mut context).await
    {
        Ok(Ok(AuthFunctionResponse::LoginStartSuccess { .. })) => {
            return Err(VelocityAuthError::InvalidState.with_ctx(context.writer))
        }
        Ok(Ok(AuthFunctionResponse::PluginMessageSuccess {
            address,
            profile,
            mojang_key,
        })) => (address, profile, mojang_key),
        Err(err) => {
            return Err(VelocityAuthErrorWithWriter {
                writer: context.writer,
                error: VelocityAuthError::RegistryError(err),
            });
        }
        Ok(Err(err)) => {
            return Err(VelocityAuthErrorWithWriter {
                writer: context.writer,
                error: err,
            });
        }
    };

    if mojang_key_init.ne(&mojang_key) {
        return Err(
            VelocityAuthError::DataMismatch("mojang_key".to_string()).with_ctx(context.writer)
        );
    }

    Ok(super::AuthenticatedClient {
        read_write: (
            auth_pipeline.noop_decryption().clear_registry(),
            context.writer.noop_encryption(),
        ),
        profile,
        key,
        mojang_key,
        overridden_address: Some(address),
    })
}

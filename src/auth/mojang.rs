use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

use cipher::NewCipher;
use drax::transport::encryption::EncryptionStream;
use drax::VarInt;
use num_bigint::BigInt;
use rand::RngCore;
use reqwest::StatusCode;
use tokio::io::{AsyncRead, AsyncWrite};
use uuid::Uuid;

use crate::auth::AuthConfiguration;
use crate::crypto::{private_key_to_der, CapturedRsaError};
use crate::pin_fut;
use crate::pipeline::{AsyncMinecraftProtocolPipeline, MinecraftProtocolWriter};
use crate::protocol::handshaking::sb::Handshake;
use crate::protocol::login::cb::{EncryptionRequest, SetCompression};
use crate::protocol::login::sb::{EncryptionResponse, EncryptionResponseData, LoginStart};
use crate::protocol::login::{IdentifiedKey, MojangIdentifiedKey, VerifyError};
use crate::protocol::GameProfile;
use crate::registry::{MappedAsyncPacketRegistry, MutAsyncPacketRegistry, RegistryError};

use super::MOJANG_KEY;

pub enum AuthClientState {
    ExpectingLoginStart,
    ExpectingEncryptionResponse {
        verify_bytes: Vec<u8>,
        login_username: String,
    },
}

pub struct AuthClientContext<W: AsyncWrite + Send + Sync + Unpin + Sized> {
    state: AuthClientState,
    key: Option<IdentifiedKey>,
    auth_config: Arc<AuthConfiguration>,
    writer: MinecraftProtocolWriter<W>,
}

#[derive(Debug)]
pub enum KeyError {
    Expired,
    NoHolder,
    NoKey,
    MojangKeyServerError,
    InvalidKey(CapturedRsaError),
    InvalidIdentifiedKey(CapturedRsaError),
}

#[derive(Debug)]
pub enum ValidationError {
    VerifyMismatch,
    DataSignatureInvalid,
    InvalidData,
    MojangAuthError,
    MojangAuthFailure,
    InvalidSharedSecret,
}

pub enum AuthError<W: AsyncWrite + Unpin + Sized + Send + Sync> {
    InvalidState(MinecraftProtocolWriter<W>),
    KeyError(MinecraftProtocolWriter<W>, KeyError),
    ValidationError(MinecraftProtocolWriter<W>, ValidationError),
    TransportError(MinecraftProtocolWriter<W>, drax::transport::Error),
    RegistryError(MinecraftProtocolWriter<W>, RegistryError),
}

impl<W: AsyncWrite + Unpin + Sized + Send + Sync> Display for AuthError<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidState(_) => write!(f, "Invalid authentication state during login."),
            AuthError::KeyError(_, key_err) => match key_err {
                KeyError::Expired => write!(f, "Player key expired."),
                KeyError::NoHolder => write!(f, "No holder for player key found."),
                KeyError::NoKey => write!(f, "No key when expected."),
                KeyError::MojangKeyServerError => write!(f, "Mojang key failed to parse."),
                KeyError::InvalidKey(err) => write!(f, "Player key invalid.\n{}", err),
                KeyError::InvalidIdentifiedKey(err) => {
                    write!(f, "Player key improperly formatted.\n{}", err)
                }
            },
            AuthError::ValidationError(_, validation_err) => match validation_err {
                ValidationError::VerifyMismatch => write!(f, "Encryption verification mismatch."),
                ValidationError::DataSignatureInvalid => {
                    write!(f, "Encryption data signature is invalid.")
                }
                ValidationError::InvalidData => write!(f, "Input encryption data was invalid."),
                ValidationError::MojangAuthError => {
                    write!(f, "Failed to authenticate with mojang.")
                }
                ValidationError::MojangAuthFailure => {
                    write!(f, "Mojang failed to respond to authentication.")
                }
                ValidationError::InvalidSharedSecret => {
                    write!(
                        f,
                        "Invalid shared secret, could not construct encryption stream."
                    )
                }
            },
            AuthError::TransportError(_, err) => {
                write!(f, "Generic transport error.\n{}", err)
            }
            AuthError::RegistryError(_, registry_error) => match registry_error {
                RegistryError::NoHandlerFound((protocol_version, packet_id), _) => {
                    write!(
                        f,
                        "No packet handler found for proto: {}, packet: {}",
                        protocol_version, packet_id
                    )
                }
                RegistryError::DraxTransportError(err) => {
                    write!(f, "Generic transport error.\n{}", err)
                }
            },
        }
    }
}

impl<W: AsyncWrite + Unpin + Sized + Send + Sync> Debug for AuthError<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl<W: AsyncWrite + Unpin + Sized + Send + Sync> std::error::Error for AuthError<W> {}

pub enum AuthFunctionResponse {
    InvalidState,
    KeyError(KeyError),
    ValidationError(ValidationError),
    TransportError(drax::transport::Error),
    LoginStartPass {
        mojang_key: Option<MojangIdentifiedKey>,
        key: Option<IdentifiedKey>,
        name: String,
        sig_holder: Option<Uuid>,
    },
    AuthComplete {
        profile: GameProfile,
        shared_secret: Vec<u8>,
    },
}

async fn login_start<W: AsyncWrite + Send + Sync + Unpin + Sized>(
    context: &mut AuthClientContext<W>,
    login_start: LoginStart,
) -> AuthFunctionResponse {
    log::trace!("Got login start: {:?}", login_start);

    if !matches!(context.state, AuthClientState::ExpectingLoginStart) {
        return AuthFunctionResponse::InvalidState;
    }

    let key = match (
        login_start.sig_data.as_ref(),
        login_start.sig_holder.as_ref(),
    ) {
        (Some(sig_data), Some(sig_holder)) => {
            if sig_data.has_expired() {
                return AuthFunctionResponse::KeyError(KeyError::Expired);
            }

            let mojang_der = match crate::crypto::key_from_der(MOJANG_KEY) {
                Ok(key) => key,
                Err(_) => return AuthFunctionResponse::KeyError(KeyError::MojangKeyServerError),
            };

            match sig_data.verify_incoming_data(&mojang_der, sig_holder) {
                Err(VerifyError::CapturedRsa(err)) => {
                    return AuthFunctionResponse::KeyError(KeyError::InvalidKey(err))
                }
                Err(VerifyError::DraxTransport(err)) => {
                    return AuthFunctionResponse::TransportError(err)
                }
                _ => (),
            }

            match IdentifiedKey::new(&sig_data.public_key) {
                Ok(identified_key) => Some(identified_key),
                Err(err) => {
                    return AuthFunctionResponse::KeyError(KeyError::InvalidIdentifiedKey(err))
                }
            }
        }
        (Some(_), None) => {
            return AuthFunctionResponse::KeyError(KeyError::NoHolder);
        }
        (None, None) | (None, Some(_)) => {
            if context.auth_config.force_key_authentication {
                return AuthFunctionResponse::KeyError(KeyError::NoKey);
            }
            None
        }
    };

    let key_der = private_key_to_der(&context.auth_config.server_private_key);
    let mut verify_token = [0, 0, 0, 0];
    rand::thread_rng().fill_bytes(&mut verify_token);

    let encryption_request = EncryptionRequest {
        server_id: format!(""),
        public_key: Some(key_der),
        verify_token: Some(Vec::from(verify_token)),
        legacy_public_key: None,
        legacy_verify_token: None,
    };

    if let Err(err) = context.writer.write_packet(&encryption_request).await {
        return AuthFunctionResponse::TransportError(err);
    }

    context.state = AuthClientState::ExpectingEncryptionResponse {
        verify_bytes: Vec::from(verify_token),
        login_username: login_start.name.clone(),
    };

    AuthFunctionResponse::LoginStartPass {
        mojang_key: login_start.sig_data,
        key,
        name: login_start.name.clone(),
        sig_holder: login_start.sig_holder,
    }
}

async fn encryption_response<W: AsyncWrite + Send + Sync + Unpin + Sized>(
    context: &mut AuthClientContext<W>,
    encryption_response: EncryptionResponse,
) -> AuthFunctionResponse {
    let (expected_verify, username) = match &context.state {
        AuthClientState::ExpectingEncryptionResponse {
            verify_bytes,
            login_username,
        } => (verify_bytes, login_username),
        _ => return AuthFunctionResponse::InvalidState,
    };

    let server_key = &context.auth_config.server_private_key;

    if let Some(key) = context.key.as_ref() {
        match encryption_response.response_data {
            Some(EncryptionResponseData::VerifyTokenData(_)) => {
                return AuthFunctionResponse::ValidationError(ValidationError::InvalidData);
            }
            Some(EncryptionResponseData::MessageSignature {
                salt,
                message_signature,
            }) => {
                use sha2::Digest;
                let message = expected_verify.clone();

                let mut hasher = sha2::Sha256::new();
                hasher.update(&message);
                hasher.update(&{
                    let mut value = salt;
                    let mut result = [0u8; 8];
                    for i in 0..8 {
                        result[7 - i] = (value & 255) as u8;
                        value >>= 8;
                    }
                    result
                });
                let hasher = hasher.finalize();

                if let Err(_) = key.verify_data_signature(&message_signature, &hasher) {
                    return AuthFunctionResponse::ValidationError(
                        ValidationError::DataSignatureInvalid,
                    );
                }
            }
            _ => {}
        }
    } else {
        match encryption_response.response_data {
            Some(EncryptionResponseData::VerifyTokenData(data)) => {
                let resp = match server_key.decrypt(crate::crypto::Padding::PKCS1v15Encrypt, &data)
                {
                    Ok(resp) => resp,
                    Err(_) => {
                        return AuthFunctionResponse::ValidationError(ValidationError::InvalidData)
                    }
                };
                if expected_verify.ne(&resp) {
                    return AuthFunctionResponse::ValidationError(ValidationError::VerifyMismatch);
                }
            }
            Some(EncryptionResponseData::MessageSignature { .. }) => {
                return AuthFunctionResponse::ValidationError(ValidationError::InvalidData);
            }
            _ => {}
        }
    }

    let shared_secret = match server_key.decrypt(
        crate::crypto::Padding::PKCS1v15Encrypt,
        &encryption_response.shared_secret,
    ) {
        Ok(shared_secret) => shared_secret,
        Err(_) => return AuthFunctionResponse::ValidationError(ValidationError::InvalidData),
    };

    #[inline]
    fn hash_server_id(server_id: &str, shared_secret: &[u8], public_key: &[u8]) -> String {
        use md5::Digest;
        let mut hasher = sha1::Sha1::new();
        hasher.update(server_id);
        hasher.update(shared_secret);
        hasher.update(public_key);
        let bytes = hasher.finalize();
        let bigint = BigInt::from_signed_bytes_be(bytes.as_slice());
        format!("{:x}", bigint)
    }

    let hashed_server_id = hash_server_id("", &shared_secret, &private_key_to_der(server_key));

    #[inline]
    fn def_auth_server() -> String {
        "https://sessionserver.mojang.com".to_string()
    }

    let auth_url = context
        .auth_config
        .auth_url
        .as_ref()
        .cloned()
        .unwrap_or_else(def_auth_server);

    let url = format!(
        "{}/session/minecraft/hasJoined?username={}&serverId={}",
        auth_url, username, hashed_server_id
    );

    let response = match reqwest::get(url).await {
        Ok(response) => response,
        Err(_) => return AuthFunctionResponse::ValidationError(ValidationError::MojangAuthError),
    };

    if response.status() == StatusCode::from_u16(204).expect("204 is a valid code") {
        return AuthFunctionResponse::ValidationError(ValidationError::MojangAuthFailure);
    } else if response.status() != StatusCode::from_u16(200).expect("200 is a valid code") {
        return AuthFunctionResponse::ValidationError(ValidationError::MojangAuthError);
    }

    let profile = match response.json::<GameProfile>().await {
        Ok(profile) => profile,
        Err(_) => return AuthFunctionResponse::ValidationError(ValidationError::MojangAuthError),
    };

    AuthFunctionResponse::AuthComplete {
        profile,
        shared_secret,
    }
}

pub async fn auth_client<
    IC: Send + Sync,
    IO: Send + Sync,
    R: AsyncRead + Unpin + Sized + Send + Sync,
    W: AsyncWrite + Unpin + Sized + Send + Sync,
    Reg: MutAsyncPacketRegistry<IC, IO> + Send + Sync,
>(
    auth_pipeline: AsyncMinecraftProtocolPipeline<R, IC, IO, Reg>,
    write: W,
    handshake: Handshake,
    auth_config: Arc<AuthConfiguration>,
) -> Result<super::AuthenticatedClient<R, W>, AuthError<W>> {
    let mut auth_pipeline = auth_pipeline.rewrite_registry(handshake.protocol_version);
    auth_pipeline.clear_data();
    auth_pipeline.register(pin_fut!(login_start));
    auth_pipeline.register(pin_fut!(encryption_response));

    let mut context = AuthClientContext::<W> {
        state: AuthClientState::ExpectingLoginStart,
        key: None,
        auth_config: auth_config.clone(),
        writer: MinecraftProtocolWriter::from_handshake(write, &handshake),
    };

    let matched = match auth_pipeline.execute_next_packet(&mut context).await {
        Ok(matched) => matched,
        Err(err) => return Err(AuthError::RegistryError(context.writer, err)),
    };

    let (mojang_key, sig_holder) = match matched {
        AuthFunctionResponse::LoginStartPass {
            mojang_key,
            key,
            sig_holder,
            ..
        } => {
            context.key = key;
            (mojang_key, sig_holder)
        }
        AuthFunctionResponse::ValidationError(err) => {
            return Err(AuthError::ValidationError(context.writer, err))
        }
        AuthFunctionResponse::TransportError(err) => {
            return Err(AuthError::TransportError(context.writer, err))
        }
        AuthFunctionResponse::KeyError(err) => {
            return Err(AuthError::KeyError(context.writer, err))
        }
        AuthFunctionResponse::InvalidState => return Err(AuthError::InvalidState(context.writer)),
        _ => return Err(AuthError::InvalidState(context.writer)),
    };

    let matched = match auth_pipeline.execute_next_packet(&mut context).await {
        Ok(matched) => matched,
        Err(err) => return Err(AuthError::RegistryError(context.writer, err)),
    };

    let (mut new_read, mut new_write, profile, key) = match matched {
        AuthFunctionResponse::AuthComplete {
            profile,
            shared_secret,
        } => {
            if let Some(expected_uuid) = sig_holder.as_ref() {
                if profile.id.ne(expected_uuid) {
                    return Err(AuthError::ValidationError(
                        context.writer,
                        ValidationError::InvalidData,
                    ));
                }
            }

            macro_rules! stream {
                ($shared_secret:ident, $packet_writer:expr) => {
                    match EncryptionStream::new_from_slices(&$shared_secret, &$shared_secret) {
                        Ok(stream) => stream,
                        Err(_) => {
                            return Err(AuthError::ValidationError(
                                $packet_writer,
                                ValidationError::InvalidSharedSecret,
                            ))
                        }
                    }
                };
            }

            let read_stream = stream!(shared_secret, context.writer);
            let write_stream = stream!(shared_secret, context.writer);

            let new_read = auth_pipeline.enable_decryption(read_stream);

            let AuthClientContext { key, writer, .. } = context;

            let new_write = writer.enable_encryption(write_stream);
            (new_read, new_write, profile, key)
        }
        AuthFunctionResponse::ValidationError(err) => {
            return Err(AuthError::ValidationError(context.writer, err))
        }
        AuthFunctionResponse::TransportError(err) => {
            return Err(AuthError::TransportError(context.writer, err))
        }
        AuthFunctionResponse::KeyError(err) => {
            return Err(AuthError::KeyError(context.writer, err))
        }
        AuthFunctionResponse::InvalidState => return Err(AuthError::InvalidState(context.writer)),
        _ => return Err(AuthError::InvalidState(context.writer)),
    };

    if auth_config.compression_threshold >= 0 {
        if let Err(err) = new_write
            .write_packet(&SetCompression {
                threshold: auth_config.compression_threshold as VarInt,
            })
            .await
        {
            log::warn!("Failed to enable compression {}.", err);
        } else {
            new_read.enable_compression(auth_config.compression_threshold);
            new_write.enable_compression(auth_config.compression_threshold);
        }
    };

    Ok(super::AuthenticatedClient {
        read_write: (
            new_read.with_registry(MappedAsyncPacketRegistry::new(handshake.protocol_version)),
            new_write,
        ),
        profile,
        key,
        mojang_key,
        overridden_address: None,
        sig_holder,
    })
}

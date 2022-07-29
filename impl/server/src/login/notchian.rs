use crate::client_connection::Connection;
use encryption_utils::{private_key_to_der, MCPrivateKey};
use mc_registry::client_bound::login::{
    Disconnect, EncryptionRequest, LoginSuccess, ServerId, SetCompression,
};
use mc_registry::mappings::Mappings;
use mc_registry::registry::{arc_lock, LockedContext, StateRegistry, StateRegistryHandle};
use mc_registry::server_bound::login::{EncryptionResponse, EncryptionResponseData, LoginStart};
use mc_registry::shared_types::login::{IdentifiedKey, LoginUsername};
use mc_serializer::primitive::{Chat, VarInt};
use rand::RngCore;
use std::sync::Arc;

use mc_registry::shared_types::GameProfile;
use num_bigint::BigInt;
use reqwest::StatusCode;
use serde_json::json;

const MOJANG_KEY: &[u8] = include_bytes!("yggdrasil_session_pubkey.der");

struct LoginContext {
    game_profile: Option<GameProfile>,
    username: Option<LoginUsername>,
    server_key: Arc<MCPrivateKey>,
    config: NotchianLoginConfig,
    player_key: Option<IdentifiedKey>,
    player_verify: Option<Vec<u8>>,
    shared_secret: Option<Vec<u8>>,
}

async fn disconnect<S: Into<String>>(connection: &mut Connection, reason: S) -> anyhow::Result<()> {
    let into = reason.into();
    connection
        .send_packet(Disconnect {
            reason: Chat::from(
                json! {
                    {
                        "text": into
                    }
                }
                .to_string(),
            ),
        })
        .await?;
    Ok(())
}

#[mc_registry_derive::packet_handler]
fn login_start_handler(context: LockedContext<LoginContext>, packet: LoginStart) {
    let mut context_write = context.write().await;
    context_write.username = Some(packet.name.clone());

    if packet.sig_data.0 {
        let signature = packet
            .sig_data
            .1
            .expect("Signature data expected but not found.");
        if signature.has_expired() {
            anyhow::bail!("Player key was found but expired.");
        }

        signature.verify_signature(&encryption_utils::key_from_der(MOJANG_KEY)?)?;

        context_write.player_key = Some(IdentifiedKey::new(&signature.public_key.1)?);
    } else if context_write.config.force_key_authentication {
        anyhow::bail!("Player key was expected but not found.");
    }
}

fn hash_server_id(server_id: &str, shared_secret: &[u8], public_key: &[u8]) -> String {
    use md5::Digest;
    let mut hasher = sha1::Sha1::new();
    hasher.update(server_id);
    hasher.update(shared_secret);
    hasher.update(public_key);
    digest(hasher.finalize().as_slice())
}

fn digest(bytes: &[u8]) -> String {
    let bigint = BigInt::from_signed_bytes_be(bytes);
    format!("{:x}", bigint)
}

#[mc_registry_derive::packet_handler]
fn encryption_response_handler(context: LockedContext<LoginContext>, packet: EncryptionResponse) {
    let mut context_write = context.write().await;
    let verify = context_write
        .player_verify
        .as_ref()
        .expect("Verify tokens not found but were expected.");
    let player_key = &context_write.player_key;
    let server_key = &context_write.server_key;
    if let Some(player_key) = player_key {
        match packet.response_data {
            EncryptionResponseData::VerifyTokenData(_) => {
                anyhow::bail!("Salt not found but expected.")
            }
            EncryptionResponseData::MessageSignature {
                salt,
                message_signature: (_, signature),
            } => {
                use sha2::Digest;
                let message = verify.clone();

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
                let message = hasher.as_slice();

                player_key.verify_data_signature(&signature, message)?;
            }
        }
    } else {
        match packet.response_data {
            EncryptionResponseData::VerifyTokenData((_, data)) => {
                let response =
                    server_key.decrypt(encryption_utils::Padding::PKCS1v15Encrypt, &data)?;
                if verify.ne(&response) {
                    anyhow::bail!("Verification mismatch.");
                }
            }
            EncryptionResponseData::MessageSignature { .. } => {
                anyhow::bail!("Salt found while player key is not.")
            }
        }
    }

    let shared_secret = server_key.decrypt(
        encryption_utils::Padding::PKCS1v15Encrypt,
        &packet.shared_secret.1,
    )?;

    let generated_server_id = hash_server_id("", &shared_secret, &private_key_to_der(server_key));

    // optimize later
    let session_server: String = if let Ok(var) = std::env::var("NITROGEN_SESSION_SERVER") {
        var
    } else {
        String::from("https://sessionserver.mojang.com/session/minecraft/hasJoined")
    };
    let url = format!(
        "{}?username={}&serverId={}",
        session_server,
        context_write.username.as_ref().unwrap(),
        generated_server_id
    );

    let response = reqwest::get(url).await?;
    if response.status() == StatusCode::from_u16(204)? {
        anyhow::bail!("Failed to authenticate with mojang.");
    } else if response.status() != StatusCode::from_u16(200)? {
        anyhow::bail!(
            "Received a {} status code from mojang auth server.",
            response.status().as_u16()
        );
    }

    let game_profile = response.json::<GameProfile>().await?;
    context_write.game_profile = Some(game_profile);
    context_write.shared_secret = Some(shared_secret);
}

async fn handle_packet_failure(
    connection: &mut Connection,
    packet_handler: anyhow::Result<()>,
) -> anyhow::Result<bool> {
    Ok(if let Err(error) = packet_handler {
        disconnect(
            connection,
            format!("Failure during loging sequence. {:?}", error),
        )
        .await?;
        true
    } else {
        false
    })
}

#[derive(Copy, Clone)]
pub struct NotchianLoginConfig {
    pub force_key_authentication: bool,
    pub compression_threshold: VarInt,
}

pub async fn wrapped_handle_login(
    connection: &mut Connection,
    config: NotchianLoginConfig,
) -> Option<(GameProfile, Option<IdentifiedKey>)> {
    match handle_login(connection, config).await {
        Ok(Some((profile, key))) => Some((profile, key)),
        Ok(None) => None,
        Err(_) => {
            disconnect(connection, "Server error.").await.unwrap_or(());
            None
        }
    }
}

pub async fn handle_login(
    connection: &mut Connection,
    config: NotchianLoginConfig,
) -> anyhow::Result<Option<(GameProfile, Option<IdentifiedKey>)>> {
    let mut registry = StateRegistry::<LoginContext>::fail_on_invalid(
        connection.connection_into().protocol_version(),
    );

    LoginStart::attach_to_register(
        &mut registry,
        login_start_handler as StateRegistryHandle<LoginContext>,
    );

    let locked_registry = arc_lock(registry);

    let next_packet = connection.read_packet().await?;

    let context = arc_lock(LoginContext {
        game_profile: None,
        username: None,
        server_key: Arc::clone(&connection.server_key()),
        config,
        player_key: None,
        player_verify: None,
        shared_secret: None,
    });
    let emit_result = StateRegistry::emit(
        Arc::clone(&locked_registry),
        Arc::clone(&context),
        next_packet,
    )
    .await;
    if handle_packet_failure(connection, emit_result).await? {
        return Ok(None);
    }

    let mut context_write = context.write().await;

    let key_der = private_key_to_der(&connection.server_key());
    let mut verify_token = [0, 0, 0, 0];
    rand::thread_rng().fill_bytes(&mut verify_token);
    context_write.player_verify = Some(Vec::from(verify_token));
    drop(context_write);

    let encryption_request = EncryptionRequest {
        server_id: ServerId::from(""),
        public_key: (VarInt::try_from(key_der.len())?, key_der),
        verify_token: (VarInt::from(4), Vec::from(verify_token)),
    };
    connection.send_packet(encryption_request).await?;

    let mut registry_write = locked_registry.write().await;
    registry_write.clear_mappings();
    EncryptionResponse::attach_to_register(
        &mut registry_write,
        encryption_response_handler as StateRegistryHandle<LoginContext>,
    );
    drop(registry_write);

    let next_packet = connection.read_packet().await?;

    let emit_result = StateRegistry::emit(
        Arc::clone(&locked_registry),
        Arc::clone(&context),
        next_packet,
    )
    .await;

    if let Err(err) = emit_result {
        let context_read = context.read().await;

        if let Some(secret) = context_read.shared_secret.as_ref() {
            let encryption_split = mc_buffer::encryption::Codec::new(secret)?;
            connection.enable_crypt(encryption_split);
        }

        disconnect(connection, format!("Error in connection: {:?}", err)).await?;
        return Ok(None);
    }

    let context_read = context.read().await;

    let secret = context_read.shared_secret.as_ref().unwrap();
    let encryption_split = mc_buffer::encryption::Codec::new(secret)?;

    connection.enable_crypt(encryption_split);

    if context_read.config.compression_threshold > 0 {
        connection
            .send_packet(SetCompression {
                threshold: context_read.config.compression_threshold,
            })
            .await?;
    }

    let game_profile = context_read
        .game_profile
        .as_ref()
        .map(Clone::clone)
        .unwrap();

    let login_success = LoginSuccess {
        uuid: game_profile.id,
        username: LoginUsername::from(game_profile.name.to_string()),
        properties: (
            VarInt::try_from(game_profile.properties.len())?,
            game_profile.properties.iter().map(|x| x.into()).collect(),
        ),
    };

    let player_key = context_read.player_key.as_ref().cloned();

    connection.send_packet(login_success).await?;

    Ok(Some((game_profile, player_key)))
}

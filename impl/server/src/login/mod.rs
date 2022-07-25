use mc_registry::shared_types::GameProfile;
use mc_registry::shared_types::login::{IdentifiedKey, LoginUsername, MCIdentifiedKey};
use mc_serializer::serde::ProtocolVersion;

use crate::client_connection::Connection;
use crate::login::notchian::NotchianLoginConfig;

pub mod notchian;

pub struct AuthenticatedPlayerConnection {
    pub profile: GameProfile,
    connection: Connection,
    player_key: Option<IdentifiedKey>,
}

impl AuthenticatedPlayerConnection {
    pub fn verify_player_signature(&self, message: &[&[u8]], signature: &[u8]) -> anyhow::Result<()> {
        match &self.player_key {
            None => anyhow::bail!("Attempted to send signature without player key."),
            Some(key) => {
                use sha2::Digest;
                let mut sha = sha2::Sha256::new();
                for message_part in message {
                    sha.update(message_part);
                }
                let hasher = sha.finalize();
                let bytes = hasher.as_slice();
                key.verify_data_signature(signature, bytes)?;
                Ok(())
            }
        }
    }
}

pub async fn notchian_login(config: NotchianLoginConfig, mut connection: Connection) -> Option<AuthenticatedPlayerConnection> {
    notchian::wrapped_handle_login(&mut connection, config).await.map(move |(profile, player_key)| {
        AuthenticatedPlayerConnection {
            profile,
            connection,
            player_key,
        }
    })
}

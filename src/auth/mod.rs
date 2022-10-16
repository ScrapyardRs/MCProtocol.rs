use crate::crypto::MCPrivateKey;
use crate::pipeline::BlankMcReadWrite;
use crate::protocol::login::{IdentifiedKey, MojangIdentifiedKey};
use crate::protocol::GameProfile;
use drax::transport::encryption::{DecryptRead, EncryptedWriter};
use tokio::io::{AsyncRead, AsyncWrite};
use uuid::Uuid;

pub mod bungee;
pub mod mojang;
pub mod velocity;

const MOJANG_KEY: &[u8] = include_bytes!("yggdrasil_session_pubkey.der");

pub struct AuthConfiguration {
    pub server_private_key: MCPrivateKey,
    pub compression_threshold: isize,
    pub force_key_authentication: bool,
    pub auth_url: Option<String>,
}

pub struct AuthenticatedClient<
    R: AsyncRead + Send + Sync + Unpin + Sized,
    W: AsyncWrite + Send + Sync + Unpin + Sized,
> {
    pub read_write: BlankMcReadWrite<DecryptRead<R>, EncryptedWriter<W>>,
    pub profile: GameProfile,
    pub key: Option<IdentifiedKey>,
    pub sig_holder: Option<Uuid>,
    pub mojang_key: Option<MojangIdentifiedKey>,
    pub overridden_address: Option<String>,
}

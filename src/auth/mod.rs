use crate::crypto::MCPrivateKey;

pub mod mojang;

const MOJANG_KEY: &[u8] = include_bytes!("yggdrasil_session_pubkey.der");

pub struct AuthConfiguration {
    pub server_private_key: MCPrivateKey,
    pub compression_threshold: isize,
    pub force_key_authentication: bool,
    pub auth_url: Option<String>,
}

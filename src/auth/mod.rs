use crate::crypto::MCPrivateKey;

pub mod mojang;

const MOJANG_KEY: &[u8] = include_bytes!("yggdrasil_session_pubkey.der");

pub struct AuthConfiguration {
    server_private_key: MCPrivateKey,
    compression_threshold: isize,
    force_key_authentication: bool,
    auth_url: Option<String>,
}

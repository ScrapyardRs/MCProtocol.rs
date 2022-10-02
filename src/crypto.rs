use data_encoding::Specification;
use openssl::sha::sha1;
use rand::rngs::OsRng;
use rsa::hash::Hash;

use rsa::{BigUint, PaddingScheme, PublicKey, PublicKeyParts, RsaPublicKey};

pub type MCPrivateKey = rsa::RsaPrivateKey;
pub type MCPublicKey = RsaPublicKey;
pub type Padding = PaddingScheme;

pub const SHA1_HASH: Hash = Hash::SHA1;
pub const SHA256_HASH: Hash = Hash::SHA2_256;

pub fn new_key() -> rsa::errors::Result<MCPrivateKey> {
    let mut rng = OsRng;
    rsa::RsaPrivateKey::new(&mut rng, 1024)
}

#[derive(Debug)]
pub enum CapturedRsaError {
    RsaError(rsa::errors::Error),
    RsaDerError(rsa_der::Error),
    SpecificationError(data_encoding::SpecificationError),
}

impl std::error::Error for CapturedRsaError {}

impl std::fmt::Display for CapturedRsaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RsaError(rsa_err) => rsa_err.fmt(f),
            Self::RsaDerError(rsa_der_err) => rsa_der_err.fmt(f),
            Self::SpecificationError(spec_err) => spec_err.fmt(f),
        }
    }
}

pub fn key_from_der(der: &[u8]) -> std::result::Result<MCPublicKey, CapturedRsaError> {
    let (n, e) =
        rsa_der::public_key_from_der(der).map_err(|err| CapturedRsaError::RsaDerError(err))?;
    RsaPublicKey::new(BigUint::from_bytes_be(&n), BigUint::from_bytes_be(&e))
        .map_err(|err| CapturedRsaError::RsaError(err))
}

pub fn private_key_to_der(key: &MCPrivateKey) -> Vec<u8> {
    let pub_key = RsaPublicKey::from(key);
    rsa_der::public_key_to_der(&pub_key.n().to_bytes_be(), &pub_key.e().to_bytes_be())
}

pub fn encode_key_pem(
    expiry: u64,
    public_key: &[u8],
) -> std::result::Result<String, data_encoding::SpecificationError> {
    let mut spec = Specification::new();
    spec.symbols
        .push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/");
    spec.padding = Some('=');
    spec.wrap.width = 76;
    spec.wrap.separator.push('\n');
    let spec = spec.encoding()?;

    Ok(format!(
        "{}-----BEGIN RSA PUBLIC KEY-----\n{}-----END RSA PUBLIC KEY-----\n",
        expiry,
        spec.encode(public_key)
    ))
}

pub fn verify_signature(
    hash: Option<Hash>,
    verify_key: &MCPublicKey,
    signature: &[u8],
    message: &[u8],
) -> rsa::errors::Result<()> {
    verify_key.verify(PaddingScheme::PKCS1v15Sign { hash }, message, signature)?;
    Ok(())
}

pub fn sha1_message(bytes: &[u8]) -> [u8; 20] {
    sha1(bytes)
}

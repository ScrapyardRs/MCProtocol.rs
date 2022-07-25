use rsa::hash::Hash;
use data_encoding::{BASE64_MIME, Specification};
use openssl::sha::{sha1, sha256};
use rand::rngs::OsRng;
use ring::error::{KeyRejected, Unspecified};
use ring::signature::{UnparsedPublicKey, RSA_PKCS1_SHA256, RSA_PKCS1_2048_8192_SHA256, RsaKeyPair, VerificationAlgorithm};
use rsa::{BigUint, hash, PaddingScheme, PublicKey, PublicKeyParts, RsaPublicKey};
use rsa::errors::Error;
use rsa::PaddingScheme::PKCS1v15Encrypt;
use rsa::pkcs1::ToRsaPrivateKey;
use rsa::pkcs8::ToPublicKey;
use rsa_der::public_key_from_der;

pub type MCPrivateKey = rsa::RsaPrivateKey;
pub type MCPublicKey = rsa::RsaPublicKey;
pub type Padding = PaddingScheme;

pub const SHA1_HASH: Hash = Hash::SHA1;
pub const SHA256_HASH: Hash = Hash::SHA2_256;

pub fn new_key() -> anyhow::Result<MCPrivateKey> {
    let mut rng = OsRng;
    let key = rsa::RsaPrivateKey::new(&mut rng, 1024)?;
    Ok(key)
}

pub fn key_from_der(der: &[u8]) -> anyhow::Result<MCPublicKey> {
    let (n, e) = rsa_der::public_key_from_der(der)?;
    let key = RsaPublicKey::new(BigUint::from_bytes_be(&n), BigUint::from_bytes_be(&e))?;
    Ok(key)
}

pub fn private_key_to_der(key: &MCPrivateKey) -> Vec<u8> {
    let pub_key = RsaPublicKey::from(key);

    rsa_der::public_key_to_der(
        &pub_key.n().to_bytes_be(),
        &pub_key.e().to_bytes_be(),
    )
}

pub fn encode_key_pem(expiry: u64, public_key: &[u8]) -> anyhow::Result<String> {
    let mut spec = Specification::new();
    spec.symbols.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/");
    spec.padding = Some('=');
    spec.wrap.width = 76;
    spec.wrap.separator.push_str("\n");
    let spec = spec.encoding()?;

    Ok(format!("{}-----BEGIN RSA PUBLIC KEY-----\n{}-----END RSA PUBLIC KEY-----\n", expiry, spec.encode(public_key)))
}

pub fn verify_signature(hash: Option<Hash>, verify_key: &MCPublicKey, signature: &[u8], message: &[u8]) -> anyhow::Result<()> {
    verify_key.verify(PaddingScheme::PKCS1v15Sign { hash }, message, signature)?;
    Ok(())
}

pub fn sha1_message(bytes: &[u8]) -> [u8; 20] {
    sha1(bytes)
}

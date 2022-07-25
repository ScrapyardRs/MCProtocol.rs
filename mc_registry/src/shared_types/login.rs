use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use encryption_utils::{key_from_der, MCPrivateKey, MCPublicKey, sha1_message};
use mc_serializer::primitive::VarInt;

mc_serializer::auto_string!(LoginUsername, 16);

#[derive(mc_serializer_derive::MCSerde, Debug, Clone)]
pub struct MCIdentifiedKey {
    pub timestamp: u64,
    pub public_key: (VarInt, Vec<u8>),
    pub signature: (VarInt, Vec<u8>),
}

#[derive(Clone)]
pub struct IdentifiedKey {
    public_key: MCPublicKey,
}

impl MCIdentifiedKey {
    pub fn has_expired(&self) -> bool {
        let key_instant = UNIX_EPOCH + Duration::from_millis(self.timestamp as u64);
        SystemTime::now() > key_instant
    }

    pub fn verify_signature(&self, verify_against: &MCPublicKey) -> anyhow::Result<()> {
        let encoded_pem = encryption_utils::encode_key_pem(self.timestamp, &self.public_key.1)?;
        encryption_utils::verify_signature(
            Some(encryption_utils::SHA1_HASH),
            verify_against,
            &self.signature.1,
            sha1_message(encoded_pem.as_bytes()).as_slice(),
        )
    }
}

impl IdentifiedKey {
    pub fn new(key: &[u8]) -> anyhow::Result<Self> {
        Ok(Self {
            public_key: key_from_der(key)?,
        })
    }

    pub fn verify_data_signature(&self, signature: &[u8], data: &[u8]) -> anyhow::Result<()> {
        encryption_utils::verify_signature(
            Some(encryption_utils::SHA256_HASH),
            &self.public_key,
            signature,
            data,
        )
    }
}
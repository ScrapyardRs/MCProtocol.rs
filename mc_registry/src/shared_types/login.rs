use encryption_utils::{key_from_der, MCPublicKey};

mc_serializer::auto_string!(LoginUsername, 16);

#[derive(Clone)]
pub struct IdentifiedKey {
    public_key: MCPublicKey,
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

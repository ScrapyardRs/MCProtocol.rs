use std::io::{Read, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use encryption_utils::{MCPublicKey, sha1_message};
use crate::client_bound::login::LoginProperty;
use mc_serializer::wrap_struct_context;
use mc_serializer::serde::{Contextual, Deserialize, ProtocolVersion, Serialize, SerializerContext};
use mc_serializer::contextual;
use mc_serializer::primitive::{Identifier, VarInt};

pub mod login;
pub mod play;

#[derive(mc_serializer_derive::Serial, Debug, Clone)]
pub struct MCIdentifiedKey {
    pub timestamp: u64,
    pub public_key: (VarInt, Vec<u8>),
    pub signature: (VarInt, Vec<u8>),
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

#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct Property {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

contextual!(Property);

impl Serialize for Property {
    fn serialize<W: Write>(&self, writer: &mut W, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<()> {
        wrap_struct_context!("name", Identifier::from(&self.name).serialize(writer, protocol_version))?;
        wrap_struct_context!("value", Identifier::from(&self.value).serialize(writer, protocol_version))?;

        match self.signature.as_ref() {
            None => wrap_struct_context!("sig_exists", false.serialize(writer, protocol_version))?,
            Some(sig) => {
                wrap_struct_context!("sig_exists", true.serialize(writer, protocol_version))?;
                wrap_struct_context!("sig", Identifier::from(sig).serialize(writer, protocol_version))?
            }
        }
        Ok(())
    }

    fn size(&self, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<i32> {
        let mut size = 0;
        size += wrap_struct_context!("name", Identifier::from(&self.name).size(protocol_version))?;
        size += wrap_struct_context!("value", Identifier::from(&self.value).size(protocol_version))?;

        match self.signature.as_ref() {
            None => size += wrap_struct_context!("sig_exists", false.size(protocol_version))?,
            Some(sig) => {
                size += wrap_struct_context!("sig_exists", true.size(protocol_version))?;
                size += wrap_struct_context!("sig", Identifier::from(sig).size(protocol_version))?
            }
        }
        Ok(size)
    }
}

impl Deserialize for Property {
    fn deserialize<R: Read>(reader: &mut R, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<Self> {
        let name = wrap_struct_context!("name", Identifier::deserialize(reader, protocol_version))?;
        let value = wrap_struct_context!("value", Identifier::deserialize(reader, protocol_version))?;

        let sig_exists = wrap_struct_context!("sig_exists", bool::deserialize(reader, protocol_version))?;
        let signature = if sig_exists {
            Some(wrap_struct_context!("sig", Identifier::deserialize(reader, protocol_version))?)
        } else {
            None
        };
        Ok(Self {
            name: name.to_string(),
            value: value.to_string(),
            signature: signature.map(|identifier| identifier.to_string()),
        })
    }
}

impl From<&LoginProperty> for Property {
    fn from(property: &LoginProperty) -> Self {
        Self {
            name: property.name.to_string(),
            value: property.value.to_string(),
            signature: property.signature.1.as_ref().map(ToString::to_string),
        }
    }
}

#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct GameProfile {
    pub id: uuid::Uuid,
    pub name: String,
    pub properties: Vec<Property>,
}

contextual!(GameProfile);

impl GameProfile {
    pub fn properties_size(&self) -> mc_serializer::serde::Result<VarInt> {
        wrap_struct_context!("properties_size", VarInt::try_from(self.properties.len())
            .map_err(|err| mc_serializer::serde::Error::TryFromIntError(err, SerializerContext::new(Self::context(), format!("Failed to convert {} into a VarInt.", self.properties.len())))))
    }
}

impl Serialize for GameProfile {
    fn serialize<W: Write>(&self, writer: &mut W, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<()> {
        wrap_struct_context!("id", self.id.serialize(writer, protocol_version))?;
        wrap_struct_context!("name", self.id.serialize(writer, protocol_version))?;
        let size = self.properties_size()?;
        wrap_struct_context!("properties_size", size.serialize(writer, protocol_version))?;
        wrap_struct_context!("properties", self.properties.serialize(writer, protocol_version))
    }

    fn size(&self, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<i32> {
        let mut size = 0;
        size += wrap_struct_context!("id", self.id.size(protocol_version))?;
        size += wrap_struct_context!("name", self.id.size(protocol_version))?;
        let props_size = self.properties_size()?;
        size += wrap_struct_context!("properties_size", props_size.size(protocol_version))?;
        wrap_struct_context!("properties", self.properties.size(protocol_version))
            .map(|x| x + size)
    }
}

impl Deserialize for GameProfile {
    fn deserialize<R: Read>(reader: &mut R, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<Self> {
        let id = wrap_struct_context!("id", uuid::Uuid::deserialize(reader, protocol_version))?;
        let name = wrap_struct_context!("name", Identifier::deserialize(reader, protocol_version))?;
        let properties = wrap_struct_context!("properties", <(VarInt, Vec<Property>)>::deserialize(reader, protocol_version))?;
        Ok(Self {
            id,
            name: name.to_string(),
            properties: properties.1,
        })
    }
}

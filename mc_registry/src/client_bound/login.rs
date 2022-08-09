use mc_serializer::primitive::{Identifier, VarInt};

use crate::shared_types::login::LoginUsername;
use crate::shared_types::Property;

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct Disconnect {
    #[json(262144)]
    pub reason: mc_chat::Chat,
}

mc_serializer::auto_string!(ServerId, 20);

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct EncryptionRequest {
    pub server_id: ServerId,
    pub public_key: (VarInt, Vec<u8>),
    pub verify_token: (VarInt, Vec<u8>),
}

mc_serializer::auto_string!(PropertyName, 32767);
mc_serializer::auto_string!(PropertyValue, 32767);
mc_serializer::auto_string!(PropertySignature, 32767);

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct LoginProperty {
    pub name: PropertyName,
    pub value: PropertyValue,
    pub signature: (bool, Option<PropertySignature>),
}

impl From<&Property> for LoginProperty {
    fn from(property: &Property) -> Self {
        Self {
            name: PropertyName::from(property.name.to_string()),
            value: PropertyValue::from(property.value.to_string()),
            signature: match &property.signature {
                None => (false, None),
                Some(something) => (true, Some(PropertySignature::from(something.to_string()))),
            },
        }
    }
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct LoginSuccess {
    pub uuid: uuid::Uuid,
    pub username: LoginUsername,
    pub properties: (VarInt, Vec<LoginProperty>),
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct SetCompression {
    pub threshold: VarInt,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct LoginPluginRequest {
    pub message_id: VarInt,
    pub channel: Identifier,
    pub data: Vec<u8>,
}

crate::create_mappings! {
    Disconnect {
        def 0x00;
    }

    EncryptionRequest {
        def 0x01;
    }

    LoginSuccess {
        def 0x02;
    }

    SetCompression {
        def 0x03;
    }

    LoginPluginRequest {
        def 0x04;
    }
}

use crate::shared_types::login::LoginUsername;
use mc_serializer::primitive::VarInt;
use mc_serializer::serde::Contextual;
use crate::shared_types::MCIdentifiedKey;

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct LoginStart {
    pub name: LoginUsername,
    pub sig_data: (bool, Option<MCIdentifiedKey>),
    pub sig_holder: (bool, Option<uuid::Uuid>),
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(bool)]
pub enum EncryptionResponseData {
    #[key(true)]
    VerifyTokenData((VarInt, Vec<u8>)),
    #[key(false)]
    MessageSignature {
        salt: i64,
        message_signature: (VarInt, Vec<u8>),
    },
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct EncryptionResponse {
    pub shared_secret: (VarInt, Vec<u8>),
    pub response_data: EncryptionResponseData,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct LoginPluginResponse {
    pub message_id: VarInt,
    pub successful: bool,
    pub data: Vec<u8>,
}

crate::create_mappings! {
    LoginStart {
        def 0x00;
    }

    EncryptionResponse {
        def 0x01;
    }

    LoginPluginResponse {
        def 0x02;
    }
}

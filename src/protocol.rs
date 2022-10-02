pub mod handshaking {
    pub mod sb {
        use drax::VarInt;

        #[derive(drax_derive::DraxTransport, Debug, Copy, Clone)]
        #[drax(key = {match VarInt})]
        pub enum NextState {
            Handshaking,
            Status,
            Login,
        }

        #[derive(drax_derive::DraxTransport, Debug, Clone)]
        pub struct Handshake {
            pub protocol_version: VarInt,
            #[drax(limit = 255)]
            pub server_address: String,
            pub server_port: u16,
            pub next_state: NextState,
        }

        use crate::registry::UNKNOWN_VERSION;

        crate::import_registrations! {
            Handshake {
                UNKNOWN_VERSION -> 0x00,
            }
        }
    }

    pub mod cb {}
}

pub mod status {
    pub mod sb {
        #[derive(drax_derive::DraxTransport, Debug, Clone, Copy)]
        pub struct Request;

        #[derive(drax_derive::DraxTransport, Debug, Clone, Copy)]
        pub struct Ping {
            pub start_time: i64,
        }

        impl From<super::cb::Pong> for Ping {
            fn from(pong: super::cb::Pong) -> Self {
                Self {
                    start_time: pong.start_time,
                }
            }
        }

        crate::import_registrations! {
            Request {
                760 -> 0x00,
            }
            Ping {
                760 -> 0x01,
            }
        }
    }

    pub mod cb {
        #[derive(serde_derive::Serialize, serde_derive::Deserialize, Debug)]
        pub struct StatusResponseVersion {
            pub name: String,
            pub protocol: i32,
        }

        #[derive(serde_derive::Serialize, serde_derive::Deserialize, Debug)]
        pub struct StatusResponsePlayerSample {
            pub name: String,
            pub id: uuid::Uuid,
        }

        #[derive(serde_derive::Serialize, serde_derive::Deserialize, Debug)]
        pub struct StatusResponsePlayers {
            pub max: i32,
            pub online: i32,
            pub sample: Vec<StatusResponsePlayerSample>,
        }

        #[derive(serde_derive::Serialize, serde_derive::Deserialize, Debug)]
        pub struct StatusResponse {
            pub version: StatusResponseVersion,
            pub players: StatusResponsePlayers,
            pub description: crate::chat::Chat,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub favicon: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "previewsChat")]
            pub previews_chat: Option<bool>,
        }

        #[derive(drax_derive::DraxTransport)]
        pub struct Response(#[drax(json = 32767)] pub StatusResponse);

        #[derive(drax_derive::DraxTransport, Debug, Clone, Copy)]
        pub struct Pong {
            pub start_time: i64,
        }

        impl From<super::sb::Ping> for Pong {
            fn from(ping: super::sb::Ping) -> Self {
                Self {
                    start_time: ping.start_time,
                }
            }
        }

        crate::import_registrations! {
            Response {
                760 -> 0x00,
            }
            Pong {
                760 -> 0x01,
            }
        }
    }
}

pub mod login {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use drax::SizedVec;

    use crate::crypto::{key_from_der, MCPublicKey};

    #[derive(drax_derive::DraxTransport, Debug, Clone)]
    pub struct MojangIdentifiedKey {
        pub timestamp: u64,
        pub public_key: SizedVec<u8>,
        pub signature: SizedVec<u8>,
    }

    impl MojangIdentifiedKey {
        pub fn has_expired(&self) -> bool {
            let key_instant = UNIX_EPOCH + Duration::from_millis(self.timestamp as u64);
            SystemTime::now() > key_instant
        }

        pub fn verify_signature(
            &self,
            verify_against: &MCPublicKey,
        ) -> std::result::Result<(), crate::crypto::CapturedRsaError> {
            let encoded_pem = crate::crypto::encode_key_pem(self.timestamp, &self.public_key)
                .map_err(crate::crypto::CapturedRsaError::SpecificationError)?;
            crate::crypto::verify_signature(
                Some(crate::crypto::SHA1_HASH),
                verify_against,
                &self.signature,
                crate::crypto::sha1_message(encoded_pem.as_bytes()).as_slice(),
            )
            .map_err(crate::crypto::CapturedRsaError::RsaError)
        }
    }

    #[derive(Clone)]
    pub struct IdentifiedKey {
        public_key: MCPublicKey,
    }

    impl IdentifiedKey {
        pub fn new(key: &[u8]) -> std::result::Result<Self, crate::crypto::CapturedRsaError> {
            Ok(Self {
                public_key: key_from_der(key)?,
            })
        }

        pub fn verify_data_signature(
            &self,
            signature: &[u8],
            data: &[u8],
        ) -> rsa::errors::Result<()> {
            crate::crypto::verify_signature(
                Some(crate::crypto::SHA256_HASH),
                &self.public_key,
                signature,
                data,
            )
        }
    }

    pub mod sb {
        use drax::Maybe;
        use drax::SizedVec;
        use drax::VarInt;
        use uuid::Uuid;

        use super::MojangIdentifiedKey;

        #[derive(drax_derive::DraxTransport, Debug)]
        pub struct LoginStart {
            #[drax(limit = 16)]
            pub name: String,
            pub sig_data: Maybe<MojangIdentifiedKey>,
            pub sig_holder: Maybe<Uuid>,
        }

        #[derive(drax_derive::DraxTransport, Debug)]
        #[drax(key = {match bool})]
        pub enum EncryptionResponseData {
            #[drax(key = {true})]
            VerifyTokenData(SizedVec<u8>),
            #[drax(key = {false})]
            MessageSignature {
                salt: i64,
                message_signature: SizedVec<u8>,
            },
        }

        #[derive(drax_derive::DraxTransport, Debug)]
        pub struct EncryptionResponse {
            pub shared_secret: SizedVec<u8>,
            pub response_data: EncryptionResponseData,
        }

        #[derive(drax_derive::DraxTransport, Debug)]
        pub struct LoginPluginResponse {
            pub message_id: VarInt,
            pub successful: bool,
            pub data: Vec<u8>,
        }

        crate::import_registrations! {
            LoginStart {
                760 -> 0x00,
            }
            EncryptionResponse {
                760 -> 0x01,
            }
            LoginPluginResponse {
                760 -> 0x02,
            }
        }
    }

    pub mod cb {
        use drax::{Maybe, SizedVec};

        #[derive(drax_derive::DraxTransport, Debug)]
        pub struct Disconnect {
            #[drax(json = 262144)]
            pub reason: crate::chat::Chat,
        }

        #[derive(drax_derive::DraxTransport, Debug)]
        pub struct EncryptionRequest {
            #[drax(limit = 20)]
            pub server_id: String,
            pub public_key: SizedVec<u8>,
            pub verify_token: SizedVec<u8>,
        }

        #[derive(drax_derive::DraxTransport, Debug)]
        pub struct LoginProperty {
            pub name: String,
            pub value: String,
            pub signature: Maybe<String>,
        }

        #[derive(drax_derive::DraxTransport, Debug)]
        pub struct LoginSuccess {
            pub uuid: uuid::Uuid,
            #[drax(limit = 16)]
            pub username: String,
            pub properties: SizedVec<LoginProperty>,
        }

        #[derive(drax_derive::DraxTransport, Debug)]
        pub struct SetCompression {
            pub threshold: drax::VarInt,
        }

        #[derive(drax_derive::DraxTransport, Debug)]
        pub struct LoginPluginRequest {
            pub message_id: drax::VarInt,
            pub channel: String,
            pub data: Vec<u8>,
        }

        crate::import_registrations! {
            Disconnect {
                760 -> 0x00,
            }
            EncryptionRequest {
                760 -> 0x01,
            }
            LoginSuccess {
                760 -> 0x02,
            }
            SetCompression {
                760 -> 0x03,
            }
            LoginPluginRequest {
                760 -> 0x04,
            }
        }
    }
}

pub mod play;

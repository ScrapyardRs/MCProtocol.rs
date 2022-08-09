use mc_chat::Chat;
use mc_serializer::serde::{ProtocolVersion, ProtocolVersionSpec};
use crate::server_bound::status::Ping;
use mc_serializer::contextual;

#[derive(serde_derive::Serialize, serde_derive::Deserialize, Debug)]
pub struct StatusResponseVersion {
    pub name: String,
    pub protocol: i32,
}

impl From<ProtocolVersion> for StatusResponseVersion {
    fn from(protocol_version: ProtocolVersion) -> Self {
        let spec = ProtocolVersionSpec::from(protocol_version);
        Self {
            name: spec.1,
            protocol: spec.0,
        }
    }
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
    pub description: Chat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "previewsChat")]
    pub previews_chat: Option<bool>,
}

contextual!(StatusResponseVersion);
contextual!(StatusResponsePlayerSample);
contextual!(StatusResponsePlayers);
contextual!(StatusResponse);

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct Response(#[json(32767)] pub StatusResponse);

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct Pong {
    pub start_time: i64,
}

impl From<Ping> for Pong {
    fn from(ping: Ping) -> Self {
        Self {
            start_time: ping.start_time,
        }
    }
}

crate::create_mappings! {
    Response {
        def 0x00;
    }

    Pong {
        def 0x01;
    }
}

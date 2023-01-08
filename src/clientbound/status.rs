use crate::common::chat::Chat;
use drax::transport::packet::serde_json::JsonDelegate;

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
pub struct StatusPlayers {
    pub max: isize,
    pub online: isize,
    pub sample: Vec<Player>,
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
pub struct StatusVersion {
    pub name: String,
    pub protocol: i32,
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
pub struct StatusResponse {
    pub description: Chat,
    pub players: StatusPlayers,
    pub version: StatusVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    #[serde(rename = "enforcesSecureChat")]
    pub enforces_secure_chat: bool,
}

registry! {
    registry ClientboundStatusRegistry {
        /// The response to a [`crate::serverbound::status::Request`] packet.
        struct Response {
            /// The response to the request
            response: JsonDelegate<StatusResponse>
        },

        /// The response to a [`crate::serverbound::status::Ping`] packet.
        struct Pong {
            /// The response to the ping
            payload: u64
        }
    }
}

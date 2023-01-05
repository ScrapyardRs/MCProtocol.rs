use crate::common::chat::Chat;
use drax::transport::packet::serde_json::JsonDelegate;

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Player {
    id: String,
    name: String,
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
pub struct StatusPlayers {
    max: usize,
    online: usize,
    sample: Vec<Player>,
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
pub struct StatusVersion {
    name: String,
    protocol: i32,
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
pub struct StatusResponse {
    description: Chat,
    players: StatusPlayers,
    version: StatusVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    favicon: Option<String>,
    #[serde(rename = "enforcesSecureChat")]
    enforces_secure_chat: bool,
}

registry! {
    registry ClientboundStatusRegistry {
        /// The response to a [`crate::serverbound::status::Request`] packet.
        struct Response {
            /// The response to the request
            response: JsonDelegate<StatusResponse>,
        },

        /// The response to a [`crate::serverbound::status::Ping`] packet.
        struct Pong {
            /// The response to the ping
            payload: u64,
        }
    }
}

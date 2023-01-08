use drax::prelude::Uuid;
use drax::struct_packet_components;
use drax::transport::packet::option::Maybe;

struct_packet_components! {
    #[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize, Clone)]
    GameProfileProperty {
        name: String,
        value: String,
        signature: #[serde(skip_serializing_if = "Option::is_none")] Maybe<String>
    }

    #[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize, Clone)]
    GameProfile {
        id: Uuid,
        name: String,
        properties: Vec<GameProfileProperty>
    }
}
#[derive(Debug, serde_derive::Serialize)]
pub struct GameProfileProperty2 {
    pub name: <String as ::drax::transport::packet::PacketComponent<ctx_type!(())>>::ComponentType,
    pub value: <String as ::drax::transport::packet::PacketComponent<ctx_type!(())>>::ComponentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature:
        <Maybe<String> as ::drax::transport::packet::PacketComponent<ctx_type!(())>>::ComponentType,
}

pub mod bit_set;
pub mod bit_storage;
pub mod chat;
pub mod chunk;
pub mod play;

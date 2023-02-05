use drax::prelude::Uuid;
use drax::struct_packet_components;
use drax::transport::packet::option::Maybe;

struct_packet_components! {
    #[derive(serde_derive::Serialize, serde_derive::Deserialize, Clone)]
    GameProfileProperty {
        name: String,
        value: String,
        signature: #[serde(skip_serializing_if = "Option::is_none")] Maybe<String>
    }

    #[derive(serde_derive::Serialize, serde_derive::Deserialize, Clone)]
    GameProfile {
        id: Uuid,
        name: String,
        properties: Vec<GameProfileProperty>
    }
}

#[cfg(feature = "play")]
pub mod bit_set;
#[cfg(feature = "play")]
pub mod bit_storage;
pub mod chat;
#[cfg(feature = "play")]
pub mod chunk;
#[cfg(feature = "play")]
pub mod play;
#[cfg(feature = "play")]
pub mod registry;

use drax::prelude::Uuid;
use drax::struct_packet_components;
use drax::transport::packet::option::Maybe;

struct_packet_components! {
    GameProfileProperty {
        name: String,
        value: String,
        signature: Maybe<String>,
    }

    GameProfile {
        id: Uuid,
        name: String,
        properties: Vec<GameProfileProperty>,
    }
}

pub mod chat;

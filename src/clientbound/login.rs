use crate::common::chat::Chat;
use crate::common::GameProfile;
use drax::transport::packet::option::Maybe;
use drax::transport::packet::primitive::VarInt;
use drax::transport::packet::serde_json::JsonDelegate;
use drax::transport::packet::string::LimitedString;
use drax::transport::packet::vec::ByteDrain;

registry! {
    registry ClientboundLoginRegistry {
        struct LoginDisconnect {
            reason: JsonDelegate<Chat>,
        },

        struct Hello {
            server_id: LimitedString<20>,
            public_key: Vec<u8>,
            challenge: Vec<u8>,
        },

        struct LoginGameProfile {
            game_profile: GameProfile,
        },

        struct LoginCompression {
            threshold: VarInt,
        },

        struct CustomQuery {
            transaction_id: VarInt,
            identifier: String,
            data: Maybe<ByteDrain>,
        }
    }
}

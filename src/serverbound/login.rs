use drax::transport::packet::option::Maybe;
use drax::transport::packet::primitive::VarInt;
use drax::transport::packet::string::LimitedString;
use drax::transport::packet::vec::ByteDrain;

registry! {
    registry ServerBoundLoginRegsitry {
        /// The packet which starts a login flow.
        struct Hello {
            /// The name of the player
            name: LimitedString<16>,
            /// The profile id of the player; conditionally present
            profile_id: Maybe<drax::prelude::Uuid>,
        },

        /// The C2S key packet
        struct Key {
            /// The shared secret
            key_bytes: Vec<u8>,
            /// An encrypted version of a challenge the server sends.
            encrypted_challenge: Vec<u8>,
        },

        /// The login plugin response packet
        struct CustomQuery {
            /// The transaction id
            transaction_id: VarInt,
            /// A drain of the rest of the bytes; conditionally present.
            data: Maybe<ByteDrain>,
        }
    }
}

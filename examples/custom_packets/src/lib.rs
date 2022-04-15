use mc_protocol::packets;
use mc_protocol::prelude::*;

packets! {
    CustomPayload {
        field f1: VarInt,
        field f2: VarInt,

        mapping ProtocolVersion::Handshake => 0x01 {
            for f1 = { packet -> packet.f1 }
                | auto as VarInt
            for f2 = { packet -> packet.f2 }
                | auto as VarInt

            = deserializer {
                Ok(Self {
                    f1, f2
                })
            }
        }
    }
}

use mc_serializer::primitive::VarInt;
use mc_serializer::serde::Contextual;

#[derive(mc_serializer_derive::Serial, Debug, Copy, Clone)]
#[key(VarInt)]
pub enum NextState {
    #[key(VarInt::from(1))]
    Status,
    #[key(VarInt::from(2))]
    Login,
}

mc_serializer::auto_string!(ServerAddress, 255);

#[derive(mc_serializer_derive::Serial, Debug, Clone)]
pub struct Handshake {
    pub protocol_version: VarInt,
    pub server_address: ServerAddress,
    pub server_port: u16,
    pub next_state: NextState,
}

crate::create_mappings! {
    Handshake {
        def 0x00;
    }
}

use mc_serializer::primitive::VarInt;

#[derive(mc_serializer_derive::MCSerde, Debug, Copy, Clone)]
#[key(VarInt)]
pub enum NextState {
    #[key(VarInt::from(1))]
    Status,
    #[key(VarInt::from(2))]
    Login,
}

mc_serializer::auto_string!(ServerAddress, 255);

#[derive(mc_serializer_derive::MCSerde, Debug)]
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

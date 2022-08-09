use crate::client_bound::status::Pong;

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct Request;

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct Ping {
    pub start_time: i64,
}

impl From<Pong> for Ping {
    fn from(pong: Pong) -> Self {
        Self {
            start_time: pong.start_time,
        }
    }
}

crate::create_mappings! {
    Request {
        def 0x00;
    }

    Ping {
        def 0x01;
    }
}

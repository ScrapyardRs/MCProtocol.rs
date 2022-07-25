use crate::server_bound::status::Ping;
mc_serializer::auto_string!(JSONResponse, 32767);

#[derive(mc_serializer_derive::MCSerde, Debug)]
pub struct Response {
    pub json_response: JSONResponse,
}

#[derive(mc_serializer_derive::MCSerde, Debug)]
pub struct Pong {
    pub start_time: i64,
}

impl From<Ping> for Pong {
    fn from(ping: Ping) -> Self {
        Self { start_time: ping.start_time }
    }
}

crate::create_mappings! {
    Response {
        def 0x00;
    }

    Pong {
        def 0x01;
    }
}

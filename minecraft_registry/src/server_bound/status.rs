#[derive(minecraft_serde_derive::MCSerde, Debug)]
pub struct Request {}

#[derive(minecraft_serde_derive::MCSerde, Debug)]
pub struct Ping {
    pub start_time: i64,
}

crate::create_mappings! {
    RequestMappings: Request {
        def 0x00;
    }

    PingMappings: Ping {
        def 0x01;
    }
}

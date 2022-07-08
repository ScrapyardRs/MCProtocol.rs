minecraft_serde::auto_string!(JSONResponse, 32767);

#[derive(minecraft_serde_derive::MCSerde, Debug)]
pub struct Response {
    pub json_response: JSONResponse,
}

#[derive(minecraft_serde_derive::MCSerde, Debug)]
pub struct Pong {
    pub start_time: i64,
}

crate::create_mappings! {
    ResponseMappings: Response {
        def 0x00;
    }

    PongMappings: Pong {
        def 0x01;
    }
}

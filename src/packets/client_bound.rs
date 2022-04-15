use crate::prelude::*;

pub mod login {
    use super::*;
    use uuid::Uuid;

    pub fn echo_sheet(version: ProtocolVersion) -> ProtocolSheet<NoContext> {
        let mut sheet = ProtocolSheet::new(version);
        sheet.register_packet_handle(Disconnect::echo_packet_handle());
        sheet.register_packet_handle(EncryptionRequest::echo_packet_handle());
        sheet.register_packet_handle(LoginSuccess::echo_packet_handle());
        sheet.register_packet_handle(SetCompression::echo_packet_handle());
        sheet.register_packet_handle(LoginPluginRequest::echo_packet_handle());
        sheet
    }

    crate::auto_string!(ServerId, 20);
    crate::auto_string!(Username, 16);

    crate::packets! {
        Disconnect {
            field reason: Chat,

            mapping ProtocolVersion::V118R2 => 0x00 {
                for reason = {packet -> &packet.reason}
                    | auto as Chat

                = deserializer {
                    Ok(Self {
                        reason,
                    })
                }
            }
        }

        EncryptionRequest {
            field server_id: ServerId,
            field pub_key: (VarInt, Vec<u8>),
            field verify: (VarInt, Vec<u8>),

            mapping ProtocolVersion::V118R2 => 0x01 {
                for server_id = {packet -> &packet.server_id}
                    | auto as ServerId
                for pub_key = {packet -> &packet.pub_key}
                    | auto as (VarInt, Vec<u8>)
                for verify = {packet -> &packet.verify}
                    | auto as (VarInt, Vec<u8>)

                = deserializer {
                    Ok(Self {
                        server_id,
                        pub_key,
                        verify,
                    })
                }
            }
        }

        LoginSuccess {
            field uuid: Uuid,
            field username: Username,

            mapping ProtocolVersion::V118R2 => 0x02 {
                for uuid = {packet -> &packet.uuid}
                    | auto as uuid::Uuid
                for username = {packet -> &packet.username}
                    | auto as Username

                = deserializer {
                    Ok(Self {
                        uuid,
                        username,
                    })
                }
            }
        }

        SetCompression {
            field threshold: VarInt,

            mapping ProtocolVersion::V118R2 => 0x03 {
                for threshold = {packet -> &packet.threshold}
                    | auto as VarInt

                = deserializer {
                    Ok(Self {
                        threshold,
                    })
                }
            }
        }

        LoginPluginRequest {
            field message_id: VarInt,
            field channel: Identifier,
            field data: Vec<u8>,

            mapping ProtocolVersion::V118R2 => 0x04 {
                for message_id = {packet -> &packet.message_id}
                    | auto as VarInt
                for channel = {packet -> &packet.channel}
                    | auto as Identifier
                for data = {packet -> &packet.data}
                    | auto as Vec<u8>

                = deserializer {
                    Ok(Self {
                        message_id,
                        channel,
                        data,
                    })
                }
            }
        }
    }
}

pub mod status {
    use super::*;

    pub fn echo_sheet(version: ProtocolVersion) -> ProtocolSheet<NoContext> {
        let mut sheet = ProtocolSheet::new(version);
        sheet.register_packet_handle(StatusResponse::echo_packet_handle());
        sheet.register_packet_handle(Pong::echo_packet_handle());
        sheet
    }

    crate::auto_string!(JSONResponse, 32767);

    crate::packets! {
        StatusResponse {
            field json_response: JSONResponse,

            mapping ProtocolVersion::V118R2 => 0x00 {
                for json_response = {packet -> &packet.json_response}
                    | auto as JSONResponse

                = deserializer {
                    Ok(Self {
                        json_response
                    })
                }
            }
        }

        Pong {
            field payload: i64,

            mapping ProtocolVersion::V118R2 => 0x01 {
                for payload = {packet -> &packet.payload}
                    | auto as i64

                = deserializer {
                    Ok(Self {
                        payload
                    })
                }
            }
        }
    }
}

use crate::prelude::*;

pub mod play {
    use super::*;

    pub fn echo_sheet(version: ProtocolVersion) -> ProtocolSheet<NoContext> {
        let mut sheet = ProtocolSheet::new(version);
        sheet.register_packet_handle(Teams::echo_packet_handle());
        sheet
    }

    crate::auto_string!(TeamName, 16);
    crate::auto_string!(Size32String, 32);
    crate::auto_string!(EntityIdentifier, 40);
    crate::auto_enum! {
        TeamModeInfo by u8 {
            0; CreateTeam {
                team_display_name: Chat,
                friendly_flags: u8,
                name_tag_visibility: Size32String,
                collision_rule: Size32String,
                team_color: VarInt,
                team_prefix: Chat,
                team_suffix: Chat,
                entities: Vec<EntityIdentifier>,
            }
            1; RemoveTeam {}
            2; UpdateTeamInfo {
                team_display_name: Chat,
                friendly_flags: u8,
                name_tag_visibility: Size32String,
                collision_rule: Size32String,
                team_color: VarInt,
                team_prefix: Chat,
                team_suffix: Chat,
            }
            3; AddEntitiesToTeam {
                entities: Vec<EntityIdentifier>,
            }
            4; RemoveEntitiesFromTeam {
                entities: Vec<EntityIdentifier>,
            }
        }
    }

    crate::packets! {
        Teams {
            field team_name: TeamName,
            field mode_info: (u8, TeamModeInfo),

            mapping ProtocolVersion::V118R2 => 0x55 {
                for team_name = {packet -> &packet.team_name}
                    | auto as TeamName
                for mode_info = {packet -> &packet.mode_info}
                    | auto as (u8, TeamModeInfo)

                = deserializer {
                    Ok(Self {
                        team_name,
                        mode_info,
                    })
                }
            }
        }
    }
}

pub mod login {
    use super::*;

    pub fn echo_sheet(version: ProtocolVersion) -> ProtocolSheet<NoContext> {
        let mut sheet = ProtocolSheet::new(version);
        sheet.register_packet_handle(LoginStart::echo_packet_handle());
        sheet.register_packet_handle(EncryptionResponse::echo_packet_handle());
        sheet.register_packet_handle(LoginPluginResponse::echo_packet_handle());
        sheet
    }

    crate::auto_string!(Username, 16);

    crate::packets! {
        LoginStart {
            field name: Username,

            mapping ProtocolVersion::V118R2 => 0x00 {
                for name = {packet -> &packet.name}
                    | auto as Username

                = deserializer {
                    Ok(Self {
                        name
                    })
                }
            }
        }

        EncryptionResponse {
            field shared_secret: (VarInt, Vec<u8>),
            field verify: (VarInt, Vec<u8>),

            mapping ProtocolVersion::V118R2 => 0x01 {
                for shared_secret = {packet -> &packet.shared_secret}
                    | auto as (VarInt, Vec<u8>)
                for verify = {packet -> &packet.verify}
                    | auto as (VarInt, Vec<u8>)

                = deserializer {
                    Ok(Self {
                        shared_secret,
                        verify,
                    })
                }
            }
        }

        LoginPluginResponse {
            field message_id: VarInt,
            field successful: bool,
            field data: Option<Vec<u8>>,

            mapping ProtocolVersion::V118R2 => 0x02 {
                for message_id = {packet -> &packet.message_id}
                    | auto as VarInt
                for successful = {packet -> &packet.successful}
                    | auto as bool
                for data = {packet -> &packet.data}
                    | auto as Option<Vec<u8>>, if (successful)

                = deserializer {
                    Ok(Self {
                        message_id,
                        successful,
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
        sheet.register_packet_handle(StatusRequest::echo_packet_handle());
        sheet.register_packet_handle(Ping::echo_packet_handle());
        sheet
    }

    crate::packets! {
        StatusRequest {
            mapping ProtocolVersion::V118R2 => 0x0 {
                = deserializer { Ok(Self {}) }
            }
        }

        Ping {
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

pub mod handshaking {
    use super::*;

    pub fn echo_sheet(version: ProtocolVersion) -> ProtocolSheet<NoContext> {
        let mut sheet = ProtocolSheet::new(version);
        sheet.register_packet_handle(Handshake::echo_packet_handle());
        sheet
    }

    crate::auto_string!(ServerAddress, 255);

    crate::auto_enum! {
        NextState by VarInt |* {
            1; Status {}
            2; Login {}
        }
    }

    crate::packets! {
        Handshake {
            field protocol_version: VarInt,
            field server_address: ServerAddress,
            field server_port: u16,
            field next_state: (VarInt, NextState),

            mapping ProtocolVersion::Handshake => 0x0 {
                for protocol_version = {packet -> &packet.protocol_version}
                    | auto as VarInt
                for server_address = {packet -> &packet.server_address}
                    | auto as ServerAddress
                for server_port = {packet -> &packet.server_port}
                    | auto as u16
                for next_state = {packet -> &packet.next_state}
                    | auto as (VarInt, NextState)

                = deserializer {
                    Ok(Self {
                        protocol_version,
                        server_address,
                        server_port,
                        next_state
                    })
                }
            }
        }
    }
}

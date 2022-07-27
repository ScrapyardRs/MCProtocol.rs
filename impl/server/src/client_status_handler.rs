use crate::client_connection::Connection;
use mc_registry::client_bound::status::{JSONResponse, Pong, Response};
use mc_registry::mappings::Mappings;
use mc_registry::registry::{arc_lock, LockedContext, StateRegistry};
use mc_registry::server_bound::status::{Ping, Request};
use mc_serializer::serde::ProtocolVersionSpec;
use std::sync::Arc;

#[derive(serde_derive::Serialize, Clone)]
pub struct PlayerSample {
    pub name: String,
    pub id: String,
}

pub struct StatusPart {
    // todo: support component stuff maybe in the future
    motd: String,
    max_players: i32,
    total_online: i32,
    player_sample: Vec<PlayerSample>,
    favicon: Option<String>,
}

#[derive(Default)]
pub struct StatusPartBuilder {
    motd: Option<String>,
    max_players: Option<i32>,
    total_online: Option<i32>,
    player_sample: Option<Vec<PlayerSample>>,
    favicon: Option<String>,
}

impl StatusPartBuilder {
    pub fn motd<S: Into<String>>(mut self, motd: S) -> StatusPartBuilder {
        self.motd = Some(motd.into());
        self
    }

    pub fn max_players(mut self, max_players: i32) -> StatusPartBuilder {
        self.max_players = Some(max_players);
        self
    }

    pub fn total_online(mut self, total_online: i32) -> StatusPartBuilder {
        self.total_online = Some(total_online);
        self
    }

    pub fn player_sample(mut self, player_sample: Vec<PlayerSample>) -> StatusPartBuilder {
        self.player_sample = Some(player_sample);
        self
    }

    pub fn favicon<S: Into<String>>(mut self, favicon: S) -> StatusPartBuilder {
        self.favicon = Some(favicon.into());
        self
    }
}

impl From<StatusPartBuilder> for StatusPart {
    fn from(builder: StatusPartBuilder) -> Self {
        StatusPart {
            max_players: builder.max_players.unwrap_or(0),
            total_online: builder.total_online.unwrap_or(0),
            motd: builder.motd.unwrap_or(String::from("Hello World!")),
            player_sample: builder.player_sample.unwrap_or_default(),
            favicon: builder.favicon,
        }
    }
}

#[derive(serde_derive::Serialize)]
struct VersionInfo {
    name: String,
    protocol: i32,
}

#[derive(serde_derive::Serialize)]
struct PlayerInfo {
    max: i32,
    online: i32,
    sample: Vec<PlayerSample>,
}

#[derive(serde_derive::Serialize)]
struct DescriptionInfo {
    text: String,
}

#[derive(serde_derive::Serialize)]
struct StatusResponseJson {
    version: VersionInfo,
    players: PlayerInfo,
    description: DescriptionInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    favicon: Option<String>,
    previews_chat: bool,
}

impl From<(ProtocolVersionSpec, StatusPart)> for StatusResponseJson {
    fn from((spec, part): (ProtocolVersionSpec, StatusPart)) -> Self {
        StatusResponseJson {
            version: VersionInfo {
                name: spec.1,
                protocol: spec.0,
            },
            players: PlayerInfo {
                max: part.max_players,
                online: part.total_online,
                sample: part.player_sample,
            },
            description: DescriptionInfo { text: part.motd },
            favicon: part.favicon,
            previews_chat: false, // todo handle this when true
        }
    }
}

struct StatusClientContext {
    connection: Connection,
    status_info: StatusResponseJson,
    complete: bool,
}

#[mc_registry_derive::packet_handler(Request)]
fn handle_status_request(context: LockedContext<StatusClientContext>) {
    let context_read = context.read().await;
    let response = &context_read.status_info;
    let packet_response = Response {
        json_response: JSONResponse::from(serde_json::to_string(response)?),
    };
    drop(context_read);
    let mut context_write = context.write().await;
    context_write
        .connection
        .send_packet(packet_response)
        .await?;
}

#[mc_registry_derive::packet_handler]
fn handle_ping(packet: Ping, context: LockedContext<StatusClientContext>) {
    let pong: Pong = packet.into();
    let mut context_write = context.write().await;
    context_write.connection.send_packet(pong).await?;
    context_write.complete = true;
}

pub async fn handle_status<SPB: Into<StatusPart>>(
    connection: Connection,
    part_builder: SPB,
) -> anyhow::Result<()> {
    let mut registry = StateRegistry::new(connection.connection_into().protocol_version());
    Request::attach_to_register(&mut registry, handle_status_request);
    Ping::attach_to_register(&mut registry, handle_ping);
    let registry = arc_lock(registry);

    let status_info = (
        connection.connection_into().protocol_version().to_spec(),
        part_builder.into(),
    )
        .into();
    let context = arc_lock(StatusClientContext {
        connection,
        status_info,
        complete: false,
    });

    while {
        let context_read = context.read().await;
        let complete = context_read.complete;
        drop(context_read);
        !complete
    } {
        let mut context_write = context.write().await;
        let next_packet = context_write.connection.read_packet().await?;
        drop(context_write);
        StateRegistry::emit(Arc::clone(&registry), Arc::clone(&context), next_packet).await?;
    }

    Ok(())
}

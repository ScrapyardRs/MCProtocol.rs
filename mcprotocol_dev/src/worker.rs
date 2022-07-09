use minecraft_buffer::readable_client::Client;
use minecraft_buffer::{write_locked, write_to_client};
use minecraft_registry::client_bound::status::{
    JSONResponse, Pong, PongMappings, Response, ResponseMappings,
};
use minecraft_registry::mappings::Mappings;
use minecraft_registry::packet_handlers;
use minecraft_registry::registry::{LockedContext, StateRegistry};
use minecraft_registry::server_bound::handshaking::NextState;
use minecraft_registry::server_bound::status::{PingMappings, RequestMappings};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

#[derive(serde_derive::Serialize)]
struct VersionStatusObject {
    name: String,
    protocol: i32,
}

#[derive(serde_derive::Serialize)]
struct PlayerSampleStatusObject {
    name: String,
    id: String,
}

#[derive(serde_derive::Serialize)]
struct PlayersStatusObject {
    max: i32,
    online: i32,
    sample: Vec<PlayerSampleStatusObject>,
}

#[derive(serde_derive::Serialize)]
struct DescriptionStatusObject {
    text: String,
}

#[derive(serde_derive::Serialize)]
struct StatusObject {
    version: VersionStatusObject,
    players: PlayersStatusObject,
    description: DescriptionStatusObject,
    #[serde(skip_serializing_if = "Option::is_none")]
    favicon: Option<String>,
}

struct ClientContext {
    client: Client,
    continue_read: bool,
}

packet_handlers! {
    fn status_request_handler<RequestMappings, ClientContext>(context, _registry, _request) {
        write_locked! { |context => unlocked_context| {
            let mut client = &mut unlocked_context.client;
            let protocol = client.protocol_version;
            let spec = protocol.to_spec();

            let response_object = StatusObject {
                version: VersionStatusObject {
                    name: String::from(&spec.1),
                    protocol: spec.0,
                },
                players: PlayersStatusObject {
                    max: 1,
                    online: 0,
                    sample: Vec::new(),
                },
                description: DescriptionStatusObject {
                    text: String::from("Hello world!"),
                },
                favicon: None,
            };

            let response = Response {
                json_response: JSONResponse::from(serde_json::to_string(&response_object)?),
            };
            write_to_client!(client, ResponseMappings, response);
        }}
    }

    fn ping_handler<PingMappings, ClientContext>(context, _registry, ping) {
        write_locked! { |context => unlocked_context| {
            let mut client = &mut unlocked_context.client;
            let pong = Pong {
                start_time: ping.start_time
            };
            write_to_client!(client, PongMappings, pong);
            unlocked_context.continue_read = false;
        }}
    }
}

async fn forward_status_handler(
    locked_context: LockedContext<ClientContext>,
) -> anyhow::Result<()> {
    let context_read = locked_context.read().await;
    let protocol_version = context_read.client.protocol_version;
    drop(context_read);

    let mut registry = StateRegistry::new(protocol_version);
    RequestMappings::attach_to_register(&mut registry, status_request_handler);
    PingMappings::attach_to_register(&mut registry, ping_handler);
    let locked_registry = Arc::new(RwLock::new(registry));

    while {
        let context_read = locked_context.read().await;
        let continue_read = context_read.continue_read;
        drop(context_read);
        continue_read
    } {
        let mut context_write = locked_context.write().await;
        let next_packet = context_write.client.read_packet().await?;
        drop(context_write);
        StateRegistry::emit(
            Arc::clone(&locked_registry),
            Arc::clone(&locked_context),
            next_packet,
        )
            .await?;
    }
    Ok(())
}

pub async fn attach_worker(stream: TcpStream, _addr: SocketAddr) -> anyhow::Result<()> {
    let (client, next_state) = Client::from_tcp_stream_basic(stream).await?;
    let context = ClientContext {
        client,
        continue_read: true,
    };
    let locked_context = Arc::new(RwLock::new(context));
    match next_state {
        NextState::Status => forward_status_handler(locked_context).await?,
        NextState::Login => (),
    }
    Ok(())
}

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::RwLock;

use minecraft_buffer::readable_client::Client;
use minecraft_buffer::write_to_client;
use minecraft_registry::client_bound::status::{
    JSONResponse, Pong, PongMappings, Response, ResponseMappings,
};
use minecraft_registry::mappings::Mappings;
use minecraft_registry::registry::{LockedContext, StateRegistry};
use minecraft_registry::server_bound::handshaking::NextState;
use minecraft_registry::server_bound::status::{PingMappings, RequestMappings};
use minecraft_registry_derive::packet_handler;

struct ClientContext {
    client: Client,
    continue_read: bool,
}

#[packet_handler(ClientContext, RequestMappings)]
fn status_request_handler(context: LockedContext<ClientContext>) {
    let mut unlocked = context.write().await;
    let mut client = &mut unlocked.client;
    let protocol = client.protocol_version;
    let (protocol, name) = protocol.to_spec();

    let json = serde_json::json! {
        {
            "version": {
                "name": name,
                "protocol": protocol
            },
            "players": {
                "max": 1,
                "online": 0,
                "sample": []
            },
            "description": {
                "text": "Hello World!"
            }
        }
    };
    let response = Response {
        json_response: JSONResponse::from(json.to_string()),
    };
    write_to_client!(client, ResponseMappings, response);
}

#[packet_handler(ClientContext, PingMappings)]
fn ping_handler(
    context: LockedContext<ClientContext>,
    packet: minecraft_registry::server_bound::status::Ping,
) {
    let mut unlocked = context.write().await;
    let mut client = &mut unlocked.client;
    let pong = Pong {
        start_time: packet.start_time,
    };
    write_to_client!(client, PongMappings, pong);
    unlocked.continue_read = false;
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

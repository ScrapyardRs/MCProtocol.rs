use mc_buffer::assign_key;
use mc_buffer::buffer::PacketWriter;
use mc_buffer::engine::{BufferRegistryEngine, BufferRegistryEngineContext, Context};
use mc_registry::client_bound::play::Ping;
use mc_registry::create_registry;
use mc_registry::registry::{arc_lock, LockedContext, StateRegistry};
use mc_registry::server_bound::handshaking::Handshake;
use mc_serializer::serde::ProtocolVersion;
use mc_serializer::serde::ProtocolVersion::Unknown;
use tokio::net::TcpStream;

assign_key!(HandshakeKey, Handshake);

#[allow(clippy::needless_lifetimes)]
#[mc_registry_derive::packet_handler]
async fn engine_handshake_handle<'registry>(
    packet: Handshake,
    mut context: LockedContext<Context<'registry>>,
) {
    Context::insert_data::<HandshakeKey>(context.clone(), packet).await;
    context.send_packet(Ping { id: 1 }).await?;
}

pub fn handshake_reg<'a>() -> StateRegistry<'a, BufferRegistryEngineContext<'a>> {
    create_registry! { reg, Unknown {
        Handshake, engine_handshake_handle;
    }};
    reg
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let stream = TcpStream::connect("localhost:25565").await?;

    let mut engine = BufferRegistryEngine::create(stream);

    create_registry! { handshake_registry, Unknown {
        Handshake, engine_handshake_handle;
    }};

    engine
        .read_packets_until(handshake_registry, |_, share| {
            share.contains::<HandshakeKey>()
        })
        .await?;

    Ok(())
}

use mc_buffer::assign_key;
use mc_buffer::buffer::PacketWriter;
use mc_buffer::engine::{BufferRegistryEngine, BufferRegistryEngineContext, Context};
use mc_registry::client_bound::play::Ping;
use mc_registry::create_registry;
use mc_registry::registry::{LockedContext, StateRegistry};
use mc_registry::server_bound::handshaking::{Handshake, NextState, ServerAddress};

use mc_registry::client_bound::login::EncryptionRequest;
use mc_registry::server_bound::login::LoginStart;
use mc_registry::shared_types::login::LoginUsername;
use mc_serializer::serde::ProtocolVersion;
use mc_serializer::serde::ProtocolVersion::Unknown;
use tokio::net::TcpStream;

assign_key!(HandshakeKey, Handshake);

#[allow(clippy::needless_lifetimes)]
#[mc_registry_derive::packet_handler]
async fn engine_encrypt<'registry>(
    packet: EncryptionRequest,
    mut context: LockedContext<Context<'registry>>,
) {
    println!("Received encryption request: {:?}", packet);
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let stream = TcpStream::connect("localhost:25565").await?;

    let mut engine = BufferRegistryEngine::create(stream);

    create_registry! { reg, Unknown {
        EncryptionRequest, engine_encrypt
    }};

    let proto = ProtocolVersion::V119_1;

    let handshake = Handshake {
        protocol_version: proto.into(),
        server_address: ServerAddress::from("localhost"),
        server_port: 25565,
        next_state: NextState::Login,
    };

    engine.send_packet(handshake).await?;
    engine
        .send_packet(LoginStart {
            name: LoginUsername::from("Testing"),
            sig_data: (false, None),
            sig_holder: (false, None),
        })
        .await?;

    engine
        .read_packets_until(reg, |unhandled, _| {
            println!(
                "PACKET {:?}",
                unhandled.map(|unh| (unh.packet_id, unh.bytes))
            );
            true // only read 1
        })
        .await?;

    Ok(())
}

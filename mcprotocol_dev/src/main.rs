use bytes::Buf;
use futures::future::BoxFuture;
use mc_buffer::buffer::{OwnedPacketBuffer, PacketBuffer};
use mc_buffer::encryption::{Compressor, Encrypt};
use mc_chat::Chat;
use mc_registry::client_bound::login::{LoginSuccess, SetCompression};
use mc_registry::client_bound::play::{JoinGame, Ping};
use mc_registry::mappings::Mappings;
use mc_registry::registry::{
    arc_lock, LockedContext, LockedStateRegistry, StateRegistry, UnhandledContext,
};
use mc_registry::server_bound::handshaking::{Handshake, NextState, ServerAddress};
use mc_registry::server_bound::login::LoginStart;
use mc_registry::server_bound::play::Pong;
use mc_registry::shared_types::login::LoginUsername;
use mc_serializer::serde::ProtocolVersion;
use std::io::Cursor;
use std::sync::Arc;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::tcp::WriteHalf;
use tokio::net::TcpStream;
use mc_serializer::ext::write_nbt;

struct Test {
    owned_write: OwnedWriteHalf,
}

pub type PacketWriterFuture<'a> = BoxFuture<'a, anyhow::Result<()>>;

pub trait PacketWriter<W: AsyncWrite + Unpin + Send + Sync>: Send + Sync {
    fn writer(&mut self) -> &mut W;

    fn compressor(&self) -> Option<&Compressor>;

    fn encrypt(&mut self, buffer: &mut Vec<u8>);

    fn protocol_version(&self) -> ProtocolVersion;

    fn send_packet<'a, Packet: Mappings<PacketType = Packet> + Send + Sync + 'a>(
        &'a mut self,
        packet: Packet,
    ) -> PacketWriterFuture<'a> {
        Box::pin(async move {
            let buffer = Packet::create_packet_buffer(self.protocol_version(), packet)?;

            let mut buffer = if let Some(compressor) = self.compressor() {
                compressor.compress(buffer)?
            } else {
                Compressor::uncompressed(buffer)?
            };

            self.encrypt(&mut buffer);

            let mut buffer = Cursor::new(buffer);

            while buffer.has_remaining() {
                self.writer().write_buf(&mut buffer).await?;
            }
            Ok(())
        })
    }
}

impl PacketWriter<OwnedWriteHalf> for Test {
    fn writer(&mut self) -> &mut OwnedWriteHalf {
        &mut self.owned_write
    }

    fn compressor(&self) -> Option<&Compressor> {
        None
    }

    #[inline]
    fn encrypt(&mut self, _: &mut Vec<u8>) {}

    fn protocol_version(&self) -> ProtocolVersion {
        ProtocolVersion::V119_1
    }
}

#[mc_registry_derive::packet_handler]
fn handle_login_success(
    packet: LoginSuccess,
    _context: LockedContext<Test>,
    registry: LockedStateRegistry<Test>,
) {
    println!("Login Success! {:?}", packet);
    let mut lock = registry.write().await;
    lock.clear_mappings();
    JoinGame::attach_to_register(&mut lock, handle_join_game);
    Ping::attach_to_register(&mut lock, handle_ping);
}

#[mc_registry_derive::packet_handler]
fn handle_set_compression(packet: SetCompression, _context: LockedContext<Test>) {
    println!("Set Compression! {:?}", packet);
}

#[mc_registry_derive::packet_handler]
fn handle_join_game(packet: JoinGame, _context: LockedContext<Test>) {
    let mut bytes = Vec::new();
    write_nbt(&packet.codec, &mut bytes, ProtocolVersion::V119_1)?;
    println!("Packet: {:#?}", packet);
}

#[mc_registry_derive::packet_handler]
fn handle_ping(packet: Ping, context: LockedContext<Test>) {
    println!("Server ping.");
    let mut lock = context.write().await;
    lock.send_packet(Pong { id: packet.id }).await?;
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut registry = StateRegistry::new(ProtocolVersion::V119_1);
    LoginSuccess::attach_to_register(&mut registry, handle_login_success);
    SetCompression::attach_to_register(&mut registry, handle_set_compression);
    let registry = arc_lock(registry);

    let connection = TcpStream::connect("localhost:25565").await?;
    let (read, write) = connection.into_split();

    let mut packet_buffer = OwnedPacketBuffer::new(read);
    let mut context = Test { owned_write: write };

    let handshake = Handshake {
        protocol_version: ProtocolVersion::V119_1.get_protocol_id().into(),
        server_address: ServerAddress::from("localhost"),
        server_port: 25565,
        next_state: NextState::Login,
    };
    let login_start = LoginStart {
        name: LoginUsername::from("KekW"),
        sig_data: (false, None),
        sig_holder: (false, None),
    };

    context.send_packet(handshake).await?;
    context.send_packet(login_start).await?;

    let context = arc_lock(context);

    loop {
        let next_packet = packet_buffer.loop_read().await?;
        match StateRegistry::emit(
            Arc::clone(&registry),
            Arc::clone(&context),
            Cursor::new(next_packet),
        )
        .await?
        {
            None => {}
            Some(unhandled) => {
                println!(
                    "Received packet ID {} of size {}",
                    unhandled.packet_id,
                    unhandled.bytes.len()
                );
            }
        }
    }
}

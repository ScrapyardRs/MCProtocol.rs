use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;
use bytes::Buf;
use encryption_utils::MCPrivateKey;
use mc_buffer::buffer::MinecraftPacketBuffer;
use mc_buffer::encryption::{Codec, Compressor, Encrypt};
use mc_registry::mappings::Mappings;
use mc_registry::registry::{arc_lock, LockedContext, StateRegistry, StateRegistryHandle};
use mc_registry::server_bound::handshaking::{Handshake, NextState, ServerAddress};
use mc_serializer::primitive::VarInt;
use mc_serializer::serde::ProtocolVersion;
use crate::client_status_handler::StatusPart;

pub struct Connection {
    server_key: Arc<MCPrivateKey>,
    socket_address: SocketAddr,
    connection_info: InitialConnectionInfo,
    write_half: OwnedWriteHalf,
    packet_buffer: MinecraftPacketBuffer,
    compressor: Option<Compressor>,
    encryption: Option<Encrypt>,
}

impl Connection {
    pub fn socket_address(&self) -> SocketAddr {
        self.socket_address.clone()
    }

    pub fn connection_into(&self) -> &InitialConnectionInfo {
        &self.connection_info
    }

    pub fn server_key(&self) -> Arc<MCPrivateKey> { Arc::clone(&self.server_key) }
}

#[derive(Default, Debug)]
struct ConnectionPart {
    next_state: Option<NextState>,
    protocol_version: Option<ProtocolVersion>,
    virtual_host: Option<ServerAddress>,
    virtual_port: Option<u16>,
}

pub struct InitialConnectionInfo {
    next_state: NextState,
    protocol_version: ProtocolVersion,
    virtual_host: ServerAddress,
    virtual_port: u16,
}

impl InitialConnectionInfo {
    pub fn next_state(&self) -> NextState {
        self.next_state
    }

    pub fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    pub fn virtual_host(&self) -> ServerAddress {
        ServerAddress::from(self.virtual_host.to_string())
    }

    pub fn virtual_port(&self) -> u16 {
        self.virtual_port
    }
}

#[mc_registry_derive::packet_handler]
fn handle_handshake(context: LockedContext<ConnectionPart>, packet: Handshake) {
    let mut writeable_context = context.write().await;
    writeable_context.next_state = Some(packet.next_state);
    writeable_context.protocol_version = Some(ProtocolVersion::from(packet.protocol_version));
    writeable_context.virtual_host = Some(packet.server_address);
    writeable_context.virtual_port = Some(packet.server_port);
}

impl Connection {
    pub async fn send_packet<Packet: Mappings<PacketType=Packet>>(&mut self, packet: Packet) -> anyhow::Result<()> {
        let buffer = Packet::create_packet_buffer(self.connection_info.protocol_version, packet)?;

        let mut buffer = if let Some(compressor) = self.compressor.as_ref() {
            compressor.compress(buffer)?
        } else {
            Compressor::uncompressed(buffer)?
        };

        if let Some(encryptor) = self.encryption.as_mut() {
            encryptor.encrypt(&mut buffer);
        }

        let mut buffer = Cursor::new(buffer);

        while buffer.has_remaining() {
            self.write_half.write_buf(&mut buffer).await?;
        }
        Ok(())
    }

    pub async fn read_packet(&mut self) -> anyhow::Result<Cursor<Vec<u8>>> {
        let next = self.packet_buffer.loop_read().await?;

        let buffer = if let Some(compressor) = self.compressor.as_ref() {
            compressor.decompress(next)?
        } else {
            next
        };

        Ok(Cursor::new(buffer))
    }

    pub fn enable_crypt(&mut self, crypt: (Codec, Codec)) {
        let (encryption, decryption) = crypt;
        self.packet_buffer.enable_decryption(decryption);
        self.encryption = Some(Encrypt::new(encryption));
    }

    pub fn enable_compression(&mut self, threshold: VarInt) {
        self.compressor = Some(Compressor::new(threshold));
    }

    pub async fn handle_status_with_data<SPB: Into<StatusPart>>(self, spb: SPB) -> anyhow::Result<()> {
        crate::client_status_handler::handle_status(self, spb).await
    }

    pub async fn from_initial_connection(socket_address: SocketAddr, stream: TcpStream, server_key: Arc<MCPrivateKey>) -> anyhow::Result<Connection> {
        let (read_half, write_half) = stream.into_split();
        let mut packet_buffer = MinecraftPacketBuffer::new(read_half);

        let mut registry = StateRegistry::new(ProtocolVersion::Handshake);

        Handshake::attach_to_register(
            &mut registry,
            handle_handshake as StateRegistryHandle<ConnectionPart>,
        );

        let registry_lock = arc_lock(registry);

        let context = Default::default();
        let context_lock = arc_lock(context);

        StateRegistry::emit(
            registry_lock,
            Arc::clone(&context_lock),
            Cursor::new(packet_buffer.loop_read().await?),
        ).await?;

        let context_read = context_lock.read().await;

        if context_read.protocol_version == Some(ProtocolVersion::Unknown) {
            anyhow::bail!("Could not recognize protocol version from value.");
        }
        let state = InitialConnectionInfo {
            next_state: context_read.next_state.unwrap(),
            protocol_version: context_read.protocol_version.unwrap(),
            virtual_host: context_read.virtual_host.as_ref().unwrap().clone(),
            virtual_port: context_read.virtual_port.unwrap(),
        };

        Ok(
            Connection {
                server_key,
                socket_address,
                connection_info: state,
                write_half,
                packet_buffer,
                compressor: None,
                encryption: None,
            },
        )
    }
}
use crate::buffer::MinecraftPacketBuffer;
use crate::encryption::{Aes128Cfb8Dec, Aes128Cfb8Enc};
use aes::cipher::BlockEncryptMut;
use bytes::Buf;
use flate2::bufread::{ZlibDecoder, ZlibEncoder};
use flate2::Compression;
use minecraft_registry::mappings::Mappings;
use minecraft_registry::registry::StateRegistry;
use minecraft_registry::server_bound::handshaking::{Handshake, HandshakeMappings, NextState};
use minecraft_serde::primitive::VarInt;
use minecraft_serde::serde::{ProtocolVersion, Serialize};
use std::io::{Cursor, Read};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

struct Compressor {
    threshold: i32,
}

impl Compressor {
    fn compress(&self, mut packet: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let initial_size = packet.len();
        if initial_size >= self.threshold as usize {
            let initial_size = VarInt::try_from(initial_size)?;
            let mut encoder = ZlibEncoder::new(packet.as_slice(), Compression::default());
            let mut compressed = Vec::new();
            encoder.read_to_end(&mut compressed)?;

            let compressed_length = VarInt::try_from(compressed.len())?;
            let uncompressed_length_length = initial_size.size()?;
            let total_length_data = compressed_length + VarInt::from(uncompressed_length_length);

            let mut result = Cursor::new(Vec::with_capacity(
                (total_length_data.size()? + uncompressed_length_length).try_into()?,
            ));
            total_length_data.serialize(&mut result)?;
            initial_size.serialize(&mut result)?;

            let mut inner = result.into_inner();
            inner.append(&mut compressed);
            Ok(inner)
        } else {
            let initial_size = VarInt::try_from(initial_size)?;
            let total_length_data = VarInt::from(1i32 /* 0 = 1 byte */) + initial_size;

            let mut result = Vec::with_capacity((total_length_data.size()?).try_into()?);
            total_length_data.serialize(&mut result)?;
            VarInt::from(0).serialize(&mut result)?;
            result.append(&mut packet);
            Ok(result)
        }
    }

    fn decompress(&self, packet: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let mut cursor = Cursor::new(packet);
        let (decompressed_length_size, decompressed_length) = VarInt::decode_and_size(&mut cursor)?;
        let remaining_bytes = &cursor.into_inner()[decompressed_length_size.try_into()?..];

        Ok(if decompressed_length == 0 {
            Vec::from(remaining_bytes)
        } else {
            let mut target = Vec::with_capacity(decompressed_length.try_into()?);
            ZlibDecoder::new(remaining_bytes).read_to_end(&mut target)?;
            target
        })
    }

    fn uncompressed(mut packet: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let length = VarInt::try_from(packet.len())?;
        let length_size = length.size()?;
        let mut writer = Vec::with_capacity((length + length_size).try_into()?);
        length.serialize(&mut writer)?;
        writer.append(&mut packet);
        Ok(writer)
    }
}

struct Encryptor {
    encryption: Aes128Cfb8Enc,
}

impl Encryptor {
    fn encrypt(&mut self, packet: &mut Vec<u8>) {
        let slice = packet.as_mut_slice();
        self.encryption.encrypt_block_mut(slice.into());
    }
}

#[derive(Default)]
struct HandshakeStateStore {
    state: Option<Handshake>,
}

minecraft_registry::packet_handlers! {
    fn handshake_handler<HandshakeMappings, HandshakeStateStore>(version_store, _registry, handshake) {
        let mut x = version_store.write().await;
        x.state = Some(handshake);
    }
}

pub struct Client {
    write_half: OwnedWriteHalf,
    packet_buffer: MinecraftPacketBuffer,
    pub protocol_version: ProtocolVersion,
    compressor: Option<Compressor>,
    encryption: Option<Encryptor>,
}

#[macro_export]
macro_rules! read_locked {
    (|$locked:ident => $unlocked:ident| {
        $($function_tokens:tt)+
    }) => {
        let $unlocked = $locked.read().await;
        $($function_tokens)+
        drop($unlocked);
    }
}

#[macro_export]
macro_rules! write_locked {
    (|$locked:ident => $unlocked:ident| {
        $($function_tokens:tt)+
    }) => {
        let mut $unlocked = $locked.write().await;
        $($function_tokens)+
        drop($unlocked);
    }
}

#[macro_export]
macro_rules! write_to_client (($client:ident, $mappings:ty, $value:ident) => ($crate::readable_client::Client::send_packet::<$mappings>(&mut $client, $value).await?));

impl Client {
    pub async fn send_packet<M: Mappings>(&mut self, packet: M::PacketType) -> anyhow::Result<()> {
        let buffer = M::create_packet_buffer(self.protocol_version, packet)?;

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

    pub fn enable_crypt(&mut self, crypt: (Aes128Cfb8Enc, Aes128Cfb8Dec)) {
        let (encryption, decryption) = crypt;
        self.packet_buffer.enable_decryption(decryption);
        self.encryption = Some(Encryptor { encryption });
    }

    pub fn enable_compression(&mut self, threshold: i32) {
        self.compressor = Some(Compressor { threshold });
    }

    pub async fn from_tcp_stream_basic(stream: TcpStream) -> anyhow::Result<(Client, NextState)> {
        let (read_half, write_half) = stream.into_split();
        let mut packet_buffer = MinecraftPacketBuffer::new(read_half);

        let mut registry = StateRegistry::new(ProtocolVersion::Handshake);

        HandshakeMappings::attach_to_register(&mut registry, handshake_handler);

        let registry_lock = Arc::new(RwLock::new(registry));

        let context = Default::default();
        let context_lock = Arc::new(RwLock::new(context));

        StateRegistry::emit(
            registry_lock,
            Arc::clone(&context_lock),
            Cursor::new(packet_buffer.loop_read().await?),
        )
        .await?;

        let context_read = context_lock.read().await;
        let state = context_read.state.as_ref().unwrap();

        Ok((
            Client {
                write_half,
                packet_buffer,
                protocol_version: ProtocolVersion::from(state.protocol_version.clone()),
                compressor: None,
                encryption: None,
            },
            state.next_state,
        ))
    }
}

use crate::encryption::{Codec, Compressor};
use anyhow::Context;
use bytes::{Buf, BufMut, BytesMut};
use mc_registry::mappings::Mappings;
use mc_serializer::primitive::VarInt;
use mc_serializer::serde::ProtocolVersion;

use std::io::Cursor;

use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::RwLock;

const BUFFER_CAPACITY: usize = 2097154; // static value from wiki.vg

pub type PacketFuture<'a, T> = futures::future::BoxFuture<'a, anyhow::Result<T>>;
pub type RawFuture<'a, T> = futures::future::BoxFuture<'a, T>;

pub enum BufferState {
    Waiting,
    PacketReady,
    Error(String),
}

impl<T: PacketWriter> PacketWriter for Arc<RwLock<T>> {
    fn send_packet<'a, Packet: Mappings<PacketType = Packet> + Send + Sync + 'a>(
        &'a mut self,
        packet: Packet,
    ) -> PacketFuture<'a, ()> {
        let self_clone = Arc::clone(self);
        Box::pin(async move {
            let mut write = self_clone.write().await;
            write.send_packet(packet).await
        })
    }
}

pub trait PacketReader: Send + Sync {
    fn poll(&mut self) -> PacketFuture<BufferState>;

    fn loop_read(&mut self) -> PacketFuture<Vec<u8>>;
}

pub trait PacketReaderGeneric: PacketReader {
    fn read(&mut self) -> PacketFuture<usize>;

    fn bytes(&self) -> &BytesMut;

    fn bytes_mut(&mut self) -> &mut BytesMut;

    fn decoded(&self) -> &BytesMut;

    fn decoded_mut(&mut self) -> &mut BytesMut;

    fn decryption_mut(&mut self) -> Option<&mut Codec>;

    fn compression(&self) -> Option<&Compressor>;

    fn construct_all(&mut self) -> (&mut BytesMut, &mut BytesMut, Option<&mut Codec>);

    fn len(&self) -> (usize, usize) {
        (self.bytes().len(), self.decoded().len())
    }

    fn capacity(&self) -> (usize, usize) {
        (self.bytes().remaining(), self.decoded().remaining())
    }

    fn is_empty(&self) -> bool {
        self.bytes().is_empty() && self.decoded().is_empty()
    }

    fn is_packet_available(&self) -> bool {
        let mut cursor: Cursor<&[u8]> = Cursor::new(self.decoded().chunk());

        if let Ok((size, length)) = VarInt::decode_and_size(&mut cursor) {
            (length + size) <= self.decoded().len()
        } else {
            false
        }
    }
}

pub trait PacketWriter: Send + Sync {
    fn send_packet<'a, Packet: Mappings<PacketType = Packet> + Send + Sync + 'a>(
        &'a mut self,
        packet: Packet,
    ) -> PacketFuture<'a, ()>;
}

pub trait PacketWriterGeneric: PacketWriter {
    fn writer(&mut self) -> &mut OwnedWriteHalf;

    fn compression(&self) -> Option<&Compressor>;

    fn encrypt(&mut self, buffer: &mut Vec<u8>);

    fn protocol_version(&self) -> ProtocolVersion;
}

impl<T: PacketReaderGeneric> PacketReader for T {
    fn poll(&mut self) -> PacketFuture<BufferState> {
        Box::pin(async move {
            if self.is_packet_available() {
                return Ok(BufferState::PacketReady);
            }

            let size_read = match tokio::time::timeout(Duration::from_secs(10), self.read()).await {
                Ok(result) => result?,
                Err(_) => return Ok(BufferState::Error(String::from("Client read timeout."))),
            };

            let size_read = size_read.min(self.decoded().capacity() - self.decoded().len());

            if size_read == 0 {
                return Ok(if self.is_packet_available() {
                    BufferState::PacketReady
                } else if self.decoded().capacity() == self.decoded().len() {
                    log::error!(
                        "Packet too big! Failed at: Capacity {}, length {}",
                        self.decoded().capacity(),
                        self.decoded().len()
                    );
                    BufferState::Error(String::from(
                        "Next packet was too big to decode, something went wrong.",
                    ))
                } else if self.len() == (0, 0) {
                    BufferState::Error(String::from("Read sink empty."))
                } else {
                    BufferState::Waiting
                });
            }

            let (bytes, decoded, decryption) = self.construct_all();

            let read_half = bytes.chunks_mut(size_read).next().unwrap();

            if let Some(decryption) = decryption {
                decryption.decrypt(read_half);
            }

            decoded.put_slice(read_half);
            log::info!("IN: {}", size_read);

            bytes.advance(size_read);
            bytes.reserve(BUFFER_CAPACITY - bytes.len());

            Ok(if self.is_packet_available() {
                BufferState::PacketReady
            } else {
                BufferState::Waiting
            })
        })
    }

    fn loop_read(&mut self) -> PacketFuture<Vec<u8>> {
        Box::pin(async move {
            loop {
                match self.poll().await? {
                    BufferState::PacketReady => {
                        let mut cursor = Cursor::new(self.decoded().chunk());
                        let (length_size, length) = VarInt::decode_and_size(&mut cursor)?;

                        self.decoded_mut().advance(length_size.try_into()?);
                        let cursor: Vec<u8> = self
                            .decoded_mut()
                            .chunks(length.try_into()?)
                            .next()
                            .unwrap()
                            .to_vec();

                        self.decoded_mut().advance(length.try_into()?);
                        let len = self.decoded().len();
                        self.decoded_mut().reserve(BUFFER_CAPACITY - len);

                        let cursor = match self.compression() {
                            None => cursor,
                            Some(compressor) => compressor.decompress(cursor)?,
                        };

                        log::info!("OUT: {} <len: {:?}>", length + length_size, self.len());

                        return Ok(cursor);
                    }
                    BufferState::Error(buffer_error) => anyhow::bail!(buffer_error),
                    _ => (),
                }
            }
        })
    }
}

impl<T: PacketWriterGeneric> PacketWriter for T {
    fn send_packet<'a, Packet: Mappings<PacketType = Packet> + Send + Sync + 'a>(
        &'a mut self,
        packet: Packet,
    ) -> PacketFuture<'a, ()> {
        Box::pin(async move {
            let buffer = Packet::create_packet_buffer(self.protocol_version(), packet)?;

            let mut buffer = if let Some(compressor) = self.compression() {
                compressor.compress(buffer)?
            } else {
                Compressor::uncompressed(buffer)?
            };

            self.encrypt(&mut buffer);

            self.writer().write_all(&buffer).await?;
            Ok(())
        })
    }
}

pub struct BorrowedPacketReader<'a> {
    owned_ref: &'a mut OwnedPacketReader,
}

impl<'a> PacketReaderGeneric for BorrowedPacketReader<'a> {
    fn read(&mut self) -> PacketFuture<usize> {
        self.owned_ref.read()
    }

    fn bytes(&self) -> &BytesMut {
        self.owned_ref.bytes()
    }

    fn bytes_mut(&mut self) -> &mut BytesMut {
        self.owned_ref.bytes_mut()
    }

    fn decoded(&self) -> &BytesMut {
        self.owned_ref.decoded()
    }

    fn decoded_mut(&mut self) -> &mut BytesMut {
        self.owned_ref.decoded_mut()
    }

    fn decryption_mut(&mut self) -> Option<&mut Codec> {
        self.owned_ref.decryption_mut()
    }

    fn compression(&self) -> Option<&Compressor> {
        self.owned_ref.compression()
    }

    fn construct_all(&mut self) -> (&mut BytesMut, &mut BytesMut, Option<&mut Codec>) {
        self.owned_ref.construct_all()
    }
}

pub struct OwnedPacketReader {
    read_half: OwnedReadHalf,
    bytes: BytesMut,
    decoded: BytesMut,
    decryption: Option<Codec>,
    compressor: Option<Compressor>,
}

impl PacketReaderGeneric for OwnedPacketReader {
    fn read(&mut self) -> PacketFuture<usize> {
        Box::pin(async move {
            self.read_half
                .read_buf(&mut self.bytes)
                .await
                .context("Failed to read buf.")
        })
    }
    fn bytes(&self) -> &BytesMut {
        &self.bytes
    }
    fn bytes_mut(&mut self) -> &mut BytesMut {
        &mut self.bytes
    }
    fn decoded(&self) -> &BytesMut {
        &self.decoded
    }
    fn decoded_mut(&mut self) -> &mut BytesMut {
        &mut self.decoded
    }
    fn decryption_mut(&mut self) -> Option<&mut Codec> {
        self.decryption.as_mut()
    }

    fn compression(&self) -> Option<&Compressor> {
        self.compressor.as_ref()
    }

    fn construct_all(&mut self) -> (&mut BytesMut, &mut BytesMut, Option<&mut Codec>) {
        (&mut self.bytes, &mut self.decoded, self.decryption.as_mut())
    }
}

impl OwnedPacketReader {
    pub fn new(read_half: OwnedReadHalf) -> Self {
        Self {
            read_half,
            bytes: BytesMut::with_capacity(BUFFER_CAPACITY),
            decoded: BytesMut::with_capacity(BUFFER_CAPACITY),
            decryption: None,
            compressor: None,
        }
    }

    pub fn enable_decryption(&mut self, codec: Codec) {
        self.decryption = Some(codec);
    }

    pub fn enable_compression(&mut self, compressor: Compressor) {
        self.compressor = Some(compressor);
    }

    pub fn borrow_buffer(&mut self) -> BorrowedPacketReader {
        BorrowedPacketReader { owned_ref: self }
    }
}

pub struct BorrowedPacketWriter<'a> {
    owned_ref: &'a mut OwnedPacketWriter,
}

impl<'a> PacketWriterGeneric for BorrowedPacketWriter<'a> {
    fn writer(&mut self) -> &mut OwnedWriteHalf {
        self.owned_ref.writer()
    }

    fn compression(&self) -> Option<&Compressor> {
        self.owned_ref.compression()
    }

    fn encrypt(&mut self, buffer: &mut Vec<u8>) {
        self.owned_ref.encrypt(buffer)
    }

    fn protocol_version(&self) -> ProtocolVersion {
        self.owned_ref.protocol_version()
    }
}

pub struct OwnedPacketWriter {
    write_half: OwnedWriteHalf,
    protocol_version: ProtocolVersion,
    encryption: Option<Codec>,
    compressor: Option<Compressor>,
}

impl PacketWriterGeneric for OwnedPacketWriter {
    fn writer(&mut self) -> &mut OwnedWriteHalf {
        &mut self.write_half
    }

    fn compression(&self) -> Option<&Compressor> {
        self.compressor.as_ref()
    }

    fn encrypt(&mut self, buffer: &mut Vec<u8>) {
        if let Some(decryption) = self.encryption.as_mut() {
            decryption.encrypt(buffer)
        }
    }

    fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }
}

impl OwnedPacketWriter {
    pub fn new(write_half: OwnedWriteHalf) -> Self {
        Self {
            protocol_version: ProtocolVersion::Handshake,
            write_half,
            encryption: None,
            compressor: None,
        }
    }

    pub fn enabled_encryption(&mut self, codec: Codec) {
        self.encryption = Some(codec);
    }

    pub fn enabled_compression(&mut self, compressor: Compressor) {
        self.compressor = Some(compressor);
    }

    pub fn borrow_buffer(&mut self) -> BorrowedPacketWriter {
        BorrowedPacketWriter { owned_ref: self }
    }

    pub fn update_protocol_version(&mut self, new_protocol_version: ProtocolVersion) {
        self.protocol_version = new_protocol_version;
    }
}

impl From<OwnedReadHalf> for OwnedPacketReader {
    fn from(read: OwnedReadHalf) -> Self {
        Self::new(read)
    }
}

impl From<OwnedWriteHalf> for OwnedPacketWriter {
    fn from(write: OwnedWriteHalf) -> Self {
        Self::new(write)
    }
}

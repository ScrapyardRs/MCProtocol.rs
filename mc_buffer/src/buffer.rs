use bytes::{Buf, BufMut, BytesMut};
use mc_serializer::primitive::VarInt;
use std::io::Cursor;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::tcp::OwnedReadHalf;
use crate::encryption::Decrypt;

pub struct MinecraftPacketBuffer {
    read_half: OwnedReadHalf,
    bytes: BytesMut,
    decoded: BytesMut,
    decryption: Option<Decrypt>,
}

const BUFFER_CAPACITY: usize = 2097154; // static value from wiki.vg

pub enum BufferState {
    Waiting,
    PacketReady,
    Error(String),
}

impl MinecraftPacketBuffer {
    pub fn new(read_half: OwnedReadHalf) -> Self {
        MinecraftPacketBuffer {
            read_half,
            bytes: BytesMut::with_capacity(BUFFER_CAPACITY),
            decoded: BytesMut::with_capacity(BUFFER_CAPACITY),
            decryption: None,
        }
    }

    pub fn len(&self) -> (usize, usize) {
        (self.bytes.len(), self.decoded.len())
    }

    pub fn enable_decryption(&mut self, codec: crate::encryption::Codec) {
        self.decryption = Some(Decrypt::new(codec));
    }

    fn is_packet_available(&self) -> bool {
        let mut cursor: Cursor<&[u8]> = Cursor::new(self.decoded.chunk());

        if let Ok((size, length)) = VarInt::decode_and_size(&mut cursor) {
            (length + size) <= self.decoded.len()
        } else {
            false
        }
    }

    pub async fn poll(&mut self) -> anyhow::Result<BufferState> {
        if self.is_packet_available() {
            return Ok(BufferState::PacketReady);
        }

        let size_read = match tokio::time::timeout(
            Duration::from_secs(10),
            self.read_half.read_buf(&mut self.bytes),
        )
            .await
        {
            Ok(result) => result?,
            Err(_) => {
                return Ok(BufferState::Error(String::from("Client read timeout.")));
            }
        }
            .min(self.decoded.capacity() - self.decoded.len());

        if size_read == 0 {
            return Ok(if self.is_packet_available() {
                BufferState::PacketReady
            } else if self.decoded.capacity() == self.decoded.len() {
                log::error!(
                    "Packet too big! Failed at: Capacity {}, length {}",
                    self.decoded.capacity(),
                    self.decoded.len()
                );
                BufferState::Error(String::from(
                    "Next packet was too big to decode, something went wrong.",
                ))
            } else {
                BufferState::Waiting
            });
        }

        let read_half = self.bytes.chunks_mut(size_read).next().unwrap();

        if let Some(decryption) = &mut self.decryption {
            decryption.decrypt(read_half.into());
        }

        self.decoded.put_slice(read_half);

        self.bytes.advance(size_read);
        self.bytes.reserve(BUFFER_CAPACITY - self.bytes.len());

        Ok(if self.is_packet_available() {
            BufferState::PacketReady
        } else {
            BufferState::Waiting
        })
    }

    pub async fn loop_read(&mut self) -> anyhow::Result<Vec<u8>> {
        loop {
            match self.poll().await? {
                BufferState::PacketReady => {
                    let mut cursor = Cursor::new(self.decoded.chunk());
                    let (length_size, length) = VarInt::decode_and_size(&mut cursor)?;
                    self.decoded.advance(length_size.try_into()?);
                    let cursor: Vec<u8> = self
                        .decoded
                        .chunks(length.try_into()?)
                        .next()
                        .unwrap()
                        .to_vec();

                    self.decoded.advance(length.try_into()?);
                    self.decoded.reserve(BUFFER_CAPACITY - self.decoded.len());
                    return Ok(cursor);
                }
                BufferState::Error(buffer_error) => anyhow::bail!(buffer_error),
                _ => (),
            }
        }
    }
}

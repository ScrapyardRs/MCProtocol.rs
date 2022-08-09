use crate::encryption::Decrypt;
use bytes::{Buf, BufMut, BytesMut};
use mc_serializer::primitive::VarInt;
use std::io::Cursor;
use std::time::Duration;
use anyhow::Context;
use tokio::io::AsyncReadExt;
use tokio::net::tcp::ReadHalf;
use tokio::net::tcp::OwnedReadHalf;

const BUFFER_CAPACITY: usize = 2097154; // static value from wiki.vg

pub type PacketBufferFuture<'a, T> = futures::future::BoxFuture<'a, anyhow::Result<T>>;

pub enum BufferState {
    Waiting,
    PacketReady,
    Error(String),
}

pub trait PacketBuffer: Send + Sync {
    fn read(&mut self) -> PacketBufferFuture<usize>;

    fn bytes(&self) -> &BytesMut;

    fn bytes_mut(&mut self) -> &mut BytesMut;

    fn decoded(&self) -> &BytesMut;

    fn decoded_mut(&mut self) -> &mut BytesMut;

    fn decryption_mut(&mut self) -> Option<&mut Decrypt>;

    fn construct_all(&mut self) -> (&mut BytesMut, &mut BytesMut, Option<&mut Decrypt>);

    fn len(&self) -> (usize, usize) {
        (self.bytes().len(), self.decoded().len())
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

    fn poll(&mut self) -> PacketBufferFuture<BufferState> {
        Box::pin(async move {
            if self.is_packet_available() {
                return Ok(BufferState::PacketReady);
            }

            let size_read = match tokio::time::timeout(Duration::from_secs(10), self.read()).await {
                Ok(result) => result?,
                Err(_) => return Ok(BufferState::Error(String::from("Client read timeout."))),
            }.min(self.decoded().capacity() - self.decoded().len());

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
                } else {
                    BufferState::Waiting
                });
            }

            let (bytes, decoded, decryption) = self.construct_all();

            let read_half = bytes.chunks_mut(size_read).next().unwrap();

            if let Some(decryption) = decryption { decryption.decrypt(read_half); }

            decoded.put_slice(read_half);

            bytes.advance(size_read);
            bytes.reserve(BUFFER_CAPACITY - bytes.len());

            Ok(if self.is_packet_available() {
                BufferState::PacketReady
            } else {
                BufferState::Waiting
            })
        })
    }

    fn loop_read(&mut self) -> PacketBufferFuture<Vec<u8>> {
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
                        return Ok(cursor);
                    }
                    BufferState::Error(buffer_error) => anyhow::bail!(buffer_error),
                    _ => (),
                }
            }
        })
    }
}

macro_rules! buffer_impl {
    ($obj:ident$(<$lifetime:lifetime>)?) => {
        impl$(<$lifetime>)? PacketBuffer for $obj$(<$lifetime>)? {
            fn read<'buffer_read_impl>(&'buffer_read_impl mut self) -> PacketBufferFuture<'buffer_read_impl, usize> {
                Box::pin(async move { self.read_half.read_buf(&mut self.bytes).await.context("Failed to read buf.") })
            }
            fn bytes(&self) -> &BytesMut { &self.bytes }
            fn bytes_mut(&mut self) -> &mut BytesMut { &mut self.bytes }
            fn decoded(&self) -> &BytesMut { &self.decoded }
            fn decoded_mut(&mut self) -> &mut BytesMut { &mut self.decoded }
            fn decryption_mut(&mut self) -> Option<&mut Decrypt> { self.decryption.as_mut() }
            fn construct_all(&mut self) -> (&mut BytesMut, &mut BytesMut, Option<&mut Decrypt>) {
                (&mut self.bytes, &mut self.decoded, self.decryption.as_mut())
            }
        }
    }
}

pub struct BorrowedPacketBuffer<'a> {
    read_half: ReadHalf<'a>,
    bytes: BytesMut,
    decoded: BytesMut,
    decryption: Option<Decrypt>,
}

pub struct OwnedPacketBuffer {
    read_half: OwnedReadHalf,
    bytes: BytesMut,
    decoded: BytesMut,
    decryption: Option<Decrypt>,
}

buffer_impl!(BorrowedPacketBuffer<'a>);
buffer_impl!(OwnedPacketBuffer);

impl<'a> BorrowedPacketBuffer<'a> {
    pub fn new(read_half: ReadHalf<'a>) -> Self {
        Self {
            read_half,
            bytes: BytesMut::with_capacity(BUFFER_CAPACITY),
            decoded: BytesMut::with_capacity(BUFFER_CAPACITY),
            decryption: None,
        }
    }

    pub fn enable_decryption(&mut self, codec: crate::encryption::Codec) {
        self.decryption = Some(Decrypt::new(codec));
    }

    pub fn into_owned_buffer(self, owned_half: OwnedReadHalf) -> OwnedPacketBuffer {
        let BorrowedPacketBuffer { bytes, decoded, decryption, .. } = self;
        OwnedPacketBuffer {
            read_half: owned_half,
            bytes,
            decoded,
            decryption,
        }
    }
}

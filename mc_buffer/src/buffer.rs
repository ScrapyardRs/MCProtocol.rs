use crate::encryption::Decrypt;
use anyhow::Context;
use bytes::{Buf, BufMut, BytesMut};
use mc_serializer::primitive::VarInt;
use std::borrow::Borrow;
use std::io::Cursor;
use std::ops::Deref;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::ReadHalf;

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

    fn poll(&mut self) -> PacketBufferFuture<BufferState> {
        Box::pin(async move {
            println!("POLL BEGIN");
            if self.is_packet_available() {
                return Ok(BufferState::PacketReady);
            }

            let size_read =
                match tokio::time::timeout(Duration::from_secs(10), self.read()).await {
                    Ok(result) => result?,
                    Err(_) => return Ok(BufferState::Error(String::from("Client read timeout."))),
                }
                .min(self.decoded().capacity() - self.decoded().len());

            println!("POLL END {}", size_read);

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

    fn loop_read(&mut self) -> PacketBufferFuture<Vec<u8>> {
        println!("Create loop read future.");
        Box::pin(async move {
            println!("Inner loop read future");
            loop {
                match self.poll().await? {
                    BufferState::PacketReady => {
                        println!("PACKET READY!");
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

pub struct BorrowedPacketBuffer<'a> {
    owned_ref: &'a mut OwnedPacketBuffer,
}

impl<'a> PacketBuffer for BorrowedPacketBuffer<'a> {
    fn read(&mut self) -> PacketBufferFuture<usize> {
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

    fn decryption_mut(&mut self) -> Option<&mut Decrypt> {
        self.owned_ref.decryption_mut()
    }

    fn construct_all(&mut self) -> (&mut BytesMut, &mut BytesMut, Option<&mut Decrypt>) {
        self.owned_ref.construct_all()
    }
}

pub struct OwnedPacketBuffer {
    read_half: OwnedReadHalf,
    bytes: BytesMut,
    decoded: BytesMut,
    decryption: Option<Decrypt>,
}

impl PacketBuffer for OwnedPacketBuffer {
    fn read(&mut self) -> PacketBufferFuture<usize> {
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
    fn decryption_mut(&mut self) -> Option<&mut Decrypt> {
        self.decryption.as_mut()
    }
    fn construct_all(&mut self) -> (&mut BytesMut, &mut BytesMut, Option<&mut Decrypt>) {
        (&mut self.bytes, &mut self.decoded, self.decryption.as_mut())
    }
}

impl OwnedPacketBuffer {
    pub fn new(read_half: OwnedReadHalf) -> Self {
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

    pub fn borrow_buffer(&mut self) -> BorrowedPacketBuffer {
        BorrowedPacketBuffer { owned_ref: self }
    }
}

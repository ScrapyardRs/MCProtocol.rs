use std::io::{Cursor, Read};
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use typemap::{Key, ShareMap};

use mc_registry::mappings::Mappings;
use mc_registry::registry::{
    arc_lock, ArcLocked, BoxedFuture, LockedContext, LockedStateRegistry, StateRegistry,
    StateRegistryHandle, UnhandledContext,
};
use mc_serializer::primitive::VarInt;
use mc_serializer::serde::ProtocolVersion;

use crate::buffer::{
    BorrowedPacketWriter, OwnedPacketReader, OwnedPacketWriter, PacketFuture, PacketReader,
    PacketWriter,
};
use crate::encryption::{Codec, Compressor};

pub type Context<'a> = BufferRegistryEngineContext<'a>;

pub struct BufferRegistryEngineContext<'a> {
    packet_sender: BorrowedPacketWriter<'a>,
    context_data: ArcLocked<ShareMap>,
}

impl<'a> BufferRegistryEngineContext<'a> {
    pub async fn insert_data<K: Key>(self_ref: Arc<RwLock<Self>>, value: K::Value)
    where
        K::Value: Send + Sync,
    {
        let read = self_ref.read().await;
        let mut map_write = read.context_data.write().await;
        map_write.insert::<K>(value);
    }

    pub async fn clone_data<K: Key>(self_ref: Arc<RwLock<Self>>) -> Option<K::Value>
    where
        K::Value: Send + Sync + Clone,
    {
        let read = self_ref.read().await;
        let map_read = read.context_data.read().await;
        map_read.get::<K>().cloned()
    }

    pub async fn map_inner(self_ref: Arc<RwLock<Self>>) -> ArcLocked<ShareMap> {
        let read = self_ref.read().await;
        Arc::clone(&read.context_data)
    }
}

impl<'a> PacketWriter for BufferRegistryEngineContext<'a> {
    fn send_packet<'b, Packet: Mappings<PacketType = Packet> + Send + Sync + 'b>(
        &'b mut self,
        packet: Packet,
    ) -> PacketFuture<'b, ()> {
        self.packet_sender.send_packet(packet)
    }
}

pub struct BufferRegistryEngine {
    packet_reader: OwnedPacketReader,
    packet_writer: OwnedPacketWriter,
    context_data: ArcLocked<ShareMap>,
}

impl BufferRegistryEngine {
    pub fn create(stream: TcpStream) -> Self {
        let (read, write) = stream.into_split();

        Self {
            packet_reader: read.into(),
            packet_writer: write.into(),
            context_data: arc_lock(ShareMap::custom()),
        }
    }

    pub fn update_protocol(&mut self, protocol_version: ProtocolVersion) {
        self.packet_writer.update_protocol_version(protocol_version);
    }

    pub async fn read_packets_until<'a, F>(
        &'a mut self,
        registry: StateRegistry<'a, BufferRegistryEngineContext<'a>>,
        predicate: F,
    ) -> anyhow::Result<()>
    where
        F: Fn(Option<UnhandledContext>, &mut RwLockWriteGuard<ShareMap>) -> bool,
    {
        let registry = arc_lock(registry);
        let context = arc_lock(BufferRegistryEngineContext::<'a> {
            packet_sender: self.packet_writer.borrow_buffer(),
            context_data: Arc::clone(&self.context_data),
        });

        loop {
            let reg_clone = Arc::clone(&registry);
            let context_clone = Arc::clone(&context);
            let next_packet = Cursor::new(self.packet_reader.loop_read().await?);
            let data = StateRegistry::emit(reg_clone, context_clone, next_packet).await?;

            let mut write_lock = self.context_data.write().await;
            if (predicate)(data, &mut write_lock) {
                break;
            }
        }
        Ok(())
    }

    pub fn set_compression(&mut self, compression: i32) {
        let compressor = Compressor::new(VarInt::from(compression));
        self.packet_reader.enable_compression(compressor);
        self.packet_writer.enabled_compression(compressor);
    }

    pub fn set_codec(&mut self, (r, w): (Codec, Codec)) {
        self.packet_reader.enable_decryption(r);
        self.packet_writer.enabled_encryption(w);
    }
}

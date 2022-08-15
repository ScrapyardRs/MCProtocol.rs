use flume::Sender;
use std::io::Cursor;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

use mc_registry::client_bound::play::Disconnect;
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::{RwLock, RwLockWriteGuard};
use tokio::task::JoinHandle;
use typemap::{Key, ShareMap};

use mc_registry::mappings::Mappings;
use mc_registry::registry::{arc_lock, ArcLocked, StateRegistry, UnhandledContext};
use mc_serializer::primitive::VarInt;
use mc_serializer::serde::ProtocolVersion;

use crate::buffer::{
    BorrowedPacketWriter, OwnedPacketReader, OwnedPacketWriter, PacketFuture, PacketReader,
    PacketWriter, PacketWriterGeneric, RawFuture,
};
use crate::encryption::{Codec, Compressor};

pub type Context<'a> = BufferRegistryEngineContext<'a>;

pub trait InnerMapAccessor<'a>: Send + Sync {
    fn data(&self) -> &ArcLocked<ShareMap>;

    fn insert_data<K: Key>(self_ref: Arc<RwLock<Self>>, value: K::Value) -> RawFuture<'a, ()>
    where
        K::Value: Send + Sync,
        Self: 'a,
    {
        Box::pin(async move {
            let read = self_ref.read().await;
            let mut map_write = read.data().write().await;
            map_write.insert::<K>(value);
        })
    }

    fn clone_data<K: Key>(self_ref: Arc<RwLock<Self>>) -> RawFuture<'a, Option<K::Value>>
    where
        K::Value: Send + Sync + Clone,
        Self: 'a,
    {
        Box::pin(async move {
            let read = self_ref.read().await;
            let map_read = read.data().read().await;
            map_read.get::<K>().cloned()
        })
    }

    fn map_inner(self_ref: Arc<RwLock<Self>>) -> RawFuture<'a, ArcLocked<ShareMap>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let read = self_ref.read().await;
            Arc::clone(read.data())
        })
    }
}

pub struct BufferRegistryEngineContext<'a> {
    packet_sender: BorrowedPacketWriter<'a>,
    context_data: ArcLocked<ShareMap>,
}

impl<'a> InnerMapAccessor<'a> for BufferRegistryEngineContext<'a> {
    fn data(&self) -> &ArcLocked<ShareMap> {
        &self.context_data
    }
}

#[derive(Clone)]
pub struct PacketSender {
    inner: Sender<Vec<u8>>,
    protocol_version: ProtocolVersion,
}

impl<'a> PacketWriter for BufferRegistryEngineContext<'a> {
    fn send_packet<'b, Packet: Mappings<PacketType = Packet> + Send + Sync + 'b>(
        &'b mut self,
        packet: Packet,
    ) -> PacketFuture<'b, ()> {
        self.packet_sender.send_packet(packet)
    }
}

impl PacketWriter for PacketSender {
    fn send_packet<'a, Packet: Mappings<PacketType = Packet> + Send + Sync + 'a>(
        &'a mut self,
        packet: Packet,
    ) -> PacketFuture<'a, ()> {
        let protocol_version = self.protocol_version;
        let send_clone = self.inner.clone();
        Box::pin(async move {
            let buffer = Packet::create_packet_buffer(protocol_version, packet)?;
            send_clone.send_async(buffer).await?;
            Ok(())
        })
    }
}

pub struct BufferRegistryEngine {
    packet_reader: OwnedPacketReader,
    packet_writer: OwnedPacketWriter,
    context_data: ArcLocked<ShareMap>,
}

pub enum EngineCloseContext {
    WriterCloseUnexpected,
    WriterCloseExpected,
    ReaderCloseUnexpected,
    ReaderCloseExpected,
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

    pub async fn insert_data<K: Key>(&self, value: K::Value)
    where
        K::Value: Send + Sync,
    {
        let mut map_write = self.context_data.write().await;
        map_write.insert::<K>(value);
    }

    pub async fn contains_data<K: Key>(&self) -> bool
    where
        K::Value: Send + Sync,
    {
        let map_read = self.context_data.read().await;
        map_read.contains::<K>()
    }

    pub async fn clone_data<K: Key>(&self) -> Option<K::Value>
    where
        K::Value: Send + Sync + Clone,
    {
        let map_read = self.context_data.read().await;
        map_read.get::<K>().cloned()
    }

    pub async fn map_inner(&self) -> ArcLocked<ShareMap> {
        Arc::clone(&self.context_data)
    }

    pub async fn clear_data(&self) {
        self.context_data.write().await.clear()
    }

    pub fn protocol_version(&self) -> ProtocolVersion {
        self.packet_writer.protocol_version()
    }

    pub async fn split_out<F, F2>(
        self,
        registry: StateRegistry<'static, ConnectedPlayerContext>,
        context_initializer: F,
        termination_hook: F2,
    ) -> anyhow::Result<PacketSender>
    where
        F: FnOnce(&mut RwLockWriteGuard<ShareMap>),
        F2: FnOnce(EngineCloseContext),
    {
        let protocol_version = self.protocol_version();

        let BufferRegistryEngine {
            mut packet_reader,
            mut packet_writer,
            context_data,
        } = self;
        {
            let mut data = context_data.write().await;
            data.clear();
            (context_initializer)(&mut data);
            drop(data)
        }

        let (flume_packet_writer, flume_packet_reader) = flume::unbounded();

        let packet_sender = PacketSender {
            inner: flume_packet_writer,
            protocol_version,
        };

        let packet_writer_handle: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
            loop {
                let next_packet = flume_packet_reader.recv_async().await?;

                let mut next_packet = if let Some(compressor) = packet_writer.compression() {
                    compressor.compress(next_packet)?
                } else {
                    Compressor::uncompressed(next_packet)?
                };

                packet_writer.encrypt(&mut next_packet);

                packet_writer.writer().write_all(&next_packet).await?;
            }
        });

        let writer_context_sender = packet_sender.clone();
        let packet_reader_handle: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
            let locked_registry = arc_lock(registry);
            let context = arc_lock(ConnectedPlayerContext {
                packet_sender: writer_context_sender,
                data: context_data,
            });

            loop {
                let reg_clone = Arc::clone(&locked_registry);
                let context_clone = Arc::clone(&context);
                let next_packet = Cursor::new(packet_reader.loop_read().await?);
                if let Some(unhandled) =
                    StateRegistry::emit(reg_clone, context_clone, next_packet).await?
                {
                    log::warn!("Unhandled Packet: {}", unhandled);
                }
            }
        });

        let mut selector_packet_sender_clone = packet_sender.clone();
        select! {
            val = packet_writer_handle => {
                if let Err(err) = &val {
                    log::error!("Packet writer closed with error: {:?}", err);
                    (termination_hook)((EngineCloseContext::WriterCloseUnexpected));
                } else {
                    log::trace!("Packet writer closed without an error.");
                    (termination_hook)((EngineCloseContext::WriterCloseExpected));
                }
            }
            val = packet_reader_handle => {
                if let Err(err) = &val {
                    log::error!("Packet writer closed with error: {:?}", err);
                    (termination_hook)((EngineCloseContext::ReaderCloseUnexpected));
                } else {
                    log::trace!("Packet writer closed without an error.");
                    (termination_hook)((EngineCloseContext::ReaderCloseExpected));
                }
            }
        };

        Ok(packet_sender)
    }
}

pub struct ConnectedPlayerContext {
    packet_sender: PacketSender,
    data: ArcLocked<ShareMap>,
}

impl InnerMapAccessor<'static> for ConnectedPlayerContext {
    fn data(&self) -> &ArcLocked<ShareMap> {
        &self.data
    }
}

impl PacketWriter for ConnectedPlayerContext {
    fn send_packet<'a, Packet: Mappings<PacketType = Packet> + Send + Sync + 'a>(
        &'a mut self,
        packet: Packet,
    ) -> PacketFuture<'a, ()> {
        self.packet_sender.send_packet(packet)
    }
}

impl PacketWriter for BufferRegistryEngine {
    fn send_packet<'a, Packet: Mappings<PacketType = Packet> + Send + Sync + 'a>(
        &'a mut self,
        packet: Packet,
    ) -> PacketFuture<'a, ()> {
        self.packet_writer.send_packet(packet)
    }
}

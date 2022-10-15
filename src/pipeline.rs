use crate::prelude::BoxFuture;
use crate::protocol::handshaking::sb::Handshake;
use crate::registry::{
    AsyncPacketRegistry, MCPacketWriter, MappedAsyncPacketRegistry, MutAsyncPacketRegistry,
    ProtocolVersionKey, RegistrationCandidate, RegistryError, UNKNOWN_VERSION,
};
use drax::prelude::{AsyncRead, BytesMut};
use drax::transport::buffered_reader::DraxTransportPipeline;
use drax::transport::buffered_writer::FrameSizeAppender;
use drax::transport::encryption::{DecryptRead, EncryptedWriter, EncryptionStream};
use drax::transport::frame::{FrameDecoder, FrameEncoder, PacketFrame};
use drax::transport::pipeline::{BoxedChain, ChainProcessor, ProcessChainLink, ShareChain};
use drax::transport::{DraxTransport, TransportProcessorContext};
use drax::{link, share_link, VarInt};
use std::io::Cursor;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub type BlankAsyncProtocolPipeline<R> =
    AsyncMinecraftProtocolPipeline<R, (), (), MappedAsyncPacketRegistry<(), ()>>;

pub struct AsyncMinecraftProtocolPipeline<
    R: AsyncRead + Send + Sync,
    Context: Send + Sync,
    PacketOutput: Send + Sync,
    Reg: AsyncPacketRegistry<Context, PacketOutput> + Send + Sync,
> {
    read: R,
    registry: Reg,
    processor_context: TransportProcessorContext,
    drax_transport: DraxTransportPipeline<PacketFrame>,
    _phantom_context: PhantomData<Context>,
    _phantom_packet_output: PhantomData<PacketOutput>,
}

impl<R: AsyncRead + Send + Sync, Context: Send + Sync, PacketOutput: Send + Sync>
    AsyncMinecraftProtocolPipeline<
        R,
        Context,
        PacketOutput,
        MappedAsyncPacketRegistry<Context, PacketOutput>,
    >
{
    pub fn empty(read: R) -> Self {
        let context = TransportProcessorContext::new();

        let pipeline = DraxTransportPipeline::new(
            Arc::new(FrameDecoder::new(-1)),
            BytesMut::with_capacity(crate::MC_BUFFER_CAPACITY),
        );

        Self {
            read,
            registry: MappedAsyncPacketRegistry::new(UNKNOWN_VERSION),
            processor_context: context,
            drax_transport: pipeline,
            _phantom_context: Default::default(),
            _phantom_packet_output: Default::default(),
        }
    }

    pub fn from_handshake(read: R, handshake: &Handshake) -> Self {
        Self::from_protocol_version(read, handshake.protocol_version)
    }

    pub fn from_protocol_version(read: R, protocol_version: VarInt) -> Self {
        let mut context = TransportProcessorContext::new();
        context.insert_data::<ProtocolVersionKey>(protocol_version);

        let pipeline = DraxTransportPipeline::new(
            Arc::new(FrameDecoder::new(-1)),
            BytesMut::with_capacity(crate::MC_BUFFER_CAPACITY),
        );

        Self {
            read,
            registry: MappedAsyncPacketRegistry::new(protocol_version),
            processor_context: context,
            drax_transport: pipeline,
            _phantom_context: Default::default(),
            _phantom_packet_output: Default::default(),
        }
    }
}

impl<
        R: AsyncRead + Unpin + Sized + Send + Sync,
        Context: Send + Sync,
        PacketOutput: Send + Sync,
        Reg: AsyncPacketRegistry<Context, PacketOutput> + Send + Sync,
    > AsyncMinecraftProtocolPipeline<R, Context, PacketOutput, Reg>
{
    pub fn rewrite_registry<NC: Send + Sync, NP: Send + Sync>(
        self,
        protocol_version: VarInt,
    ) -> AsyncMinecraftProtocolPipeline<R, NC, NP, MappedAsyncPacketRegistry<NC, NP>> {
        let Self {
            read,
            registry,
            processor_context,
            drax_transport,
            ..
        } = self;

        let mut context = TransportProcessorContext::new();
        context.insert_data::<ProtocolVersionKey>(protocol_version);

        AsyncMinecraftProtocolPipeline {
            read,
            registry: MappedAsyncPacketRegistry::new(protocol_version),
            processor_context: context,
            drax_transport,
            _phantom_context: Default::default(),
            _phantom_packet_output: Default::default(),
        }
    }

    pub fn clear_registry<NC: Send + Sync, NP: Send + Sync>(
        self,
    ) -> AsyncMinecraftProtocolPipeline<R, NC, NP, MappedAsyncPacketRegistry<NC, NP>> {
        let Self {
            read,
            registry,
            processor_context,
            drax_transport,
            ..
        } = self;
        AsyncMinecraftProtocolPipeline {
            read,
            registry: MappedAsyncPacketRegistry::new(registry.staple()),
            processor_context,
            drax_transport,
            _phantom_context: Default::default(),
            _phantom_packet_output: Default::default(),
        }
    }

    pub fn enable_decryption(
        mut self,
        stream: EncryptionStream,
    ) -> AsyncMinecraftProtocolPipeline<DecryptRead<R>, Context, PacketOutput, Reg> {
        let Self {
            read,
            registry,
            processor_context,
            drax_transport,
            _phantom_context,
            _phantom_packet_output,
        } = self;
        AsyncMinecraftProtocolPipeline::<DecryptRead<R>, Context, PacketOutput, Reg> {
            read: DecryptRead::new(read, stream),
            registry,
            processor_context,
            drax_transport,
            _phantom_context,
            _phantom_packet_output,
        }
    }

    pub async fn execute_next_packet(
        &mut self,
        context: &mut Context,
    ) -> Result<PacketOutput, RegistryError> {
        log::trace!("Executing next packet...");
        self.execute_next_packet_timeout(context, Duration::from_secs(30))
            .await
    }

    pub async fn execute_next_packet_timeout(
        &mut self,
        context: &mut Context,
        timeout: Duration,
    ) -> Result<PacketOutput, RegistryError> {
        log::trace!("Reading next packet with timeout {:?}", timeout);
        let next_packet_future = self
            .drax_transport
            .read_transport_packet(&mut self.processor_context, &mut self.read);
        let next_packet = match tokio::time::timeout(timeout, next_packet_future).await {
            Ok(out) => out.map_err(RegistryError::DraxTransportError)?,
            Err(_) => {
                return Err(RegistryError::DraxTransportError(
                    drax::transport::Error::Unknown(Some(format!(
                        "Failed to read a new packet within {:?} ({} seconds)",
                        timeout,
                        timeout.as_secs()
                    ))),
                ));
            }
        };
        log::trace!("Executing!");
        Ok(self
            .registry
            .execute(context, &mut self.processor_context, next_packet.data)?
            .await)
    }
}

impl<
        R: AsyncRead + Send + Sync,
        Context: Send + Sync,
        PacketOutput: Send + Sync,
        Reg: AsyncPacketRegistry<Context, PacketOutput> + Send + Sync,
    > AsyncMinecraftProtocolPipeline<R, Context, PacketOutput, Reg>
{
    pub fn with_registry<
        NContext: Send + Sync,
        NPacketOutput: Send + Sync,
        Reg2: AsyncPacketRegistry<NContext, NPacketOutput> + Send + Sync,
    >(
        self,
        registry: Reg2,
    ) -> AsyncMinecraftProtocolPipeline<R, NContext, NPacketOutput, Reg2> {
        let Self {
            read,
            processor_context,
            drax_transport,
            ..
        } = self;
        AsyncMinecraftProtocolPipeline::<R, NContext, NPacketOutput, Reg2> {
            read,
            registry,
            processor_context,
            drax_transport,
            _phantom_context: Default::default(),
            _phantom_packet_output: Default::default(),
        }
    }

    pub fn clear_data(&mut self) {
        let protocol = self
            .processor_context
            .retrieve_data::<ProtocolVersionKey>()
            .cloned();
        self.processor_context.clear_data();
        if let Some(protocol) = protocol {
            self.processor_context
                .insert_data::<ProtocolVersionKey>(protocol)
        }
    }

    pub fn retrieve_data<T: crate::prelude::Key>(&self) -> Option<&T::Value>
    where
        T::Value: Send,
    {
        self.processor_context.retrieve_data::<T>()
    }

    pub fn retrieve_data_mut<T: crate::prelude::Key>(&mut self) -> Option<&mut T::Value>
    where
        T::Value: Send,
    {
        self.processor_context.retrieve_data_mut::<T>()
    }

    pub fn insert_data<K: crate::prelude::Key>(&mut self, value: K::Value)
    where
        <K as drax::prelude::Key>::Value: Send,
    {
        self.processor_context.insert_data::<K>(value)
    }

    pub fn enable_compression(&mut self, threshold: isize) {
        if threshold >= 0 {
            self.drax_transport
                .update_chain(Arc::new(FrameDecoder::new(threshold)));
        }
    }
}

impl<
        R: AsyncRead + Send + Sync,
        Context: Send + Sync,
        PacketOutput: Send + Sync,
        Reg: MutAsyncPacketRegistry<Context, PacketOutput> + Send + Sync,
    > AsyncMinecraftProtocolPipeline<R, Context, PacketOutput, Reg>
{
    pub fn register<
        T: DraxTransport + RegistrationCandidate,
        Func: (for<'a> Fn(&'a mut Context, T) -> BoxFuture<'a, PacketOutput>) + 'static + Send + Sync,
    >(
        &mut self,
        func: Func,
    ) {
        self.registry.register(func)
    }
}

pub struct MinecraftProtocolWriter<W: Send + Sync> {
    protocol_version: VarInt,
    write: W,
    write_pipeline: ShareChain<PacketFrame, Vec<u8>>,
}

impl<W: Send + Sync> MinecraftProtocolWriter<W> {
    pub fn protocol_version(&self) -> VarInt {
        self.protocol_version
    }

    pub fn update_protocol(&mut self, protocol: VarInt) {
        self.protocol_version = protocol;
    }

    pub fn enable_compression(&mut self, threshold: isize) {
        if threshold >= 0 {
            self.write_pipeline =
                Arc::new(share_link!(FrameEncoder::new(threshold), FrameSizeAppender));
        }
    }
}

impl<W: AsyncWrite + Unpin + Sized + Send + Sync> MinecraftProtocolWriter<W> {
    pub fn from_handshake(write: W, handshake: &Handshake) -> Self {
        Self::from_protocol_version(write, handshake.protocol_version)
    }

    pub fn from_protocol_version(write: W, protocol_version: VarInt) -> Self {
        Self {
            protocol_version,
            write,
            write_pipeline: Arc::new(share_link!(FrameEncoder::new(-1), FrameSizeAppender)),
        }
    }

    pub fn buffer_packet<T: DraxTransport + RegistrationCandidate + Send + Sync + 'static>(
        packet: T,
        protocol_version: VarInt,
    ) -> drax::transport::Result<PacketFrame> {
        let packet_id = match T::scoped_registration(protocol_version) {
            None => {
                return drax::transport::Error::cause(format!(
                "Packet ID not found for protocol version {}. No further information available.",
                protocol_version
            ))
            }
            Some(packet_id) => packet_id,
        };
        let mut context = TransportProcessorContext::new();
        context.insert_data::<ProtocolVersionKey>(protocol_version);
        let frame = MCPacketWriter.process(&mut context, (packet_id, Box::new(packet)))?;
        Ok(frame)
    }

    pub async fn write_packet<T: DraxTransport + RegistrationCandidate + Send + Sync + 'static>(
        &mut self,
        packet: T,
    ) -> drax::transport::Result<()> {
        let protocol_version = self.protocol_version;
        let mut context = TransportProcessorContext::new();
        context.insert_data::<ProtocolVersionKey>(protocol_version);
        let packet_buffer = Self::buffer_packet(packet, protocol_version)?;
        let packet_buffer = self.write_pipeline.process(&mut context, packet_buffer)?;
        self.write
            .write_all(&packet_buffer)
            .await
            .map_err(drax::transport::Error::TokioError)
    }

    pub async fn write_buffered_packet(
        &mut self,
        packet: PacketFrame,
    ) -> drax::transport::Result<()> {
        let mut context = TransportProcessorContext::new();
        context.insert_data::<ProtocolVersionKey>(self.protocol_version);
        let packet_buffer = self.write_pipeline.process(&mut context, packet)?;
        self.write
            .write_all(&packet_buffer)
            .await
            .map_err(drax::transport::Error::TokioError)
    }

    pub fn enable_encryption(
        self,
        stream: EncryptionStream,
    ) -> MinecraftProtocolWriter<EncryptedWriter<W>> {
        let Self {
            protocol_version,
            write,
            write_pipeline,
        } = self;
        MinecraftProtocolWriter {
            protocol_version,
            write: EncryptedWriter::new(write, stream),
            write_pipeline,
        }
    }
}

pub type McReadWrite<R, Ctx, Out, Reg, W> = (
    AsyncMinecraftProtocolPipeline<R, Ctx, Out, Reg>,
    MinecraftProtocolWriter<W>,
);

pub type BlankMcReadWrite<R, W> = McReadWrite<R, (), (), MappedAsyncPacketRegistry<(), ()>, W>;

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
use drax::transport::pipeline::{BoxedChain, ProcessChainLink, ShareChain};
use drax::transport::{DraxTransport, TransportProcessorContext};
use drax::{link, VarInt};
use std::io::Cursor;
use std::marker::PhantomData;
use std::time::Duration;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub type BlankAsyncProtocolPipeline<R> =
    AsyncMinecraftProtocolPipeline<R, (), (), MappedAsyncPacketRegistry<(), ()>>;

pub struct AsyncMinecraftProtocolPipeline<
    R: AsyncRead,
    Context,
    PacketOutput,
    Reg: AsyncPacketRegistry<Context, PacketOutput>,
> {
    read: R,
    registry: Reg,
    processor_context: TransportProcessorContext,
    drax_transport: DraxTransportPipeline<PacketFrame>,
    _phantom_context: PhantomData<Context>,
    _phantom_packet_output: PhantomData<PacketOutput>,
}

impl<R: AsyncRead, Context, PacketOutput>
    AsyncMinecraftProtocolPipeline<
        R,
        Context,
        PacketOutput,
        MappedAsyncPacketRegistry<Context, PacketOutput>,
    >
{
    pub fn empty(read: R) -> Self {
        let mut context = TransportProcessorContext::new();

        let pipeline = DraxTransportPipeline::new(
            Box::new(FrameDecoder::new(-1)),
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
        let mut context = TransportProcessorContext::new();
        context.insert_data::<ProtocolVersionKey>(handshake.protocol_version);

        let pipeline = DraxTransportPipeline::new(
            Box::new(FrameDecoder::new(-1)),
            BytesMut::with_capacity(crate::MC_BUFFER_CAPACITY),
        );

        Self {
            read,
            registry: MappedAsyncPacketRegistry::new(handshake.protocol_version),
            processor_context: context,
            drax_transport: pipeline,
            _phantom_context: Default::default(),
            _phantom_packet_output: Default::default(),
        }
    }
}

impl<
        R: AsyncRead + Unpin + Sized,
        Context,
        PacketOutput,
        Reg: AsyncPacketRegistry<Context, PacketOutput>,
    > AsyncMinecraftProtocolPipeline<R, Context, PacketOutput, Reg>
{
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
        self.execute_next_packet_timeout(context, Duration::from_secs(30))
            .await
    }

    pub async fn execute_next_packet_timeout(
        &mut self,
        context: &mut Context,
        timeout: Duration,
    ) -> Result<PacketOutput, RegistryError> {
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
        Ok(self
            .registry
            .execute(context, &mut self.processor_context, next_packet.data)?
            .await)
    }
}

impl<R: AsyncRead, Context, PacketOutput, Reg: AsyncPacketRegistry<Context, PacketOutput>>
    AsyncMinecraftProtocolPipeline<R, Context, PacketOutput, Reg>
{
    pub fn into_inner_read(self) -> R {
        self.read
    }

    pub fn with_registry<
        NContext,
        NPacketOutput,
        Reg2: AsyncPacketRegistry<NContext, NPacketOutput>,
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

    pub fn insert_data<K: crate::prelude::Key>(&mut self, value: K::Value)
    where
        <K as drax::prelude::Key>::Value: Send,
    {
        self.processor_context.insert_data::<K>(value)
    }

    pub fn enable_compression(&mut self, threshold: isize) {
        if threshold >= 0 {
            self.drax_transport
                .update_chain(Box::new(FrameDecoder::new(threshold)));
        }
    }
}

impl<R: AsyncRead, Context, PacketOutput, Reg: MutAsyncPacketRegistry<Context, PacketOutput>>
    AsyncMinecraftProtocolPipeline<R, Context, PacketOutput, Reg>
{
    pub fn register<
        T: DraxTransport + RegistrationCandidate,
        Func: (for<'a> Fn(&'a mut Context, T) -> BoxFuture<'a, PacketOutput>) + 'static,
    >(
        &mut self,
        func: Func,
    ) {
        self.registry.register(func)
    }
}

pub struct MinecraftProtocolWriter<W> {
    protocol_version: VarInt,
    write: W,
    write_pipeline: BoxedChain<(VarInt, Box<dyn DraxTransport>), Vec<u8>>,
}

impl<W> MinecraftProtocolWriter<W> {
    pub fn enable_compression(&mut self, threshold: isize) {
        if threshold >= 0 {
            self.write_pipeline = Box::new(link!(
                MCPacketWriter,
                FrameEncoder::new(threshold),
                FrameSizeAppender
            ));
        }
    }
}

impl<W: AsyncWrite + Unpin + Sized> MinecraftProtocolWriter<W> {
    pub fn from_handshake(write: W, handshake: &Handshake) -> Self {
        Self {
            protocol_version: handshake.protocol_version,
            write,
            write_pipeline: Box::new(link!(
                MCPacketWriter,
                FrameEncoder::new(-1),
                FrameSizeAppender
            )),
        }
    }

    pub async fn write_packet<T: DraxTransport + RegistrationCandidate + 'static>(
        &mut self,
        packet: T,
    ) -> drax::transport::Result<()> {
        let packet_id = match T::scoped_registration(self.protocol_version) {
            None => {
                return drax::transport::Error::cause(format!(
                "Packet ID not found for protocol version {}. No further information available.",
                self.protocol_version
            ))
            }
            Some(packet_id) => packet_id,
        };
        let mut context = TransportProcessorContext::new();
        context.insert_data::<ProtocolVersionKey>(self.protocol_version);
        let packet_buffer = self
            .write_pipeline
            .process(&mut context, (packet_id, Box::new(packet)))?;
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

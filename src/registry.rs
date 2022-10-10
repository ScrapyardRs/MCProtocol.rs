use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::io::Cursor;
use std::marker::PhantomData;
use std::ops::RangeInclusive;
use std::rc::Rc;
use std::sync::Arc;

use drax::prelude::BoxFuture;
use drax::transport::frame::PacketFrame;
use drax::transport::pipeline::ChainProcessor;
use drax::transport::{DraxTransport, Error, Result, TransportProcessorContext};
use drax::VarInt;

pub const UNKNOWN_VERSION: VarInt = -2;
pub const ALL_VERSIONS: VarInt = -1;

pub struct MCPacketWriter;
impl ChainProcessor for MCPacketWriter {
    type Input = (VarInt, Box<dyn DraxTransport + Send + Sync>);
    type Output = PacketFrame;

    fn process(
        &self,
        context: &mut TransportProcessorContext,
        (packet_id, transport): Self::Input,
    ) -> Result<Self::Output>
    where
        Self::Input: Sized,
    {
        let mut packet_buffer = Cursor::new(Vec::with_capacity(
            transport.precondition_size(context)?
                + drax::extension::size_var_int(packet_id, context)?,
        ));
        drax::extension::write_var_int_sync(packet_id, context, &mut packet_buffer)?;
        transport.write_to_transport(context, &mut packet_buffer)?;
        Ok(PacketFrame {
            data: packet_buffer.into_inner(),
        })
    }
}

pub struct ProtocolVersionKey;
impl drax::prelude::Key for ProtocolVersionKey {
    type Value = VarInt;
}

#[derive(Debug)]
pub enum RegistryError {
    NoHandlerFound((VarInt, VarInt), Vec<u8>),
    DraxTransportError(drax::transport::Error),
}

impl Display for RegistryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::NoHandlerFound((protocol_version, packet_id), _) => {
                write!(
                    f,
                    "No handler found for protocol version: {}, packet id: {}",
                    protocol_version, packet_id
                )
            }
            RegistryError::DraxTransportError(error) => {
                write!(f, "Error found outside of handler, {}", error)
            }
        }
    }
}

pub trait MutAsyncPacketRegistry<Context, Output>: AsyncPacketRegistry<Context, Output> {
    fn register<
        T: DraxTransport + RegistrationCandidate,
        Func: (for<'a> Fn(&'a mut Context, T) -> BoxFuture<'a, Output>) + 'static + Send + Sync,
    >(
        &mut self,
        func: Func,
    );
}

pub trait AsyncPacketRegistry<Context, Output> {
    fn execute<'a>(
        &'a self,
        context: &'a mut Context,
        transport_context: &'a mut TransportProcessorContext,
        data: Vec<u8>,
    ) -> std::result::Result<BoxFuture<'a, Output>, RegistryError>;
}

impl std::error::Error for RegistryError {}

impl From<Error> for RegistryError {
    fn from(drax_err: Error) -> Self {
        RegistryError::DraxTransportError(drax_err)
    }
}

type AsyncPacketFunction<Context, Output> = dyn (for<'a> Fn(
        Cursor<Vec<u8>>,
        &'a mut Context,
        &'a mut TransportProcessorContext,
    ) -> Result<BoxFuture<'a, Output>>)
    + 'static
    + Send
    + Sync;

pub struct MappedAsyncPacketRegistry<Context: Send + Sync, Output: Send + Sync> {
    staple: VarInt,
    mappings: HashMap<(VarInt, VarInt), Arc<AsyncPacketFunction<Context, Output>>>,
}

impl<Context: Send + Sync, Output: Send + Sync> Default
    for MappedAsyncPacketRegistry<Context, Output>
{
    fn default() -> Self {
        Self {
            staple: UNKNOWN_VERSION,
            mappings: HashMap::new(),
        }
    }
}

impl<Context: Send + Sync, Output: Send + Sync> MappedAsyncPacketRegistry<Context, Output> {
    pub fn new(protocol_version: VarInt) -> Self {
        Self {
            staple: protocol_version,
            mappings: HashMap::new(),
        }
    }
}

impl<Context: Send + Sync, Output: Send + Sync> MutAsyncPacketRegistry<Context, Output>
    for MappedAsyncPacketRegistry<Context, Output>
{
    fn register<
        T: DraxTransport + RegistrationCandidate,
        Func: for<'a> Fn(&'a mut Context, T) -> BoxFuture<'a, Output> + 'static + Send + Sync,
    >(
        &mut self,
        func: Func,
    ) {
        let wrapped = Arc::new(Box::new(
            for<'a> move |mut cursor: Cursor<Vec<u8>>,
                          custom: &'a mut Context,
                          context: &'a mut TransportProcessorContext|
                          -> Result<BoxFuture<'a, Output>> {
                let packet = T::read_from_transport(context, &mut cursor)?;
                Ok::<_, drax::transport::Error>((func)(custom, packet))
            },
        ));
        if self.staple == ALL_VERSIONS || self.staple == UNKNOWN_VERSION {
            log::trace!("Using all version schematic for registration.");
            T::register_all(|key| {
                log::trace!("Registering {:?}", key);
                self.mappings.insert(key, wrapped.clone());
            });
        } else {
            if let Some(packet_id) = T::scoped_registration(self.staple) {
                log::trace!("Registering {:?}", (self.staple, packet_id));
                self.mappings.insert((self.staple, packet_id), wrapped);
            }
        }
    }
}

impl<Context: Send + Sync, Output: Send + Sync> AsyncPacketRegistry<Context, Output>
    for MappedAsyncPacketRegistry<Context, Output>
{
    fn execute<'a>(
        &'a self,
        context: &'a mut Context,
        transport_context: &'a mut TransportProcessorContext,
        data: Vec<u8>,
    ) -> std::result::Result<BoxFuture<'a, Output>, RegistryError> {
        let mut data_cursor = Cursor::new(data);
        let packet_id = drax::extension::read_var_int_sync(transport_context, &mut data_cursor)
            .map_err(RegistryError::DraxTransportError)?;

        let protocol_version = if self.staple > 0 {
            self.staple
        } else {
            transport_context
                .retrieve_data::<ProtocolVersionKey>()
                .cloned()
                .unwrap_or(UNKNOWN_VERSION)
        };

        match self.mappings.get(&(protocol_version, packet_id)).cloned() {
            Some(func) => (func)(data_cursor, context, transport_context)
                .map_err(RegistryError::DraxTransportError),
            None => {
                return Err(RegistryError::NoHandlerFound(
                    (protocol_version, packet_id),
                    data_cursor.into_inner(),
                ))
            }
        }
    }
}

macro_rules! async_reg_ref_impl {
    ($wrapper:ident) => {
        impl<Context: Send + Sync, Output: Send + Sync> AsyncPacketRegistry<Context, Output>
            for $wrapper<MappedAsyncPacketRegistry<Context, Output>>
        {
            fn execute<'a>(
                &'a self,
                context: &'a mut Context,
                transport_context: &'a mut TransportProcessorContext,
                data: Vec<u8>,
            ) -> std::result::Result<BoxFuture<'a, Output>, RegistryError> {
                MappedAsyncPacketRegistry::execute(self, context, transport_context, data)
            }
        }
    };
}

async_reg_ref_impl!(Box);
async_reg_ref_impl!(Arc);
async_reg_ref_impl!(Rc);

#[macro_export]
macro_rules! pin_fut {
    ($handle:expr) => {
        |t, ctx| Box::pin($handle(t, ctx))
    };
}

pub trait RegistrationCandidate {
    fn register_all<F: FnMut((VarInt, VarInt))>(function: F);

    fn scoped_registration(protocol_version: VarInt) -> Option<VarInt>;
}

pub struct Importer {
    range: RangeInclusive<i32>,
    packet_id: i32,
}

impl Importer {
    pub fn consume<F: FnMut((VarInt, VarInt))>(self, function: &mut F) {
        self.range.for_each(|x| (function)((x, self.packet_id)));
    }
}

impl From<(VarInt, VarInt)> for Importer {
    fn from((single, packet_id): (VarInt, VarInt)) -> Self {
        Importer {
            range: single..=single,
            packet_id,
        }
    }
}

impl From<(RangeInclusive<VarInt>, VarInt)> for Importer {
    fn from((range, packet_id): (RangeInclusive<VarInt>, VarInt)) -> Self {
        Importer { range, packet_id }
    }
}

#[macro_export]
macro_rules! import_registrations {
    ($($item:ident {
        $($from:tt$(..$to:tt)? -> $id:tt,)*
    })*) => {
        $(
                impl $crate::registry::RegistrationCandidate for $item {
                    fn register_all<F: FnMut((drax::VarInt, drax::VarInt))>(mut function: F) {
                        $($crate::registry::Importer::from(($from$(..=$to)?, $id)).consume(&mut function);)*
                    }

                    fn scoped_registration(protocol_version: drax::VarInt) -> Option<drax::VarInt> {
                        match protocol_version {
                            $($from$(..=$to)? => Some($id),)*
                            _ => None,
                        }
                    }
                }
        )*
    };
}

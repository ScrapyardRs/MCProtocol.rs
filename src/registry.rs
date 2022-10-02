use std::{collections::HashMap, future::Future, io::Cursor, ops::Range, sync::Arc};

use drax::{
    transport::{
        frame::PacketFrame, pipeline::ChainProcessor, DraxTransport, TransportProcessorContext,
    },
    VarInt,
};

pub const UNKNOWN_VERSION: VarInt = -2;
pub const ALL_VERSIONS: VarInt = -1;

pub struct WrappedMappingProcessor<O> {
    boxed_inner: Arc<
        dyn Fn(
            &mut drax::transport::TransportProcessorContext,
            Cursor<Vec<u8>>,
        ) -> drax::transport::Result<Box<dyn Future<Output = O>>>,
    >,
}

pub struct PacketRegistryProtocol;
impl drax::prelude::Key for PacketRegistryProtocol {
    type Value = VarInt;
}

#[derive(Clone)]
pub struct PacketRegistry<O> {
    stapled_version: VarInt,
    // (protocol version, packet id) -> packet
    mappings: HashMap<(VarInt, VarInt), Arc<WrappedMappingProcessor<O>>>,
}

macro_rules! wrap_processor {
    ($($tt:tt)*) => {
        Arc::new(WrappedMappingProcessor {
            boxed_inner: Arc::new($($tt)*),
        })
    }
}

impl<O> PacketRegistry<O> {
    pub fn new(version: VarInt) -> Self {
        Self {
            stapled_version: version,
            mappings: HashMap::new(),
        }
    }

    pub fn read_only(self) -> ReadOnlyPacketRegistry<O> {
        ReadOnlyPacketRegistry {
            inner: Arc::new(self),
        }
    }

    fn register_internal<T: RegistrationCandidate>(
        &mut self,
        wrapped: Arc<WrappedMappingProcessor<O>>,
    ) {
        if self.stapled_version == ALL_VERSIONS || self.stapled_version == UNKNOWN_VERSION {
            T::register_all(|mapping| {
                self.mappings.insert(mapping, wrapped.clone());
            });
        } else {
            if let Some(packet_id) = T::scoped_registration(self.stapled_version) {
                self.mappings
                    .insert((self.stapled_version, packet_id), wrapped.clone());
            }
        }
    }

    pub fn register<
        T: RegistrationCandidate + DraxTransport,
        F: Future<Output = O> + 'static,
        F1: Fn(T) -> F + 'static,
    >(
        &mut self,
        function: F1,
    ) {
        let wrapped = wrap_processor!(move |context, mut cursor| {
            let packet = T::read_from_transport(context, &mut cursor)?;
            Ok(Box::new((function)(packet)))
        });
        self.register_internal::<T>(wrapped)
    }

    pub fn register_with_context<
        Context: drax::prelude::Key<Value = Arc<Context>> + Send + Sync,
        T: RegistrationCandidate + DraxTransport,
        F: Future<Output = O> + 'static,
        F1: Fn(T, Arc<Context>) -> F + 'static,
    >(
        &mut self,
        function: F1,
    ) {
        let wrapped = wrap_processor!(move |context, mut cursor| {
            let packet = T::read_from_transport(context, &mut cursor)?;
            let context = context
                .retrieve_data::<Context>()
                .cloned()
                .expect("Context should exist.");
            Ok(Box::new((function)(packet, context)))
        });
        self.register_internal::<T>(wrapped)
    }
}

#[derive(Clone)]
pub struct ReadOnlyPacketRegistry<O> {
    inner: Arc<PacketRegistry<O>>,
}

impl<O> ChainProcessor for ReadOnlyPacketRegistry<O> {
    type Input = PacketFrame;
    type Output = Box<dyn std::future::Future<Output = O>>;

    fn process<'a>(
        &'a mut self,
        context: &'a mut TransportProcessorContext,
        input: Self::Input,
    ) -> drax::transport::Result<Self::Output> {
        let mut cursor = Cursor::new(input.data);
        let protocol_version = context
            .retrieve_data::<PacketRegistryProtocol>()
            .map(|x| *x)
            .unwrap_or(UNKNOWN_VERSION);
        let packet_id = if self.inner.stapled_version == ALL_VERSIONS {
            drax::extension::read_var_int_sync(context, &mut cursor)?
        } else {
            self.inner.stapled_version
        };
        let packet_mappings = self.inner.mappings.get(&(protocol_version, packet_id));
        match packet_mappings {
            Some(mapping) => (mapping.boxed_inner)(context, cursor),
            None => {
                return drax::transport::Error::cause(format!(
                    "No packet found with ({}, {})",
                    protocol_version, packet_id
                ))
            }
        }
    }
}

impl<O> ChainProcessor for PacketRegistry<O> {
    type Input = PacketFrame;
    type Output = Box<dyn std::future::Future<Output = O>>;

    fn process<'a>(
        &'a mut self,
        context: &'a mut TransportProcessorContext,
        input: Self::Input,
    ) -> drax::transport::Result<Self::Output> {
        let mut cursor = Cursor::new(input.data);
        let protocol_version = context
            .retrieve_data::<PacketRegistryProtocol>()
            .map(|x| *x)
            .unwrap_or(UNKNOWN_VERSION);
        let packet_id = if self.stapled_version == ALL_VERSIONS {
            drax::extension::read_var_int_sync(context, &mut cursor)?
        } else {
            self.stapled_version
        };
        let packet_mappings = self.mappings.get(&(protocol_version, packet_id));
        match packet_mappings {
            Some(mapping) => (mapping.boxed_inner)(context, cursor),
            None => {
                return drax::transport::Error::cause(format!(
                    "No packet found with ({}, {})",
                    protocol_version, packet_id
                ))
            }
        }
    }
}

pub trait RegistrationCandidate {
    fn register_all<F: FnMut((VarInt, VarInt))>(function: F);

    fn scoped_registration(protocol_version: VarInt) -> Option<VarInt>;
}

pub struct Importer {
    range: Range<i32>,
    protocol_version: i32,
}

impl Importer {
    pub fn consume<F: FnMut((drax::VarInt, drax::VarInt))>(self, function: &mut F) {
        let proto = self.protocol_version;
        self.range.for_each(|x| (function)((proto, x)));
    }
}

impl From<(VarInt, VarInt)> for Importer {
    fn from((single, protocol_version): (VarInt, VarInt)) -> Self {
        Importer {
            range: single..single,
            protocol_version,
        }
    }
}

impl From<(Range<VarInt>, VarInt)> for Importer {
    fn from((range, protocol_version): (Range<VarInt>, VarInt)) -> Self {
        Importer {
            range,
            protocol_version,
        }
    }
}

#[macro_export]
macro_rules! import_registrations {
    ($($item:ident {
        $($from:literal$(..$to:literal)? -> $id:literal,)*
    })*) => {
        $(
                impl $crate::registry::RegistrationCandidate for $item {
                    fn register_all<F: FnMut((drax::VarInt, drax::VarInt))>(mut function: F) {
                        $($crate::registry::Importer::from(($from$(..$to)?, $id)).consume(&mut function);)*
                    }

                    fn scoped_registration(protocol_version: drax::VarInt) -> Option<drax::VarInt> {
                        match protocol_version {
                            $($from$(..$to)? => Some($id),)*
                            _ => None,
                        }
                    }
                }
        )*
    };
}

#[macro_export]
macro_rules! key_context {
    ($ctx:ident) => {
        impl drax::prelude::Key for $ctx {
            type Value = std::sync::Arc<$ctx>;
        }
    };
}

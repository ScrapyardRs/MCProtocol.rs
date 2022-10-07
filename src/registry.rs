use crate::auth::mojang::Context;
use drax::prelude::BoxFuture;
use drax::transport::{DraxTransport, Result, TransportProcessorContext};
use drax::VarInt;
use std::collections::HashMap;
use std::future::Future;
use std::io::Cursor;
use std::marker::PhantomData;
use std::ops::RangeInclusive;
use std::rc::Rc;
use std::sync::Arc;

pub const UNKNOWN_VERSION: VarInt = -2;
pub const ALL_VERSIONS: VarInt = -1;

type AsyncPacketFunction<Context, Output> = dyn (for<'a> Fn(
        Cursor<Vec<u8>>,
        &'a mut Context,
        &'a mut TransportProcessorContext,
    ) -> Result<BoxFuture<'a, Output>>)
    + 'static;

pub struct AsyncPacketRegistry<Context, Output> {
    staple: VarInt,
    mappings: HashMap<(VarInt, VarInt), Arc<AsyncPacketFunction<Context, Output>>>,
}

impl<Context, Output> Default for AsyncPacketRegistry<Context, Output> {
    fn default() -> Self {
        Self {
            staple: UNKNOWN_VERSION,
            mappings: HashMap::new(),
        }
    }
}

impl<Context, Output> AsyncPacketRegistry<Context, Output> {
    pub fn new(protocol_version: VarInt) -> Self {
        Self {
            staple: protocol_version,
            mappings: HashMap::new(),
        }
    }

    pub fn register<
        T: DraxTransport + RegistrationCandidate,
        Func: (for<'a> Fn(&'a mut Context, T) -> BoxFuture<'a, Output>) + 'static,
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
            T::register_all(|key| {
                self.mappings.insert(key, wrapped.clone());
            });
        } else {
            if let Some(packet_id) = T::scoped_registration(self.staple) {
                self.mappings.insert((self.staple, packet_id), wrapped);
            }
        }
    }
}

#[macro_export]
macro_rules! pkt_ctx {
    ($handle:ident) => {
        |t, ctx| Box::pin($handle(t, ctx))
    };
}

// start anew

// pub mod old {
//     use drax::transport::frame::PacketFrame;
//     use drax::transport::pipeline::ChainProcessor;
//     use drax::{
//         link,
//         prelude::Key,
//         transport::{DraxTransport, Result, TransportProcessorContext},
//         VarInt,
//     };
//     use std::marker::PhantomData;
//     use std::rc::Rc;
//     use std::{collections::HashMap, future::Future, io::Cursor, ops::RangeInclusive, sync::Arc};
//     use std::future::IntoFuture;
//     use std::pin::Pin;
//     use drax::prelude::BoxFuture;
//
//     pub struct AsyncTypeTransport<
//         Output,
//         T: DraxTransport,
//         Func: for<'a> Fn(T, &'a mut TransportProcessorContext) -> BoxFuture<'a, Output>,
//     > {
//         pub inner: Func,
//         pub _phantom_output: PhantomData<Output>,
//         pub _phantom_t: PhantomData<T>,
//     }
//
//     impl<
//         Output,
//         T: DraxTransport,
//         Func: for<'a> Fn(T, &'a mut TransportProcessorContext) -> BoxFuture<'a, Output>,
//     > ChainProcessor for AsyncTypeTransport<Output, T, Func>
//     {
//         type Input = T;
//         type Output = Pin<Box<dyn Future<Output=Output>>>;
//
//         fn process<'a>(
//             &'a mut self,
//             context: &'a mut TransportProcessorContext,
//             input: Self::Input,
//         ) -> Result<Self::Output> {
//             Ok((self.inner)(input, context))
//         }
//     }
//
//     struct TypeRegistrationCandidate<T: DraxTransport> {
//         _phantom_marker: PhantomData<T>,
//     }
//
//     impl<T: DraxTransport> ChainProcessor for TypeRegistrationCandidate<T> {
//         type Input = Cursor<Vec<u8>>;
//         type Output = T;
//
//         fn process<'a>(
//             &'a mut self,
//             context: &'a mut TransportProcessorContext,
//             mut input: Self::Input,
//         ) -> Result<Self::Output> {
//             T::read_from_transport(context, &mut input)
//         }
//     }
//
//     pub struct PacketRegistryProtocol;
//
//     impl Key for PacketRegistryProtocol {
//         type Value = VarInt;
//     }
//
//     pub trait PacketRegistry<Mapping: Clone> {
//         type ChainOutput;
//
//         fn staple(&self) -> VarInt;
//
//         fn mappings(&mut self) -> &mut HashMap<(VarInt, VarInt), Mapping>;
//
//         fn register_internal<T: RegistrationCandidate>(&mut self, mapping: Mapping) {
//             let staple = self.staple();
//             let map = self.mappings();
//             if staple == ALL_VERSIONS || staple == UNKNOWN_VERSION {
//                 T::register_all(|key| {
//                     map.insert(key, mapping.clone());
//                 });
//             } else {
//                 if let Some(packet_id) = T::scoped_registration(staple) {
//                     map.insert((staple, packet_id), mapping);
//                 }
//             }
//         }
//
//         fn execute_simple(
//             &mut self,
//             context: &mut TransportProcessorContext,
//             cursor: Cursor<Vec<u8>>,
//             mapping: Mapping,
//         ) -> Result<Self::ChainOutput>;
//
//         fn process_chain_internal<'a>(
//             &'a mut self,
//             context: &'a mut TransportProcessorContext,
//             input: PacketFrame,
//         ) -> Result<Self::ChainOutput> {
//             let mut cursor = Cursor::new(input.data);
//             let protocol_version = context
//                 .retrieve_data::<PacketRegistryProtocol>()
//                 .map(|x| *x)
//                 .unwrap_or(UNKNOWN_VERSION);
//             let packet_id = if self.staple() == ALL_VERSIONS {
//                 drax::extension::read_var_int_sync(context, &mut cursor)?
//             } else {
//                 self.staple()
//             };
//             let packet_mappings = self.mappings().get(&(protocol_version, packet_id)).cloned();
//             match packet_mappings {
//                 Some(mapping) => self.execute_simple(context, cursor, mapping),
//                 None => {
//                     return drax::transport::Error::cause(format!(
//                         "No packet found with ({}, {})",
//                         protocol_version, packet_id
//                     ));
//                 }
//             }
//         }
//     }
//
// // type PacketFunction<Output> =
// // dyn for<'a> Fn(&'a mut TransportProcessorContext, Cursor<Vec<u8>>) -> Result<Box<Output>>;
// //
// // pub struct SyncPacketRegistry<Output> {
// //     stapled_version: VarInt,
// //     mappings: HashMap<(VarInt, VarInt), Rc<PacketFunction<Output>>>,
// // }
//
//     macro_rules! reg_funcs {
//     ($ty:ty) => {
//         fn staple(&self) -> VarInt {
//             self.stapled_version
//         }
//
//         fn mappings(&mut self) -> &mut HashMap<(VarInt, VarInt), $ty> {
//             &mut self.mappings
//         }
//
//         fn execute_simple(
//             &mut self,
//             context: &mut TransportProcessorContext,
//             cursor: Cursor<Vec<u8>>,
//             mapping: $ty,
//         ) -> Result<Self::ChainOutput> {
//             todo!()
//             // mapping.process(context, cursor)
//         }
//     };
// }
//
// // impl<Output> PacketRegistry<Rc<PacketFunction<Output>>> for SyncPacketRegistry<Output> {
// //     type ChainOutput = Box<Output>;
// //
// //     reg_funcs!(Rc<PacketFunction<Output>>);
// // }
// //
// // impl<Output> SyncPacketRegistry<Output> {
// //     pub fn new(stapled: VarInt) -> Self {
// //         Self {
// //             stapled_version: stapled,
// //             mappings: HashMap::default(),
// //         }
// //     }
// //
// //     pub fn register<T: RegistrationCandidate + DraxTransport, F: Fn(T) -> Output>(
// //         &mut self,
// //         function: F,
// //     ) {
// //         self.register_internal::<T>(Rc::new(move |context, mut cursor| {
// //             let packet = T::read_from_transport(context, &mut cursor)?;
// //             Ok(Box::new((function)(packet)))
// //         }));
// //     }
// //
// //     pub fn register_with_context<
// //         T: RegistrationCandidate + DraxTransport,
// //         F: for<'a> Fn(T, &'a mut TransportProcessorContext) -> Output,
// //     >(
// //         &mut self,
// //         function: F,
// //     ) {
// //         self.register_internal::<T>(Rc::new(move |context, mut cursor| {
// //             let packet = T::read_from_transport(context, &mut cursor)?;
// //             Ok(Box::new((function)(packet, context)))
// //         }));
// //     }
// // }
// //
// // impl<Output> ChainProcessor for SyncPacketRegistry<Output> {
// //     type Input = PacketFrame;
// //     type Output = Box<Output>;
// //
// //     fn process<'a>(
// //         &'a mut self,
// //         context: &'a mut TransportProcessorContext,
// //         input: Self::Input,
// //     ) -> Result<Self::Output> {
// //         self.process_chain_internal(context, input)
// //     }
// // }
//
//     pub type AsyncPacketFunction<Output> = dyn for<'a> Fn(Cursor<Vec<u8>>, &'a mut TransportProcessorContext) -> BoxFuture<'a, Output>;
//
//     impl<Output> PacketRegistry<Arc<AsyncPacketFunction<Output>>> for AsyncPacketRegistry<Output> {
//         type ChainOutput = Box<dyn Future<Output=Output>>;
//
//         reg_funcs!(Arc<AsyncPacketFunction<Output>>);
//     }
//
//     pub struct AsyncPacketRegistry<Output> {
//         stapled_version: VarInt,
//         mappings: HashMap<(VarInt, VarInt), Arc<AsyncPacketFunction<Output>>>,
//     }
//
//     impl<Output: 'static> AsyncPacketRegistry<Output> {
//         pub fn new(stapled: VarInt) -> Self {
//             Self {
//                 stapled_version: stapled,
//                 mappings: HashMap::default(),
//             }
//         }
//
//         pub fn register<
//             T: RegistrationCandidate + DraxTransport,
//             F: Future<Output=Output>,
//             F1: Fn(T) -> F,
//         >(
//             &mut self,
//             function: F1,
//         ) {
//             todo!()
//         }
//
//         pub fn register_with_context<T: RegistrationCandidate + DraxTransport, F, F1>(
//             &mut self,
//             function: F1,
//         ) where
//             F1: for<'a> Fn(T, &'a mut TransportProcessorContext) -> BoxFuture<'a, Output>,
//         {
//             let type_registrar = TypeRegistrationCandidate::<T> {
//                 _phantom_marker: Default::default(),
//             };
//             let transport = AsyncTypeTransport {
//                 inner: move |t, ctx| Box::pin((function)(t, ctx)),
//                 _phantom_output: Default::default(),
//                 _phantom_t: Default::default(),
//             };
//             let transport_link = link!(type_registrar, transport);
//         }
//     }
//
//     impl<Output> ChainProcessor for AsyncPacketRegistry<Output> {
//         type Input = PacketFrame;
//         type Output = Box<dyn Future<Output=Output>>;
//
//         fn process<'a>(
//             &'a mut self,
//             context: &'a mut TransportProcessorContext,
//             input: Self::Input,
//         ) -> Result<Self::Output> {
//             self.process_chain_internal(context, input)
//         }
//     }
//
//     pub trait RegistrationCandidate {
//         fn register_all<F: FnMut((VarInt, VarInt))>(function: F);
//
//         fn scoped_registration(protocol_version: VarInt) -> Option<VarInt>;
//     }
//
//     pub struct Importer {
//         range: RangeInclusive<i32>,
//         protocol_version: i32,
//     }
//
//     impl Importer {
//         pub fn consume<F: FnMut((VarInt, VarInt))>(self, function: &mut F) {
//             let proto = self.protocol_version;
//             self.range.for_each(|x| (function)((proto, x)));
//         }
//     }
//
//     impl From<(VarInt, VarInt)> for Importer {
//         fn from((single, protocol_version): (VarInt, VarInt)) -> Self {
//             Importer {
//                 range: single..=single,
//                 protocol_version,
//             }
//         }
//     }
//
//     impl From<(RangeInclusive<VarInt>, VarInt)> for Importer {
//         fn from((range, protocol_version): (RangeInclusive<VarInt>, VarInt)) -> Self {
//             Importer {
//                 range,
//                 protocol_version,
//             }
//         }
//     }
//
//     #[macro_export]
//     macro_rules! import_registrations {
//     ($($item:ident {
//         $($from:tt$(..$to:tt)? -> $id:tt,)*
//     })*) => {
//         $(
//                 impl $crate::registry::RegistrationCandidate for $item {
//                     fn register_all<F: FnMut((drax::VarInt, drax::VarInt))>(mut function: F) {
//                         $($crate::registry::Importer::from(($from$(..=$to)?, $id)).consume(&mut function);)*
//                     }
//
//                     fn scoped_registration(protocol_version: drax::VarInt) -> Option<drax::VarInt> {
//                         match protocol_version {
//                             $($from$(..=$to)? => Some($id),)*
//                             _ => None,
//                         }
//                     }
//                 }
//         )*
//     };
// }
//
//     #[macro_export]
//     macro_rules! key_context {
//     ($ctx:ident) => {
//         impl drax::prelude::Key for $ctx {
//             type Value = std::sync::Arc<$ctx>;
//         }
//     };
// }
// }

pub trait RegistrationCandidate {
    fn register_all<F: FnMut((VarInt, VarInt))>(function: F);

    fn scoped_registration(protocol_version: VarInt) -> Option<VarInt>;
}

pub struct Importer {
    range: RangeInclusive<i32>,
    protocol_version: i32,
}

impl Importer {
    pub fn consume<F: FnMut((VarInt, VarInt))>(self, function: &mut F) {
        let proto = self.protocol_version;
        self.range.for_each(|x| (function)((proto, x)));
    }
}

impl From<(VarInt, VarInt)> for Importer {
    fn from((single, protocol_version): (VarInt, VarInt)) -> Self {
        Importer {
            range: single..=single,
            protocol_version,
        }
    }
}

impl From<(RangeInclusive<VarInt>, VarInt)> for Importer {
    fn from((range, protocol_version): (RangeInclusive<VarInt>, VarInt)) -> Self {
        Importer {
            range,
            protocol_version,
        }
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

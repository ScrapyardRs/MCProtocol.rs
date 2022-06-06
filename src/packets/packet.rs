use crate::types::VarInt;
use crate::{Decodable, ProtocolDecodable, ProtocolVersion};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use futures::future::BoxFuture;

pub struct NoContext;

pub type PacketFuture<'handle_life> = BoxFuture<'handle_life, anyhow::Result<()>>;

pub type MetaPacketHandle<Context, Type> = fn(&mut ProtocolSheet<Context>, &mut Context, Type) -> PacketFuture<'static>;
pub type GenericPacketHandle<Context> = Box<
    dyn Fn(
        &mut ProtocolSheet<Context>,
        &mut Context,
        ProtocolVersion,
        &mut Cursor<Vec<u8>>,
    ) -> PacketFuture<'static>,
>;

pub trait StaticProtocolMappings {
    fn get_protocol_mappings() -> Vec<(ProtocolVersion, VarInt)>;
}

pub trait ProtocolSheetEchoCandidate {
    fn echo_packet_handle<Context: Send + Sync>() -> MetaPacketHandle<Context, Self>;
}

pub struct ProtocolSheet<Context: Send + Sync> {
    pub protocol_version: ProtocolVersion,
    mappings: HashMap<(ProtocolVersion, VarInt), Arc<GenericPacketHandle<Context>>>,
}

impl<Context> ProtocolSheet<Context>
    where
        Context: Send + Sync + 'static,
{
    pub fn new(protocol_version: ProtocolVersion) -> Self {
        Self {
            protocol_version,
            mappings: HashMap::default(),
        }
    }

    pub fn register_packet_handle<
        T: StaticProtocolMappings + ProtocolDecodable + Send + Sync + 'static,
    >(
        &mut self,
        meta_handle: MetaPacketHandle<Context, T>,
    ) {
        let protocol_decodable = T::decode_from_protocol;

        let generic_packet_handle: Arc<GenericPacketHandle<Context>> = Arc::new(Box::new(
            move |sheet, context, protocol_version, raw_buf| {
                let t_resolved = protocol_decodable(protocol_version, raw_buf).unwrap();
                meta_handle(sheet, context, t_resolved)
            }
        ));

        for mapping in T::get_protocol_mappings() {
            self.register_generic(mapping, Arc::clone(&generic_packet_handle));
        }
    }

    fn register_generic(
        &mut self,
        protocol_mapping: (ProtocolVersion, VarInt),
        generic_handle: Arc<GenericPacketHandle<Context>>,
    ) {
        self.mappings.insert(protocol_mapping, generic_handle);
    }

    pub fn call_generic(
        &mut self,
        context: &mut Context,
        mut raw_buf: Cursor<Vec<u8>>,
    ) -> anyhow::Result<PacketFuture<'static>> {
        let packet_id = VarInt::decode(&mut raw_buf)?;
        let generic_packet_handle = &self.mappings.get(&(self.protocol_version, packet_id));
        if let Some(packet_handle) = generic_packet_handle {
            Ok(Arc::clone(packet_handle)(self, context, self.protocol_version, &mut raw_buf))
        } else {
            Ok(Box::pin(async move { Ok(()) }))
        }
    }

    pub fn clear(&mut self) {
        self.mappings.clear();
    }
}

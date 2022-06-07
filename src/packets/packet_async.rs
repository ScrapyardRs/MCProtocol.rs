use super::StaticProtocolMappings;
use crate::types::VarInt;
use crate::{Decodable, ProtocolVersion};
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct NoContext;

pub type LockedSheet<Context> = Arc<RwLock<ProtocolSheet<Context>>>;
pub type LockedContext<Context> = Arc<RwLock<Context>>;
pub type MetaHandle<Context> = fn(
    ProtocolVersion,
    LockedSheet<Context>,
    LockedContext<Context>,
    Cursor<Vec<u8>>,
) -> BoxFuture<'static, anyhow::Result<()>>;
pub type PacketHandle<Context, PacketType> = fn(
    LockedSheet<Context>,
    LockedContext<Context>,
    PacketType,
) -> BoxFuture<'static, anyhow::Result<()>>;

#[macro_export]
macro_rules! wrap_async_packet_handle {
    ($(fn $function_ident:ident<$context_type:ty, $packet_type:ty>($sheet_ident:ident, $context_ident:ident, $packet_ident:ident) {
        $($handle_tokens:tt)+
    })*) => {
        $(fn $function_ident(
            protocol_version: $crate::ProtocolVersion,
            sheet: $crate::packets::packet_async::LockedSheet<$context_type>,
            context: $crate::packets::packet_async::LockedContext<$context_type>,
            mut raw_buf: std::io::Cursor<Vec<u8>>,
        ) -> $crate::prelude::BoxFuture<'static, anyhow::Result<()>> {
            Box::pin(async move {
                let packet: $packet_type = $crate::encoding::ProtocolDecodable::decode_from_protocol(protocol_version, &mut raw_buf)?;
                let $sheet_ident = std::sync::Arc::clone(&sheet);
                let $context_ident = std::sync::Arc::clone(&context);
                let $packet_ident = packet;
                $($handle_tokens)+
                Ok(())
            })
        })*
    };
    ($(fn $function_ident:ident<$packet_type:ty>($sheet_ident:ident, $packet_ident:ident) {
        $($handle_tokens:tt)+
    })*) => {
        $(fn $function_ident(
            protocol_version: $crate::ProtocolVersion,
            sheet: $crate::packets::packet_async::LockedSheet<$crate::packets::packet_async::NoContext>,
            _: $crate::packets::packet_async::LockedContext<$crate::packets::packet_async::NoContext>,
            mut raw_buf: std::io::Cursor<Vec<u8>>,
        ) -> $crate::prelude::BoxFuture<'static, anyhow::Result<()>> {
            Box::pin(async move {
                let packet: $packet_type = $crate::encoding::ProtocolDecodable::decode_from_protocol(protocol_version, &mut raw_buf)?;
                let $sheet_ident = std::sync::Arc::clone(&sheet);
                let $packet_ident = packet;
                $($handle_tokens)+
                Ok(())
            })
        })*
    }
}

pub struct ProtocolSheet<Context: Send + Sync> {
    pub protocol_version: ProtocolVersion,
    mappings: HashMap<(ProtocolVersion, VarInt), Arc<MetaHandle<Context>>>,
}

impl<Context: Send + Sync> ProtocolSheet<Context> {
    pub fn new(protocol_version: ProtocolVersion) -> Self {
        Self {
            protocol_version,
            mappings: HashMap::new(),
        }
    }

    pub fn register_packet_handle<T: StaticProtocolMappings>(
        &mut self,
        meta_handle: MetaHandle<Context>,
    ) {
        let arc_handle = Arc::new(meta_handle);
        for mapping in T::get_protocol_mappings() {
            self.register_generic(mapping, Arc::clone(&arc_handle));
        }
    }

    fn register_generic(
        &mut self,
        protocol_mapping: (ProtocolVersion, VarInt),
        generic_handle: Arc<MetaHandle<Context>>,
    ) {
        self.mappings.insert(protocol_mapping, generic_handle);
    }

    pub async fn call_generic(
        self_lock: Arc<RwLock<Self>>,
        context: Arc<RwLock<Context>>,
        mut raw_buf: Cursor<Vec<u8>>,
    ) -> anyhow::Result<()> {
        let self_read_lock = self_lock.read().await;
        let packet_id = VarInt::decode(&mut raw_buf)?;
        let protocol_version = self_read_lock.protocol_version;
        let generic_packet_handle = self_read_lock.mappings.get(&(protocol_version, packet_id));
        if let Some(packet_handle) = generic_packet_handle {
            let arc_clone = Arc::clone(packet_handle);
            drop(self_read_lock);
            (arc_clone)(
                protocol_version,
                Arc::clone(&self_lock),
                Arc::clone(&context),
                raw_buf,
            )
            .await
        } else {
            Ok(())
        }
    }

    pub fn clear(&mut self) {
        self.mappings.clear();
    }
}

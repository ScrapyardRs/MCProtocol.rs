use crate::mappings::Mappings;
use futures::future::BoxFuture;
use minecraft_serde::primitive::VarInt;
use minecraft_serde::serde::{Deserialize, ProtocolVersion};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type BoxedFuture = BoxFuture<'static, crate::Result<()>>;
pub type LockedContext<Context> = Arc<RwLock<Context>>;
pub type LockedStateRegistry<Context> = Arc<RwLock<StateRegistry<Context>>>;
pub type StateRegistryHandle<Context> = fn(
    LockedContext<Context>,
    LockedStateRegistry<Context>,
    ProtocolVersion,
    Cursor<Vec<u8>>,
) -> BoxedFuture;

#[macro_export]
macro_rules! state_registry_handle {
    (fn $function_identifier:ident<$context_type:ty>(
        $context_ident:ident,
        $registry_ident:ident,
        $protocol_version_ident:ident,
        $buf_ident:ident
    ) -> anyhow::Result<()> {
        $($function_tokens:tt)+
    }) => {
        fn $function_identifier(
            $context_ident: $crate::registry::LockedContext<$context_type>,
            $registry_ident: $crate::registry::LockedStateRegistry<$context_type>,
            $protocol_version_ident: minecraft_serde::serde::ProtocolVersion,
            $buf_ident: std::io::Cursor<Vec<u8>>,
        ) -> $crate::registry::BoxedFuture {
            Box::pin(async move {
                $($function_tokens)+
            })
        }
    }
}

pub struct StateRegistry<Context> {
    protocol_version: ProtocolVersion,
    mappings: HashMap<VarInt, Arc<StateRegistryHandle<Context>>>,
}

impl<Context> StateRegistry<Context> {
    pub fn attach_mappings<MappingsType: Mappings>(
        &mut self,
        handle: StateRegistryHandle<Context>,
    ) {
        self.mappings.insert(
            MappingsType::retrieve_packet_id(self.protocol_version),
            Arc::new(handle),
        );
    }

    pub fn clear_mappings(&mut self) {
        self.mappings.clear()
    }

    pub async fn emit(
        arc_self: LockedStateRegistry<Context>,
        context: LockedContext<Context>,
        mut packet_buffer: Cursor<Vec<u8>>,
    ) -> anyhow::Result<()> {
        let packet_id = VarInt::deserialize(&mut packet_buffer)?;
        let self_read_lock = arc_self.read().await;
        let protocol_version = self_read_lock.protocol_version;
        let handler = self_read_lock.mappings.get(&packet_id);
        if let Some(handler) = handler {
            let cloned_handler = Arc::clone(&handler);
            drop(self_read_lock);
            (cloned_handler)(context, arc_self, protocol_version, packet_buffer).await?;
        }
        Ok(())
    }

    pub fn new(protocol_version: ProtocolVersion) -> Self {
        Self {
            protocol_version,
            mappings: HashMap::default(),
        }
    }
}

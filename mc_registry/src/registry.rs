use crate::mappings::Mappings;
use futures::future::BoxFuture;
use mc_serializer::primitive::VarInt;
use mc_serializer::serde::{Deserialize, ProtocolVersion};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type BoxedFuture = BoxFuture<'static, crate::Result<()>>;

pub type ArcLocked<Item> = Arc<RwLock<Item>>;
pub type LockedContext<Context> = ArcLocked<Context>;
pub type LockedStateRegistry<Context> = ArcLocked<StateRegistry<Context>>;
pub type StateRegistryHandle<Context> = fn(
    LockedContext<Context>,
    LockedStateRegistry<Context>,
    ProtocolVersion,
    Cursor<Vec<u8>>,
) -> BoxedFuture;
pub type FailureHandle<Context> = fn(LockedContext<Context>, VarInt) -> BoxedFuture;

pub fn arc_lock<T>(object: T) -> ArcLocked<T> {
    Arc::new(RwLock::new(object))
}

pub struct StateRegistry<Context> {
    protocol_version: ProtocolVersion,
    mappings: HashMap<VarInt, Arc<StateRegistryHandle<Context>>>,
    fail_on_invalid: bool,
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
            let handler = Arc::clone(handler);
            drop(self_read_lock);
            (handler)(context, arc_self, protocol_version, packet_buffer).await?;
        } else if self_read_lock.fail_on_invalid {
            anyhow::bail!("Failed to process invalid packet ID {:?}", packet_id);
        }
        Ok(())
    }

    pub fn new(protocol_version: ProtocolVersion) -> Self {
        Self {
            protocol_version,
            mappings: HashMap::default(),
            fail_on_invalid: false,
        }
    }

    pub fn fail_on_invalid(protocol_version: ProtocolVersion) -> Self {
        Self {
            protocol_version,
            mappings: HashMap::default(),
            fail_on_invalid: true,
        }
    }
}

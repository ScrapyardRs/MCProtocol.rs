use crate::mappings::Mappings;
use futures::future::BoxFuture;
use mc_serializer::primitive::VarInt;
use mc_serializer::serde::{Deserialize, ProtocolVersion};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::RwLock;

#[macro_export]
macro_rules! needless_life(() => (#[allow(clippy::needless_lifetimes)]));

pub type BoxedFuture<'a> = BoxFuture<'a, crate::Result<()>>;

pub type ArcLocked<Item> = Arc<RwLock<Item>>;
pub type LockedContext<Context> = ArcLocked<Context>;
pub type LockedStateRegistry<'a, Context> = ArcLocked<StateRegistry<'a, Context>>;
pub type StateRegistryHandle<'a, Context> = fn(
    LockedContext<Context>,
    LockedStateRegistry<'a, Context>,
    ProtocolVersion,
    Cursor<Vec<u8>>,
) -> BoxedFuture<'a>;
pub type FailureHandle<Context> = fn(LockedContext<Context>, VarInt) -> BoxedFuture<'static>;

pub fn arc_lock<T>(object: T) -> ArcLocked<T> {
    Arc::new(RwLock::new(object))
}

pub struct StateRegistry<'a, Context> {
    protocol_version: ProtocolVersion,
    mappings: HashMap<VarInt, Arc<StateRegistryHandle<'a, Context>>>,
    fail_on_invalid: bool,
}

pub struct UnhandledContext {
    pub packet_id: VarInt,
    pub bytes: Vec<u8>,
}

impl<'a, Context> StateRegistry<'a, Context> {
    pub fn attach_mappings<MappingsType: Mappings>(
        &mut self,
        handle: StateRegistryHandle<'a, Context>,
    ) {
        if let Ok(id) = MappingsType::retrieve_packet_id(self.protocol_version) {
            drop(self.mappings.insert(id, Arc::new(handle)));
        }
    }

    pub fn clear_mappings(&mut self) {
        self.mappings.clear()
    }

    pub async fn emit(
        arc_self: LockedStateRegistry<'a, Context>,
        context: LockedContext<Context>,
        mut packet_buffer: Cursor<Vec<u8>>,
    ) -> anyhow::Result<Option<UnhandledContext>> {
        let self_read_lock = arc_self.read().await;
        let protocol_version = self_read_lock.protocol_version;
        let packet_id = VarInt::deserialize(&mut packet_buffer, protocol_version)?;
        let handler = self_read_lock.mappings.get(&packet_id);
        if let Some(handler) = handler {
            let handler = Arc::clone(handler);
            drop(self_read_lock);
            (handler)(context, arc_self, protocol_version, packet_buffer).await?;
            Ok(None)
        } else {
            if self_read_lock.fail_on_invalid {
                anyhow::bail!("Failure to understand packet id {}", packet_id)
            }
            Ok(Some(UnhandledContext {
                packet_id,
                bytes: packet_buffer.into_inner(),
            }))
        }
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

#[macro_export]
macro_rules! register_handler {
    ($registry:ident, $mappings:ty, $handler:expr) => {
        $registry.attach_mappings::<$mappings>($handler);
    };
}

#[macro_export]
macro_rules! create_registry {
    ($registry_ident:ident, $protocol_version:ident {
        $($packet:ty, $handler:ident)*
    }) => {
        let mut $registry_ident = {
            let mut $registry_ident = $crate::registry::StateRegistry::new($protocol_version);
            $($crate::register_handler!($registry_ident, $packet, $handler))*;
            $registry_ident
        };
    };
}

pub mod client_bound;
pub mod gen;
pub mod packet;
pub mod packet_async;
pub mod prelude;
pub mod server_bound;

pub use prelude::*;
use crate::{ProtocolVersion, VarInt};

pub trait StaticProtocolMappings {
    fn get_protocol_mappings() -> Vec<(ProtocolVersion, VarInt)>;
}

pub mod client_bound;
pub mod gen;
pub mod packet;
pub mod packet_async;
pub mod prelude;
pub mod server_bound;

use crate::{ProtocolVersion, VarInt};
pub use prelude::*;

pub trait StaticProtocolMappings {
    fn get_protocol_mappings() -> Vec<(ProtocolVersion, VarInt)>;
}

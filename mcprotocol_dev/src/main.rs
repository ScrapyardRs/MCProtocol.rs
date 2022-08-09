use mc_registry::mappings::Mappings;
use mc_registry::registry::{LockedContext, StateRegistry};
use mc_registry::server_bound::handshaking::Handshake;
use mc_serializer::serde::ProtocolVersion;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut registry = StateRegistry::new(ProtocolVersion::Unknown);
    Handshake::attach_to_register(&mut registry, test);

    Ok(())
}

struct Context;

// #[mc_registry_derive::packet_handler]
// pub fn test(packet: Handshake, context: LockedContext<Context>) {
//     let mut context_write = context.write().await;
// }

fn test<'registry>(
    __context: LockedContext<Context>,
    __registry: mc_registry::registry::LockedStateRegistry<'registry, Context>,
    __protocol_version: mc_serializer::serde::ProtocolVersion,
    __buffer: std::io::Cursor<Vec<u8>>,
) -> mc_registry::registry::BoxedFuture<'registry> {
    Box::pin(async move {
        let __packet = mc_registry::mappings::create_packet::<Handshake>(__protocol_version, __buffer)?;
        let packet = __packet;
        let context = __context;
        { let mut context_write = context.write().await; }
        Ok(())
    })
}


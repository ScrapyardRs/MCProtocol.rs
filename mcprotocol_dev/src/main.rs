use mc_registry::registry::LockedContext;
use mc_registry::server_bound::handshaking::Handshake;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Ok(())
}

struct HandshakeContext;

#[mc_registry_derive::packet_handler]
pub fn accept_handshake(packet: Handshake, context: LockedContext<HandshakeContext>) {

}

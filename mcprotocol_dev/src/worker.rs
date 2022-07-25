use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use encryption_utils::MCPrivateKey;
use mc_impl_server::client_connection::Connection;
use mc_impl_server::client_status_handler::StatusPartBuilder;
use mc_impl_server::login::notchian::NotchianLoginConfig;
use mc_impl_server::login::notchian_login;
use mc_registry::server_bound::handshaking::NextState;
use mc_serializer::primitive::VarInt;

pub async fn attach_worker(socket_addr: SocketAddr, stream: TcpStream, server_key: Arc<MCPrivateKey>) -> anyhow::Result<()> {
    let connection = Connection::from_initial_connection(socket_addr, stream, server_key).await?;
    match connection.connection_into().next_state() {
        NextState::Status => {
            let status_builder = StatusPartBuilder::default()
                .max_players(10)
                .total_online(5)
                .motd("Hello World!");
            connection.handle_status_with_data(status_builder).await?;
        }
        NextState::Login => {
            if let Some(authenticated_connection) = notchian_login(NotchianLoginConfig {
                force_key_authentication: true,
                compression_threshold: VarInt::from(0),
            }, connection).await {
                println!("We have a connection!, profile: {:?}", authenticated_connection.profile);
                // do something with the authenticated connection now
            } else {
                return Ok(());
            }
        }
    }
    Ok(())
}
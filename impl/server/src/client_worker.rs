use crate::client_connection::Connection;
use crate::client_status_handler::StatusPart;
use mc_registry::server_bound::handshaking::NextState;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;

pub(crate) struct ServerInfo<SPB: Into<StatusPart> + 'static + Send + Sync> {
    pub(crate) server_key: Arc<encryption_utils::MCPrivateKey>,
    pub(crate) config: Arc<super::server::Config<SPB>>,
}

pub(crate) async fn initialize_connection<SPB: Into<StatusPart> + 'static + Send + Sync>(
    addr: SocketAddr,
    stream: TcpStream,
    server_info: ServerInfo<SPB>,
) -> anyhow::Result<()> {
    let connection =
        Connection::from_initial_connection(addr, stream, Arc::clone(&server_info.server_key))
            .await?;
    match connection.connection_into().next_state() {
        NextState::Status => {
            let spb = (server_info.config.status_retriever())(connection.connection_into())?;
            super::client_status_handler::handle_status(connection, spb).await
        }
        NextState::Login => {
            match server_info.config.forwarding_mode().login(connection).await {
                Some(authenticated_player) => {
                    log::info!("Logged in player: {:?}", authenticated_player.profile)
                }
                None => {
                    log::warn!("Failure to validate authenticated player.");
                }
            }
            Ok(())
        }
    }
}

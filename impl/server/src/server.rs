use crate::client_connection::{Connection, InitialConnectionInfo};
use crate::client_status_handler::StatusPart;
use crate::client_worker::ServerInfo;
use crate::login::notchian::NotchianLoginConfig;
use crate::login::AuthenticatedPlayerConnection;
use std::sync::Arc;
use tokio::net::TcpListener;

pub type StatusRetrievalFunction<SPB> = fn(&InitialConnectionInfo) -> anyhow::Result<SPB>;

pub enum ForwardingMode {
    Notchian(NotchianLoginConfig),
}

impl ForwardingMode {
    pub async fn login(&self, connection: Connection) -> Option<AuthenticatedPlayerConnection> {
        match self {
            ForwardingMode::Notchian(config) => {
                crate::login::notchian_login(*config, connection).await
            }
        }
    }
}

pub struct Config<SPB: Into<StatusPart> + 'static + Send + Sync> {
    forwarding_mode: Arc<ForwardingMode>,
    bind: String,
    status_retriever: Arc<StatusRetrievalFunction<SPB>>,
}

impl<SPB: Into<StatusPart> + 'static + Send + Sync> Config<SPB> {
    pub fn new<IS: Into<String>>(
        forwarding_mode: ForwardingMode,
        bind: IS,
        status_retriever: StatusRetrievalFunction<SPB>,
    ) -> Self {
        Self {
            forwarding_mode: Arc::new(forwarding_mode),
            bind: bind.into(),
            status_retriever: Arc::new(status_retriever),
        }
    }

    pub fn forwarding_mode(&self) -> Arc<ForwardingMode> {
        Arc::clone(&self.forwarding_mode)
    }

    pub fn status_retriever(&self) -> Arc<StatusRetrievalFunction<SPB>> {
        Arc::clone(&self.status_retriever)
    }
}

#[macro_export]
macro_rules! config {
    (
        $(forwarding_mode: ($($forwarding_tokens:tt)*);)?
        $(force_key_auth: $force_key_auth:literal;)?
        $(bind: $bind:expr;)?
        status_retriever: |$conn_ident:ident| {$($status_retriever_tokens:tt)*};
    ) => {
        {
            let __forwarding_mode = $crate::server::ForwardingMode::Notchian(
                $crate::login::notchian::NotchianLoginConfig {
                    force_key_authentication: true,
                    compression_threshold: mc_serializer::primitive::VarInt::from(0),
                }
            );
            $(
                let __forwarding_mode = $($forwarding_tokens)*;
            )?
            let __bind = "0.0.0.0:25565";
            $(
                let __bind = $bind;
            )?
            let __status_retriever = |$conn_ident: &$crate::client_connection::InitialConnectionInfo| { $($status_retriever_tokens)* };

            $crate::server::Config::new(
                __forwarding_mode,
                __bind,
                __status_retriever,
            )
        }
    }
}

pub struct Server<SPB: Into<StatusPart> + 'static + Send + Sync> {
    server_key: Arc<encryption_utils::MCPrivateKey>,
    config: Arc<Config<SPB>>,
}

impl<SPB: Into<StatusPart> + 'static + Send + Sync> Server<SPB> {
    pub fn new(config: Config<SPB>) -> anyhow::Result<Server<SPB>> {
        let me = Self {
            server_key: Arc::new(encryption_utils::new_key()?),
            config: Arc::new(config),
        };
        Ok(me)
    }

    pub async fn boot_server(self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.config.bind.to_string()).await?;
        loop {
            let server_key = Arc::clone(&self.server_key);
            let config = Arc::clone(&self.config);
            let (stream, addr) = listener.accept().await?;
            tokio::spawn(async move {
                let server_key = server_key;
                let config = config;
                if let Err(err) = super::client_worker::initialize_connection(
                    addr,
                    stream,
                    ServerInfo { server_key, config },
                )
                    .await
                {
                    log::error!("Error during initial client connection {}", err);
                }
            });
        }
    }
}

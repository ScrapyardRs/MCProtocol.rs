use mc_impl_server::config;
use mc_impl_server::client_status_handler::StatusPartBuilder;
use mc_impl_server::server::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .apply()?;

    let server_config = config! {
        status_retriever: |conn| {
            Ok(StatusPartBuilder::default()
                .total_online(5)
                .max_players(10)
                .motd(format!("Example MOTD, hello world :) You're on V: {:?}", conn.protocol_version())))
        };
    };
    let server = Server::new(server_config)?;
    server.boot_server().await?;
    Ok(())
}

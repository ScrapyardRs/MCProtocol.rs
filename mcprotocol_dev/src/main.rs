use std::sync::Arc;
use tokio::net::TcpListener;

mod worker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    spin_listener().await?;
    Ok(())
}

async fn spin_listener() -> anyhow::Result<()> {
    let server_key = Arc::new(encryption_utils::new_key()?);
    let bind = TcpListener::bind("127.0.0.1:25565").await?;
    while let Ok((stream, addr)) = bind.accept().await {
        let new_key = Arc::clone(&server_key);
        tokio::spawn(async move {
            if let Err(err) = worker::attach_worker(addr, stream, new_key).await {
                println!("Error: {}", err);
            }
        });
    }
    Ok(())
}

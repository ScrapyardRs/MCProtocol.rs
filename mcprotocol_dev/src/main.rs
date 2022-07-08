use tokio::net::TcpListener;

mod worker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    spin_listener().await?;
    Ok(())
}

async fn spin_listener() -> anyhow::Result<()> {
    let bind = TcpListener::bind("127.0.0.1:25565").await?;
    while let Ok((stream, addr)) = bind.accept().await {
        tokio::spawn(async move {
            if let Err(err) = worker::attach_worker(stream, addr).await {
                println!("Error: {}", err);
            }
        });
    }
    Ok(())
}

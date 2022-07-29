use tokio::net::TcpStream;
use mc_registry::client_bound::status::Response;

pub struct ServerPing {
    pub response: Response,
    pub latency: i64,
}

pub async fn ping<A>(address: A) -> anyhow::Result<ServerPing> {
    let (read, write) = TcpStream::connect(address).await?.into_split();

    todo!()
}

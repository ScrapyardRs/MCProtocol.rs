mod login_sampler;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    login_sampler::run().await?;
    Ok(())
}

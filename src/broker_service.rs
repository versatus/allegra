#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let broker = conductor::broker::Broker::new(
        "127.0.0.1:5555", "127.0.0.1:5556"
    ).await?;

    broker.start().await?;
    Ok(())
}

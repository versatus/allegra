use allegra::state::StateManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pd_endpoints = vec!["127.0.0.1:2379"];
    let subscriber_uri = "127.0.0.1:5556";
    let publisher_uri = "127.0.0.1:5555";
    let mut state_manager = StateManager::new(
        pd_endpoints,
        subscriber_uri,
        publisher_uri
    ).await?;

    state_manager.run().await?;

    Ok(())
}

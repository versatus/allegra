use allegra::state::StateManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;

    let pd_endpoints = vec!["127.0.0.1:2379"];
    log::info!("Established pd_endpoints");
    let subscriber_uri = "127.0.0.1:5556";
    log::info!("Established subscriber_uri");
    let publisher_uri = "127.0.0.1:5555";
    log::info!("Established publisher_uri");
    let mut state_manager = StateManager::new(
        pd_endpoints,
        subscriber_uri,
        publisher_uri
    ).await?;

    state_manager.run().await?;

    Ok(())
}

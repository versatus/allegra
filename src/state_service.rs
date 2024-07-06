use allegra::{helpers::{load_or_get_publisher_uri, load_or_get_subscriber_uri, load_or_get_pd_endpoints}, state::StateManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;

    let pd_endpoints = load_or_get_pd_endpoints(None).await?;
    log::info!("Established pd_endpoints");
    let subscriber_uri = load_or_get_subscriber_uri(None).await?;
    log::info!("Established subscriber_uri");
    let publisher_uri = load_or_get_publisher_uri(None).await?;
    log::info!("Established publisher_uri");
    let mut state_manager = StateManager::new(
        pd_endpoints,
        &subscriber_uri,
        &publisher_uri
    ).await?;

    state_manager.run().await?;

    Ok(())
}

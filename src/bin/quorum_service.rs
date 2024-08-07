use allegra::{
    dht::QuorumManager,
    helpers::{load_or_create_ethereum_address, load_or_get_public_ip_addresss},
    network::peer::Peer,
    statics::*,
};
use tokio::task::spawn;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    log::info!("Attempting to load ethereum address");
    let wallet_address = load_or_create_ethereum_address(None).await?;
    log::info!("Attempting to load public ip address");
    let ip_address = load_or_get_public_ip_addresss(None).await?;
    log::info!("Attempting to create local peer from wallet and ip addresses");
    let local_peer = Peer::new(wallet_address, ip_address);
    log::info!("Attempting to create QuorumManager");
    let mut quorum_manager = QuorumManager::new(
        &std::env::var("SUBSCRIBER_ADDRESS").unwrap_or(DEFAULT_SUBSCRIBER_ADDRESS.to_string()),
        &std::env::var("PUBLISHER_ADDRESS").unwrap_or(DEFAULT_PUBLISHER_ADDRESS.to_string()),
        local_peer,
    )
    .await?;

    log::info!("established channel");
    let quorum_manager_handle = spawn(async move {
        let _ = quorum_manager.run().await;
    });
    log::info!("setup quorum_manager thread");

    quorum_manager_handle.await?;

    Ok(())
}

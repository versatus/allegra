use allegra::{dht::{Peer, QuorumManager}, helpers::get_public_ip};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;

    let local_peer_id = Uuid::new_v4();
    let address = get_public_ip().await?;
    let local_peer = Peer::new(local_peer_id, address);
    let mut quorum_manager = QuorumManager::new(
        "127.0.0.1:5556",
        "127.0.0.1:5555",
        local_peer
    ).await?;

    let (_stop_tx, mut stop_rx) = tokio::sync::mpsc::channel(1024);
    log::info!("established channel");
    let quorum_manager_handle = tokio::task::spawn(async move {
        let _ = quorum_manager.run(&mut stop_rx).await;
    });
    log::info!("setup quorum_manager thread");

    quorum_manager_handle.await?;

    Ok(())
}

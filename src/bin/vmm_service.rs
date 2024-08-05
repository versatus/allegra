use allegra::core::vmm::vm_manager::VmManager;
use allegra::helpers::helpers::{load_or_create_ethereum_address, load_or_get_public_ip_addresss};
use allegra::Peer;


#[tokio::main]
async fn main() -> std::io::Result<()> {
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    log::info!("Attempting to load ethereum address");
    let wallet_address = load_or_create_ethereum_address(None).await?;
    log::info!("Attempting to load public ip address");
    let ip_address = load_or_get_public_ip_addresss(None).await?;
    log::info!("Attempting to create local peer from wallet and ip addresses");
    let local_peer = Peer::new(wallet_address, ip_address);
    log::info!("Attempting to create QuorumManager");

    let mut vmm = VmManager::new(2222, local_peer).await?;

    log::info!("created vmm manager");

    let (_stop_tx, mut stop_rx) = tokio::sync::mpsc::channel(1024);
    log::info!("established channel");
    let vmm_handle = tokio::task::spawn(async move {
        let _ = vmm.run(&mut stop_rx).await;
    });
    log::info!("setup vmm thread");

    vmm_handle.await?;
    Ok(())
}

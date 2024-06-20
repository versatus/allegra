use allegra::vmm::VmManager;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;

    let mut vmm = VmManager::new(2222).await?;

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

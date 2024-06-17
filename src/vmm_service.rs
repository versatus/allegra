#[tokio::main]
async fn main() -> std::io::Result<()> {

    let pd_endpoints = vec!["127.0.0.1:2379"];
    let mut vmm = VmManager::new(
        pd_endpoints,
        None,
        2222,
        event_broker.clone()
    ).await?;
    log::info!("created vmm manager");

    let task_cache = vmm.task_cache();
    log::info!("create task cache");

    let (_stop_tx, mut stop_rx) = tokio::sync::mpsc::channel(1024);
    log::info!("established channel");
    log::info!("established channel");
    let vmm_handle = tokio::task::spawn(async move {
        let _ = vmm.run(
            &mut rx,
            &mut stop_rx
        ).await;
    });
    log::info!("setup vmm thread");

    Ok(())
}

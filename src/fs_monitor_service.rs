use libretto::{client::LibrettoClient, pubsub::FilesystemPublisher, watcher::monitor_directory};
use allegra::helpers::load_or_get_vmm_filesystem;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;

    log::info!("Creating LibrettoClient...");
    let client = LibrettoClient::new("127.0.0.1:5556", "127.0.0.1:5555").await?;
    log::info!("Setting up event handler for Libretto...");
    let event_handler = tokio::spawn(
        async move {
            client.run().await?;
            Ok::<(), std::io::Error>(())
        }
    );

    let dir = load_or_get_vmm_filesystem(None).await?; 
    log::info!("Acquired filesystem to monitor: {dir}...");
    let dir_current_state: Vec<String> = std::fs::read_dir(dir.clone())?.into_iter().filter_map(|p| {
        match p {
            Ok(path) => Some(path.path().display().to_string()),
            _ => None
        }
    }).collect();
    log::info!("Current state of dir to monitor: {:?}", dir_current_state);
    let publisher = FilesystemPublisher::new("127.0.0.1:5555").await?;
    let monitor = tokio::spawn(async move {
        let _ = monitor_directory(
            &dir,
            publisher
        ).await;
    });

    let _ = tokio::join!(monitor, event_handler);

    Ok(())
}

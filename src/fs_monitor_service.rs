use libretto::{client::LibrettoClient, pubsub::FilesystemPublisher, watcher::monitor_directory};
use allegra::statics::DEFAULT_LXD_STORAGE_POOL;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let client = LibrettoClient::new("127.0.0.1:5556", "127.0.0.1:5556").await?;
    let event_handler = tokio::spawn(
        async move {
            client.run().await?;
            Ok::<(), std::io::Error>(())
        }
    );

    let dir = std::env::var(
        "LXD_STORAGE_POOL"
    ).unwrap_or_else(|_| {
        DEFAULT_LXD_STORAGE_POOL.to_string()
    });
    dbg!(&dir);
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

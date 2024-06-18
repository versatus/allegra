use libretto::watcher::monitor_directory;
use libretto::client::handle_events;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::VecDeque;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref DEFAULT_LXD_STORAGE_POOL: &'static str = "/mnt/libretto/lxd-storage-pool"; 
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (_stop_tx, stop_rx) = std::sync::mpsc::channel();
    let queue = Arc::new(
        RwLock::new(
            VecDeque::new()
        )
    );

    let event_handling_queue = queue.clone();
    let event_handler = tokio::spawn(async move {
        handle_events(event_handling_queue.clone()).await;
    });

    let dir = std::env::var(
        "LXD_STORAGE_POOL"
    ).unwrap_or_else(|_| {
        DEFAULT_LXD_STORAGE_POOL.to_string()
    });
    dbg!(&dir);
    let monitor = tokio::spawn(async move {
        let _ = monitor_directory(
            &dir,
            queue,
            stop_rx
        ).await;
    });

    let _ = tokio::join!(monitor, event_handler);

    Ok(())
}

use std::net::SocketAddr;
use allegra::rpc::VmmServer;
use tarpc::server::{self, incoming::Incoming, Channel};
use tarpc::tokio_serde::formats::Json;
use futures::{
    prelude::*
};
use futures::stream::StreamExt;
use allegra::rpc::Vmm;
use allegra::vmm::VmManager;

async fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
    tokio::spawn(fut);
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;

    let addr: SocketAddr = "0.0.0.0:29292".parse().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;

    let mut listener = tarpc::serde_transport::tcp::listen(&addr, Json::default).await?;
    dbg!(listener.local_addr());

    let (tx, rx) = tokio::sync::mpsc::channel(1024);
    let (_stop_tx, stop_rx) = tokio::sync::mpsc::channel(1024);
    let pd_endpoints = vec!["127.0.0.1:2379"];
    let vmm = VmManager::new(
        pd_endpoints, None
    ).await?;

    let task_cache = vmm.task_cache();
    tokio::task::spawn(
        vmm.run(rx, stop_rx)
    );
    let pd_endpoints = vec!["127.0.0.1:2379"];
    let tikv_client = tikv_client::RawClient::new(
        pd_endpoints
    ).await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    })?;
    listener.config_mut().max_frame_length(usize::MAX);
    listener
        .filter_map(|r| future::ready(r.ok()))
        .map(server::BaseChannel::with_defaults)
        .max_channels_per_key(1, |t| t.transport().peer_addr().unwrap().ip())
        .map(|channel| {
            let server = VmmServer {
                network: "lxdbr0".to_string(),
                port: 2222,
                vmm_sender: tx.clone(),
                tikv_client: tikv_client.clone(),
                task_cache: task_cache.clone()
            };
            channel.execute(server.serve()).for_each(spawn)
        })
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;

    Ok(())
}

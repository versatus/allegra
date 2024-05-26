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


    let (tx, rx) = tokio::sync::mpsc::channel(1024);
    let (_stop_tx, stop_rx) = tokio::sync::mpsc::channel(1024);
    let pd_endpoints = vec!["127.0.0.1:2379"];
    let vmm = VmManager::new(
        pd_endpoints, None, 2223
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

    let mut next_port = 2223;
    loop {
        let mut listener = tarpc::serde_transport::tcp::listen(&addr, Json::default).await?; 
        listener.config_mut().max_frame_length(usize::MAX);
        tokio::select! {
            connection = listener.next() => {
                match connection {
                    Some(Ok(transport)) => {
                        log::info!("Accepted connection");
                        let vmmserver = VmmServer {
                            network: "lxdbr0".to_string(),
                            port: next_port,
                            vmm_sender: tx.clone(),
                            tikv_client: tikv_client.clone(),
                            task_cache: task_cache.clone()
                        };
                        let channel = server::BaseChannel::with_defaults(transport);
                        channel.execute(vmmserver.serve()).for_each_concurrent(None, |f| {
                            tokio::spawn(async move {
                                f.await
                            });
                            futures::future::ready(())
                        }).await;
                    }
                    _ => {}
                }

            }
        }
        next_port += 1;
    }


    Ok(())
}

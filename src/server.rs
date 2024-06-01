use allegra::grpc::VmmService;

use futures::{
    prelude::*
};

use allegra::allegra_rpc::{vmm_server::VmmServer};
use allegra::vmm::VmManager;
use tonic::{transport::Server};

pub async fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
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


    let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
    let (_stop_tx, mut stop_rx) = tokio::sync::mpsc::channel(1024);
    let pd_endpoints = vec!["127.0.0.1:2379"];
    let mut vmm = VmManager::new(
        pd_endpoints, None, 2223
    ).await?;

    let task_cache = vmm.task_cache();
    let vmm_handle = tokio::task::spawn(async move {
        let _ = vmm.run(&mut rx, &mut stop_rx).await;
    });
    let pd_endpoints = vec!["127.0.0.1:2379"];
    let tikv_client = tikv_client::RawClient::new(
        pd_endpoints
    ).await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    })?;

    let next_port = 2223;

    let service = VmmService {
        network: "lxdbr0".to_string(),
        port: next_port,
        vmm_sender: tx.clone(),
        tikv_client,
        task_cache,
    };

    let vmmserver = VmmServer::new(
        service
    );

    let addr = "[::1]:50051".parse().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;

    Server::builder().add_service(vmmserver).serve(addr).await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;

    vmm_handle.await?;
    Ok(())
}

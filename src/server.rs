#![allow(unused)]
use allegra::broker::broker::EventBroker;
use allegra::dht::{Peer, AllegraNetworkState};
use allegra::grpc::VmmService;

use allegra::helpers::get_public_ip;
use futures::{
    prelude::*
};

use allegra::allegra_rpc::{vmm_server::VmmServer};
use allegra::vmm::VmManager;
use tokio::sync::RwLock;
use tonic::{transport::Server};
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast::channel};

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

    let local_peer_id = uuid::Uuid::new_v4();
    let local_peer_address = get_public_ip().await?; 

    let local_peer = Peer::new(local_peer_id, local_peer_address);

    let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
    let (_stop_tx, mut stop_rx) = tokio::sync::mpsc::channel(1024);
    let pd_endpoints = vec!["127.0.0.1:2379"];

    let event_broker = Arc::new(
        Mutex::new(
            EventBroker::new()
        )
    );


    let mut network_state = Arc::new(
        RwLock::new(
            AllegraNetworkState::new(
                event_broker.clone()
            )
        )
    );

    let mut guard = network_state.write().await; 
    guard.add_peer(&local_peer).await?;

    #[cfg(not(feature="bootstrap"))]
    {
        let bootstrap_addr = std::env::var(
            "BOOTSTRAP_ADDR"
        ).expect(
            "If not configured as bootsrap node, bootstrap node is required"
        );
        let bootstrap_id = std::env::var(
            "BOOTSTRAP_ID"
        ).expect(
            "If not configured as boostrap node, bootstrap node is required"
        ); 
        let bootstrap_peer = Peer::new(
            uuid::Uuid::parse_str(
                &bootstrap_id
            ).expect(
                "bootstrap_id must be valid uuid v4"
            ), bootstrap_addr
        );
        guard.add_peer(&bootstrap_peer).await?;
    }
    drop(guard);

    let mut vmm = VmManager::new(
        pd_endpoints,
        None,
        2222,
        event_broker.clone()
    ).await?;

    let task_cache = vmm.task_cache();

    let vmm_handle = tokio::task::spawn(async move {
        let _ = vmm.run(
            &mut rx,
            &mut stop_rx
        ).await;
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

    let next_port = 2222;

    let service = VmmService {
        local_peer,
        network_state,
        network: "lxdbr0".to_string(),
        port: next_port,
        vmm_sender: tx.clone(),
        tikv_client,
        task_cache,
        event_broker: event_broker.clone()
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

    Server::builder().add_service(
        vmmserver
    ).serve(
        addr
    ).await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;

    vmm_handle.await?;

    Ok(())
}

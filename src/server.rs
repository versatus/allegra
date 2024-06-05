#![allow(unused)]
use allegra::broker::broker::EventBroker;
use allegra::client::NetworkClient;
use allegra::dht::{Peer, AllegraNetworkState};
use allegra::event::{Event, NetworkEvent};
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

    let mut guard = event_broker.lock().await;
    guard.get_or_create_topic("Network".to_string());
    guard.get_or_create_topic("Vmm".to_string());
    guard.get_or_create_topic("Dht".to_string());

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
    let bootstrap_addr = std::env::var(
        "BOOTSTRAP_ADDR"
    ).expect(
        "If not configured as bootsrap node, bootstrap node is required"
    );
    #[cfg(not(feature="bootstrap"))]
    let bootstrap_id = std::env::var(
        "BOOTSTRAP_ID"
    ).expect(
        "If not configured as boostrap node, bootstrap node is required"
    ); 
    #[cfg(not(feature="bootstrap"))]
    let bootstrap_peer = Peer::new(
        uuid::Uuid::parse_str(
            &bootstrap_id
        ).expect(
            "bootstrap_id must be valid uuid v4"
        ), bootstrap_addr
    );

    #[cfg(not(feature="bootstrap"))]
    guard.add_peer(&bootstrap_peer).await?;

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

    let mut guard = event_broker.lock().await;
    let mut network_rx = guard.subscribe("Network".to_string()).await;
    drop(guard);

    let mut network_client = NetworkClient::new(
        network_rx, 
        local_peer.id().to_string(),
        local_peer.address().to_string()
    ).await?;

    let network_client_handle = tokio::task::spawn(async move {
        let _ = network_client.run();
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
        local_peer: local_peer.clone(),
        network_state: network_state.clone(),
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

    #[cfg(not(feature="bootstrap"))]
    let guard = network_state.read().await;
    #[cfg(not(feature="bootstrap"))]
    let peer_number = guard.peers().len() + 1;
    #[cfg(not(feature="bootstrap"))]
    let event = Event::NetworkEvent(
        NetworkEvent::NewPeer { 
            peer_id: local_peer.id().to_string(), 
            peer_address: local_peer.address().to_string(), 
            peer_number: peer_number as u32, 
            dst: bootstrap_peer.address().to_string() 
        }
    ); 
    #[cfg(not(feature="bootstrap"))]
    drop(guard);

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

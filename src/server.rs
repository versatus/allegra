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
use std::collections::VecDeque;

use allegra::allegra_rpc::{vmm_server::VmmServer, FILE_DESCRIPTOR_SET};
use allegra::vmm::VmManager;
use tokio::sync::RwLock;
use tonic::{transport::Server};
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast::channel};
use libretto::client::handle_events;
use libretto::watcher;
use tonic_reflection::server::Builder;

lazy_static::lazy_static! {
    static ref DEFAULT_LXD_STORAGE_DIR: &'static str = "/home/ans/projects/sandbox/test-dir/";     
}

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

    log::info!("logger set up");
    let pd_endpoints = vec!["127.0.0.1:2379"];
    log::info!("created pd endpoints");

    let tikv_client = tikv_client::RawClient::new(
        pd_endpoints
    ).await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    })?;
    let local_peer_id = uuid::Uuid::new_v4();
    log::info!("local_peer_id = {}", &local_peer_id.to_string());
    let local_peer_address = get_public_ip().await?; 
    log::info!("local_peer_address: {}", &local_peer_address);

    let local_peer = Peer::new(local_peer_id, local_peer_address);
    log::info!("local peer created");

    let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
    log::info!("established channel");
    let (_stop_tx, mut stop_rx) = tokio::sync::mpsc::channel(1024);
    log::info!("established channel");
    let pd_endpoints = vec!["127.0.0.1:2379"];
    log::info!("set pd endpoints");

    let event_broker = Arc::new(
        Mutex::new(
            EventBroker::new()
        )
    );
    log::info!("set up event broker");

    let mut guard = event_broker.lock().await;
    log::info!("acquired event broker guard");
    guard.get_or_create_topic("Network".to_string());
    log::info!("created network topic");
    guard.get_or_create_topic("Vmm".to_string());
    log::info!("created vmm topic");
    guard.get_or_create_topic("Dht".to_string());
    log::info!("created dht topic");
    drop(guard);
    log::info!("dropped event broker guard");

    let mut network_state = Arc::new(
        RwLock::new(
            AllegraNetworkState::new(
                event_broker.clone()
            )
        )
    );
    log::info!("created network state");

    let mut guard = network_state.write().await; 
    log::info!("acquired network state guard");
    guard.add_peer(&local_peer).await?;
    log::info!("added self to network state");

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

    #[cfg(not(feature="bootstrap"))]
    drop(guard);



    let mut vmm = VmManager::new(
        pd_endpoints,
        None,
        2222,
        event_broker.clone()
    ).await?;
    log::info!("created vmm manager");

    let task_cache = vmm.task_cache();
    log::info!("create task cache");

    let vmm_handle = tokio::task::spawn(async move {
        let _ = vmm.run(
            &mut rx,
            &mut stop_rx
        ).await;
    });
    log::info!("setup vmm thread");
/*
    let mut guard = event_broker.lock().await;
    log::info!("acquired event broker guard");
    let mut network_rx = guard.subscribe("Network".to_string()).await;
    log::info!("acquired network topic rx");
    drop(guard);
    log::info!("dropped event broker guard");
    let mut network_client = NetworkClient::new(
        network_rx, 
        local_peer.id().to_string(),
        local_peer.address().to_string()
    ).await?;
    log::info!("created network client");

    let network_client_handle = tokio::task::spawn(async move {
        let _ = network_client.run();
    });
    log::info!("setup network client thread");

    log::info!("created tikv client");

    let next_port = 2222;
    log::info!("established network port");
*/
/*
    #[cfg(not(feature="bootstrap"))]
    let guard = network_state.read().await;
    #[cfg(not(feature="bootstrap"))]
    let event = Event::NetworkEvent(
        NetworkEvent::NewPeer { 
            peer_id: local_peer.id().to_string(), 
            peer_address: local_peer.address().to_string(), 
            dst: bootstrap_peer.address().to_string() 
        }
    ); 
    #[cfg(not(feature="bootstrap"))]
    drop(guard);
*/
/*    
    let (_stop_tx, stop_rx) = std::sync::mpsc::channel();
    log::info!("created channel");
    let queue = Arc::new(
        RwLock::new(
            VecDeque::new()
        )
    );
    log::info!("created queue");

    let event_handling_queue = queue.clone();
    log::info!("initiated event handling queue");

    let handle_monitor_events = tokio::spawn(async move {
        handle_events(event_handling_queue.clone()).await;
    });
    log::info!("started monitor event handler thread");
    let directory_to_monitor = std::env::var("LXD_STORAGE_DIR").unwrap_or_else(|_| {
        DEFAULT_LXD_STORAGE_DIR.to_string()
    });

    log::info!("acquired directory to monitor");

    let monitor_directory = tokio::spawn(async move {
        watcher::monitor_directory(&directory_to_monitor, queue, stop_rx).await;
    });

    log::info!("started monitor directory thread");
    */

    let service = VmmService {
//        local_peer: local_peer.clone(),
//        network_state: network_state.clone(),
//        network: "lxdbr0".to_string(),
//        port: next_port,
//        vmm_sender: tx.clone(),
//        tikv_client,
//        task_cache,
//        event_broker: event_broker.clone()
    };

    log::info!("created vmm service");

    let vmmserver = VmmServer::new(
        service
    );

    log::info!("created vmm server");

    let addr = "0.0.0.0:50051".parse().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;
    log::info!("established address to listen on for grpc");

    log::info!("running grpc server on {}", &addr);
    let reflection_service = Builder::configure().register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET).build().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;
    Server::builder().add_service(vmmserver)
        .add_service(reflection_service)
        .serve(addr).await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;
    vmm_handle.await?;
    // handle_monitor_events.await;
    // monitor_directory.await;

    Ok(())
}

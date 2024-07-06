use allegra::account::TaskId;
use allegra::event::{NetworkEvent, QuorumEvent};
use allegra::{dht::Peer, statics::DEFAULT_NETWORK};
use allegra::grpc::VmmService;
use allegra::helpers::{load_bootstrap_node, load_or_create_ethereum_address, load_or_get_public_ip_addresss};
use allegra::publish::{GenericPublisher, NetworkTopic, QuorumTopic};
use allegra::subscribe::RpcResponseSubscriber;
use futures::prelude::*;
use allegra::allegra_rpc::{vmm_server::VmmServer, FILE_DESCRIPTOR_SET};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tonic::transport::Server;
use uuid::Uuid;
use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::Mutex;
use tonic_reflection::server::Builder;
use conductor::publisher::PubStream;
#[allow(unused)]
use allegra::statics::{
    DEFAULT_LXD_STORAGE_POOL,
    DEFAULT_GRPC_ADDRESS,
    DEFAULT_SUBSCRIBER_ADDRESS,
    DEFAULT_PUBLISHER_ADDRESS
};

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

    let wallet_address = load_or_create_ethereum_address(None).await?; 
    log::info!("local wallet address: {}", &wallet_address);

    let ip_address = load_or_get_public_ip_addresss(None).await?; 
    log::info!("local ip address = {}", &ip_address);

    let local_peer = Peer::new(wallet_address, ip_address);
    log::info!("local peer created");


    let publisher_uri = std::env::var(
        "PUBLISHER_ADDRESS"
    ).unwrap_or(
        DEFAULT_PUBLISHER_ADDRESS.to_string()
    );

    let publisher = Arc::new(
        Mutex::new(
            GenericPublisher::new(&publisher_uri).await?
        )
    );

    let to_dial = if let Ok(bootstrap_nodes) = load_bootstrap_node(None).await {
        bootstrap_nodes.par_iter().filter_map(|(wallet, ip)| {
            let bootstrap_peer = Peer::new(
                wallet.clone(), ip.clone() 
            );

            if bootstrap_peer != local_peer {
                log::warn!("bootstrap peer: {:?} is not local peer, dialing...", &bootstrap_peer);
                Some(bootstrap_peer)
            } else {
                None
            }
        }).collect::<Vec<Peer>>()
    } else {
        vec![]
    };

    let mut guard = publisher.lock().await;

    for peer in to_dial {
        let event_id = Uuid::new_v4().to_string();
        let task_id = TaskId::new(Uuid::new_v4().to_string());
        let received_from = local_peer.clone();
        let event = QuorumEvent::NewPeer { event_id, task_id, peer: peer.clone(), received_from: received_from.clone()};
        guard.publish(
            Box::new(QuorumTopic), 
            Box::new(event)
        ).await?;

        let event_id = Uuid::new_v4().to_string();
        let task_id = TaskId::new(Uuid::new_v4().to_string());
        log::warn!("Sending network event to Bootstrap new peer");
        let event = NetworkEvent::NewPeer { 
            event_id,
            task_id,
            peer_id: local_peer.wallet_address_hex().clone(), 
            peer_address: local_peer.ip_address().to_string(), 
            dst: peer.ip_address().to_string(),
            received_from: received_from.clone(),
        };
        guard.publish(
            Box::new(NetworkTopic),
            Box::new(event)
        ).await?;
    }

    let event_id = Uuid::new_v4().to_string();
    let task_id = TaskId::new(Uuid::new_v4().to_string());
    let event = QuorumEvent::NewPeer { event_id, task_id, peer: local_peer.clone(), received_from: received_from.clone() };
    log::info!("publishing event to add self to quorum...");
    guard.publish(
        Box::new(QuorumTopic),
        Box::new(event),
    ).await?;
    drop(guard);

    let next_port = 2222;
    log::info!("established network port");


    let subscriber_uri = std::env::var(
        "SUBSCRIBER_ADDRESS"
    ).unwrap_or(
        DEFAULT_SUBSCRIBER_ADDRESS.to_string()
    );
    
    let subscriber = Arc::new(
        Mutex::new(
            RpcResponseSubscriber::new(&subscriber_uri).await?
        )
    );

    let lxd_network_interface = std::env::var(
        "LXD_NETWORK_INTERFACE"
    ).unwrap_or(
        DEFAULT_NETWORK.to_string()
    );

    let service = VmmService {
        local_peer: local_peer.clone(),
        network: lxd_network_interface,
        port: next_port,
        task_log: Arc::new(Mutex::new(HashSet::new())),
        publisher: publisher.clone(),
        subscriber: subscriber.clone()
    };

    let vmmserver = VmmServer::new(
        service
    );
    log::info!("created vmm server");

    let addr = std::env::var(
        "GRPC_ADDRESS"
    ).unwrap_or(
        DEFAULT_GRPC_ADDRESS.to_string()
    ).parse()
    .map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;

    log::info!("established address to listen on for grpc...");

    log::info!("running grpc server on {}", &addr);
    let reflection_service = Builder::configure()
        .register_encoded_file_descriptor_set(
            FILE_DESCRIPTOR_SET
        ).build().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        }
    )?;

    log::info!("set up reflection service for grpc server...");

    Server::builder().add_service(vmmserver)
        .add_service(reflection_service)
        .serve(addr).await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;

    Ok(())
}

use allegra::dht::Peer;
use allegra::grpc::VmmService;

use allegra::helpers::get_public_ip;
use allegra::publish::GenericPublisher;
use allegra::subscribe::RpcResponseSubscriber;
use futures::prelude::*;

use allegra::allegra_rpc::{vmm_server::VmmServer, FILE_DESCRIPTOR_SET};
use tonic::transport::Server;
use std::sync::Arc;
use tokio::sync::Mutex;
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

    let local_peer_id = uuid::Uuid::new_v4();
    log::info!("local_peer_id = {}", &local_peer_id.to_string());
    let local_peer_address = get_public_ip().await?; 
    log::info!("local_peer_address: {}", &local_peer_address);
    let local_peer = Peer::new(local_peer_id, local_peer_address);
    log::info!("local peer created");


    /*
     * TODO(asmith): Replace with publish event to Quorum topic
    let mut quorum_manager = Arc::new(RwLock::new(QuorumManager::new()));
    log::info!("created quorum_manager");

    let mut guard = quorum_manager.write().await; 
    log::info!("acquired quorum_manager guard");
    guard.add_peer(&local_peer).await?;
    log::info!("added self to network state");
    */

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
        let _bootstrap_peer = Peer::new(
            uuid::Uuid::parse_str(
                &bootstrap_id
            ).expect(
                "bootstrap_id must be valid uuid v4"
            ), bootstrap_addr
        );
    }
/*
 * TODO: Replace with publishing events to Quorum & Network topic
    #[cfg(not(feature="bootstrap"))]
    guard.add_peer(&bootstrap_peer).await?;

    #[cfg(not(feature="bootstrap"))]
    drop(guard);

    //let mut guard = event_broker.lock().await;
    //log::info!("acquired event broker guard");
    //let mut network_rx = guard.subscribe("Network".to_string()).await;
    //log::info!("acquired network topic rx");
    //drop(guard);
    log::info!("dropped event broker guard");
    let mut network_client = NetworkClient::new(
        //TODO(asmith): Replace with subscriber
        //network_rx, 
        local_peer.id().to_string(),
        local_peer.address().to_string()
    ).await?;
    log::info!("created network client");

    let network_client_handle = tokio::task::spawn(async move {
        let _ = network_client.run();
    });
    log::info!("setup network client thread");

    log::info!("created tikv client");


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

    log::info!("started monitor directory thread");
    let (tx, mut rx) = tokio::sync::mpsc::channel(1024);

    */

    let next_port = 2222;
    log::info!("established network port");
    
    let vmm_publisher = Arc::new(
        Mutex::new(
            GenericPublisher::new("127.0.0.1:5555").await?
        )
    );

    let service = VmmService {
        local_peer: local_peer.clone(),
        network: "lxdbr0".to_string(),
        port: next_port,
        publisher: vmm_publisher,
        subscriber: RpcResponseSubscriber::new("127.0.0.1:5556").await?
    };

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

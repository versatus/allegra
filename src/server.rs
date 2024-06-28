use allegra::{dht::Peer, statics::DEFAULT_NETWORK};
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

    let local_peer_id = uuid::Uuid::new_v4();
    log::info!("local_peer_id = {}", &local_peer_id.to_string());
    let local_peer_address = get_public_ip().await?; 
    log::info!("local_peer_address: {}", &local_peer_address);
    let local_peer = Peer::new(local_peer_id, local_peer_address);
    log::info!("local peer created");


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

    let next_port = 2222;
    log::info!("established network port");

    let publisher_uri = std::env::var(
        "PUBLISHER_ADDRESS"
    ).unwrap_or(
        DEFAULT_PUBLISHER_ADDRESS.to_string()
    );

    let subscriber_uri = std::env::var(
        "SUBSCRIBER_ADDRESS"
    ).unwrap_or(
        DEFAULT_SUBSCRIBER_ADDRESS.to_string()
    );
    
    let publisher = Arc::new(
        Mutex::new(
            GenericPublisher::new(&publisher_uri).await?
        )
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

#![allow(unused)]
use allegra::grpc::VmmService;
use allegra::vm_types::VmType;
use allegra::allegra_rpc::{InstanceCreateParams, InstanceStopParams};
use tonic::transport::Server;
use std::sync::{Arc, RwLock};
use lru::LruCache;
use std::time::Duration;
use std::num::NonZeroUsize;
use allegra::allegra_rpc::vmm_server::VmmServer;
use allegra::allegra_rpc::vmm_client::VmmClient;


#[tokio::main]
async fn main() -> std::io::Result<()> {
/*
    let pd_endpoints = vec!["127.0.0.1:2379"];
    let tikv_client = tikv_client::RawClient::new(
        pd_endpoints
    ).await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    })?;
    let (tx, _rx) = tokio::sync::mpsc::channel(1024);
    let task_cache = Arc::new(
        RwLock::new(
            LruCache::new(
                NonZeroUsize::new(
                    1024
                ).ok_or(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Unable to create non-zero usize"
                    )
                )?
            )
        )
    ); 
    
    let next_port = 2222;

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

    tokio::spawn(async move {
        Server::builder().add_service(vmmserver).serve(addr).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        Ok::<(), std::io::Error>(())
    });

    let mut client = VmmClient::connect("http://[::1]:50051").await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;

    let name = "testVm".to_string();
    let distro = "ubuntu".to_string();
    let version = "22.04".to_string();
    let vmtype = VmType::T2Nano.to_string();

    let vmm_response = client.create_vm(
        InstanceCreateParams {
            name: name.clone(),
            distro,
            version,
            vmtype,
            sig: "testSignature".to_string(),
            recovery_id: u32::default()
        }
    ).await;

    println!("{:?}", vmm_response);

    tokio::time::sleep(Duration::from_secs(120)).await;

    let stop_vm = InstanceStopParams { name, sig: "testSignature".to_string(), recovery_id: u32::default() };
    let vmm_response = client.shutdown_vm(stop_vm).await;

    println!("{:?}", vmm_response);

*/
    Ok(())
}

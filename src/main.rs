use allegra::rpc::{VmmServer, VmmClient};
use allegra::vm_types::VmType;
use allegra::params::{InstanceCreateParams, InstanceStopParams};
use tarpc::{context, client};
use tarpc::server::Channel;
use std::sync::{Arc, RwLock};
use lru::LruCache;
use std::time::Duration;
use allegra::rpc::Vmm;
use futures::prelude::*;
use std::num::NonZeroUsize;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (client_transport, server_transport) = tarpc::transport::channel::unbounded();
    let pd_endpoints = vec!["127.0.0.1:2379"];
    let tikv_client = tikv_client::RawClient::new(
        pd_endpoints
    ).await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    })?;
    let server = tarpc::server::BaseChannel::with_defaults(server_transport);
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
    tokio::spawn(
        server.execute(
            VmmServer {
                network: "lxdbr0".to_string(),
                port: 2222,
                vmm_sender: tx.clone(),
                tikv_client,
                task_cache
            }.serve()
        )
        .for_each(|response| async move {
            tokio::spawn(response);
        }));

    let client = VmmClient::new(client::Config::default(), client_transport).spawn();

    let name = "testVm".to_string();
    let distro = "ubuntu".to_string();
    let version = "22.04".to_string();
    let vmtype = VmType::T2Nano;

    let vmm_response = client.create_vm(
        context::current(),
        InstanceCreateParams {
            name: name.clone(),
            distro,
            version,
            vmtype,
            sig: "testSignature".to_string(),
            recovery_id: u8::default()
        }
    ).await;

    println!("{:?}", vmm_response);

    tokio::time::sleep(Duration::from_secs(120)).await;

    let mut context = context::current();
    context.deadline = std::time::SystemTime::now() + Duration::from_secs(30);
    let stop_vm = InstanceStopParams { name, sig: "testSignature".to_string(), recovery_id: u8::default() };
    let vmm_response = client.shutdown_vm(context, stop_vm).await;

    println!("{:?}", vmm_response);

    Ok(())
}

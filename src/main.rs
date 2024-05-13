use allegra::rpc::{VmmServer, VmmClient};
use allegra::vm_types::VmType;
use allegra::vmm::{InstanceCreateParams, InstanceStopParams};
use tarpc::{context, client};
use tarpc::server::{Channel};


use std::time::Duration;
use allegra::rpc::Vmm;
use futures::{
    prelude::*
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (client_transport, server_transport) = tarpc::transport::channel::unbounded();

    let server = tarpc::server::BaseChannel::with_defaults(server_transport);
    tokio::spawn(
        server.execute(
            VmmServer {
                network: "lxdbr0".to_string(),
                port: 2222,
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
        }
    ).await;

    println!("{:?}", vmm_response);

    tokio::time::sleep(Duration::from_secs(120)).await;

    let mut context = context::current();
    context.deadline = std::time::SystemTime::now() + Duration::from_secs(30);
    let stop_vm = InstanceStopParams { name };
    let vmm_response = client.shutdown_vm(context, stop_vm).await;

    println!("{:?}", vmm_response);

    Ok(())
}

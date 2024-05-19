use std::net::SocketAddr;
use tarpc::{client, context, tokio_serde::formats::Json};
use allegra::{rpc::VmmClient, vmm::{InstanceCreateParams, InstanceStopParams}, vm_types::VmType};


#[tokio::main]
async fn main() -> std::io::Result<()> {

    let addr: SocketAddr = "127.0.0.1:29292".parse().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;

    let mut transport = tarpc::serde_transport::tcp::connect(addr, Json::default);
    transport.config_mut().max_frame_length(usize::MAX);

    let client = VmmClient::new(client::Config::default(), transport.await?).spawn();

    let mut context = context::current();
    context.deadline = std::time::SystemTime::now() + std::time::Duration::from_secs(60);

//    let res = client.create_vm(
//        context,
//        InstanceCreateParams {
//            name: "testRpcClientServerVm".to_string(),
//            distro: "ubuntu".to_string(),
//            version: "22.04".to_string(),
//            vmtype: VmType::T2Micro, 
//            sig: "testSignature".to_string()
//        }
//    ).await;

//    println!("{res:#?}");

//    let mut context = context::current();
//    context.deadline = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
//    tokio::time::sleep(tokio::time::Duration::from_secs(240)).await;
    let res = client.shutdown_vm(
        context,
        InstanceStopParams {
            name: "testRpcClientServerVm".to_string(),
            sig: "testSignature".to_string()
        }
    ).await;

    println!("{res:#?}");

    Ok(())
}

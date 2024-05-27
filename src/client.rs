use allegra::allegra_rpc::InstanceStopParams;
use allegra::{vm_types::VmType, allegra_rpc::InstanceCreateParams};
use allegra::allegra_rpc::vmm_client::VmmClient;


#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut client = VmmClient::connect("http://[::1]:50051").await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;

    let res = client.create_vm(
        InstanceCreateParams {
            name: "testRpcClientServerVm".to_string(),
            distro: "ubuntu".to_string(),
            version: "22.04".to_string(),
            vmtype: VmType::T2Micro.to_string(), 
            sig: "testSignature".to_string(),
            recovery_id: u32::default(),
        }
    ).await;

    println!("{res:#?}");

    tokio::time::sleep(tokio::time::Duration::from_secs(240)).await;
    let res = client.shutdown_vm(
        InstanceStopParams {
            name: "testRpcClientServerVm".to_string(),
            sig: "testSignature".to_string(),
            recovery_id: u32::default(),
        }
    ).await;

    println!("{res:#?}");

    Ok(())
}

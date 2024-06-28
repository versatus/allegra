use allegra::allegra_rpc::MessageHeader;
use allegra::helpers::get_public_ip;
use allegra::allegra_rpc::PingMessage;
use allegra::allegra_rpc::vmm_client::VmmClient;


#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut client = VmmClient::connect("http://127.0.0.1:50051").await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;

    let peer_id = uuid::Uuid::new_v4();
    let message_id = uuid::Uuid::new_v4();
    let peer_address = get_public_ip().await?; 
    let header = MessageHeader {
        peer_id: peer_id.to_string(),
        peer_address,
        message_id: message_id.to_string()
    };

    let request = PingMessage {
        header: Some(header),
    };

    let res = client.ping(
        request
    ).await;

    println!("{res:#?}");

    /*
    tokio::time::sleep(tokio::time::Duration::from_secs(240)).await;
    let res = client.shutdown_vm(
        InstanceStopParams {
            name: "testRpcClientServerVm".to_string(),
            sig: "testSignature".to_string(),
            recovery_id: u32::default(),
        }
    ).await;

    println!("{res:#?}");
    */

    Ok(())
}

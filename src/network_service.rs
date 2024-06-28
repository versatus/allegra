use allegra::client::NetworkClient;
use allegra::helpers::get_public_ip;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let local_peer_id = Uuid::new_v4().to_string();
    let local_peer_address = get_public_ip().await?;
    let subscriber_uri = "127.0.0.1:5556";
    let publisher_uri = "127.0.0.1:5555";
    let networking_client = NetworkClient::new(
        local_peer_id, 
        local_peer_address, 
        subscriber_uri, 
        publisher_uri
    ).await?;

    networking_client.run().await?;

    Ok(())
}

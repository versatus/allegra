use allegra::client::NetworkClient;
use allegra::helpers::{load_or_create_ethereum_address, load_or_get_public_ip_addresss};
use allegra::network::peer::Peer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let wallet_address = load_or_create_ethereum_address(None).await?;
    log::info!("local wallet address: {}", &wallet_address);

    let ip_address = load_or_get_public_ip_addresss(None).await?;
    log::info!("local ip address = {}", &ip_address);

    let local_peer = Peer::new(wallet_address, ip_address);
    let subscriber_uri = "127.0.0.1:5556";
    let publisher_uri = "127.0.0.1:5555";
    let networking_client = NetworkClient::new(local_peer, subscriber_uri, publisher_uri).await?;

    networking_client.run().await?;

    Ok(())
}

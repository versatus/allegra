use std::net::SocketAddr;
use allegra::rpc::VmmServer;
use tarpc::server::{self, incoming::Incoming, Channel};
use tarpc::tokio_serde::formats::Json;
use futures::{
    prelude::*
};
use futures::stream::StreamExt;
use allegra::rpc::Vmm;

async fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
    tokio::spawn(fut);
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let addr: SocketAddr = "127.0.0.1:29292".parse().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;

    let mut listener = tarpc::serde_transport::tcp::listen(&addr, Json::default).await?;
    dbg!(listener.local_addr());

    listener.config_mut().max_frame_length(usize::MAX);
    listener
        .filter_map(|r| future::ready(r.ok()))
        .map(server::BaseChannel::with_defaults)
        .max_channels_per_key(1, |t| t.transport().peer_addr().unwrap().ip())
        .map(|channel| {
            let server = VmmServer {
                network: "lxdbr0".to_string(),
                port: 2222,
            };
            channel.execute(server.serve()).for_each(spawn)
        })
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;

    Ok(())
}

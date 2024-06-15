use allegra::grpc_light::LightVmmService;

use allegra::allegra_rpc::{vmm_server::VmmServer, FILE_DESCRIPTOR_SET};
use tonic::transport::Server;
use tonic_reflection::server::Builder;

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

    let reflection_service = Builder::configure().register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET).build().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e
        )
    })?;

    let addr = "0.0.0.0:50051".parse().unwrap();
    let light_server = LightVmmService {};
    let vmmserver = VmmServer::new(
        light_server
    );
    log::info!("created vmm server");
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

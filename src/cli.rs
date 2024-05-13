use std::net::SocketAddr;
use std::time::{SystemTime, Duration};

use allegra::rpc::VmmClient;
use allegra::vmm::{InstanceCreateParams, InstanceStartParams, InstanceStopParams, InstanceAddPubkeyParams, InstanceDeleteParams};
use clap::{Parser, Subcommand};


use tarpc::{context, client};
use tarpc::tokio_serde::formats::Json;


#[derive(Parser)]
#[command(name = "AllegraCli")]
#[command(version = "0.1.0")]
#[command(about = "enables the provisioning, starting, stopping, and deleting of VMs in the Versatus Network")]
struct Cli {
    #[command(subcommand)]
    command: AllegraCommands
}


#[derive(Subcommand)]
enum AllegraCommands {
    Create(InstanceCreateParams),
    Start(InstanceStartParams),
    Stop(InstanceStopParams),
    AddPubkey(InstanceAddPubkeyParams),
    Delete(InstanceDeleteParams),
}

#[tokio::main]
async fn main() -> std::io::Result<()> {

    let cli = Cli::parse();

    match &cli.command {
        AllegraCommands::Create(params) => {
            println!("Creating an Allegra Instance: {:?}", &params);
            //TODO(asmith): Replace with environment variable
            let addr: SocketAddr = "127.0.0.1:29292".parse().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            let mut client_transport = tarpc::serde_transport::tcp::connect(addr, Json::default);
            client_transport.config_mut().max_frame_length(usize::MAX);
            let vmclient = VmmClient::new(
                client::Config::default(),
                client_transport.await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?
            ).spawn();

            let resp = vmclient.create_vm(context::current(), params.clone()).await;
            println!("Response: {:?}", resp);
        },
        AllegraCommands::Start(params) => {
            println!("Starting an Allegra Instance: {:?}", params);
            let addr: SocketAddr = "127.0.0.1:29292".parse().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            let mut client_transport = tarpc::serde_transport::tcp::connect(addr, Json::default);
            client_transport.config_mut().max_frame_length(usize::MAX);
            let vmclient = VmmClient::new(
                client::Config::default(),
                client_transport.await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?
            ).spawn();

            let mut ctx = context::current();
            ctx.deadline = SystemTime::now() + Duration::from_secs(30); 
            let response = vmclient.start_vm(ctx, params.clone()).await;
            println!("Response: {:?}", response);
        },
        AllegraCommands::Stop(params) => {
            println!("Stopping an Allegra Instance: {:?}", params);
            let addr: SocketAddr = "127.0.0.1:29292".parse().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            let mut client_transport = tarpc::serde_transport::tcp::connect(addr, Json::default);
            client_transport.config_mut().max_frame_length(usize::MAX);
            let vmclient = VmmClient::new(
                client::Config::default(),
                client_transport.await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?
            ).spawn();

            let mut ctx = context::current();
            ctx.deadline = SystemTime::now() + Duration::from_secs(30); 
            let response = vmclient.shutdown_vm(ctx, params.clone()).await;
            println!("Response: {:?}", response);

        },
        AllegraCommands::AddPubkey(params) => {
            println!("Adding a public key to an Allegra instance: {:?}", params);
            let addr: SocketAddr = "127.0.0.1:29292".parse().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            let mut client_transport = tarpc::serde_transport::tcp::connect(addr, Json::default);
            client_transport.config_mut().max_frame_length(usize::MAX);
            let vmclient = VmmClient::new(
                client::Config::default(),
                client_transport.await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?
            ).spawn();

            let mut ctx = context::current();
            ctx.deadline = SystemTime::now() + Duration::from_secs(30); 
            let response = vmclient.set_ssh_pubkey(ctx, params.clone()).await; 
            println!("Response: {:?}", response);
        },
        AllegraCommands::Delete(params) => {
            println!("Deleting an Allegra Instance: {:?}", params);
            let addr: SocketAddr = "127.0.0.1:29292".parse().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            let mut client_transport = tarpc::serde_transport::tcp::connect(addr, Json::default);
            client_transport.config_mut().max_frame_length(usize::MAX);
            let vmclient = VmmClient::new(
                client::Config::default(),
                client_transport.await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?
            ).spawn();

            let mut ctx = context::current();
            ctx.deadline = SystemTime::now() + Duration::from_secs(30); 
            let response = vmclient.delete_vm(ctx, params.clone()).await; 
            println!("Response: {:?}", response);
        }
    }

    Ok(())
}

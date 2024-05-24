use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::time::{SystemTime, Duration};
use allegra::{generate_new_wallet, WalletInfo, enter_ssh_session};
use allegra::params::{
    InstanceCreateParams, 
    InstanceStartParams, 
    InstanceStopParams, 
    InstanceAddPubkeyParams, 
    InstanceDeleteParams, 
    InstanceExposeServiceParams,
    InstanceGetSshDetails,
};
use allegra::rpc::VmmClient;
use allegra::cli::commands::AllegraCommands;
use allegra::cli::helpers::{
    create_allegra_rpc_client,
    generate_signature_from_command,
};
use clap::Parser;
use tarpc::context;


#[derive(Parser)]
#[command(name = "AllegraCli")]
#[command(version = "0.1.0")]
#[command(about = "enables the provisioning, starting, stopping, and deleting of VMs in the Versatus Network")]
struct Cli {
    #[command(subcommand)]
    command: AllegraCommands
}

#[tokio::main]
async fn main() -> std::io::Result<()> {

    let cli = Cli::parse();

    match &cli.command {
        AllegraCommands::Wallet { new, display, save, path } => {
            println!("Creating an Allegra Wallet");
            let wallet = if *new {
                generate_new_wallet()?
            } else {
                let mut buffer = Vec::new();
                let mut file = OpenOptions::new()
                    .read(true)
                    .open(path)?;

                file.read_to_end(&mut buffer)?;
                let wallet_info: WalletInfo = serde_json::from_slice(&buffer).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string()
                    )
                })?;

                wallet_info
            };

            if *display {
                println!("{:#?}", wallet);
            }

            if *save {
                let mut file = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(path)?;

                let bytes = serde_json::to_vec(&wallet)?;

                file.write_all(&bytes)?;
            }
        }
        AllegraCommands::Create {
            name, distro, version, vmtype, .. 
        } => {
            println!("Creating an Allegra Instance {:?}", &name);
            let vmclient = create_allegra_rpc_client().await?;

            let (sig, recover_id) = generate_signature_from_command(cli.command.clone())?;
            let params = InstanceCreateParams {
                name: name.clone(),
                distro: distro.clone(), 
                version: version.clone(), 
                vmtype: vmtype.clone(),
                sig: hex::encode(&sig.to_bytes()), 
                recovery_id: recover_id.into(),
            };

            let resp = vmclient.create_vm(context::current(), params.clone()).await;
            println!("Response: {:?}", resp);
        },
        AllegraCommands::Start {
            name, console, stateless, ..
        } => {
            let vmclient = create_allegra_rpc_client().await?;

            let (sig, recover_id) = generate_signature_from_command(cli.command.clone())?;
            let params = InstanceStartParams {
                name: name.clone(),
                console: *console,
                stateless: *stateless,
                sig: hex::encode(&sig.to_bytes()),
                recovery_id: recover_id.into()
            };

            let mut ctx = context::current();
            ctx.deadline = SystemTime::now() + Duration::from_secs(30); 
            let response = vmclient.start_vm(ctx, params.clone()).await;
            println!("Response: {:?}", response);
        },
        AllegraCommands::Stop {
            name, ..
        } => {
            println!("Stopping an Allegra Instance: {:?}", &name);
            let vmclient = create_allegra_rpc_client().await?;

            let (sig, recover_id) = generate_signature_from_command(cli.command.clone())?;
            let params = InstanceStopParams {
                name: name.clone(),
                sig: hex::encode(&sig.to_bytes()),
                recovery_id: recover_id.into()
            };


            let mut ctx = context::current();
            ctx.deadline = SystemTime::now() + Duration::from_secs(30); 
            let response = vmclient.shutdown_vm(ctx, params.clone()).await;
            println!("Response: {:?}", response);

        },
        AllegraCommands::AddPubkey {
            name, pubkey, ..
        } => {
            println!("Adding a public key to an Allegra instance: {:?}", &name);
            let vmclient = create_allegra_rpc_client().await?;

            let (sig, recover_id) = generate_signature_from_command(cli.command.clone())?;
            let params = InstanceAddPubkeyParams {
                name: name.clone(),
                pubkey: pubkey.clone(),
                sig: hex::encode(&sig.to_bytes()),
                recovery_id: recover_id.into()
            };


            let mut ctx = context::current();
            ctx.deadline = SystemTime::now() + Duration::from_secs(30); 
            let response = vmclient.set_ssh_pubkey(ctx, params.clone()).await; 
            println!("Response: {:?}", response);
        },
        AllegraCommands::Delete {
            name, force, interactive, ..
        } => {
            println!("Deleting an Allegra Instance: {:?}", &name);
            let vmclient = create_allegra_rpc_client().await?;

            let (sig, recover_id) = generate_signature_from_command(cli.command.clone())?;
            let params = InstanceDeleteParams {
                name: name.clone(),
                force: *force,
                interactive: *interactive,
                sig: hex::encode(&sig.to_bytes()),
                recovery_id: recover_id.into()
            };


            let mut ctx = context::current();
            ctx.deadline = SystemTime::now() + Duration::from_secs(30); 
            let response = vmclient.delete_vm(ctx, params.clone()).await; 
            println!("Response: {:?}", response);
        },
        AllegraCommands::ExposeService {
            name, port, service_type, .. 
        } => {
            println!("Exposing ports on an Allegra instance: {:?}", &name);
            let vmclient = create_allegra_rpc_client().await?;

            let (sig, recover_id) = generate_signature_from_command(cli.command.clone())?;
            let params = InstanceExposeServiceParams {
                name: name.clone(),
                port: port.clone(),
                service_type: service_type.clone(),
                sig: hex::encode(&sig.to_bytes()),
                recovery_id: recover_id.into()
            };


            let mut ctx = context::current();
            ctx.deadline = SystemTime::now() + Duration::from_secs(30); 
            let response = vmclient.expose_vm_ports(ctx, params.clone()).await; 
            println!("Response: {:?}", response);
        },
        AllegraCommands::GetSshDetails{
            name, owner, keypath, username
        } => {
            println!("Getting ssh details for an Allegra instance: {:?}", &name);
            let vmclient = create_allegra_rpc_client().await?;
            let params = InstanceGetSshDetails {
                owner: owner.clone(),
                name: name.clone(),
                keypath: None,
                username: None
            };
            let mut ctx = context::current();
            ctx.deadline = SystemTime::now() + Duration::from_secs(30); 
            let response = vmclient.get_ssh_details(ctx, params.clone()).await; 
            println!("Response: {:?}", response);
        }
        AllegraCommands::PollTask { owner, task_id } => {
            let vmclient = create_allegra_rpc_client().await?;
            let mut ctx = context::current();
            ctx.deadline = SystemTime::now() + Duration::from_secs(30);
            let response = vmclient.get_task_status(ctx, owner.clone(), task_id.clone()).await;
            println!("Response: {:?}", response);
        }
        AllegraCommands::Ssh { name, owner, keypath, username } => {
            let vmclient = create_allegra_rpc_client().await?;
            let mut ctx = context::current();
            ctx.deadline = SystemTime::now() + Duration::from_secs(30);
            let params = InstanceGetSshDetails { 
                name: name.clone(), 
                owner: owner.clone(), 
                keypath: Some(keypath.clone()),
                username: Some(username.clone())
            };
            enter_ssh_session(&vmclient, params, ctx).await?;
        }
    }

    Ok(())
}

use std::fs::OpenOptions;
use std::io::{Read, Write};
use allegra::{generate_new_wallet, WalletInfo, enter_ssh_session};
use allegra::allegra_rpc::{
    InstanceCreateParams, 
    InstanceStartParams, 
    InstanceStopParams, 
    InstanceAddPubkeyParams, 
    InstanceDeleteParams, 
    InstanceExposeServiceParams,
    InstanceGetSshDetails,
    GetTaskStatusRequest
};

use allegra::cli::commands::AllegraCommands;
use allegra::cli::helpers::{
    create_allegra_rpc_client,
    generate_signature_from_command,
};
use clap::Parser;
use rayon::iter::{ParallelIterator, IntoParallelRefIterator};


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
    simple_logger::init_with_level(log::Level::Info)
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;

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
            name, distro, version, vmtype, endpoint, memory, vcpus, cpu,
            metadata, os_variant, host_device, network, disk, filesystem,
            controller, input, graphics, sound, video, smartcard, redirdev,
            memballoon, tpm, rng, panic, shmem, memdev, vsock, iommu, watchdog,
            serial, parallel, channel, console, install, cdrom, location, pxe,
            import, boot, idmap, features, clock, launch_security, numatune,
            boot_dev, unattended, print_xml, dry_run, connect, virt_type,
            cloud_init, .. 
        } => {
            println!("Creating an Allegra Instance {:?}", &name);
            let mut vmclient = create_allegra_rpc_client(endpoint).await?;

            let (sig, recover_id) = generate_signature_from_command(cli.command.clone())?;
            let recovery_id: u8 = recover_id.into();
            let params = InstanceCreateParams {
                name: name.clone(),
                distro: distro.clone(), 
                version: version.clone(), 
                vmtype: vmtype.to_string(),
                sig: hex::encode(&sig.to_bytes()), 
                recovery_id: recovery_id as u32,
                sync: Some(false),
                memory, vcpus, cpu, os_variant, 
                metadata, host_device, network, disk, filesystem,
                controller, input, graphics, sound, video, smartcard, redirdev,
                memballoon, tpm, rng, panic, shmem, memdev, vsock, iommu,
                watchdog, serial, parallel, channel, console, install, cdrom,
                location, pxe, import, boot, idmap, features, clock, launch_security,
                numatune, boot_dev, unattended, print_xml, dry_run, connect,
                virt_type, cloud_init
            };

            let resp = vmclient.create_vm(params.clone()).await;
            println!("Response: {:?}", resp);
        },
        AllegraCommands::Start {
            name, console, stateless, endpoint, ..
        } => {
            let mut vmclient = create_allegra_rpc_client(endpoint).await?;

            let (sig, recover_id) = generate_signature_from_command(cli.command.clone())?;
            let recovery_id: u8 = recover_id.into();
            let params = InstanceStartParams {
                name: name.clone(),
                console: *console,
                stateless: *stateless,
                sig: hex::encode(&sig.to_bytes()),
                recovery_id: recovery_id as u32
            };

            let response = vmclient.start_vm(params.clone()).await;
            println!("Response: {:?}", response);
        },
        AllegraCommands::Stop {
            name, endpoint, ..
        } => {
            println!("Stopping an Allegra Instance: {:?}", &name);
            let mut vmclient = create_allegra_rpc_client(endpoint).await?;

            let (sig, recover_id) = generate_signature_from_command(cli.command.clone())?;
            let recovery_id: u8 = recover_id.into();
            let params = InstanceStopParams {
                name: name.clone(),
                sig: hex::encode(&sig.to_bytes()),
                recovery_id: recovery_id as u32
            };

            let response = vmclient.shutdown_vm(params.clone()).await;
            println!("Response: {:?}", response);

        },
        AllegraCommands::AddPubkey {
            name, pubkey, endpoint, ..
        } => {
            println!("Adding a public key to an Allegra instance: {:?}", &name);
            let mut vmclient = create_allegra_rpc_client(endpoint).await?;

            let (sig, recover_id) = generate_signature_from_command(cli.command.clone())?;
            let recovery_id: u8 = recover_id.into();
            let params = InstanceAddPubkeyParams {
                name: name.clone(),
                pubkey: pubkey.clone(),
                sig: hex::encode(&sig.to_bytes()),
                recovery_id: recovery_id as u32
            };


            let response = vmclient.set_ssh_pubkey(params.clone()).await; 
            println!("Response: {:?}", response);
        },
        AllegraCommands::Delete {
            name, force, interactive, endpoint, ..
        } => {
            println!("Deleting an Allegra Instance: {:?}", &name);
            let mut vmclient = create_allegra_rpc_client(endpoint).await?;

            let (sig, recover_id) = generate_signature_from_command(cli.command.clone())?;
            let recovery_id: u8 = recover_id.into();
            let params = InstanceDeleteParams {
                name: name.clone(),
                force: *force,
                interactive: *interactive,
                sig: hex::encode(&sig.to_bytes()),
                recovery_id: recovery_id as u32
            };

            let response = vmclient.delete_vm(params.clone()).await; 
            println!("Response: {:?}", response);
        },
        AllegraCommands::ExposeService {
            name, port, service_type, endpoint, .. 
        } => {
            println!("Exposing ports on an Allegra instance: {:?}", &name);
            let mut vmclient = create_allegra_rpc_client(endpoint).await?;

            let port: Vec<u32> = port.par_iter().map(|p| *p as u32).collect();
            let service_type: Vec<i32> = service_type.par_iter().map(|s| s.clone().into()).collect();
            let (sig, recover_id) = generate_signature_from_command(cli.command.clone())?;
            let recovery_id: u8 = recover_id.into();
            let params = InstanceExposeServiceParams {
                name: name.clone(),
                port: port.clone(),
                service_type: service_type.clone(),
                sig: hex::encode(&sig.to_bytes()),
                recovery_id: recovery_id as u32
            };


            let response = vmclient.expose_vm_ports(params.clone()).await; 
            println!("Response: {:?}", response);
        },
        AllegraCommands::GetSshDetails{
            name, owner, endpoint, ..
        } => {
            println!("Getting ssh details for an Allegra instance: {:?}", &name);
            let mut vmclient = create_allegra_rpc_client(endpoint).await?;
            let params = InstanceGetSshDetails {
                owner: owner.clone(),
                name: name.clone(),
                keypath: None,
                username: None
            };
            let response = vmclient.get_ssh_details(params.clone()).await; 
            println!("Response: {:?}", response);
        }
        AllegraCommands::PollTask { owner, task_id, endpoint } => {
            let mut vmclient = create_allegra_rpc_client(endpoint).await?;
            let request = GetTaskStatusRequest { owner: owner.clone(), id: task_id.clone() };

            let response = vmclient.get_task_status(request).await;
            println!("Response: {:?}", response);
        }
        AllegraCommands::Ssh { name, owner, keypath, username, endpoint } => {
            let mut vmclient = create_allegra_rpc_client(endpoint).await?;
            let params = InstanceGetSshDetails { 
                name: name.clone(), 
                owner: owner.clone(), 
                keypath: Some(keypath.clone()),
                username: Some(username.clone())
            };
            enter_ssh_session(&mut vmclient, params).await?;
        }
    }

    Ok(())
}

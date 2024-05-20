use std::time::{SystemTime, Duration};
use allegra::params::{
    InstanceCreateParams, 
    InstanceStartParams, 
    InstanceStopParams, 
    InstanceAddPubkeyParams, 
    InstanceDeleteParams, 
    InstanceExposePortParams,
    InstanceGetSshDetails,
};
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

    let (sig, recover_id) = generate_signature_from_command(cli.command.clone())?;

    match &cli.command {
        AllegraCommands::Create {
            name, distro, version, vmtype, .. 
        } => {
            println!("Creating an Allegra Instance {:?}", &name);
            let vmclient = create_allegra_rpc_client().await?;

            let mut params = InstanceCreateParams {
                name: name.clone(),
                distro: distro.clone(), 
                version: version.clone(), 
                vmtype: vmtype.clone(),
                sig: String::default(), 
                recovery_id: u8::default(),
            };

            params.sig = hex::encode(&sig.to_bytes());
            params.recovery_id = recover_id.into();

            let resp = vmclient.create_vm(context::current(), params.clone()).await;
            println!("Response: {:?}", resp);
        },
        AllegraCommands::Start {
            name, console, stateless, ..
        } => {
            let vmclient = create_allegra_rpc_client().await?;

            let mut params = InstanceStartParams {
                name: name.clone(),
                console: *console,
                stateless: *stateless,
                sig: String::default(),
                recovery_id: u8::default()
            };
            params.sig = hex::encode(&sig.to_bytes());
            params.recovery_id = recover_id.into();

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

            let mut params = InstanceStopParams {
                name: name.clone(),
                sig: String::default(),
                recovery_id: u8::default()
            };

            params.sig = hex::encode(&sig.to_bytes());
            params.recovery_id = recover_id.into();

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

            let mut params = InstanceAddPubkeyParams {
                name: name.clone(),
                pubkey: pubkey.clone(),
                sig: String::default(), 
                recovery_id: u8::default()
            };

            params.sig = hex::encode(&sig.to_bytes());
            params.recovery_id = recover_id.into();

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

            let mut params = InstanceDeleteParams {
                name: name.clone(),
                force: *force,
                interactive: *interactive,
                sig: String::default(), 
                recovery_id: u8::default()
            };

            params.sig = hex::encode(&sig.to_bytes());
            params.recovery_id = recover_id.into();

            let mut ctx = context::current();
            ctx.deadline = SystemTime::now() + Duration::from_secs(30); 
            let response = vmclient.delete_vm(ctx, params.clone()).await; 
            println!("Response: {:?}", response);
        },
        AllegraCommands::ExposePorts {
            name, port, .. 
        } => {
            println!("Exposing ports on an Allegra instance: {:?}", &name);
            let vmclient = create_allegra_rpc_client().await?;
            let mut params = InstanceExposePortParams {
                name: name.clone(),
                port: port.clone(),
                sig: String::default(), 
                recovery_id: u8::default()
            };

            params.sig = hex::encode(&sig.to_bytes());
            params.recovery_id = recover_id.into();

            let mut ctx = context::current();
            ctx.deadline = SystemTime::now() + Duration::from_secs(30); 
            let response = vmclient.expose_vm_ports(ctx, params.clone()).await; 
            println!("Response: {:?}", response);
        },
        AllegraCommands::GetSshDetails{
            name, .. 
        } => {
            println!("Getting ssh details for an Allegra instance: {:?}", &name);
            let vmclient = create_allegra_rpc_client().await?;
            let mut params = InstanceGetSshDetails {
                name: name.clone(),
                sig: String::default(), 
                recovery_id: u8::default()
            };

            params.sig = hex::encode(&sig.to_bytes());
            params.recovery_id = recover_id.into();

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
    }

    Ok(())
}

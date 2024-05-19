use std::io::Read;
use std::net::SocketAddr;
use std::time::{SystemTime, Duration};
use ethers_core::k256::SecretKey;
use ethers_core::k256::ecdsa::SigningKey;
use ethers_core::k256::ecdsa::RecoveryId;
use ethers_core::k256::ecdsa::Signature;
use sha3::{Digest, Sha3_256};
use allegra::rpc::VmmClient;
use allegra::params::{
    InstanceCreateParams, 
    InstanceStartParams, 
    InstanceStopParams, 
    InstanceAddPubkeyParams, 
    InstanceDeleteParams, 
    InstanceExposePortParams,
    InstanceGetSshDetails, Payload,
};
use allegra::vm_types::VmType;
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

#[derive(Clone, Subcommand)]
enum AllegraCommands {
    #[command(name = "create")]
    Create {
        name: String,
        #[arg(long, short)]
        distro: String,
        #[arg(long, short)]
        version: String,
        #[arg(long, short='t')]
        vmtype: VmType,
        #[arg(long, short)]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short)]
        from_file: Option<bool>,
        #[arg(long, short)]
        path: Option<String>,
        #[arg(long, short)]
        kp_index: Option<usize>
    },
    #[command(name = "start")]
    Start {
        #[arg(long, short)]
        name: String,
        #[arg(long, short)]
        console: bool,
        #[arg(long, short)]
        stateless: bool,
        #[arg(long, short='k')]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short)]
        from_file: Option<bool>,
        #[arg(long, short)]
        path: Option<String>,
        #[arg(long, short)]
        kp_index: Option<usize>
    },
    #[command(name = "stop")]
    Stop {
        #[arg(long, short)]
        name: String,
        #[arg(long, short)]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short)]
        from_file: Option<bool>,
        #[arg(long, short)]
        path: Option<String>,
        #[arg(long, short)]
        kp_index: Option<usize>
    },
    #[command(name = "add-pubkey")]
    AddPubkey {
        #[arg(long, short)]
        name: String,
        #[arg(long, short)]
        pubkey: String,
        #[arg(long, short)]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short)]
        from_file: Option<bool>,
        #[arg(long, short)]
        path: Option<String>,
        #[arg(long, short)]
        kp_index: Option<usize>
    },
    #[command(name = "delete")]
    Delete {
        #[arg(long, short)]
        name: String,
        #[arg(long, short)]
        force: bool,
        #[arg(long, short)]
        interactive: bool,
        #[arg(long, short)]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short)]
        from_file: Option<bool>,
        #[arg(long, short)]
        path: Option<String>,
        #[arg(long, short)]
        kp_index: Option<usize>
    },
    #[command(name = "expose-ports")]
    ExposePorts {
        #[arg(long, short)]
        name: String,
        #[arg(long, short)]
        port: Vec<u16>,
        #[arg(long, short)]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short)]
        from_file: Option<bool>,
        #[arg(long, short)]
        path: Option<String>,
        #[arg(long, short)]
        kp_index: Option<usize>
    },
    #[command(name = "get-ssh")]
    GetSshDetails {
        #[arg(long, short)]
        name: String,
        #[arg(long, short)]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short)]
        from_file: Option<bool>,
        #[arg(long, short)]
        path: Option<String>,
        #[arg(long, short)]
        kp_index: Option<usize>
    },
}

impl AllegraCommands {
    pub fn from_file(&self) -> Option<bool> {
        match self {
            Self::Create { from_file, .. } => from_file.clone(),
            Self::Stop { from_file, .. } => from_file.clone(),
            Self::Start { from_file, .. } => from_file.clone(),
            Self::Delete { from_file, .. } => from_file.clone(),
            Self::AddPubkey { from_file, .. } => from_file.clone(),
            Self::ExposePorts { from_file, .. } => from_file.clone(),
            Self::GetSshDetails { from_file, .. } => from_file.clone(),
        }
    }

    pub fn sk(&self) -> Option<String> {
        match self {
            Self::Create { sk, .. } => sk.clone(),
            Self::Stop { sk, .. } => sk.clone(),
            Self::Start { sk, .. } => sk.clone(),
            Self::Delete { sk, .. } => sk.clone(),
            Self::AddPubkey { sk, .. } => sk.clone(),
            Self::ExposePorts { sk, .. } => sk.clone(),
            Self::GetSshDetails { sk, .. } => sk.clone(),
        }

    }

    pub fn mnemonic(&self) -> Option<String> {
        match self {
            Self::Create { mnemonic, .. } => mnemonic.clone(),
            Self::Stop { mnemonic, .. } => mnemonic.clone(),
            Self::Start { mnemonic, .. } => mnemonic.clone(),
            Self::Delete { mnemonic, .. } => mnemonic.clone(),
            Self::AddPubkey { mnemonic, .. } => mnemonic.clone(),
            Self::ExposePorts { mnemonic, .. } => mnemonic.clone(),
            Self::GetSshDetails { mnemonic, .. } => mnemonic.clone(),
        }
    }

    pub fn path(&self) -> Option<String> {
        match self {
            Self::Create { path, .. } => path.clone(),
            Self::Stop { path, .. } => path.clone(),
            Self::Start { path, .. } => path.clone(),
            Self::Delete { path, .. } => path.clone(),
            Self::AddPubkey { path, .. } => path.clone(),
            Self::ExposePorts { path, .. } => path.clone(),
            Self::GetSshDetails { path, .. } => path.clone(),
        }
    }

    pub fn kp_index(&self) -> Option<usize> {
        match self {
            Self::Create { kp_index, .. } => kp_index.clone(),
            Self::Stop { kp_index, .. } => kp_index.clone(),
            Self::Start { kp_index, .. } => kp_index.clone(),
            Self::Delete { kp_index, .. } => kp_index.clone(),
            Self::AddPubkey { kp_index, .. } => kp_index.clone(),
            Self::ExposePorts { kp_index, .. } => kp_index.clone(),
            Self::GetSshDetails { kp_index, .. } => kp_index.clone(),
        }
    }
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
    }

    Ok(())
}

fn generate_signature_from_command(command: AllegraCommands) -> std::io::Result<(Signature, RecoveryId)> {
    let params: Box<dyn Payload> = match command {
        AllegraCommands::Create { ref name, ref distro, ref version, ref vmtype, .. } => {
            Box::new(
                InstanceCreateParams {
                    name: name.clone(),
                    distro: distro.clone(), 
                    version: version.clone(), 
                    vmtype: vmtype.clone(),
                    sig: String::default(), 
                    recovery_id: u8::default(),
                }
            )
        }
        AllegraCommands::Start { ref name, console, stateless, .. } => {
            Box::new(
                InstanceStartParams {
                    name: name.clone(),
                    console,
                    stateless,
                    sig: String::default(),
                    recovery_id: u8::default()
                }
            )
        }
        AllegraCommands::Stop { ref name, .. } => {
            Box::new(
                InstanceStopParams {
                    name: name.clone(),
                    sig: String::default(),
                    recovery_id: u8::default()
                }
            )
        }
        AllegraCommands::Delete { ref name, force, interactive, .. } => {
            Box::new(
                InstanceDeleteParams {
                    name: name.clone(),
                    force,
                    interactive,
                    sig: String::default(),
                    recovery_id: u8::default(),
                }
            )
        }
        AllegraCommands::AddPubkey { ref name, ref pubkey, .. } => {
            Box::new(
                InstanceAddPubkeyParams {
                    name: name.clone(),
                    pubkey: pubkey.clone(),
                    sig: String::default(),
                    recovery_id: u8::default()
                }
            )
        }
        AllegraCommands::ExposePorts { ref name, ref port, .. } => {
            Box::new(
                InstanceExposePortParams {
                    name: name.clone(),
                    port: port.clone(),
                    sig: String::default(),
                    recovery_id: u8::default()
                }
            )
        }
        AllegraCommands::GetSshDetails { ref name, .. } => {
            Box::new(
                InstanceGetSshDetails {
                    name: name.clone(),
                    sig: String::default(),
                    recovery_id: u8::default()
                }
            )
        }
    };

    if let Some(ff) = command.from_file() {
        if let Some(fp) = command.path() {
            return Ok(generate_signature(
                params, ff, fp.to_string(), command.kp_index()
            )?)
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "filepath required if from_file is true"
            ))
        }
    } else {
        if let Some(into_sk) = command.sk()  {
            return Ok(generate_signature(params, false, into_sk.to_string(), None)?)
        } else if let Some(into_sk) = command.mnemonic() {
            return Ok(generate_signature(params, false, into_sk.to_string(), None)?)
        } else {
            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "if not from file, either secret key string or mnemonic is required"
                )
            )
        }
    };
}

fn generate_signature(
    p: Box<dyn Payload>,
    from_file: bool,
    into_sk: String,
    kp_index: Option<usize>
) -> std::io::Result<(Signature, RecoveryId)> {
    let payload: String = p.into_payload();

    let mut hasher = Sha3_256::new();
    hasher.update(payload.as_bytes());
    let hash = hasher.finalize().to_vec();

    if from_file {
        let index = if let Some(kpi) = kp_index {
            kpi
        } else {
            0
        };

        let sks: Vec<String> = {
            let mut buffer = Vec::new(); 

            let mut file = std::fs::OpenOptions::new()
            .read(true)
            .open(into_sk)?;

            file.read_to_end(&mut buffer)?;

            let sk_vec: Vec<String> = serde_json::from_slice(&buffer).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string()
                )
            })?;

            sk_vec
        };

        let sk = sks.get(index).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("index {} in sk vec does not exist", index)
            )
        )?.as_bytes();

        let sk: SigningKey = SecretKey::from_slice(
            &hex::decode(&sk).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string()
                )}
            )?).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string()
                )
            }
        )?.into();

        let (signature, recovery_id) = sk.sign_recoverable(&hash).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;
        return Ok((signature, recovery_id))
    }

    let sk: SigningKey = SecretKey::from_slice(
        &hex::decode(&into_sk).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )}
        )?).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        }
    )?.into();

    let (signature, recovery_id) = sk.sign_recoverable(&hash).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    })?;

    return Ok((signature, recovery_id))
}

pub async fn create_allegra_rpc_client() -> std::io::Result<VmmClient> {
    //TODO(asmith): Replace SocketAddr with environment variable or default
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

    Ok(vmclient)
}

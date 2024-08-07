use crate::allegra_rpc::{vmm_client::VmmClient, Features};
use crate::allegra_rpc::{
    InstanceAddPubkeyParams, InstanceCreateParams, InstanceDeleteParams,
    InstanceExposeServiceParams, InstanceGetSshDetails, InstanceStartParams, InstanceStopParams,
};
use crate::cli::commands::AllegraCommands;
use crate::payload_impls::Payload;
use crate::virt_install::CloudInit;
use alloy::signers::k256::elliptic_curve::SecretKey;
use bip39::{Language, Mnemonic};
use ethers_core::{
    k256::ecdsa::{RecoveryId, Signature, SigningKey},
    rand::{rngs::OsRng, RngCore},
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use ssh2::Session;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use termion::{async_stdin, raw::IntoRawMode};
use tokio::net::TcpStream;
use tonic::transport::Channel;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletInfo {
    mnemonic: String,
    signing_key: String,
    verifying_keys: HashMap<u8, String>,
    addresses: HashMap<String, String>,
}

pub fn generate_new_wallet() -> std::io::Result<WalletInfo> {
    let mut entropy = [0u8; 16];
    OsRng.fill_bytes(&mut entropy);
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error trying to create Mnemonic from entropy {e}"),
        )
    })?;

    println!("Mnemonic: {:?}", &mnemonic);

    let seed = mnemonic.to_seed("");

    println!("Seed: {:?}", &seed);
    let master_key: SigningKey = SecretKey::from_slice(&seed[..32])
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error trying to create secret key from slice: {e}"),
            )
        })?
        .into();

    println!("SigningKey: {:?}", &master_key);
    let public_key = master_key.verifying_key();
    println!("VerifyingKey: {:?}", &public_key);

    let address = ethers_core::utils::public_key_to_address(public_key);

    let mnemonic_string = mnemonic.to_string();
    let mut verifying_keys = HashMap::new();
    let public_key_str = hex::encode(&public_key.to_encoded_point(false).to_bytes());
    verifying_keys.insert(0, public_key_str.clone());
    let mut addresses = HashMap::new();
    addresses.insert(
        public_key_str.clone(),
        format!("0x{}", hex::encode(address.0)),
    );

    let wallet_info = WalletInfo {
        mnemonic: mnemonic_string,
        signing_key: hex::encode(&master_key.to_bytes()),
        verifying_keys,
        addresses,
    };

    Ok(wallet_info)
}

pub fn generate_signature_from_command(
    command: AllegraCommands,
) -> std::io::Result<(Signature, RecoveryId)> {
    let params: Box<dyn Payload> = match command {
        AllegraCommands::Create {
            ref name,
            ref distro,
            ref version,
            ref vmtype,
            ref memory,
            ref vcpus,
            ref cpu,
            ref metadata,
            ref os_variant,
            ref host_device,
            ref network,
            ref disk,
            ref filesystem,
            ref controller,
            ref input,
            ref graphics,
            ref sound,
            ref video,
            ref smartcard,
            ref redirdev,
            ref memdev,
            ref tpm,
            ref rng,
            ref panic,
            ref memballoon,
            ref shmem,
            ref vsock,
            ref iommu,
            ref import,
            ref boot,
            ref boot_dev,
            ref cdrom,
            ref watchdog,
            ref serial,
            ref channel,
            ref console,
            ref parallel,
            ref install,
            ref location,
            ref pxe,
            ref idmap,
            ref features,
            ref clock,
            ref launch_security,
            ref numatune,
            ref unattended,
            ref print_xml,
            ref dry_run,
            ref connect,
            ref virt_type,
            ref cloud_init,
            ..
        } => {
            let features: Vec<Features> = features
                .iter()
                .map(|f| {
                    let parts: Vec<&str> = f.split("=").collect();
                    Features {
                        name: parts[0].to_string(),
                        feature: parts.get(1).map_or("".to_string(), |&s| s.to_string()),
                    }
                })
                .collect();

            let cloud_init: Option<CloudInit> = match cloud_init.clone() {
                Some(ci) => serde_yml::from_str(&ci).ok(),
                None => None,
            };
            Box::new(InstanceCreateParams {
                name: name.clone(),
                distro: distro.clone().into(),
                version: version.clone(),
                vmtype: vmtype.clone().to_string(),
                sig: String::default(),
                recovery_id: u32::default(),
                sync: Some(false),
                memory: memory.clone(),
                vcpus: vcpus.clone(),
                cpu: cpu.clone(),
                metadata: metadata.clone(),
                os_variant: os_variant.clone(),
                host_device: host_device.clone(),
                network: network.clone(),
                disk: disk.clone(),
                filesystem: filesystem.clone(),
                controller: controller.clone(),
                input: input.clone(),
                graphics: graphics.clone(),
                sound: sound.clone(),
                video: video.clone(),
                smartcard: smartcard.clone(),
                redirdev: redirdev.clone(),
                memballoon: memballoon.clone(),
                tpm: tpm.clone(),
                rng: rng.clone(),
                panic: panic.clone(),
                memdev: memdev.clone(),
                shmem: shmem.clone(),
                vsock: vsock.clone(),
                iommu: iommu.clone(),
                import: import.clone(),
                boot: boot.clone(),
                boot_dev: boot_dev.clone(),
                cdrom: cdrom.clone(),
                watchdog: watchdog.clone(),
                serial: serial.clone(),
                channel: channel.clone(),
                console: console.clone(),
                parallel: parallel.clone(),
                install: install.clone(),
                location: location.clone(),
                pxe: pxe.clone(),
                idmap: idmap.clone(),
                features,
                clock: clock.clone(),
                launch_security: launch_security.clone(),
                numatune: numatune.clone(),
                unattended: unattended.clone(),
                print_xml: print_xml.clone(),
                dry_run: dry_run.clone(),
                connect: connect.clone(),
                virt_type: virt_type.clone(),
                cloud_init: match cloud_init {
                    Some(ci) => Some(ci.into()),
                    None => None,
                },
            })
        }
        AllegraCommands::Start {
            ref name,
            console,
            stateless,
            ..
        } => Box::new(InstanceStartParams {
            name: name.clone(),
            console,
            stateless,
            sig: String::default(),
            recovery_id: u32::default(),
        }),
        AllegraCommands::Stop { ref name, .. } => Box::new(InstanceStopParams {
            name: name.clone(),
            sig: String::default(),
            recovery_id: u32::default(),
        }),
        AllegraCommands::Delete {
            ref name,
            force,
            interactive,
            ..
        } => Box::new(InstanceDeleteParams {
            name: name.clone(),
            force,
            interactive,
            sig: String::default(),
            recovery_id: u32::default(),
        }),
        AllegraCommands::AddPubkey {
            ref name,
            ref pubkey,
            ..
        } => Box::new(InstanceAddPubkeyParams {
            name: name.clone(),
            pubkey: pubkey.clone(),
            sig: String::default(),
            recovery_id: u32::default(),
        }),
        AllegraCommands::ExposeService {
            ref name,
            ref port,
            ref service_type,
            ..
        } => {
            let port: Vec<u32> = port.par_iter().map(|n| *n as u32).collect();

            let service_type: Vec<i32> = service_type
                .par_iter()
                .filter_map(|service| {
                    service
                        .clone()
                        .try_into()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                        .ok()
                })
                .collect();

            Box::new(InstanceExposeServiceParams {
                name: name.clone(),
                port: port.clone(),
                service_type: service_type.clone(),
                sig: String::default(),
                recovery_id: u32::default(),
            })
        }
        AllegraCommands::GetSshDetails {
            ref name,
            ref keypath,
            ref owner,
            ref username,
            ..
        } => Box::new(InstanceGetSshDetails {
            owner: owner.to_string(),
            name: name.to_string(),
            keypath: keypath.clone(),
            username: username.clone(),
        }),
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "This variant does not require a signature",
            ))
        }
    };

    if let Some(ff) = command.clone().from_file() {
        if let Some(fp) = command.path() {
            return Ok(generate_signature(
                params,
                ff,
                fp.to_string(),
                command.kp_index(),
            )?);
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "filepath required if from_file is true",
            ));
        }
    } else {
        if let Some(into_sk) = command.sk() {
            return Ok(generate_signature(
                params,
                false,
                into_sk.to_string(),
                None,
            )?);
        } else if let Some(into_sk) = command.mnemonic() {
            return Ok(generate_signature(
                params,
                false,
                into_sk.to_string(),
                None,
            )?);
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "if not from file, either secret key string or mnemonic is required",
            ));
        }
    };
}

fn generate_signature(
    p: Box<dyn Payload>,
    from_file: bool,
    into_sk: String,
    kp_index: Option<usize>,
) -> std::io::Result<(Signature, RecoveryId)> {
    let payload: String = p.into_payload();

    let mut hasher = Sha3_256::new();
    hasher.update(payload.as_bytes());
    let hash = hasher.finalize().to_vec();

    if from_file {
        let index = if let Some(kpi) = kp_index { kpi } else { 0 };

        let sks: Vec<String> = {
            let mut buffer = Vec::new();

            let mut file = std::fs::OpenOptions::new().read(true).open(into_sk)?;

            file.read_to_end(&mut buffer)?;

            let sk_vec: Vec<String> = serde_json::from_slice(&buffer)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            sk_vec
        };

        let sk = sks
            .get(index)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("index {} in sk vec does not exist", index),
            ))?
            .as_bytes();

        let sk: SigningKey = SecretKey::from_slice(
            &hex::decode(&sk)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?,
        )
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
        .into();

        let (signature, recovery_id) = sk
            .sign_recoverable(&hash)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        return Ok((signature, recovery_id));
    }

    let sk: SigningKey = SecretKey::from_slice(
        &hex::decode(&into_sk)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?,
    )
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
    .into();

    let (signature, recovery_id) = sk
        .sign_recoverable(&hash)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    return Ok((signature, recovery_id));
}

pub async fn create_allegra_rpc_client_to_addr(dst: &str) -> std::io::Result<VmmClient<Channel>> {
    log::info!("attempting to create allegra rpc client to address {}", dst);
    let endpoint = format!("http://{}", dst);
    let vmclient = VmmClient::connect(endpoint)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    Ok(vmclient)
}

pub async fn create_allegra_rpc_client(
    endpoint: &Option<String>,
) -> std::io::Result<VmmClient<Channel>> {
    let vmclient = if let Some(endpoint) = endpoint {
        let vmclient = VmmClient::connect(endpoint.clone())
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        vmclient
    } else {
        let vmclient = VmmClient::connect("http://127.0.0.1:50051")
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        vmclient
    };

    Ok(vmclient)
}

pub async fn enter_ssh_session(
    rpc_client: &mut VmmClient<Channel>,
    params: InstanceGetSshDetails,
) -> std::io::Result<()> {
    let resp = rpc_client
        .get_ssh_details(tonic::Request::new(params.clone()))
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    log::info!("{:?}", resp);
    if let Some(ssh_details) = resp.into_inner().ssh_details {
        log::info!("{:?}", ssh_details);
        let tcp = TcpStream::connect(format!("{}:{}", ssh_details.ip, ssh_details.port)).await?;
        let mut session = Session::new()?;

        log::info!("starting session...");
        session.set_tcp_stream(tcp);
        log::info!("tcp stream set...");
        session.handshake()?;
        log::info!("handshake completed...");
        log::info!("attempting to read keypath file");
        let pk = if let Some(keypath) = params.keypath {
            let mut key_file = File::open(&keypath)?;
            let mut pk = String::new();
            key_file.read_to_string(&mut pk)?;
            pk
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "keypath is required if attempting to enter an SSH session",
            ));
        };

        //TODO: allow entering username
        let username = "root";
        log::info!("authorizing user");
        session.userauth_pubkey_memory(&username, None, &pk, None)?;

        if !session.authenticated() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Authentication Failed",
            ));
        }

        log::info!("session authenticated establishing channel");
        let mut channel = session.channel_session()?;
        log::info!("channel established, requesting shell");
        channel.request_pty("xterm", None, None)?;
        channel.handle_extended_data(ssh2::ExtendedData::Merge)?;
        channel.shell()?;
        log::info!("shell acquired, establishing stdin and stdout");

        let stdout = std::io::stdout();
        let mut stdout = stdout.lock().into_raw_mode()?;
        let mut stdin = async_stdin();

        let mut buf_in = Vec::new();

        while !channel.eof() {
            let bytes_available = channel.read_window().available;
            if bytes_available > 0 {
                let mut buffer = vec![0; bytes_available as usize];
                channel.read_exact(&mut buffer)?;
                stdout.write(&buffer)?;
                stdout.flush()?;
            }

            stdin.read_to_end(&mut buf_in)?;
            buf_in.clear();

            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        channel.wait_close()?;
        return Ok(());
    }

    return Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("unable to find ssh details for {}", params.name),
    ));
}

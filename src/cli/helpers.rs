use crate::cli::commands::AllegraCommands;
use crate::params::{Payload, InstanceCreateParams, InstanceStopParams, InstanceStartParams, InstanceDeleteParams, InstanceGetSshDetails, InstanceAddPubkeyParams, InstanceExposePortParams};
use crate::rpc::VmmClient;
use std::io::Read;
use std::net::SocketAddr;
use std::collections::HashMap;
use ethers_core::k256::elliptic_curve::SecretKey;
use sha3::{Digest, Sha3_256};
use tarpc::client;
use tarpc::tokio_serde::formats::Json;
use ethers_core::{
    k256::ecdsa::{
        Signature, RecoveryId, SigningKey
    }, rand::{rngs::OsRng, RngCore}
};
use bip39::{Mnemonic, Language};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletInfo {
    mnemonic: String,
    signing_key: String,
    verifying_keys: HashMap<u8, String>,
    addresses: HashMap<String, String> 
}

pub fn generate_new_wallet() -> std::io::Result<WalletInfo> {
    let mut entropy = [0u8; 16];
    OsRng.fill_bytes(&mut entropy);
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error trying to create Mnemonic from entropy {e}")
        )
    })?;

    println!("Mnemonic: {:?}", &mnemonic);

    let seed = mnemonic.to_seed("");

    println!("Seed: {:?}", &seed);
    let master_key: SigningKey = SecretKey::from_slice(&seed[..32]).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error trying to create secret key from slice: {e}")
        )
    })?.into();

    println!("SigningKey: {:?}", &master_key);
    let public_key = master_key.verifying_key(); 
    println!("VerifyingKey: {:?}", &public_key);

    let address = ethers_core::utils::public_key_to_address(public_key); 

    let mnemonic_string = mnemonic.to_string();
    let mut verifying_keys = HashMap::new();
    let public_key_str = hex::encode(&public_key.to_encoded_point(false).to_bytes());
    verifying_keys.insert(0, public_key_str.clone());
    let mut addresses = HashMap::new();
    addresses.insert(public_key_str.clone(), format!("0x{}",hex::encode(address.0)));


    let wallet_info = WalletInfo {
        mnemonic: mnemonic_string,
        signing_key: hex::encode(&master_key.to_bytes()),
        verifying_keys,
        addresses
    };

    Ok(wallet_info)
}


pub fn generate_signature_from_command(command: AllegraCommands) -> std::io::Result<(Signature, RecoveryId)> {
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
        _ => {
            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "This variant does not require a signature"
                )
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

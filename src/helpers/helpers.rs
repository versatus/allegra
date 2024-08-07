use crate::{
    account::{Account, ExposedPort, Namespace, TaskId, TaskStatus},
    allegra_rpc::{GetPortMessage, PortResponse, SshDetails, VmResponse},
    create_allegra_rpc_client_to_addr,
    dht::QuorumManager,
    distro::Distro,
    event::{StateEvent, TaskStatusEvent},
    expose::update_nginx_config,
    network::peer::Peer,
    node::{Config, WalletConfig},
    params::ServiceType,
    payload_impls::Payload,
    publish::{GenericPublisher, StateTopic, TaskStatusTopic},
    statics::{
        DEFAULT_CONFIG_FILEPATH, DEFAULT_LXD_STORAGE_POOL, DEFAULT_PD_ENDPOINT,
        DEFAULT_PUBLISHER_ADDRESS, DEFAULT_SUBSCRIBER_ADDRESS, SUCCESS,
    },
    vm_info::{VmInfo, VmList},
    vmm::Instance,
};
use alloy::{
    network::{Ethereum, EthereumWallet, NetworkWallet},
    primitives::Address,
    signers::k256::{elliptic_curve::SecretKey, Secp256k1},
    signers::local::{coins_bip39::English, LocalSigner, MnemonicBuilder},
};
use bollard::container::InspectContainerOptions;
use conductor::publisher::PubStream;
use ethers_core::{
    k256::ecdsa::{RecoveryId, Signature, VerifyingKey},
    utils::public_key_to_address,
};
use geolocation::Locator;
use hex::FromHex;
use rand::Rng;
use rayon::prelude::*;
use sha3::{Digest, Sha3_256};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use std::{path::PathBuf, str::FromStr};
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

pub fn handle_get_instance_ip_output_success(
    output: &std::process::Output,
    name: &str,
) -> std::io::Result<VmInfo> {
    let vm_list_string = std::str::from_utf8(&output.stdout)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let vml = serde_json::from_str::<VmList>(&vm_list_string)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    vml.get(name)
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Unable to find VM {} in VmList", name),
        ))
        .cloned()
}

pub fn handle_get_instance_ip_output_failure(output: &std::process::Output) -> std::io::Error {
    if let Some(e) = std::str::from_utf8(&output.stderr)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        .err()
    {
        return e;
    }

    std::io::Error::new(std::io::ErrorKind::Other, "failure to get instance ip")
}

pub fn handle_get_instance_ip_output(
    output: std::process::Output,
    name: &str,
) -> std::io::Result<VmInfo> {
    if output.status.success() {
        handle_get_instance_ip_output_success(&output, name)
    } else {
        Err(handle_get_instance_ip_output_failure(&output))
    }
}

pub fn handle_update_iptables_output_failure(
    prerouting: &std::process::Output,
    forwarding: &std::process::Output,
    state: &std::process::Output,
) -> std::io::Result<()> {
    log::error!("{}", prerouting.status.success());
    log::error!("{}", forwarding.status.success());
    log::error!("{}", state.status.success());
    return Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        format!(
            r#"
                failed to updated iptables:
                    prerouting: {},
                    forwarding: {},
                    state: {},
                "#,
            prerouting.status.success(),
            forwarding.status.success(),
            state.status.success()
        ),
    ));
}

pub fn handle_update_iptables_output(
    prerouting: &std::process::Output,
    forwarding: &std::process::Output,
    state: &std::process::Output,
) -> std::io::Result<()> {
    if prerouting.status.success() && forwarding.status.success() && state.status.success() {
        log::info!("Successfully updated iptables...");
        return Ok(());
    } else {
        return handle_update_iptables_output_failure(prerouting, forwarding, state);
    }
}

pub async fn get_instance_ip(namespace: &Namespace) -> std::io::Result<String> {
    todo!()
}

pub async fn update_haproxy_config(
    namespace: Namespace,
    instance_ip: &str,
    internal_port: u16,
    external_port: u16,
    service_type: ServiceType,
) -> std::io::Result<()> {
    let config_path = "/etc/haproxy/haproxy.cfg";

    // Read the current configuration
    let mut file = std::fs::File::open(config_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let namespace_str = namespace.inner().to_string();

    // Prepare the new backend and frontend entries
    let backend_name = format!(
        "backend_{}_{}",
        service_type.to_string().to_lowercase(),
        namespace_str
    );

    let (frontend_mode, backend_mode) = match service_type {
        ServiceType::NodeJs => ("http", "http"),
        _ => ("tcp", "tcp"),
    };

    let backend_entry = format!(
        "\nbackend {}\n\tmode {}\n\tserver server1 {}:{}\n",
        backend_name, backend_mode, instance_ip, internal_port
    );

    let frontend_entry = format!(
        "\nfrontend {}_{}_{}\n\tbind *:{}\n\tmode {}\n\tdefault_backend {}\n",
        service_type.to_string().to_lowercase(),
        namespace_str,
        external_port,
        external_port,
        frontend_mode,
        backend_name
    );

    // Append the new entries to the configuration
    contents.push_str(&backend_entry);
    contents.push_str(&frontend_entry);

    // Write the updated configuration back to the file
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(config_path)?;
    file.write_all(contents.as_bytes())?;

    // Reload HAProxy
    std::process::Command::new("sudo")
        .args(["systemctl", "reload", "haproxy"])
        .output()?;

    Ok(())
}

pub async fn update_iptables(
    uri: &str,
    vmlist: VmList,
    owner: [u8; 20],
    namespace: Namespace,
    next_port: u16,
    service_type: ServiceType,
    task_id: TaskId,
    instance_ip: String,
    internal_port: u16,
) -> std::io::Result<([u8; 20], TaskId, TaskStatus)> {
    //TODO: Replace with HAProxy
    let mut publisher = GenericPublisher::new(uri).await?;
    log::info!("acquired instance IP: {instance_ip}...");

    update_haproxy_config(
        namespace.clone(),
        &instance_ip,
        internal_port,
        next_port,
        service_type.clone(),
    )
    .await?;

    update_ufw_out(&instance_ip)?;
    log::info!("updated_ufw...");

    let exposed_ports = vec![ExposedPort::new(next_port, None)];

    let event_id = uuid::Uuid::new_v4();
    let event = StateEvent::PutAccount {
        event_id: event_id.to_string(),
        task_id: task_id.clone(),
        task_status: TaskStatus::Success,
        owner,
        vmlist: vmlist.clone(),
        namespace: namespace.clone(),
        exposed_ports: Some(exposed_ports.clone()),
    };

    publisher
        .publish(Box::new(StateTopic), Box::new(event))
        .await?;

    log::info!(
        "published event {} to topic {}",
        event_id.to_string(),
        StateTopic
    );

    update_nginx_config(
        &instance_ip,
        internal_port
            .try_into()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
        next_port
            .try_into()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
    )
    .await?;

    log::info!("updated nginx config...");

    Ok((owner, task_id, TaskStatus::Success))
}

pub async fn update_instance(
    namespace: Namespace,
    vm_info: VmInfo,
    port_map: impl IntoParallelIterator<Item = (u16, (u16, ServiceType))>,
    //TODO(asmith): Replace with GenericPublisher
    state_client: tikv_client::RawClient,
) -> std::io::Result<()> {
    log::info!("checking if instance has entry in state...");
    //TODO(asmith): Replace with `StateEvent::GetInstance` event
    let instance = if let Ok(Some(instance_bytes)) = state_client.get(namespace.inner()).await {
        log::info!("instance has entry in state, deserializing...");
        let mut instance: Instance = serde_json::from_slice(&instance_bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        log::info!(
            "successfully acquired existing instance: {} from local state",
            namespace.inner()
        );
        instance.update_vminfo(vm_info);
        log::info!("updated vminfo...");
        instance.extend_port_mapping(port_map.into_par_iter());
        log::info!("extended portt mapping...");
        instance
    } else {
        log::info!("instance {} doesn't exist, building...", namespace.inner());
        let instance = Instance::new(namespace.clone(), vm_info, port_map, None, None);
        log::info!("successfully built instance {}...", namespace.inner());
        instance
    };

    log::info!("Instance: {:?}", &instance);

    //TODO(asmith): Replace with StateEvent::PutInstance event
    state_client
        .put(
            namespace.inner(),
            serde_json::to_vec(&instance)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?,
        )
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    log::info!(
        "successfully added instance: {} to local state",
        &namespace.inner()
    );

    Ok(())
}

pub fn handle_update_ufw_output_failure(output: &std::process::Output) -> std::io::Result<()> {
    let err = std::str::from_utf8(&output.stderr)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
        .to_string();

    log::error!("{}", &err);

    Err(std::io::Error::new(std::io::ErrorKind::Other, err))
}

pub fn handle_update_ufw_output(output: &std::process::Output) -> std::io::Result<()> {
    if output.status.success() {
        log::info!("successfully updated ufw...");
        Ok(())
    } else {
        handle_update_ufw_output_failure(output)
    }
}

pub fn update_ufw_out(instance_ip: &str) -> std::io::Result<()> {
    let output = std::process::Command::new("sudo")
        .arg("ufw")
        .arg("allow")
        .arg("out")
        .arg("from")
        .arg(instance_ip)
        .arg("to")
        .arg("any")
        .arg("port")
        .arg("443")
        .arg("proto")
        .arg("tcp")
        .output()?;

    log::info!("ran update ufw command");
    handle_update_ufw_output(&output)
}

pub fn update_ufw_in(_next_port: u16) -> std::io::Result<()> {
    let output = std::process::Command::new("sudo")
        .arg("ufw")
        .arg("allow")
        .arg("in")
        .arg("on")
        .arg("lxdbr0") //TODO(give this from vmm)
        .arg("from")
        .arg("any")
        .arg("to")
        .arg("any")
        .arg("port")
        .arg("22") //TODO allow this to be a variable
        .arg("proto")
        .arg("tcp")
        .output()?;

    handle_update_ufw_output(&output)
}

pub fn recover_owner_address(
    m: impl AsRef<[u8]>,
    sig: String,
    recovery_id: u32,
) -> std::io::Result<[u8; 20]> {
    let signature = Signature::from_str(&sig)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let verifying_key = VerifyingKey::recover_from_msg(
        m.as_ref(),
        &signature,
        RecoveryId::try_from(recovery_id.to_be_bytes()[3])
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?,
    )
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let address = public_key_to_address(&verifying_key);
    Ok(address.0)
}

pub fn recover_namespace(owner: [u8; 20], name: &str) -> Namespace {
    let mut hasher = Sha3_256::new();
    hasher.update(owner);
    hasher.update(name.as_bytes());
    let hash = hasher.finalize().to_vec();
    let hex = hex::encode(&hash[0..20]);
    Namespace::new(hex)
}

pub fn owner_address_from_string(addr: &str) -> std::io::Result<[u8; 20]> {
    let mut owner: [u8; 20] = [0; 20];
    if addr.starts_with("0x") {
        let byte_vec = hex::decode(&addr[2..])
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        owner.copy_from_slice(&byte_vec[..]);
        Ok(owner)
    } else {
        let byte_vec =
            hex::decode(&addr).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        owner.copy_from_slice(&byte_vec[..]);
        Ok(owner)
    }
}

pub async fn verify_ownership(
    //TODO(asmith): Replace with GenericPublisher
    state_client: tikv_client::RawClient,
    owner: [u8; 20],
    namespace: Namespace,
) -> std::io::Result<()> {
    //Todo(asmith): Replace with StateEvent::GetAccount event
    let account = serde_json::from_slice::<Account>(
        &state_client
            .get(owner.to_vec())
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Unable to find account for owner: {}", hex::encode(owner)),
            ))?,
    )
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    log::info!("Account: {:?}", &account);

    if account.namespaces().contains_key(&namespace) {
        return Ok(());
    }

    return Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("Unable to find namespace {} in account", namespace.inner()),
    ));
}

pub async fn update_task_status(
    uri: String,
    owner: [u8; 20],
    task_id: TaskId,
    task_status: TaskStatus,
) -> std::io::Result<()> {
    let event_id = uuid::Uuid::new_v4().to_string();
    let task_status_event = TaskStatusEvent::Update {
        owner,
        task_id: task_id.clone(),
        task_status,
        event_id,
    };

    let mut publisher = GenericPublisher::new(&uri).await?;
    publisher
        .publish(Box::new(TaskStatusTopic), Box::new(task_status_event))
        .await?;
    log::info!(
        "Published TaskStatusEvent {:?} to Topic {}",
        task_id,
        TaskStatusTopic.to_string()
    );
    Ok(())
}

pub async fn update_account(
    //TODO(asmith): Replace with GenericPublisher;
    state_client: tikv_client::RawClient,
    vmlist: VmList,
    owner: [u8; 20],
    namespace: Namespace,
    task_id: TaskId,
    task_status: TaskStatus,
    exposed_ports: Vec<ExposedPort>,
) -> std::io::Result<()> {
    let vm_info = vmlist.get(&namespace.inner());
    let account = Account::new(
        owner,
        vec![(namespace.clone(), vm_info.cloned())],
        vec![(namespace.clone(), exposed_ports.clone())],
        vec![(task_id.clone(), task_status.clone())],
    );

    log::info!("updating owner account...");
    //TODO(asmith): Replace with StateEvent::PutAccount or StateEvent::PostAccount Event
    // Probably best to use Put for overwrites/new accounts and Post for updates
    let account: Account = if let Ok(Some(account_bytes)) = state_client.get(owner.to_vec()).await {
        let mut account = match serde_json::from_slice::<Account>(&account_bytes) {
            Ok(account) => account,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            }
        };

        log::info!("acquired owner account...");
        account.update_namespace(&namespace, vm_info);
        log::info!("updated owner account namespaces...");
        account.update_exposed_ports(&namespace, exposed_ports.clone());
        log::info!("updated owner account exposed port for namespace...");
        account.update_task_status(&task_id, task_status);
        log::info!("updated owner account task status for task...");

        account
    } else {
        log::info!("owner account doesn't exist, had to create new account...");
        account
    };

    //TODO(asmith): Replace with StateEvent::PutAccount or StateEvent::PostAccount Event
    // Probably best to use Put for overwrites/new accounts and Post for updates
    state_client
        .put(
            owner.to_vec(),
            serde_json::to_vec(&account)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?,
        )
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    log::info!("successfully updated owner account in state...");
    Ok(())
}

pub async fn get_public_ip() -> std::io::Result<String> {
    let resp = reqwest::get("https://api.ipify.org")
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    let ip = resp
        .text()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    return Ok(ip);
}

pub async fn get_instance(
    namespace: Namespace,
    //Replace with GenericPublisher
    state_client: tikv_client::RawClient,
) -> std::io::Result<Instance> {
    log::info!("attempting to get instance: {}", namespace.inner());
    serde_json::from_slice(
        &state_client
            .get(namespace.inner())
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Instance does not exist in state",
            ))?,
    )
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
}

pub fn generate_task_id(params: impl Payload) -> Result<TaskId, Status> {
    let bytes = serde_json::to_vec(&params.into_payload())
        .map_err(|e| Status::new(tonic::Code::FailedPrecondition, e.to_string()))?;
    let mut hasher = Sha3_256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    Ok(TaskId::new(hex::encode(&result[..])))
}

pub async fn get_port(
    namespace: String,
    ip: &str,
    service_type: ServiceType,
) -> Result<Response<PortResponse>, Status> {
    let mut client = create_allegra_rpc_client_to_addr(&format!("http://{}:50051", ip))
        .await
        .map_err(|e| Status::from_error(Box::new(e)))?;

    let get_port_message = GetPortMessage {
        namespace,
        service_type: service_type.into(),
    };

    client.get_port(Request::new(get_port_message)).await
}

pub async fn get_ssh_details(
    network_state: Arc<RwLock<QuorumManager>>,
    namespace: Namespace,
    remote_addr: Option<SocketAddr>,
) -> Result<Response<VmResponse>, Status> {
    let peers = get_quorum_peers(network_state.clone(), namespace.clone()).await?;
    let socket_addr = remote_addr.ok_or(Status::failed_precondition(
        "Unable to acquire requestor addr",
    ))?;
    let location = geolocation::find(&socket_addr.ip().to_string()).ok();
    if let Some(location) = location {
        let latitude = location.latitude.parse::<f64>().ok();
        let longitude = location.longitude.parse::<f64>().ok();
        if let (Some(lat), Some(long)) = (latitude, longitude) {
            let peer_locations = get_peer_locations(peers.clone());
            let distances = get_peer_distances(lat, long, peer_locations);
            let closest = get_closest_peer(distances);
            if let Some((addr, _)) = closest {
                let port = get_port(namespace.inner().clone(), &addr, ServiceType::Ssh).await;
                match port {
                    Ok(resp) => {
                        return Ok(Response::new(VmResponse {
                            status: SUCCESS.to_string(),
                            details: "Successfully acquired geographically closest node"
                                .to_string(),
                            ssh_details: Some(SshDetails {
                                ip: addr,
                                port: resp.into_inner().port,
                            }),
                        }))
                    }
                    _ => {}
                }
            }
        }
    }

    get_random_ip(peers, namespace.clone()).await
}

pub async fn get_quorum_peers(
    network_state: Arc<RwLock<QuorumManager>>,
    namespace: Namespace,
) -> Result<HashSet<Peer>, Status> {
    let guard = network_state.read().await;
    let id = guard
        .get_instance_quorum_membership(&namespace)
        .ok_or(Status::not_found(format!(
            "Unable to find quorum responsible for instance: {}",
            &namespace
        )))?
        .clone();

    let quorum = guard
        .get_quorum_by_id(&id)
        .ok_or(Status::not_found(format!(
            "Unable to find quorum responsible for instance: {}",
            &namespace
        )))?
        .clone();
    let peers = quorum.peers();
    Ok(peers.clone())
}

pub fn get_peer_locations(peers: impl IntoParallelIterator<Item = Peer>) -> Vec<Locator> {
    peers
        .into_par_iter()
        .filter_map(|p| geolocation::find(&p.ip_address().to_string()).ok())
        .collect()
}

pub fn get_peer_distances(
    request_latitude: f64,
    request_longitude: f64,
    peer_locations: Vec<Locator>,
) -> Vec<(String, f64)> {
    peer_locations
        .par_iter()
        .filter_map(|loc| {
            let node_latitude = loc.latitude.parse::<f64>().ok();
            let node_longitude = loc.longitude.parse::<f64>().ok();

            match (node_latitude, node_longitude) {
                (Some(x), Some(y)) => {
                    let requestor_location = haversine::Location {
                        latitude: request_latitude,
                        longitude: request_longitude,
                    };

                    let node_location = haversine::Location {
                        latitude: x,
                        longitude: y,
                    };

                    let distance = haversine::distance(
                        requestor_location,
                        node_location,
                        haversine::Units::Kilometers,
                    );

                    Some((loc.ip.clone(), distance))
                }
                _ => None,
            }
        })
        .collect()
}

pub fn get_closest_peer(distances: Vec<(String, f64)>) -> Option<(String, f64)> {
    distances
        .into_par_iter()
        .min_by(|a, b| match a.1.partial_cmp(&b.1) {
            Some(order) => order,
            None => std::cmp::Ordering::Equal,
        })
}

pub async fn get_random_ip(
    peers: HashSet<Peer>,
    namespace: Namespace,
) -> Result<Response<VmResponse>, Status> {
    let random_ip = {
        let mut rng = rand::thread_rng();
        let len = peers.len();
        let random_choice = rng.gen_range(0..len);
        let peers_to_choose: Vec<&Peer> = peers.par_iter().collect();
        peers_to_choose[random_choice].ip_address().to_string()
    };

    if let Ok(resp) = get_port(namespace.inner().clone(), &random_ip, ServiceType::Ssh).await {
        return Ok(Response::new(VmResponse {
            status: SUCCESS.to_string(),
            details: "Successfully acquired random node, unable to acquire geographically closest"
                .to_string(),
            ssh_details: Some(SshDetails {
                ip: random_ip.to_string(),
                port: resp.into_inner().port,
            }),
        }));
    } else {
        return Err(Status::failed_precondition(
            "Unable to acquire lock on network state",
        ));
    }
}

pub fn get_payload_hash(payload_bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha3_256::new();
    hasher.update(payload_bytes);
    hasher.finalize().to_vec()
}

pub async fn load_config_from_env() -> std::io::Result<Config> {
    let path = std::env::var("CONFIG_FILEPATH").unwrap_or(DEFAULT_CONFIG_FILEPATH.to_string());

    log::info!("Attempting to load config from env variable or default: {path}");
    log::info!("current working directory: {:?}", std::env::current_dir());
    Ok(Config::from_file(&path).await?)
}

pub async fn load_config_from_path(config_path: &str) -> std::io::Result<Config> {
    log::info!("Attempting to load config from provided path");
    Ok(Config::from_file(config_path).await?)
}

pub async fn load_config_from_env_or_path(config_path: Option<&str>) -> std::io::Result<Config> {
    if let Some(path) = config_path {
        return load_config_from_path(path).await;
    }

    load_config_from_env().await
}

pub async fn load_or_create_ethereum_address(
    config_path: Option<&str>,
) -> std::io::Result<Address> {
    log::info!("Attempting to load config from env or provided path...");
    let config = load_config_from_env_or_path(config_path).await?;
    log::info!("Successfully loaded config...");
    log::info!("Config: {:?}", config);

    let config = config.node_config();
    if let Some(sk) = config.wallet_signing_key() {
        log::info!("Acquired wallet signing key as string, attempting to deserialize from bytes");
        let decoded_sk =
            hex::decode(&sk).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let secret_key: SecretKey<Secp256k1> = SecretKey::from_slice(&decoded_sk)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let signing_key = alloy::signers::k256::ecdsa::SigningKey::from(secret_key);

        log::info!("Creating local signer...");
        let local_signer = LocalSigner::from_signing_key(signing_key);

        log::info!("Creating ethereum wallet...");
        let mut wallet = EthereumWallet::default();

        log::info!("Registering local signer with ethereum wallet...");
        wallet.register_default_signer(local_signer);

        log::info!("Returning default signer address...");
        Ok(<EthereumWallet as NetworkWallet<Ethereum>>::default_signer_address(&wallet))
    } else if let Some(keypath) = config.wallet_keyfile() {
        log::info!("Attempting to create wallet from key file...");
        let file_content = tokio::fs::read_to_string(keypath).await?;

        log::info!("Attempting to deserialize key file content into Wallet Config...");
        let wallet_config: WalletConfig = serde_json::from_str(&file_content)?;

        log::info!("Attempting to deserialize ethereum address from hex string...");
        let address = Address::from_hex(wallet_config.address())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        log::info!("returning ethereum address ...");
        Ok(address)
    } else if let Some(wallet_address) = config.wallet_address() {
        log::info!(
            "Attempting to deserialize Address from hex string from config.wallet_address()..."
        );
        let address = Address::from_hex(wallet_address)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        log::info!("returning ethereum address...");
        Ok(address)
    } else {
        log::info!(
            "None of the attempts to load the address succedded, created new ethereum wallet..."
        );
        log::info!("attempting to build mnemonic phrase...");
        log::info!("attempting to write mnemonic phrase to .mnemonic file...");
        let local_signer = MnemonicBuilder::<English>::default()
            .word_count(12)
            .write_to(".mnemonic")
            .build()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        log::info!("Instantiating default ethereum wallet...");
        let mut wallet = EthereumWallet::default();

        log::info!("Registering local signer with ethereum wallet as default signer...");
        wallet.register_default_signer(local_signer);

        log::info!("returning default signer address...");
        Ok(<EthereumWallet as NetworkWallet<Ethereum>>::default_signer_address(&wallet))
    }
}

#[cfg(feature = "docker")]
pub async fn load_or_get_public_ip_addresss(
    config_path: Option<&str>,
) -> std::io::Result<SocketAddr> {
    log::info!("Attempting to load config from env or provided path...");
    let config = load_config_from_env_or_path(config_path).await?;
    log::info!("Successfully loaded config...");

    let config = config.node_config();
    if let Some(ip_addr) = config.public_ip_address() {
        log::info!(
            "Config contained declared public ip address, attempting to parse into SocketAddr..."
        );
        let socket_addr = ip_addr
            .to_string()
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        log::info!("Successfully parsed ip address string into SocketAddr, returning...");
        Ok(socket_addr)
    } else {
        log::info!("config did not include declared public ip address, attempting to acquire via http call");
        let container_name = std::env::var("CONTAINER_NAME").unwrap_or("allegra".to_string());
        let ip_addr = get_container_ip(&container_name).await?;
        log::info!("succesfully acquired ip address string via HTTP call, attempting to parse into SocketAddr...");
        let server_ip_address = format!("{}:50051", ip_addr);
        let socket_addr = server_ip_address
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        log::info!("succesfully parsed ip address string into SocketAddr, returning...");
        Ok(socket_addr)
    }
}

#[cfg(not(feature = "docker"))]
pub async fn load_or_get_public_ip_addresss(
    config_path: Option<&str>,
) -> std::io::Result<SocketAddr> {
    log::info!("Attempting to load config from env or provided path...");
    let config = load_config_from_env_or_path(config_path).await?;
    log::info!("Successfully loaded config...");

    let config = config.node_config();
    if let Some(ip_addr) = config.public_ip_address() {
        log::info!(
            "Config contained declared public ip address, attempting to parse into SocketAddr..."
        );
        let socket_addr = ip_addr
            .to_string()
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        log::info!("Successfully parsed ip address string into SocketAddr, returning...");
        Ok(socket_addr)
    } else {
        log::info!("config did not include declared public ip address, attempting to acquire via http call");
        let ip_addr = get_public_ip().await?;
        log::info!("succesfully acquired ip address string via HTTP call, attempting to parse into SocketAddr...");
        let server_ip_address = format!("{}:50051", ip_addr);
        let socket_addr = server_ip_address
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        log::info!("succesfully parsed ip address string into SocketAddr, returning...");
        Ok(socket_addr)
    }
}

pub async fn load_bootstrap_node(
    config_path: Option<&str>,
) -> std::io::Result<Vec<(Address, SocketAddr)>> {
    log::info!("Attempting to load config from env or provided path...");
    let config = load_config_from_env_or_path(config_path).await?;
    log::info!("Successfully loaded config...");

    let config = config.node_config();

    if let Some(bootstrap_ip_addresses) = config.bootstrap_ip_addresses() {
        log::info!("discovered bootstrap ip addresses in config...");
        if let Some(bootstrap_wallet_addresses) = config.bootstrap_wallet_addresses() {
            log::info!("discovered bootstrap wallet addresses in config...");
            log::info!("collecting wallet and ip addresses into pairs...");
            let bootstrap_nodes = bootstrap_wallet_addresses
                .par_iter()
                .zip(bootstrap_ip_addresses.par_iter())
                .filter_map(|(wallet, ip)| {
                    match (Address::from_hex(wallet.clone()), ip.parse::<SocketAddr>()) {
                        (Ok(wallet_address), Ok(ip_address)) => Some((wallet_address, ip_address)),
                        _ => None,
                    }
                })
                .collect::<Vec<(Address, SocketAddr)>>();
            log::info!("returning boostrap node addresses...");
            return Ok(bootstrap_nodes);
        }
    }

    log::error!("one or more conditions failed in attempting to acquire bootstrap peers...");
    return Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Boostrap node wallet address and ip address required to be bootstrapped into the network",
    ));
}

pub async fn load_or_get_vmm_filesystem(config_path: Option<&str>) -> std::io::Result<String> {
    log::info!("Attempting to load config from env or provided path...");
    let config = load_config_from_env_or_path(config_path).await?;
    log::info!("Successfully loaded config...");

    let config = config.node_config();

    if let Some(vmm_filesystem) = config.vmm_filesystem() {
        return Ok(vmm_filesystem.clone());
    }

    Ok(std::env::var("LXD_STORAGE_POOL").unwrap_or_else(|_| DEFAULT_LXD_STORAGE_POOL.to_string()))
}

pub async fn load_or_get_broker_endpoints(
    config_path: Option<&str>,
) -> std::io::Result<(String, String)> {
    log::info!("Attempting to load config from env or provided path...");
    let config = load_config_from_env_or_path(config_path).await?;
    log::info!("Successfully loaded config...");

    let config = config.node_config();
    if let (Some(broker_frontend), Some(broker_backend)) =
        (config.broker_frontend(), config.broker_backend())
    {
        return Ok((broker_frontend.clone(), broker_backend.clone()));
    }

    let broker_frontend =
        std::env::var("BROKER_FRONTEND").unwrap_or_else(|_| DEFAULT_PUBLISHER_ADDRESS.to_string());
    let broker_backend =
        std::env::var("BROKER_BACKEND").unwrap_or_else(|_| DEFAULT_SUBSCRIBER_ADDRESS.to_string());

    Ok((broker_frontend, broker_backend))
}

pub async fn load_or_get_publisher_uri(config_path: Option<&str>) -> std::io::Result<String> {
    log::info!("Attempting to load config from env or provided path...");
    let config = load_config_from_env_or_path(config_path).await?;
    log::info!("Successfully loaded config...");

    let config = config.node_config();

    if let Some(publisher_uri) = config.publisher_uri() {
        return Ok(publisher_uri.clone());
    }

    let publisher_uri =
        std::env::var("PUBLISHER_ADDRESS").unwrap_or(DEFAULT_PUBLISHER_ADDRESS.to_string());

    Ok(publisher_uri)
}

pub async fn load_or_get_subscriber_uri(config_path: Option<&str>) -> std::io::Result<String> {
    log::info!("Attempting to load config from env or provided path...");
    let config = load_config_from_env_or_path(config_path).await?;
    log::info!("Successfully loaded config...");

    let config = config.node_config();

    if let Some(subscriber_uri) = config.subscriber_uri() {
        return Ok(subscriber_uri.clone());
    }

    let publisher_uri =
        std::env::var("SUBSCRIBER_ADDRESS").unwrap_or(DEFAULT_SUBSCRIBER_ADDRESS.to_string());

    Ok(publisher_uri)
}

pub async fn load_or_get_pd_endpoints(config_path: Option<&str>) -> std::io::Result<Vec<String>> {
    log::info!("Attempting to load config from env or provided path...");
    let config = load_config_from_env_or_path(config_path).await?;
    log::info!("Successfully loaded config...");

    let config = config.node_config();

    if let Some(pd_endpoints) = config.pd_endpoints() {
        return Ok(pd_endpoints.clone());
    }

    let pd_endpoints = std::env::var("PD_ENDPOINT").unwrap_or(DEFAULT_PD_ENDPOINT.to_string());

    Ok(vec![pd_endpoints])
}

pub async fn get_container_ip(container_name: &str) -> std::io::Result<String> {
    let docker = bollard::Docker::connect_with_local_defaults()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let options = Some(InspectContainerOptions { size: false });
    match docker.inspect_container(container_name, options).await {
        Ok(container_info) => {
            if let Some(network_settings) = container_info.network_settings {
                if let Some(networks) = network_settings.networks {
                    for (_, network) in networks {
                        if let Some(ip_address) = network.ip_address {
                            return Ok(ip_address);
                        }
                    }
                }
            }
        }
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }

    return Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "unable to acquire docker container's ip address",
    ));
}

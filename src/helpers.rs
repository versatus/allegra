use crate::{
    account::{
        Account, ExposedPort, Namespace, TaskId, TaskStatus
    }, allegra_rpc::{GetPortMessage, SshDetails, PortResponse, VmResponse}, 
    dht::{QuorumManager, Peer},
    event::{StateEvent, TaskStatusEvent}, 
    expose::update_nginx_config, 
    params::{Payload, ServiceType}, 
    publish::{GenericPublisher, StateTopic, TaskStatusTopic}, 
    vm_info::{
        VmAddress, VmInfo, VmList
    }, vmm::{Instance, SUCCESS},
    create_allegra_rpc_client_to_addr
};
use std::str::FromStr;
use conductor::publisher::PubStream;
use sha3::{Digest, Sha3_256};
use ethers_core::{
    utils::public_key_to_address,
    k256::ecdsa::{
        VerifyingKey,
        Signature,
        RecoveryId
    }
};
use rayon::prelude::*;
use tonic::{Response, Status, Request};
use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::RwLock;
use std::net::SocketAddr;
use geolocation::Locator;
use rand::Rng;

pub fn handle_get_instance_ip_output_success(
    output: &std::process::Output,
    name: &str
) -> std::io::Result<VmInfo> {

    let vm_list_string = std::str::from_utf8(&output.stdout).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    })?;

    let vml = serde_json::from_str::<VmList>(&vm_list_string).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    })?; 

    vml.get(name).ok_or(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Unable to find VM {} in VmList", name)
        )
    )
}

pub fn handle_get_instance_ip_output_failure(
    output: &std::process::Output
) -> std::io::Error {
    if let Some(e) = std::str::from_utf8(&output.stderr).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    }).err() {
        return e
    }

    std::io::Error::new(
        std::io::ErrorKind::Other,
        "failure to get instance ip" 
    )
}

pub fn handle_get_instance_ip_output(
    output: std::process::Output,
    name: &str
) -> std::io::Result<VmInfo> {
    if output.status.success() {
        handle_get_instance_ip_output_success(&output, name)
    } else {
        Err(handle_get_instance_ip_output_failure(&output))
    }
}

pub fn get_instance_ip(name: &str) -> std::io::Result<String> {
    let output = std::process::Command::new("lxc")
        .args(["list", "--format", "json"])
        .output()?;

    let vminfo = handle_get_instance_ip_output(output, name)?;

    let network = vminfo.state().network().ok_or(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("unable to find network for VM {name}")
        )
    )?;

    let interface = network.enp5s0().ok_or(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("unable to find network interface (enp5s0) for vm {name}")
        )
    )?;

    let addresses = interface.addresses();

    let addr = addresses.par_iter().filter(|addr| {
        addr.family() == "inet"
    }).collect::<Vec<&VmAddress>>().pop().ok_or(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("unable to find inet family address for vm {name}")
        )
    )?;

    Ok(addr.address())
}

pub fn handle_update_iptables_output_failure(
    prerouting: &std::process::Output,
    forwarding: &std::process::Output,
    state: &std::process::Output
) -> std::io::Result<()> {
    log::error!("{}", prerouting.status.success());
    log::error!("{}", forwarding.status.success());
    log::error!("{}", state.status.success());
    return Err(
        std::io::Error::new(
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
            )
        )
    )
}

pub fn handle_update_iptables_output(
    prerouting: &std::process::Output,
    forwarding: &std::process::Output,
    state: &std::process::Output,
) -> std::io::Result<()> {
    if prerouting.status.success() && forwarding.status.success() && state.status.success() {
        log::info!("Successfully updated iptables...");
        return Ok(())
    } else {
        return handle_update_iptables_output_failure(prerouting, forwarding, state) 
    }
}

pub async fn update_iptables(
    uri: &str,
    vmlist: VmList,
    owner: [u8; 20],
    namespace: Namespace,
    next_port: u16,
    service_type: ServiceType,
    task_id: TaskId,
    internal_port: u16
) -> std::io::Result<([u8; 20], TaskId, TaskStatus)> {
    let mut publisher = GenericPublisher::new(uri).await?;
    let instance_ip = get_instance_ip(&namespace.inner())?;
    log::info!("acquired instance IP: {instance_ip}...");
    let prerouting = std::process::Command::new("sudo")
        .args(
            ["iptables", "-t", "nat", 
            "-A", "PREROUTING", "-p", 
            "tcp", "--dport", &next_port.to_string(), 
            "-j", "DNAT", "--to-destination", 
            &format!("{}:{}", &instance_ip, internal_port)
            ]
        )
        .output()?;
    
    log::info!("updated prerouting...");
    let forwarding = std::process::Command::new("sudo")
        .args(
            ["iptables", "-A", "FORWARD", "-p", "tcp", "-d",
            &instance_ip, "--dport", &internal_port.to_string(),
            "-j", "ACCEPT"
            ]
        )
        .output()?;
    log::info!("updated forwarding...");

    let state = std::process::Command::new("sudo")
        .args(
            ["iptables", "-A", "FORWARD", 
            "-m", "state", "--state", 
            "RELATED,ESTABLISHED", "-j", "ACCEPT"
            ]
        )
        .output()?;
    log::info!("updated iptables state...");

    handle_update_iptables_output(&prerouting, &forwarding, &state)?;
    log::info!("handled iptables output without error...");
    update_ufw_out(&instance_ip)?;
    log::info!("updated_ufw...");

    let exposed_ports = vec![
        ExposedPort::new(
            next_port, None
        )
    ];

    let event_id = uuid::Uuid::new_v4();
    let event = StateEvent::PutAccount { 
        event_id: event_id.to_string(),
        task_id: task_id.clone(),
        task_status: TaskStatus::Success,
        owner,
        vmlist: vmlist.clone(),
        namespace: namespace.clone(),
        exposed_ports: Some(exposed_ports.clone()) 
    };

    publisher.publish(
        Box::new(StateTopic), 
        Box::new(event)
    ).await?;

    log::info!("published event {} to topic {}", event_id.to_string(), StateTopic);

    let port_map = vec![(internal_port, (next_port, service_type))];
    let vminfo = vmlist.get(&namespace.inner()).ok_or(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("unable to find namespace {} in vmlist", &namespace)
        )
    )?;
    log::info!("acquired vminfo...");

    update_nginx_config(
        &instance_ip,
        internal_port.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?,
        next_port.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?
    ).await?;

    log::info!("updated nginx config...");

    let event_id = uuid::Uuid::new_v4();
    let event = StateEvent::PutInstance { 
        event_id: event_id.to_string(), 
        task_id: task_id.clone(), 
        task_status: TaskStatus::Success, 
        namespace: namespace.clone(), 
        vm_info: vminfo, 
        port_map: port_map.into_par_iter().collect() 
    };
    publisher.publish(
        Box::new(StateTopic), 
        Box::new(event)
    ).await?;

    log::info!("published event {} to topic {}", event_id.to_string(), StateTopic);

    Ok((owner, task_id, TaskStatus::Success))
}

pub async fn update_instance(
    namespace: Namespace,
    vm_info: VmInfo,
    port_map: impl IntoParallelIterator<Item = (u16, (u16, ServiceType))>,
    state_client: tikv_client::RawClient,
) -> std::io::Result<()> {
    log::info!("checking if instance has entry in state...");
    let instance = if let Ok(Some(instance_bytes)) = state_client.get(
        namespace.inner()
    ).await {
        log::info!("instance has entry in state, deserializing...");
        let mut instance: Instance = serde_json::from_slice(&instance_bytes).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;

        log::info!("successfully acquired existing instance: {} from local state", namespace.inner());
        instance.update_vminfo(vm_info);
        log::info!("updated vminfo...");
        instance.extend_port_mapping(port_map.into_par_iter());
        log::info!("extended portt mapping...");
        instance
    } else {
        log::info!("instance {} doesn't exist, building...", namespace.inner());
        let instance = Instance::new(
            namespace.clone(),
            vm_info,
            port_map,
            None
        );
        log::info!("successfully built instance {}...", namespace.inner());
        instance
    };

    log::info!("Instance: {:?}", &instance);

    state_client.put(
        namespace.inner(),
        serde_json::to_vec(
            &instance
        ).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other, 
                e.to_string()
            )
        }
    )?;

    log::info!("successfully added instance: {} to local state", &namespace.inner());

    Ok(())
}

pub fn handle_update_ufw_output_failure(output: &std::process::Output) -> std::io::Result<()> {
    let err = std::str::from_utf8(&output.stderr).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    })?.to_string();

    log::error!("{}", &err);

    Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            err
        )
    )
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
    recovery_id: u8
) -> std::io::Result<[u8; 20]> {
    let signature = Signature::from_str(&sig).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    })?; 

    let verifying_key = VerifyingKey::recover_from_msg(
        m.as_ref(),
        &signature,
        RecoveryId::try_from(
            recovery_id
        ).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?
        ).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;

    let address = public_key_to_address(&verifying_key);
    Ok(address.0)
}

pub fn recover_namespace(
    owner: [u8; 20],
    name: &str
) -> Namespace {
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
        let byte_vec = hex::decode(&addr[2..]).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        owner.copy_from_slice(&byte_vec[..]);
        Ok(owner)
    } else {
        let byte_vec = hex::decode(&addr).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        owner.copy_from_slice(&byte_vec[..]);
        Ok(owner)
    }
}

pub async fn verify_ownership(
    state_client: tikv_client::RawClient,
    owner: [u8; 20],
    namespace: Namespace
) -> std::io::Result<()> {

    let account = serde_json::from_slice::<Account>(
        &state_client.get(
            owner.to_vec()
        ).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?.ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Unable to find account for owner: {}", hex::encode(owner))
            )
        )?).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;

    log::info!("Account: {:?}", &account);

    if account.namespaces().contains_key(&namespace) {
        return Ok(())
    }
    
    return Err(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Unable to find namespace {} in account", namespace.inner())
        )
    )
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
        event_id
    };

    let mut publisher = GenericPublisher::new(&uri).await?;
    publisher.publish(
        Box::new(TaskStatusTopic),
        Box::new(task_status_event)
    ).await?;
    log::info!("Published TaskStatusEvent {:?} to Topic {}", task_id, TaskStatusTopic.to_string());
    Ok(())
}

pub async fn update_account(
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
        vec![(namespace.clone(), vm_info.clone())],
        vec![(namespace.clone(), exposed_ports.clone())],
        vec![(task_id.clone(), task_status.clone())]
    );

    log::info!("updating owner account...");
    let account: Account = if let Ok(Some(account_bytes)) = state_client.get(owner.to_vec()).await {
        let mut account = match serde_json::from_slice::<Account>(&account_bytes) {
            Ok(account) => account,
            Err(e) => return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other, 
                    e.to_string()
                )
            )
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

    state_client.put(
        owner.to_vec(),
        serde_json::to_vec(
            &account
        ).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other, 
                e.to_string()
            )
        }
    )?;

    log::info!("successfully updated owner account in state...");
    Ok(())
}

pub async fn get_public_ip() -> std::io::Result<String> {
    let resp = reqwest::get("https://api.ipify.org").await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    })?;
    let ip = resp.text().await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    })?;

    return Ok(ip)
}

pub async fn get_instance(
    namespace: Namespace,
    state_client: tikv_client::RawClient
) -> std::io::Result<Instance> {
    log::info!("attempting to get instance: {}", namespace.inner());
    serde_json::from_slice(
        &state_client.get(
            namespace.inner()
        ).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?.ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Instance does not exist in state"
            ) 
        )?).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })
}


pub async fn copy_instance(
    _namespace: String,
    _target: String,
    _path: String,
) -> std::io::Result<()> {
    Ok(())
}

pub async fn migrate_instance(
    _vmlist: VmList,
    _namespace: String,
    _target: String,
    _path: String, 
    _new_quorum: Option<String>,
) -> std::io::Result<()> {
    Ok(())
}

pub async fn create_temp_instance(
    namespace: &str
) -> std::io::Result<()> {
    let output = std::process::Command::new("sudo")
        .arg("lxc")
        .arg("copy")
        .arg(&namespace)
        .arg(&format!("{namespace}-temp"))
        .output()?;

    if output.status.success() {
        log::info!("temporary instance succesfully created");
        return Ok(())
    } else {
        let stderr = std::str::from_utf8(&output.stderr).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                stderr
            )
        )
    }
}

pub async fn stop_temp_instance(namespace: &str) -> std::io::Result<()> {
    let output = std::process::Command::new("sudo")
        .arg("lxc")
        .arg("stop")
        .arg(&namespace)
        .arg(&format!("{namespace}-temp"))
        .output()?;

    if output.status.success() {
        log::info!("temporary instance succesfully stopped");
        return Ok(())
    } else {
        let stderr = std::str::from_utf8(&output.stderr).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                stderr
            )
        )
    }
}

pub async fn export_temp_instance(
    namespace: &str,
    path: &str
) -> std::io::Result<String> {
    let fqp = format!("{path}/{namespace}-temp.tar.gz");
    let output = std::process::Command::new("sudo")
        .arg("lxc")
        .arg("export")
        .arg(&format!("{namespace}-temp"))
        .arg(&fqp)
        .output()?;

    if output.status.success() {
        log::info!("temporary instance succesfully exported");
        return Ok(fqp)
    } else {
        let stderr = std::str::from_utf8(&output.stderr).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                stderr
            )
        )
    }
}

pub async fn transfer_temp_instance(
    _namespace: &str,
    _path: &str,
    _dst: &str
) -> std::io::Result<()> {
    /*
    let network_event = NetworkEvent::Sync { 
        namespace: namespace.to_string(), 
        path: path.to_string(),
        target: dst.to_string(),
        last_update: None,
        dst: dst.to_string(),
    };

    let _event = Event::NetworkEvent(network_event);
    */

    log::info!("NetworkEvent successfully added to event broker");
    Ok(())
}

pub async fn remove_temp_instance(
    namespace: &str,
    path: &str
) -> std::io::Result<()> {
    let delete_output = std::process::Command::new("sudo")
        .arg("lxc")
        .arg("delete")
        .arg(&format!("{}-temp", namespace))
        .output()?;

    let rm_output = std::process::Command::new("sudo")
        .arg("rm")
        .arg("-rf")
        .arg(path)
        .output()?;

    if delete_output.status.success() && rm_output.status.success() {
        log::info!("Successfully deleted temporary instance and removed backup tarball");
        return Ok(())
    }

    return Err(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "Unable to delete or remove temporary instance from local fs"
        )
    )
}


pub fn generate_task_id(params: impl Payload) -> Result<TaskId, Status> {
    let bytes = serde_json::to_vec(&params.into_payload()).map_err(|e| {
        Status::new(tonic::Code::FailedPrecondition, e.to_string())
    })?;
    let mut hasher = Sha3_256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    Ok(TaskId::new(
        hex::encode(&result[..])
    ))
}

pub async fn get_port(
    namespace: String,
    ip: &str,
    service_type: ServiceType
) -> Result<Response<PortResponse>, Status> {
    let mut client = create_allegra_rpc_client_to_addr(&format!("http://{}:50051", ip)).await.map_err(|e| {
        Status::from_error(Box::new(e))
    })?;

    let get_port_message = GetPortMessage {
        namespace, service_type: service_type.into()
    };

    client.get_port(Request::new(get_port_message)).await
}

pub async fn get_ssh_details(
    network_state: Arc<RwLock<QuorumManager>>,
    namespace: Namespace,
    remote_addr: Option<SocketAddr>,
) -> Result<Response<VmResponse>, Status> {
    let peers = get_quorum_peers(network_state.clone(), namespace.clone()).await?;
    let socket_addr = remote_addr.ok_or(
        Status::failed_precondition(
            "Unable to acquire requestor addr"
        )
    )?;
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
                        return Ok(
                            Response::new(
                                VmResponse {
                                    status: SUCCESS.to_string(),
                                    details: "Successfully acquired geographically closest node".to_string(),
                                    ssh_details: Some(SshDetails {
                                        ip: addr,
                                        port: resp.into_inner().port
                                    })
                                }
                            )
                        )
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
) -> Result<HashSet<Peer>, Status>{
    let guard = network_state.read().await;
    let id = guard.get_instance_quorum_membership(&namespace).ok_or(
        Status::not_found(
            format!(
                "Unable to find quorum responsible for instance: {}", 
                &namespace
            )
        )
    )?.clone();

    let quorum = guard.get_quorum_by_id(&id).ok_or(
        Status::not_found(
            format!("Unable to find quorum responsible for instance: {}", &namespace)
        )
    )?.clone();
    let peers = quorum.peers();
    Ok(peers.clone())
}

pub fn get_peer_locations(peers: impl IntoParallelIterator<Item = Peer>) -> Vec<Locator> {
    peers.into_par_iter().filter_map(|p| {
        geolocation::find(p.address()).ok()
    }).collect()
}

pub fn get_peer_distances(request_latitude: f64, request_longitude: f64, peer_locations: Vec<Locator>) -> Vec<(String, f64)> {
    peer_locations.par_iter().filter_map(|loc| {
        let node_latitude = loc.latitude.parse::<f64>().ok();
        let node_longitude = loc.longitude.parse::<f64>().ok();

        match (node_latitude, node_longitude) {
            (Some(x), Some(y)) => {
                let requestor_location = haversine::Location {
                    latitude: request_latitude, longitude: request_longitude
                };

                let node_location = haversine::Location {
                    latitude: x, longitude: y
                };

                let distance = haversine::distance(
                    requestor_location, 
                    node_location, 
                    haversine::Units::Kilometers
                );
                
                Some((loc.ip.clone(), distance))
            }
            _ => None
        }
    }).collect()
}

pub fn get_closest_peer(distances: Vec<(String, f64)>) -> Option<(String, f64)> {
    distances.into_par_iter().min_by(|a, b| {
        match a.1.partial_cmp(&b.1) {
            Some(order) => order,
            None => std::cmp::Ordering::Equal
        }
    }) 
}

pub async fn get_random_ip(
    peers: HashSet<Peer>,
    namespace: Namespace
) -> Result<Response<VmResponse>, Status> {
    let random_ip = {
        let mut rng = rand::thread_rng();
        let len = peers.len();
        let random_choice = rng.gen_range(0..len);
        let peers_to_choose: Vec<&Peer> = peers.par_iter().collect();
        peers_to_choose[random_choice].address().to_string()
    };

    if let Ok(resp) = get_port(namespace.inner().clone(), &random_ip, ServiceType::Ssh).await {
        return Ok(
            Response::new(
                VmResponse {
                    status: SUCCESS.to_string(),
                    details: "Successfully acquired random node, unable to acquire geographically closest".to_string(),
                    ssh_details: Some(
                        SshDetails {
                            ip: random_ip.to_string(),
                            port: resp.into_inner().port
                        }
                    )
                }
            )
        )
    } else {
        return Err(Status::failed_precondition(
            "Unable to acquire lock on network state"
            )
        )
    }
}

pub fn get_payload_hash(payload_bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha3_256::new();
    hasher.update(payload_bytes);
    hasher.finalize().to_vec()
}

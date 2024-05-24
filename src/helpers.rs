use crate::{
    vm_info::{
        VmInfo, 
        VmList, 
        VmAddress
    }, account::{
        Namespace, 
        TaskId,
        TaskStatus,
        ExposedPort,
        Account,
    }, vmm::Instance, params::ServiceType, expose::{update_nginx_config, reload_nginx},
};
use std::str::FromStr;
use sha3::{Digest, Sha3_256};
use ethers_core::{
    utils::public_key_to_address,
    k256::ecdsa::{
        VerifyingKey,
        Signature,
        RecoveryId
    }
};
use lru::LruCache;
use std::sync::{Arc, RwLock};

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

    let addr = addresses.iter().filter(|addr| {
        addr.family() == "inet"
    }).collect::<Vec<&VmAddress>>().pop().ok_or(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("unable to find inet family address for vm {name}")
        )
    )?;

    Ok(addr.address())
}

pub fn handle_update_iptables_output_failure(output: &std::process::Output) -> std::io::Result<()> {
    let err = std::str::from_utf8(&output.stderr).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    })?;

    return Err(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            err.to_string()
        )
    )
}

pub fn handle_update_iptables_output(output: &std::process::Output) -> std::io::Result<()> {
    if output.status.success() {
        println!("Successfully updated iptables...");
        return Ok(())
    } else {
        return handle_update_iptables_output_failure(output) 
    }
}

pub async fn update_iptables(
    state_client: tikv_client::RawClient,
    vmlist: VmList,
    owner: [u8; 20],
    namespace: Namespace,
    next_port: u16,
    service_type: ServiceType,
    task_id: TaskId,
    internal_port: u16
) -> std::io::Result<([u8; 20], TaskId, TaskStatus)> {
    let instance_ip = get_instance_ip(&namespace.inner())?;
    println!("acquired instance IP: {instance_ip}");
    let output = std::process::Command::new("sudo")
        .args(
            ["iptables", "-t", "nat", 
            "-A", "PREROUTING", "-p", 
            "tcp", "--dport", &next_port.to_string(), 
            "-j", "DNAT", "--to-destination", 
            &format!("{}:{}", &instance_ip, internal_port)
            ]
        )
        .output()?;

    handle_update_iptables_output(&output)?;
    update_ufw_in(next_port)?;
    update_ufw_out(next_port)?;
    let exposed_ports = vec![
        ExposedPort::new(next_port, None)
    ];
    update_account(
        state_client.clone(),
        vmlist.clone(),
        owner,
        namespace.clone(),
        task_id.clone(),
        TaskStatus::Success,
        exposed_ports
    ).await?;

    let port_map = vec![(internal_port, (next_port, service_type))];
    let vminfo = vmlist.get(&namespace.inner()).ok_or(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("unable to find namespace {} in vmlist", &namespace)
        )
    )?;

    update_nginx_config(
        &instance_ip,
        internal_port,
        next_port
    ).await?;

    reload_nginx().await?;

    update_instance(
        namespace, 
        vminfo, 
        port_map, 
        state_client
    ).await?;

    Ok((owner, task_id, TaskStatus::Success))
}

pub async fn update_instance(
    namespace: Namespace,
    vm_info: VmInfo,
    port_map: impl IntoIterator<Item = (u16, (u16, ServiceType))>,
    state_client: tikv_client::RawClient,
) -> std::io::Result<()> {
    let instance = if let Ok(Some(instance_bytes)) = state_client.get(
        namespace.inner()
    ).await {
        let mut instance: Instance = serde_json::from_slice(&instance_bytes).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;

        log::info!("successfully acquired existing instance: {} from local state", namespace.inner());
        instance.update_vminfo(vm_info);
        instance.extend_port_mapping(port_map.into_iter());
        instance
    } else {
        log::info!("Instance {} doesn't exist, building...", namespace.inner());
        let instance = Instance::new(
            namespace.clone(),
            vm_info,
            port_map,
            None
        );
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

    Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            err
        )
    )
}

pub fn handle_update_ufw_output(output: &std::process::Output) -> std::io::Result<()> {
    if output.status.success() {
        println!("Successfully updated ufw...");
        Ok(())
    } else {
        handle_update_ufw_output_failure(output)
    }
}

pub fn update_ufw_out(next_port: u16) -> std::io::Result<()> {
    let output = std::process::Command::new("sudo")
        .arg("ufw")
        .arg("allow")
        .arg("out")
        .arg(
            format!(
                "{}/tcp",
                next_port
            )
        )
        .output()?;

    handle_update_ufw_output(&output)
}

pub fn update_ufw_in(next_port: u16) -> std::io::Result<()> {
    let output = std::process::Command::new("sudo")
        .arg("ufw")
        .arg("allow")
        .arg("in")
        .arg(
            format!(
                "{}/tcp",
                next_port
            )
        )
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

pub async fn verify_ownership(
    state_client: tikv_client::RawClient,
    owner: [u8; 20],
    namespace: Namespace
) -> std::io::Result<()> {

    let account = serde_json::from_slice::<Account>(&state_client.get(owner.to_vec()).await.map_err(|e| {
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
    state_client: tikv_client::RawClient,
    owner: [u8; 20],
    task_id: TaskId,
    task_status: TaskStatus,
    cache: &mut Arc<RwLock<LruCache<TaskId, TaskStatus>>>
) -> std::io::Result<()> {
    let mut account = serde_json::from_slice::<Account>(&state_client.get(owner.to_vec()).await.map_err(|e| {
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

    match cache.write() {
        Ok(mut guard) => {
            guard.put(task_id.clone(), task_status.clone());
            account.update_task_status(&task_id.clone(), task_status.clone());
        }
        Err(e) => {
            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string()
                )
            )
        }
    }
    
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
    })?;

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

        account.update_namespace(&namespace, vm_info);
        account.update_exposed_ports(&namespace, exposed_ports.clone());
        account.update_task_status(&task_id, task_status);

        account
    } else {
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

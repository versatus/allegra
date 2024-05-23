use lru::LruCache;
use tarpc::context;
use crate::{
    vmm::{
        VmManagerMessage, 
        SUCCESS, 
        FAILURE, 
        PENDING, Instance, 
    }, account::{
        TaskId, 
        Account, TaskStatus
    }, vm_info::{VmResponse, SshDetails},
    params::{
        InstanceCreateParams,
        InstanceStartParams,
        InstanceStopParams,
        InstanceAddPubkeyParams,
        InstanceDeleteParams, 
        Payload, 
        InstanceExposePortParams,
        InstanceGetSshDetails
    },
    helpers::{
        recover_owner_address,
        recover_namespace, get_instance
    }, enter_ssh_session
};
use tokio::sync::mpsc::Sender;
use serde::{Serialize, Deserialize};
use sha3::{Digest, Sha3_256};
use std::sync::{Arc, RwLock};

pub const MAX_PORT: u16 = 65535;
/*
Usage:
  lxc launch [<remote>:]<image> [<remote>:][<name>] [flags]

Examples:
  lxc launch ubuntu:22.04 u1

  lxc launch ubuntu:22.04 u1 < config.yaml
      Create and start a container with configuration from config.yaml

  lxc launch ubuntu:22.04 u2 -t aws:t2.micro
      Create and start a container using the same size as an AWS t2.micro (1 vCPU, 1GiB of RAM)

  lxc launch ubuntu:22.04 v1 --vm -c limits.cpu=4 -c limits.memory=4GiB
      Create and start a virtual machine with 4 vCPUs and 4GiB of RAM
*/

/*
 *Available Commands:                                                                                                                          
  alias       Manage command aliases                                                                                                         
  auth        Manage user authorization                                                                                                      
  cluster     Manage cluster members                                                                                                         
  config      Manage instance and server configuration options                                                                               
  console     Attach to instance consoles                                                                                                    
  copy        Copy instances within or in between LXD servers                                                                                
  delete      Delete instances and snapshots                                                                                                 
  exec        Execute commands in instances                                                                                                  
  export      Export instance backups                                                                                                        
  file        Manage files in instances                                                                                                      
  help        Help about any command                                                                                                         
  image       Manage images                                                                                                                  
  import      Import instance backups                                                                                                        
  info        Show instance or server information
  init        Create instances from images
  launch      Create and start instances from images
  list        List instances
  monitor     Monitor a local or remote LXD server
  move        Move instances within or in between LXD servers
  network     Manage and attach instances to networks
  operation   List, show and delete background operations
  pause       Pause instances
  profile     Manage profiles
  project     Manage projects
  publish     Publish instances as images
  query       Send a raw query to LXD
  rebuild     Rebuild instances
  remote      Manage the list of remote servers
  rename      Rename instances and snapshots
  restart     Restart instances
  restore     Restore instances from snapshots
  snapshot    Create instance snapshots
  start       Start instances
  stop        Stop instances
  storage     Manage storage pools and volumes
  version     Show local and remote versions
  warning     Manage warnings
 */

#[tarpc::service]
pub trait Vmm {
    async fn create_vm(params: InstanceCreateParams) -> VmResponse;
    async fn shutdown_vm(params: InstanceStopParams) -> VmResponse;
    async fn start_vm(
        params: InstanceStartParams
    ) -> VmResponse;
    async fn set_ssh_pubkey(
        params: InstanceAddPubkeyParams
    ) -> VmResponse;
    async fn delete_vm(params: InstanceDeleteParams) -> VmResponse;
    async fn expose_vm_ports(params: InstanceExposePortParams) -> VmResponse;
    async fn get_task_status(owner: String, id: String) -> VmResponse;
    async fn get_ssh_details(params: InstanceGetSshDetails) -> VmResponse;
    //TODO: Implement all LXC Commands
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VmmCommand {
    Create,
    Shutdown,
    Start,
    InjectAuth,
    Delete
}

#[derive(Clone)]
pub struct VmmServer {
    pub network: String,
    pub port: u16,
    pub vmm_sender: Sender<VmManagerMessage>,
    pub tikv_client: tikv_client::RawClient,
    pub task_cache: Arc<RwLock<LruCache<TaskId, TaskStatus>>> 
}

impl Vmm for VmmServer {
    async fn create_vm(self, _: context::Context, params: InstanceCreateParams) -> VmResponse {
        let task_id = if let Ok(bytes) = serde_json::to_vec(&params) {
            let mut hasher = Sha3_256::new();
            hasher.update(bytes);
            let result = hasher.finalize();
            TaskId::new(
                hex::encode(&result[..])
            )
        } else {
            return VmResponse {
                status: FAILURE.to_string(),
                details: "Unable to generate unique task id".to_string(),
                ssh_details: None,
            }
        };
        let message = VmManagerMessage::NewInstance { params, task_id: task_id.clone() };
        match self.vmm_sender.send(message).await {
            Ok(_) => {
                return VmResponse {
                    status: PENDING.to_string(),
                    details: format!("TaskId: {}", task_id.task_id()),
                    ssh_details: None
                }
            }
            Err(e) => {
                log::error!("{e}");
                return VmResponse {
                    status: FAILURE.to_string(),
                    details: e.to_string(),
                    ssh_details: None
                }
            }
        }
    }

    async fn shutdown_vm(self, _: context::Context, params: InstanceStopParams) -> VmResponse {
        let task_id = if let Ok(bytes) = serde_json::to_vec(&params) {
            let mut hasher = Sha3_256::new();
            hasher.update(bytes);
            let result = hasher.finalize();
            TaskId::new(
                hex::encode(&result[..])
            )
        } else {
            return VmResponse {
                status: FAILURE.to_string(),
                details: "Unable to generate unique task id".to_string(),
                ssh_details: None,
            }
        };
        println!("attempting to shutdown VM...");
        let message = VmManagerMessage::StopInstance { params: params.clone(), sig: params.sig, task_id: task_id.clone() };
        match self.vmm_sender.send(message).await {
            Ok(_) => {
                println!("Successfully sent shutdown message to vmm...");
                return VmResponse {
                    status: PENDING.to_string(),
                    details: format!("TaskId: {}", task_id.task_id()),
                    ssh_details: None
                }
            }
            Err(e) => {
                println!("Error sending shutdown message to vmm...");
                log::error!("{e}");
                return VmResponse {
                    status: FAILURE.to_string(),
                    details: e.to_string(),
                    ssh_details: None
                }
            }
        }
    }

    async fn start_vm(
        self, 
        _: context::Context,
        params: InstanceStartParams
    ) -> VmResponse {
        let task_id = if let Ok(bytes) = serde_json::to_vec(&params) {
            let mut hasher = Sha3_256::new();
            hasher.update(bytes);
            let result = hasher.finalize();
            TaskId::new(
                hex::encode(&result[..])
            )
        } else {
            return VmResponse {
                status: FAILURE.to_string(),
                details: "Unable to generate unique task id".to_string(),
                ssh_details: None,
            }
        };
        println!("attempting to shutdown VM...");
        let message = VmManagerMessage::StartInstance { params: params.clone(), sig: params.sig, task_id: task_id.clone() };
        match self.vmm_sender.send(message).await {
            Ok(_) => {
                println!("Successfully sent shutdown message to vmm...");
                return VmResponse {
                    status: PENDING.to_string(),
                    details: format!("TaskId: {}", task_id.task_id()),
                    ssh_details: None
                }
            }
            Err(e) => {
                println!("Error sending shutdown message to vmm...");
                log::error!("{e}");
                return VmResponse {
                    status: FAILURE.to_string(),
                    details: e.to_string(),
                    ssh_details: None
                }
            }
        }
    }

    async fn set_ssh_pubkey(
        self, 
        _: context::Context, 
        params: InstanceAddPubkeyParams
    ) -> VmResponse {
        let task_id = if let Ok(bytes) = serde_json::to_vec(&params) {
            let mut hasher = Sha3_256::new();
            hasher.update(bytes);
            let result = hasher.finalize();
            TaskId::new(
                hex::encode(&result[..])
            )
        } else {
            return VmResponse {
                status: FAILURE.to_string(),
                details: "Unable to generate unique task id".to_string(),
                ssh_details: None,
            }
        };

        let message = VmManagerMessage::InjectAuth { 
            params: params.clone(),
            sig: params.sig, 
            task_id: task_id.clone() 
        };

        match self.vmm_sender.send(message).await {
            Ok(_) => {
                println!("Successfully sent shutdown message to vmm...");
                return VmResponse {
                    status: PENDING.to_string(),
                    details: format!("TaskId: {}", task_id.task_id()),
                    ssh_details: None
                }
            }
            Err(e) => {
                println!("Error sending shutdown message to vmm...");
                log::error!("{e}");
                return VmResponse {
                    status: FAILURE.to_string(),
                    details: e.to_string(),
                    ssh_details: None
                }
            }
        }
    }

    async fn delete_vm(
        self,
        _: context::Context,
        params: InstanceDeleteParams
    ) -> VmResponse {
        let task_id = if let Ok(bytes) = serde_json::to_vec(&params) {
            let mut hasher = Sha3_256::new();
            hasher.update(bytes);
            let result = hasher.finalize();
            TaskId::new(
                hex::encode(&result[..])
            )
        } else {
            return VmResponse {
                status: FAILURE.to_string(),
                details: "Unable to generate unique task id".to_string(),
                ssh_details: None,
            }
        };

        let message = VmManagerMessage::DeleteInstance { params: params.clone(), sig: params.sig, task_id: task_id.clone() };

        match self.vmm_sender.send(message).await {
            Ok(_) => {
                println!("Successfully sent shutdown message to vmm...");
                return VmResponse {
                    status: PENDING.to_string(),
                    details: format!("TaskId: {}", task_id.task_id()),
                    ssh_details: None
                }
            }
            Err(e) => {
                println!("Error sending shutdown message to vmm...");
                log::error!("{e}");
                return VmResponse {
                    status: FAILURE.to_string(),
                    details: e.to_string(),
                    ssh_details: None
                }
            }
        }
    }

    async fn expose_vm_ports(
        self, 
        _: context::Context,
        params: InstanceExposePortParams
    ) -> VmResponse {
        let task_id = if let Ok(bytes) = serde_json::to_vec(&params) {
            let mut hasher = Sha3_256::new();
            hasher.update(bytes);
            let result = hasher.finalize();
            TaskId::new(
                hex::encode(&result[..])
            )
        } else {
            return VmResponse {
                status: FAILURE.to_string(),
                details: "Unable to generate unique task id".to_string(),
                ssh_details: None,
            }
        };

        let message = VmManagerMessage::ExposePorts { 
            params: params.clone(),
            sig: params.sig,
            task_id: task_id.clone() 
        };

        match self.vmm_sender.send(message).await {
            Ok(_) => {
                println!("Successfully sent shutdown message to vmm...");
                return VmResponse {
                    status: PENDING.to_string(),
                    details: format!("TaskId: {}", task_id.task_id()),
                    ssh_details: None
                }
            }
            Err(e) => {
                println!("Error sending shutdown message to vmm...");
                log::error!("{e}");
                return VmResponse {
                    status: FAILURE.to_string(),
                    details: e.to_string(),
                    ssh_details: None
                }
            }
        }
    }

    async fn get_task_status(
        self, 
        _: context::Context,
        owner: String,
        id: String 
    ) -> VmResponse {
        let address_bytes = if owner.starts_with("0x") { 
            let owner_string = &owner[2..];
            let address_bytes = match hex::decode(owner_string) {
                Ok(bytes) => {
                    bytes
                },
                Err(e) => {
                    return VmResponse {
                        status: FAILURE.to_string(),
                        details: e.to_string(),
                        ssh_details: None
                    }
                }
            };
            address_bytes
        } else {
            let address_bytes = match hex::decode(&owner) {
                Ok(bytes) => {
                    bytes
                }
                Err(e) => {
                    return VmResponse {
                        status: FAILURE.to_string(),
                        details: e.to_string(),
                        ssh_details: None
                    }
                }
            };
            address_bytes
        };

        match self.task_cache.write() {
            Ok(mut guard) => {
                let tso = guard.get(&TaskId::new(id.clone()));
                if let Some(task_status) = tso {
                    return VmResponse {
                        status: SUCCESS.to_string(),
                        details: task_status.to_string(),
                        ssh_details: None
                    }
                }
            },
            Err(_) => {}
        };

        match self.tikv_client.get(address_bytes.to_vec()).await {
            Ok(Some(account_bytes)) => {
                match serde_json::from_slice::<Account>(&account_bytes) {
                    Ok(account) => {
                        let task_id = TaskId::new(id.clone());
                        if let Some(ts) = account.get_task_status(&task_id) {
                            return VmResponse {
                                status: SUCCESS.to_string(),
                                details: format!("Task {} status: {:?}", &id, ts),
                                ssh_details: None
                            }
                        } else {
                            return VmResponse {
                                status: FAILURE.to_string(),
                                details: format!("Unable to find Task {}", &id),
                                ssh_details: None
                            }
                        }
                    }
                    Err(e) => return VmResponse {
                        status: FAILURE.to_string(),
                        details: format!("unable to deserialize bytes for Account for owner {owner}: {e}"),
                        ssh_details: None
                    }
                }
            }
            Ok(None) => {
                return VmResponse {
                    status: FAILURE.to_string(),
                    details: format!("unable to find account for owner {owner}"), 
                    ssh_details: None
                }
            }
            Err(e) => return VmResponse {
                status: FAILURE.to_string(),
                details: format!("error acquiring account for owner {owner} from state: {e}"),
                ssh_details: None
            }
        }
    }

    async fn get_ssh_details(
        self,
        _: context::Context,
        params: InstanceGetSshDetails
    ) -> VmResponse {
        let public_ipaddr = match reqwest::get("https://api.ipify.org").await {
            Ok(resp) => {
                match resp.text().await {
                    Ok(ip) => ip,
                    Err(e) => {
                        return VmResponse {
                            status: FAILURE.to_string(),
                            details: format!("Unable to acquire a host's public ip: {}", e),
                            ssh_details: None
                        }
                    }
                }
            }
            Err(e) => {
                return VmResponse {
                    status: FAILURE.to_string(),
                    details: format!("Unable to acquire a host's public ip: {}", e),
                    ssh_details: None
                }
            }
        };

        let owner = if params.owner.starts_with("0x") {
            let owner = &params.owner[2..];
            match hex::decode(&owner) {
                Ok(owner) => {
                    let mut owner_addr = [0u8; 20];
                    owner_addr.copy_from_slice(&owner);
                    owner_addr
                },
                Err(e) => return VmResponse {
                    status: FAILURE.to_string(),
                    details: format!("Unable to recover owner address: {e}"),
                    ssh_details: None,
                }
            }
        } else {
            match hex::decode(&params.owner) {
                Ok(owner) => {
                    let mut owner_addr = [0u8; 20];
                    owner_addr.copy_from_slice(&owner);
                    owner_addr
                }
                Err(e) => return VmResponse {
                    status: FAILURE.to_string(),
                    details: format!("Unable to recover owner address: {e}"),
                    ssh_details: None,
                }
            }
        };
        let namespace = recover_namespace(owner, &params.name);

        let instance: Instance = match get_instance(namespace.clone(), self.tikv_client.clone()).await {
            Ok(instance) => instance,
            Err(e) => return VmResponse {
                status: FAILURE.to_string(),
                details: format!("Unable to acquire the instance {}: {}", &namespace.inner(), e),
                ssh_details: None
            }
        };

        let port = match instance.port_map().get(&22).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to acquire port mapping for instance port 22"
            )
        ) {
            Ok(port) => port,
            Err(e) => return VmResponse {
                status: FAILURE.to_string(),
                details: e.to_string(),
                ssh_details: None,
            }
        };

        let ssh_details = SshDetails {
            ip: public_ipaddr,
            port: *port 
        };

        return VmResponse {
            status: SUCCESS.to_string(),
            details: format!("SSH Details Acquired"),
            ssh_details: Some(ssh_details)
        }
    }
}

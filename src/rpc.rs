use serde_json::json;
use tarpc::context;
use crate::{vmm::{InstanceCreateParams, VmResponse, InstanceStartParams, InstanceStopParams, InstanceAddPubkeyParams, InstanceDeleteParams, VmManagerMessage, SUCCESS, FAILURE, PENDING, InstanceExposePortParams}, account::TaskId};
use std::process::Command;
use tokio::sync::mpsc::Sender;
use serde::{Serialize, Deserialize};
use sha3::{Digest, Sha3_256};

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
            auth: params.pubkey, 
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
}

pub fn get_container_config(name: String) -> std::io::Result<std::fs::File> {
    let path = format!("/var/lib/lxc/{}/config", &name);
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
}

pub fn return_success(
    details: String,
    ssh_details: Option<String>
) -> VmResponse {
    return VmResponse {
        status: "Success".to_string(),
        details,
        ssh_details
    }
}

pub fn return_failure(
    details: String
) -> VmResponse {
    return VmResponse {
        status: "Failure".to_string(),
        details,
        ssh_details: None
    }
}

impl VmmServer {
    pub fn expose_instance(
        mut self,
        internal_port: u16,
        instance_ip: String,
    ) -> std::io::Result<u16> {
        let instance_port = self.assign_unique_port()?;
        update_iptables(&instance_ip, instance_port, internal_port)?;
        update_ufw(instance_port)?;
        Ok(instance_port)
    }

    fn assign_unique_port(&mut self) -> std::io::Result<u16> {
        if self.port < MAX_PORT {
            self.port += 1;
        } else {
            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Max port reached, unable to assing next port"
                )
            )
        }

        Ok(self.port)
    }
}

fn update_ufw(instance_port: u16) -> std::io::Result<String> {
    let output = Command::new("sudo")
        .arg("ufw")
        .arg("allow")
        .arg("in")
        .arg(
            format!(
                "{}/tcp",
                instance_port
            )
        )
        .output()?;

    match output.status.success() {
        true => {
            match std::str::from_utf8(&output.stdout) {
                Ok(deets) => {
                    return Ok(deets.to_string())
                }
                Err(e) => {
                    return Ok(e.to_string())
                }
            }
        }
        false => {
            match std::str::from_utf8(&output.stderr) {
                Ok(deets) => {
                    return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            deets.to_string()
                        )
                    )
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
        }
    }
}

fn update_iptables(
    instance_ip: &str, 
    instance_port: u16, 
    internal_port: u16
) -> std::io::Result<String> {
    let output = Command::new("sudo")
        .arg("iptables")
        .arg("-t")
        .arg("nat")
        .arg("-A")
        .arg("PREROUTING")
        .arg("-p")
        .arg("tcp")
        .arg("--dport")
        .arg(&instance_port.to_string())
        .arg("-j")
        .arg("DNAT")
        .arg("--to-destination")
        .arg(
            &format!(
                "{}:{}",
                instance_ip,
                internal_port
            )
        )
        .output()?;

    match output.status.success() {
        true => {
            match std::str::from_utf8(&output.stdout) {
                Ok(deets) => {
                    return Ok(deets.to_string())
                }
                Err(e) => {
                    return Ok(e.to_string())
                }
            }
        }
        false => {
            match std::str::from_utf8(&output.stderr) {
                Ok(deets) => {
                    return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            deets.to_string()
                        )
                    )
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
        }
    }
}

impl From<std::io::Error> for VmResponse {
    fn from(value: std::io::Error) -> Self {
        VmResponse { 
            status: "Failure".to_string(),
            details: value.to_string(), 
            ssh_details: None 
        }
    }
}

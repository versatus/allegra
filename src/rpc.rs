use tarpc::context;
use crate::vmm::{InstanceCreateParams, VmResponse, InstanceStartParams, InstanceStopParams, InstanceAddPubkeyParams, InstanceDeleteParams};
use std::process::Command;
use serde::{Serialize, Deserialize};

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
    //TODO: Implement all LXC Commands
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmmServer {
    pub network: String,
    pub ipv40: u8,
    pub ipv41: u8,
    pub ipv42: u8,
    pub ipv43: u8,
    pub port: u16,
}

impl Vmm for VmmServer {
    async fn create_vm(self, _: context::Context, params: InstanceCreateParams) -> VmResponse {
        let output = Command::new("lxc")
            .arg("launch")
            .arg(
                &format!(
                    "{}:{}",
                    params.distro,
                    params.version
                )
            )
            .arg(
                &format!(
                    "{}",
                    params.name
                )
            )
            .arg("--vm")
            .arg("-t")
            .arg(&params.vmtype.to_string())
            .arg("--network")
            .arg(&self.network.clone())
            .output();

        let output = match output {
            Ok(o) => o,
            Err(e) => {
                return VmResponse {
                    status: "Failure".to_string(),
                    details: format!(
                        "Failed to create VM: {}",
                        e
                    ),
                    ssh_details: None
                }
            }
        };

        if output.status.success() {
            //TODO: handle error cases gracefully
            let instance_port = self.expose_instance(
                22
            ).expect(
                "unable to acquire port"
            );

            match std::str::from_utf8(&output.stdout) {
                //TODO: Modularize the match arm logic
                Ok(deets) => {
                    return VmResponse {
                        status: "Success".to_string(),
                        details: format!(
                            "VM created successfully: {}",
                            deets
                        ),
                        ssh_details: Some(
                            format!(
                                "ssh -p {} root@hostname",
                                instance_port
                            )
                        ) 
                    }
                }
                Err(e) => {
                    return VmResponse {
                        status: "Success".to_string(),
                        details: format!(
                            "VM created successfully, but stdout could not be read: {}", 
                            e
                        ),
                        ssh_details: Some(
                            format!(
                                "ssh -p {} root@hostname",
                                instance_port
                            )
                        )
                    }
                }
            }
        } else {
            match std::str::from_utf8(&output.stderr) {
                Ok(deets) => {
                    return VmResponse {
                        status: "Failure".to_string(),
                        details: format!(
                            "VM unable to be created: {}",
                            deets
                        ),
                        ssh_details: None
                    }
                }
                Err(e) => {
                    return VmResponse {
                        status: "Failure".to_string(),
                        details: format!(
                            "VM unable to be created and stderr could not be read: {}",
                            e
                        ),
                        ssh_details: None
                    }
                }
            }
        }
    }

    async fn shutdown_vm(self, _: context::Context, params: InstanceStopParams) -> VmResponse {
        dbg!("received request to shutdown vm: {}", &params.name);
        let output = match Command::new("lxc")
            .arg("stop")
            .arg(&params.name)
            .output() {
                Ok(o) => {
                    dbg!(
                        "received output: {:?}",
                        &o
                    );
                    o
                }
                Err(e) => {
                    return VmResponse {
                        status: "Failure".to_string(),
                        details: format!(
                            "Unable to shutdown vm: {}",
                            e
                        ),
                        ssh_details: None,
                    }
                }
        };

        if output.status.success() {
            match std::str::from_utf8(&output.stdout) {
                Ok(deets) => {
                    println!("output success - {}", &deets);
                    VmResponse {
                        status: "Success".to_string(),
                        details: format!(
                            "Successfully shutdown vm: {} - {}",
                            &params.name,
                            deets
                        ),
                        ssh_details: None
                    }
                }
                Err(e) => {
                    println!("output success - no deets");
                    VmResponse {
                        status: "Success".to_string(),
                        details: format!(
                            "Successfully shutdown vm: {} - Unable to provide details: {}",
                            &params.name,
                            e
                        ),
                        ssh_details: None
                    }
                }
            }
        } else {
            match std::str::from_utf8(&output.stderr)  {
                Ok(deets) => {
                    println!("output failure - {}", &deets);
                    VmResponse {
                        status: "Failure".to_string(),
                        details: format!(
                            "Failed to shutdown vm: {} - {}",
                            &params.name,
                            deets
                        ),
                        ssh_details:  None
                    }
                }
                Err(e) => {
                    println!("output failure - {}", &e);
                    VmResponse {
                        status: "Failure".to_string(),
                        details: format!(
                            "Failed to shutdown vm: {} - Unable to provide details: {}",
                            &params.name, 
                            e
                        ),
                        ssh_details: None
                    }
                }
            }
        }
    }

    async fn start_vm(
        self, 
        _: context::Context,
        params: InstanceStartParams
    ) -> VmResponse {
        let mut command = Command::new("lxc");
        command.arg("start").arg(&params.name);

        if params.console {
            command.arg("--console");
        }

        if params.stateless {
            command.arg("--stateless");
        }

        let output = match command.output() {
            Ok(o) => o,
            Err(e) => {
                return return_failure(
                    format!(
                        "Failed to start vm: {} - {}", &params.name, e
                    )
                )
            }
        };

        if output.status.success() {
            match std::str::from_utf8(&output.stdout) {
                Ok(deets) => {
                    return_success(
                        format!(
                            "Successfully started vm: {} - {}", &params.name, &deets
                        ), None
                    )
                }
                Err(e) => {
                    return_success(
                        format!(
                            "Successfully started vm: {} - Unable to acquire details: {}",
                            &params.name, 
                            &e
                        ), None
                    )
                }
            }
        } else {
            match std::str::from_utf8(&output.stderr) {
                Ok(deets) => {
                    return_failure(
                        format!(
                            "Unable to start vm: {} - {}",
                            &params.name,
                            &deets
                        )
                    )
                }
                Err(e) => {
                    return_failure(
                        format!(
                            "Unable to start vm: {} - Unable to provide details: {}", 
                            &params.name,
                            &e
                        )
                    )
                }
            }
        }
    }

    async fn set_ssh_pubkey(
        self, 
        _: context::Context, 
        params: InstanceAddPubkeyParams
    ) -> VmResponse {
        let echo = format!(
            r#""echo '{}' >> ~/.ssh/authorized_keys""#,
            params.pubkey
        );
        let output = match Command::new("lxc")
            .arg("exec")
            .arg(&params.name)
            .arg("--")
            .arg("bash")
            .arg("-c")
            .arg(&echo)
            .output() {
                Ok(o) => o,
                Err(e) => {
                    return return_failure(
                        format!(
                            "Unable to execute command in vm: {} - {}",
                            &params.name,
                            &e
                        )
                    );
                }
            };

        if output.status.success() {
            match std::str::from_utf8(&output.stdout) {
                Ok(deets) => return return_success(
                    format!(
                        "Successfully added ssh key to vm: {} - {}",
                        &params.name,
                        &deets
                    ),
                    None
                ),
                Err(e) => return return_success(
                    format!(
                        "Successfully added ssh key to vm: {} - Unable to provide details: {}",
                        &params.name,
                        &e
                    ), 
                    None
                )
            }
        } else {
            match std::str::from_utf8(&output.stderr) {
                Ok(deets) => return return_failure(
                    format!(
                        "Unable to add ssh key to vm: {} - {}",
                        &params.name, 
                        &deets
                    )
                ),
                Err(e) => return return_failure(
                    format!(
                        "Unable to add ssh key to vm: {} - Unable to provide details: {}",
                        &params.name,
                        &e
                    )
                )
            }
        }
    }

    async fn delete_vm(
        self,
        _: context::Context,
        params: InstanceDeleteParams
    ) -> VmResponse {
        let mut command = Command::new("lxc");
        command.arg("delete").arg(&params.name);

        if params.interactive {
            command.arg("--interactive");
        }

        if params.force {
            command.arg("--force");
        }

        let output = match command.output() {
            Ok(o) => o,
            Err(e) => {
                return return_failure(
                    format!(
                        "Unable to delete vm: {} - {}",
                        &params.name,
                        &e
                    )
                )
            }
        };

        if output.status.success() {
            match std::str::from_utf8(&output.stdout) {
                Ok(deets) => return return_success(
                    format!(
                        "Successfully shutdown vm: {} - {}",
                        &params.name,
                        &deets
                    ), 
                    None
                ),
                Err(e) => return return_success(
                    format!(
                        "Successfully shutdown vm: {} - Unable to provide details: {}",
                        &params.name,
                        &e
                    ), 
                    None
                )
            }
        } else {
            match std::str::from_utf8(&output.stderr) {
                Ok(deets) => return return_failure(
                    format!(
                        "Unable to shutdown vm: {} - {}",
                        &params.name, 
                        &deets
                    )
                ),
                Err(e) => return return_failure(
                    format!(
                        "Unable to shutdown vm: {} - Unable to provide details: {}",
                        &params.name,
                        &e
                    )
                )
            }
        }

    }
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
        internal_port: u16
    ) -> std::io::Result<u16> {
        let instance_ip = self.assign_internal_ip()?;
        let instance_port = self.assign_unique_port()?;
        Self::update_iptables(&instance_ip, instance_port, internal_port)?;
        Self::update_ufw(instance_port)?;
        Ok(instance_port)
    }

    fn update_iptables(
        instance_ip: &str, 
        instance_port: u16, 
        internal_port: u16
    ) -> std::io::Result<String> {
        let output = Command::new("iptables")
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

    fn update_ufw(instance_port: u16) -> std::io::Result<String> {
        let output = Command::new("ufw")
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

    fn assign_internal_ip(&mut self) -> std::io::Result<String> {
        //TODO(asmith): Replace with constants
        if self.ipv41 == 255 &&
            self.ipv42 == 255 &&
                self.ipv43 == 255 {
                    return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "local ip address range filled, local cannot provision more ip addresses"
                        )
                    )
        }
        if self.ipv43 < 255 {
            self.ipv43 += 1;
        } else {
            if self.ipv42 < 255 {
                self.ipv43 = 0;
                self.ipv42 += 1;
            } else {
                if self.ipv43 < 255 {
                    self.ipv42 = 0;
                    self.ipv43 += 1;
                }
            }
        }

        return Ok(
            format!(
                "{}.{}.{}.{}",
                &self.ipv40,
                &self.ipv41,
                &self.ipv42,
                &self.ipv43
            )
        )
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

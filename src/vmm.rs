use crate::vm_types::VmType;
use clap::Args;
use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Serialize, Deserialize};
use tokio::task::JoinHandle;

pub const LOWEST_PORT: u16 = 2222;
pub const HIGHEST_PORT: u16 = 65535;
pub static DEFAULT_NETWORK: &'static str = "lxdbr0";
pub static SUCCESS: &'static str = "SUCCESS";
pub static FAILURE: &'static str = "FAILURE";
pub static PENDING: &'static str = "PENDING";

#[derive(Clone, Debug, Serialize, Deserialize, Args)]
pub struct InstanceCreateParams {
    #[arg(long, short)]
    pub name: String,
    #[arg(long, short)]
    pub distro: String,
    #[arg(long, short)]
    pub version: String,
    #[arg(long, short='t')]
    pub vmtype: VmType,
}

#[derive(Clone, Debug, Serialize, Deserialize, Args)]
pub struct InstanceStartParams {
    #[arg(long, short)]
    pub name: String,
    #[arg(long, short)]
    pub console: bool,
    #[arg(long, short)]
    pub stateless: bool
}

#[derive(Clone, Debug, Serialize, Deserialize, Args)]
pub struct InstanceStopParams {
    #[arg(long, short)]
    pub name: String 
}

#[derive(Clone, Debug, Serialize, Deserialize, Args)]
pub struct InstanceAddPubkeyParams {
    #[arg(long, short)]
    pub name: String,
    #[arg(long, short)]
    pub pubkey: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Args)]
pub struct InstanceDeleteParams {
    #[arg(long, short)]
    pub name: String,
    #[arg(long, short)]
    pub force: bool,
    #[arg(long, short)]
    pub interactive: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Args)]
pub struct InstanceExposePortParams {
    #[arg(long, short)]
    pub name: String,
    #[arg(long, short)]
    pub port: String,
}


#[derive(Debug)]
pub enum VmManagerMessage {
    NewInstance {
        params: InstanceCreateParams,
        owner: String,
        tx: tokio::sync::oneshot::Sender<VmResponse>
    },
    StopInstance {
        name: String,
        sig: String,
        tx: tokio::sync::oneshot::Sender<VmResponse>
    },
    DeleteInstance {
        name: String,
        sig: String,
        tx: tokio::sync::oneshot::Sender<VmResponse>
    },
    InjectAuth {
        name: String,
        sig: String,
        auth: String,
        tx: tokio::sync::oneshot::Sender<VmResponse>
    },
    StartInstance {
        name: String,
        sig: String,
        tx: tokio::sync::oneshot::Sender<VmResponse>
    },
    ExposePorts {
        name: String,
        sig: String,
        port: Vec<u16>,
        tx: tokio::sync::oneshot::Sender<VmResponse>,
    }
}

pub struct VmManager {
    network: String,
    next_port: u16,
    handles: FuturesUnordered<JoinHandle<std::io::Result<()>>>,
    vmlist: VmList,
    state_client: tikv_client::RawClient,
}

impl VmManager {
    pub async fn new<S: Into<String>>(pd_endpoints: Vec<S>, config: Option<tikv_client::Config>) -> std::io::Result<Self> {
        let network = DEFAULT_NETWORK.to_string();
        let next_port = LOWEST_PORT;
        let handles = FuturesUnordered::new();
        let vmlist = match std::process::Command::new("lxc")
            .args(["list", "--format", "json"])
            .output() {
                Ok(output) => {
                    if output.status.success() {
                        let vmlist_str = std::str::from_utf8(&output.stdout).map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e.to_string()
                            )
                        })?;
                        let vmlist = serde_json::from_str(vmlist_str).map_err(|e| { std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e.to_string()
                            )
                        })?;
                        vmlist
                    } else {
                        let err = std::str::from_utf8(&output.stderr).map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e.to_string()
                            )
                        })?.to_string();
                        return Err(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                err
                            )
                        )
                    }
                },
                Err(e) => return Err(e)
            };

        let state_client = if let Some(conf) = config {
            tikv_client::RawClient::new_with_config(pd_endpoints, conf).await.map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string()
                )
            })?
        } else {
            tikv_client::RawClient::new(pd_endpoints).await.map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string()
                )
            })?
        };

        Ok(Self {
            network: network.to_string(),
            next_port,
            handles,
            vmlist,
            state_client
        })
    }

    pub async fn run(
        mut self,
        mut message_rx: tokio::sync::mpsc::Receiver<VmManagerMessage>,
        mut stop_rx: tokio::sync::mpsc::Receiver<()>
    ) -> std::io::Result<()> {
        loop {
            tokio::select! {
                message = message_rx.recv() => {
                    if let Some(m) = message {
                        match self.handle_vmm_message(m).await {
                            Err(e) => {
                                log::error!("{e}");
                            }
                            _ => {}
                        }
                    }
                },
                stop = stop_rx.recv() => {
                    if let Some(_) = stop {
                        break
                    }
                },
                _ = self.handles.next() => {}
            }
        }

        return Ok(())
    }

    async fn handle_vmm_message(&mut self, message: VmManagerMessage) -> std::io::Result<()> {
        match message {
            VmManagerMessage::NewInstance { params, owner, tx } => {
                return self.launch_instance(params, owner, tx).await
            }
            VmManagerMessage::StartInstance { name, sig, tx } => {
                return self.start_instance(name, sig, tx).await
            }
            VmManagerMessage::InjectAuth { name, sig, auth, tx } => {
                return self.inject_authorization(name, sig, auth, tx).await
            }
            VmManagerMessage::StopInstance { name, sig, tx } => {
                return self.stop_instance(name, sig, tx).await
            }
            VmManagerMessage::DeleteInstance { name, sig, tx } => {
                return self.delete_instance(name, sig, tx).await
            }
            VmManagerMessage::ExposePorts { name, sig, port, tx } => {
                return self.expose_ports(name, sig, port, tx).await
            }
        }
    }

    async fn start_instance(
        &mut self,
        name: String,
        sig: String,
        tx: tokio::sync::oneshot::Sender<VmResponse>
    ) -> std::io::Result<()> {
        todo!()
    }

    async fn inject_authorization(
        &mut self,
        name: String, 
        sig: String, 
        auth: String, 
        tx: tokio::sync::oneshot::Sender<VmResponse>
    ) -> std::io::Result<()> {
        todo!()
    }

    async fn stop_instance(
        &mut self,
        name: String,
        sig: String,
        tx: tokio::sync::oneshot::Sender<VmResponse>
    ) -> std::io::Result<()> {
        todo!()
    }

    async fn delete_instance(
        &mut self,
        name: String,
        sig: String,
        tx: tokio::sync::oneshot::Sender<VmResponse>
    ) -> std::io::Result<()> {
        todo!()
    }

    async fn expose_ports(
        &mut self,
        name: String,
        sig: String,
        port: Vec<u16>,
        tx: tokio::sync::oneshot::Sender<VmResponse>
    ) -> std::io::Result<()> {
        todo!()
    }

    fn handle_create_output_success_and_response(
        &mut self,
        output: &std::process::Output,
        tx: tokio::sync::oneshot::Sender<VmResponse>,
        params: InstanceCreateParams,
    ) -> std::io::Result<()> {
        let details = std::str::from_utf8(&output.stdout).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?.to_string();

        println!("Started instance with name: {}", params.name);
        let resp = VmResponse { status: SUCCESS.to_string(), details, ssh_details: None };

        tx.send(resp).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{e:#?}")
            )
        })?;

        let next_port = self.next_port;

        let handle: JoinHandle<std::io::Result<()>> = tokio::spawn(
            async move {
                println!("Sleeping for 2 minutes to allow instance to fully launch...");
                tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
                println!("Updating iptables...");
                update_iptables(params.name, next_port, 22)?;
                Ok(())
        });

        self.handles.push(handle);

        return Ok(())
    }

    fn handle_create_output_failure_and_response(
        &mut self,
        output: &std::process::Output,
        tx: tokio::sync::oneshot::Sender<VmResponse>,
    ) -> std::io::Result<()> {
        let details = std::str::from_utf8(&output.stderr).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{e:#?}")
            )
        })?.to_string();

        println!("Failed to start instance");
        let resp = VmResponse { status: FAILURE.to_string(), details, ssh_details: None };

        tx.send(resp).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{e:#?}")
            )
        })?;

        return Ok(())
    }

    fn handle_create_output_and_response(
        &mut self,
        output: std::process::Output,
        tx: tokio::sync::oneshot::Sender<VmResponse>,
        params: InstanceCreateParams
    ) -> std::io::Result<()> {
        if output.status.success() {
            self.handle_create_output_success_and_response(&output, tx, params)
        } else {
            self.handle_create_output_failure_and_response(&output, tx)
        }
    }

    pub async fn launch_instance(
        &mut self,
        params: InstanceCreateParams,
        owner: String,
        tx: tokio::sync::oneshot::Sender<VmResponse>
    ) -> std::io::Result<()> {
        println!("Attempting to start instance...");
        let output = std::process::Command::new("lxc")
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
            .output()?;


        self.handle_create_output_and_response(output, tx, params)
    }
}

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

fn handle_update_iptables_output_failulre(output: &std::process::Output) -> std::io::Result<()> {
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
        return handle_update_iptables_output_failulre(output) 
    }
}

pub fn update_iptables(
    name: String,
    next_port: u16,
    internal_port: u16
) -> std::io::Result<()> {
    let instance_ip = get_instance_ip(&name)?;
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


    Ok(())
}

fn handle_update_ufw_output_failure(output: &std::process::Output) -> std::io::Result<()> {
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

fn handle_update_ufw_output(output: &std::process::Output) -> std::io::Result<()> {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct VmResponse {
    pub status: String,
    pub details: String,
    pub ssh_details: Option<String>
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VmInfo {
    name: String,
    description: String,
    status: String,
    status_code: i32,
    created_at: String,
    last_used_at: String,
    location: String,
    #[serde(rename = "type")]
    vm_type: String,
    project: String,
    architecture: String,
    ephemeral: bool,
    stateful: bool,
    profiles: Vec<String>,
    config: VmConfig,
    devices: VmDevices,
    expanded_config: VmConfig,
    expanded_devices: VmDevices,
    backups: Option<Vec<String>>,
    state: VmState,
    snapshots: Option<Vec<String>>,
}

impl VmInfo {
    pub fn name(&self) -> String {
        self.name.clone()
    }
    
    pub fn description(&self) -> String {
        self.description.clone()
    }

    pub fn status(&self) -> String {
        self.status.clone()
    }

    pub fn status_code(&self) -> i32 {
        self.status_code
    }

    pub fn created_at(&self) -> String {
        self.created_at.clone()
    }

    pub fn last_used_at(&self) -> String { 
        self.last_used_at.clone()
    }

    pub fn location(&self) -> String { 
        self.location.clone()
    }

    pub fn vm_type(&self) ->  String { 
        self.vm_type.clone()
    }

    pub fn project(&self) -> String { 
        self.project.clone()
    }

    pub fn architecture(&self) -> String { 
        self.architecture.clone()
    }

    pub fn ephemeral(&self) -> bool { 
        self.ephemeral
    }

    pub fn stateful(&self) ->  bool { 
        self.stateful
    }

    pub fn profiles(&self) -> Vec<String> { 
        self.profiles.clone()
    }

    pub fn config(&self) -> VmConfig { 
        self.config.clone()
    }

    pub fn devices(&self) -> VmDevices { 
        self.devices.clone()
    }

    pub fn expanded_config(&self) -> VmConfig { 
        self.expanded_config.clone()
    }

    pub fn expanded_devices(&self) -> VmDevices { 
        self.expanded_devices.clone()
    }

    pub fn backups(&self) -> Option<Vec<String>> { 
        self.backups.clone()
    }

    pub fn state(&self) -> VmState { 
        self.state.clone()
    }

    pub fn snapshots(&self) -> Option<Vec<String>> { 
        self.snapshots.clone()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VmConfig {
    // Add all the necessary fields from the "config" and "expanded_config" objects
    // For example:
    #[serde(rename = "image.architecture")]
    image_architecture: String,
    #[serde(rename = "image.description")]
    image_description: String,
    #[serde(rename = "image.label")]
    image_label: Option<String>,
    #[serde(rename = "image.os")]
    image_os: String,
    #[serde(rename = "image.release")]
    image_release: String,
    #[serde(rename = "image.serial")]
    image_serial: String,
    #[serde(rename = "image.type")]
    image_type: String,
    #[serde(rename = "image.version")]
    image_version: Option<String>,
    #[serde(rename = "limits.cpu")]
    limits_cpu: Option<String>,
    #[serde(rename = "limits.memory")]
    limits_memory: Option<String>,
    #[serde(rename = "volatile.base_image")]
    volatile_base_image: String,
    #[serde(rename = "volatile.cloud-init.instance-id")]
    volatile_cloud_init_instance_id: String,
    #[serde(rename = "volatile.eth0.host_name")]
    volatile_eth0_host_name: Option<String>,
    #[serde(rename = "volatile.eth0.hwaddr")]
    volatile_eth0_hwaddr: String,
    #[serde(rename = "volatile.last_state.power")]
    volatile_last_state_power: String,
    #[serde(rename = "volatile.uuid")]
    volatile_uuid: String,
    #[serde(rename = "volatile.uuid.generation")]
    volatile_uuid_generation: String,
    #[serde(rename = "volatile.vsock_id")]
    volatile_vsock_id: String,
}

impl VmConfig {
    pub fn image_architecture(&self) -> String {
        self.image_architecture.clone()
    }

    pub fn image_description(&self) -> String {
        self.image_description.clone()
    }

    pub fn image_label(&self) -> Option<String> {
        self.image_label.clone()
    }

    pub fn image_os(&self) -> String {
        self.image_os.clone()
    }

    pub fn image_release(&self) -> String {
        self.image_release.clone()
    }

    pub fn image_serial(&self) -> String {
        self.image_serial.clone()
    }

    pub fn image_type(&self) -> String {
        self.image_type.clone()
    }

    pub fn image_version(&self) -> Option<String> {
        self.image_version.clone()
    }

    pub fn limits_cpu(&self) -> Option<String> {
        self.limits_cpu.clone()
    }

    pub fn limits_memory(&self) -> Option<String> {
        self.limits_memory.clone()
    }

    pub fn volatile_base_image(&self) -> String {
        self.volatile_base_image.clone()
    }

    pub fn volatile_cloud_init_instance_id(&self) -> String {
        self.volatile_cloud_init_instance_id.clone()
    }

    pub fn volatile_eth0_host_name(&self) -> Option<String> {
        self.volatile_eth0_host_name.clone()
    }

    pub fn volatile_eth0_hwaddr(&self) -> String {
        self.volatile_eth0_hwaddr.clone()
    }

    pub fn volatile_last_state_power(&self) -> String {
        self.volatile_last_state_power.clone()
    }

    pub fn volatile_uuid(&self) -> String {
        self.volatile_uuid.clone()
    }

    pub fn volatile_uuid_generation(&self) -> String {
        self.volatile_uuid_generation.clone()
    }

    pub fn volatile_vsock_id(&self) -> String {
        self.volatile_vsock_id.clone()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VmDevices {
    eth0: Option<VmDevice>,
    root: Option<VmDevice>,
}

impl VmDevices {
    pub fn eth0(&self) -> Option<VmDevice> {
        self.eth0.clone()
    }

    pub fn root(&self) -> Option<VmDevice> {
        self.root.clone()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VmDevice {
    name: Option<String>,
    network: Option<String>,
    #[serde(rename = "type")]
    device_type: String,
    path: Option<String>,
    pool: Option<String>,
}

impl VmDevice {
    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }
    
    pub fn network(&self) -> Option<String> {
        self.network.clone()
    }

    pub fn device_type(&self) -> String {
        self.device_type.clone()
    }

    pub fn path(&self) -> Option<String> {
        self.path.clone()
    }

    pub fn pool(&self) -> Option<String> {
        self.pool.clone()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VmState {
    status: String,
    status_code: i32,
    disk: VmDisk,
    memory: VmMemory,
    network: Option<VmNetwork>,
    pid: i32,
    processes: i32,
    cpu: VmCpu,
}

impl VmState {
    pub fn status(&self) -> String {
        self.status.clone()
    }

    pub fn status_code(&self) -> i32 {
        self.status_code
    }

    pub fn disk(&self) -> VmDisk {
        self.disk.clone()
    }

    pub fn memory(&self) -> VmMemory {
        self.memory.clone()
    }

    pub fn network(&self) -> Option<VmNetwork> {
        self.network.clone()
    }

    pub fn pid(&self) -> i32 {
        self.pid
    }

    pub fn processes(&self) -> i32 {
        self.processes
    }

    pub fn cpu(&self) -> VmCpu {
        self.cpu.clone()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VmDisk {
    root: VmDiskUsage,
}

impl VmDisk {
    pub fn root(&self) -> VmDiskUsage {
        self.root.clone()
    }
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VmDiskUsage {
    usage: u64,
    total: u64,
}

impl VmDiskUsage {
    pub fn usage(&self) -> u64 {
        self.usage
    }
    
    pub fn total(&self) -> u64 {
        self.total
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VmMemory {
    usage: u64,
    usage_peak: u64,
    total: u64,
    swap_usage: u64,
    swap_usage_peak: u64,
}

impl VmMemory {
    pub fn usage(&self) -> u64 {
        self.usage
    }
    pub fn usage_peak(&self) -> u64 {
        self.usage_peak
    }
    pub fn total(&self) -> u64 {
        self.total
    }
    pub fn swap_usage(&self) -> u64 {
        self.swap_usage
    }
    pub fn swap_usage_peak(&self) -> u64 {
        self.swap_usage_peak
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VmNetwork {
    enp5s0: Option<VmNetworkInterface>,
    lo: Option<VmNetworkInterface>,
}

impl VmNetwork {
    pub fn enp5s0(&self) -> Option<VmNetworkInterface> {
        self.enp5s0.clone()
    }

    pub fn lo(&self) -> Option<VmNetworkInterface> {
        self.lo.clone()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VmNetworkInterface {
    addresses: Vec<VmAddress>,
    counters: VmCounters,
    hwaddr: String,
    host_name: String,
    mtu: i32,
    state: String,
    #[serde(rename = "type")]
    interface_type: String,
}

impl VmNetworkInterface {

    pub fn addresses(&self) -> Vec<VmAddress> {
        self.addresses.clone()
    }

    pub fn counters(&self) -> VmCounters {
        self.counters.clone()
    }

    pub fn hwaddr(&self) -> String {
        self.hwaddr.clone()
    }

    pub fn host_name(&self) -> String {
        self.host_name.clone()
    }

    pub fn mtu(&self) -> i32 {
        self.mtu
    }

    pub fn state(&self) -> String {
        self.state.clone()
    }

    pub fn interface_type(&self) -> String {
        self.interface_type.clone()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VmAddress {
    family: String,
    address: String,
    netmask: String,
    scope: String,
}

impl VmAddress {
    pub fn family(&self) -> String {
        self.family.clone()
    }

    pub fn address(&self) -> String {
        self.address.clone()
    }

    pub fn netmask(&self) -> String {
        self.netmask.clone()
    }

    pub fn scope(&self) -> String {
        self.scope.clone()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VmCounters {
    bytes_received: u64,
    bytes_sent: u64,
    packets_received: u64,
    packets_sent: u64,
    errors_received: u64,
    errors_sent: u64,
    packets_dropped_outbound: u64,
    packets_dropped_inbound: u64,
}

impl VmCounters {
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received
    }
    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent
    }
    pub fn packets_received(&self) -> u64 {
        self.packets_received
    }
    pub fn packets_sent(&self) -> u64 {
        self.packets_sent
    }
    pub fn errors_received(&self) -> u64 {
        self.errors_received
    }
    pub fn errors_sent(&self) -> u64 {
        self.errors_sent
    }
    pub fn packets_dropped_outbound(&self) -> u64 {
        self.packets_dropped_outbound
    }
    pub fn packets_dropped_inbound(&self) -> u64 {
        self.packets_dropped_inbound
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VmCpu {
    usage: u64,
}

impl VmCpu {
    pub fn usage(&self) -> u64 {
        self.usage
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VmList {
    vms: Vec<VmInfo>
}

impl VmList {
    pub fn vms(&self) -> Vec<VmInfo> {
        self.vms.clone()
    }

    pub fn get(&self, name: &str) -> Option<VmInfo> {
        let mut info_list = self.vms.clone().into_iter().filter(|info| {
            info.name() == name
        }).collect::<Vec<VmInfo>>();
        info_list.pop()
    }
}

mod test {
    
    #[test]
    fn test_deserialize_vm_info() {
        let output = std::process::Command::new("lxc")
            .arg("list")
            .arg("--format")
            .arg("json")
            .output().unwrap();

        if output.status.success() {
            let map = std::str::from_utf8(&output.stdout).unwrap();
            println!("{map:#?}");
            let info: Vec<VmInfo> = serde_json::from_str(map).unwrap(); 

            println!("{info:#?}");

        }
    }
}

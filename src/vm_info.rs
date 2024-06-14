use serde::{Serialize, Deserialize};

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
pub struct SshDetails {
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VmResponse {
    pub status: String,
    pub details: String,
    pub ssh_details: Option<SshDetails>
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
    volatile_eth0_hwaddr: Option<String>,
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

    pub fn volatile_eth0_hwaddr(&self) -> Option<String> {
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

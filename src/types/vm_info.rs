use rayon::iter::{ParallelIterator, IntoParallelIterator};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use getset::{Getters, Setters, MutGetters};

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


#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct VmInfo {
    // Basic information
    name: String,
    uuid: String,
    id: Option<u32>,
    
    // State
    state: DomainState,
    state_reason: i32,
    
    // Resources
    memory: MemoryInfo,
    vcpus: VCPUInfo,
    cpu_time: u64,
    
    // Configuration
    autostart: bool,
    persistent: bool,
    
    // OS and architecture
    os_type: String,
    os_arch: String,
    
    // Devices
    devices: DeviceInfo,
    
    // Network
    network_interfaces: Vec<NetworkInterfaceInfo>,
    
    // Storage
    storage_volumes: Vec<StorageVolumeInfo>,
    
    // Metadata
    metadata: Option<String>,
    
    // Timestamps
    creation_time: Option<DateTime<Utc>>,
    modification_time: Option<DateTime<Utc>>,
    
    // Performance
    cpu_stats: CPUStats,
    block_stats: HashMap<String, BlockStats>,
    interface_stats: HashMap<String, InterfaceStats>,
    
    // Security
    security_model: Option<SecurityInfo>,
    
    // Snapshot information
    snapshots: Vec<SnapshotInfo>,
    
    // Misc
    hostname: Option<String>,
    title: Option<String>,
    description: Option<String>,
}

impl VmInfo {
    pub fn get_primary_ip(&self) -> Option<String> {
        self.network_interfaces().iter().find_map(|interface| {
            interface.ip_addresses.iter()
                .find(|ip| ip.type_ == "inet")
                .map(|ip| ip.address.clone())
        })
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum DomainState {
    NoState,
    Running,
    Blocked,
    Paused,
    Shutdown,
    Shutoff,
    Crashed,
    PMSuspended,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct MemoryInfo {
    current: u64,
    maximum: u64,
    available: u64,
    swap_in: u64,
    swap_out: u64,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct VCPUInfo {
    current: u32,
    maximum: u32,
    cpu_affinity: Vec<bool>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct DeviceInfo {
    disks: Vec<DiskInfo>,
    interfaces: Vec<InterfaceInfo>,
    graphics: Vec<GraphicsInfo>,
    videos: Vec<VideoInfo>,
    controllers: Vec<ControllerInfo>,
    input_devices: Vec<InputDeviceInfo>,
    sound_devices: Vec<SoundDeviceInfo>,
    hostdev: Vec<HostDevInfo>,
    redirdev: Vec<RedirDevInfo>,
    smart_cards: Vec<SmartCardInfo>,
    rng: Vec<RNGInfo>,
    memory_devices: Vec<MemoryDeviceInfo>,
    tpm: Vec<TPMInfo>,
    emulator: Option<String>,
    fs: Vec<FilesystemInfo>,
    consoles: Vec<ConsoleInfo>,
    channels: Vec<ChannelInfo>,
    hubs: Vec<HubInfo>,
    watchdog: Option<WatchdogInfo>,
    memballoon: Option<MemBalloonInfo>,
    nvram: Option<NVRAMInfo>,
    panic_devices: Vec<PanicDeviceInfo>,
    shmem: Vec<ShmemInfo>,
    iommu: Option<IOMMUInfo>,
    vsock: Option<VSockInfo>,
    crypto: Vec<CryptoInfo>,
    memory_backing: Option<MemoryBackingInfo>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct NetworkInterfaceInfo {
    name: String,
    mac_address: String,
    model: String,
    ip_addresses: Vec<IPAddress>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct StorageVolumeInfo {
    name: String,
    capacity: u64,
    allocation: u64,
    path: String,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct CPUStats {
    cpu_time: u64,
    user_time: u64,
    system_time: u64,
    vcpu_time: Vec<u64>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct BlockStats {
    rd_req: u64,
    rd_bytes: u64,
    wr_req: u64,
    wr_bytes: u64,
    errors: u64,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct InterfaceStats {
    rx_bytes: u64,
    rx_packets: u64,
    rx_errors: u64,
    rx_drop: u64,
    tx_bytes: u64,
    tx_packets: u64,
    tx_errors: u64,
    tx_drop: u64,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct SecurityInfo {
    model: String,
    doi: String,
    label: String,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct SnapshotInfo {
    name: String,
    description: Option<String>,
    creation_time: DateTime<Utc>,
    state: DomainState,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct DiskInfo {
    device: String,
    target: String,
    source: String,
    driver: String,
    bus: String,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct InterfaceInfo {
    type_: String,
    mac: String,
    model: String,
    source: String,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct GraphicsInfo {
    type_: String,
    port: i32,
    listen: String,
    passwd: Option<String>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct VideoInfo {
    model: String,
    vram: u64,
    heads: u32,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct ControllerInfo {
    type_: String,
    model: String,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct IPAddress {
    address: String,
    prefix: u8,
    type_: String,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct InputDeviceInfo {
    type_: String,  // e.g., "tablet", "keyboard"
    bus: String,    // e.g., "usb", "ps2"
    model: Option<String>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct SoundDeviceInfo {
    model: String,  // e.g., "ich6", "ac97"
    codec: Option<String>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct HostDevInfo {
    type_: String,  // e.g., "usb", "pci"
    source: HostDevSource,
    mode: String,   // e.g., "subsystem"
}

#[derive(Getters, MutGetters, Setters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct RedirDevInfo {
    bus: String,        // e.g., "usb"
    type_: String,      // e.g., "spicevmc"
    server: Option<RedirDevServer>,
    boot_order: Option<u32>,
    address: Option<RedirDevAddress>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct RedirDevServer {
    address: Option<String>,
    port: Option<String>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct RedirDevAddress {
    type_: String,      // e.g., "usb"
    bus: Option<u32>,
    port: Option<u32>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct HostDevSource {
    vendor_id: Option<String>,
    product_id: Option<String>,
    address: Option<String>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct SmartCardInfo {
    mode: String,  // e.g., "host", "passthrough"
    type_: String, // e.g., "tcp", "spicevmc"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct RNGInfo {
    model: String,  // e.g., "virtio"
    backend_model: String,  // e.g., "random"
    rate_bytes: Option<u64>,
    rate_period: Option<u64>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct MemoryDeviceInfo {
    model: String,  // e.g., "dimm", "nvdimm"
    size: u64,
    target_node: Option<u32>,
    label_size: Option<u64>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct TPMInfo {
    model: String,  // e.g., "tpm-tis"
    backend_type: String,  // e.g., "passthrough", "emulator"
    version: String,  // e.g., "1.2", "2.0"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct FilesystemInfo {
    type_: String,  // e.g., "mount", "template"
    access_mode: String,  // e.g., "squash", "passthrough"
    source: String,
    target: String,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct ConsoleInfo {
    type_: String,  // e.g., "pty", "file"
    target_type: String,  // e.g., "serial", "virtio"
    target_port: Option<u32>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct ChannelInfo {
    type_: String,  // e.g., "unix", "spicevmc"
    target_type: String,  // e.g., "virtio", "guestfwd"
    target_name: Option<String>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct HubInfo {
    type_: String,  // e.g., "usb"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct WatchdogInfo {
    model: String,  // e.g., "i6300esb"
    action: String,  // e.g., "reset", "poweroff"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct MemBalloonInfo {
    model: String,  // e.g., "virtio"
    period: Option<u32>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct NVRAMInfo {
    model: String,  // e.g., "pvpanic"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct PanicDeviceInfo {
    model: String,  // e.g., "hyperv", "isa"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct ShmemInfo {
    name: String,
    size: Option<u64>,
    model: String,  // e.g., "ivshmem"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct IOMMUInfo {
    model: String,  // e.g., "intel"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct VSockInfo {
    cid: String,
    auto_cid: Option<bool>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct CryptoInfo {
    model: String,  // e.g., "virtio"
    type_: String,  // e.g., "qemu"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct MemoryBackingInfo {
    hugepages: Option<HugepagesInfo>,
    nosharepages: Option<bool>,
    locked: Option<bool>,
    source_type: Option<String>,
    access_mode: Option<String>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct HugepagesInfo {
    page_size: u64,
    nodeset: Option<String>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[serde(transparent)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct VmList {
    vms: Vec<VmInfo>
}

impl VmList {
    pub fn get(&self, name: &str) -> Option<VmInfo> {
        let mut info_list = self.vms.clone().into_par_iter().filter(|info| {
            info.name() == name
        }).collect::<Vec<VmInfo>>();
        info_list.pop()
    }
}

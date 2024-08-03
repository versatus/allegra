use chrono::{DateTime, Utc};
use derive_builder::Builder;
use derive_new::new;
use getset::{Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::Namespace;

#[derive(Debug, Serialize, Deserialize)]
pub struct SshDetails {
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VmResponse {
    pub status: String,
    pub details: String,
    pub ssh_details: Option<SshDetails>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub enum VirDomainState {
    NoState = 0,
    Running = 1,
    Blocked = 2,
    Paused = 3,
    Shutdown = 4,
    Shutoff = 5,
    Crashed = 6,
    PMSuspended = 7,
}

impl From<u32> for DomainState {
    fn from(value: u32) -> Self {
        match value {
            0 => DomainState::NoState,
            1 => DomainState::Running,
            2 => DomainState::Blocked,
            3 => DomainState::Paused,
            4 => DomainState::Shutdown,
            5 => DomainState::Shutoff,
            6 => DomainState::Crashed,
            7 => DomainState::PMSuspended,
            _ => DomainState::NoState,
        }
    }
}

impl From<VirDomainState> for DomainState {
    fn from(value: VirDomainState) -> Self {
        match value {
            VirDomainState::NoState => DomainState::NoState,
            VirDomainState::Running => DomainState::Running,
            VirDomainState::Blocked => DomainState::Blocked,
            VirDomainState::Paused => DomainState::Paused,
            VirDomainState::Shutdown => DomainState::Shutdown,
            VirDomainState::Shutoff => DomainState::Shutoff,
            VirDomainState::Crashed => DomainState::Crashed,
            VirDomainState::PMSuspended => DomainState::PMSuspended,
        }
    }
}

#[derive(Builder, new, Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
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
    devices: Option<DeviceInfo>,

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
    cpu_stats: Option<CPUStats>,
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
            interface
                .ip_addresses
                .iter()
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

#[derive(Builder, new, Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct MemoryInfo {
    current: u64,
    maximum: u64,
    available: u64,
    swap_in: u64,
    swap_out: u64,
}

#[derive(Builder, new, Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct VCPUInfo {
    current: u32,
    maximum: u32,
}

#[derive(Builder, new, Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
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

#[derive(Builder, new, Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
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
    type_: String, // e.g., "tablet", "keyboard"
    bus: String,   // e.g., "usb", "ps2"
    model: Option<String>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct SoundDeviceInfo {
    model: String, // e.g., "ich6", "ac97"
    codec: Option<String>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct HostDevInfo {
    type_: String, // e.g., "usb", "pci"
    source: HostDevSource,
    mode: String, // e.g., "subsystem"
}

#[derive(Getters, MutGetters, Setters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct RedirDevInfo {
    bus: String,   // e.g., "usb"
    type_: String, // e.g., "spicevmc"
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
    type_: String, // e.g., "usb"
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
    model: String,         // e.g., "virtio"
    backend_model: String, // e.g., "random"
    rate_bytes: Option<u64>,
    rate_period: Option<u64>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct MemoryDeviceInfo {
    model: String, // e.g., "dimm", "nvdimm"
    size: u64,
    target_node: Option<u32>,
    label_size: Option<u64>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct TPMInfo {
    model: String,        // e.g., "tpm-tis"
    backend_type: String, // e.g., "passthrough", "emulator"
    version: String,      // e.g., "1.2", "2.0"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct FilesystemInfo {
    type_: String,       // e.g., "mount", "template"
    access_mode: String, // e.g., "squash", "passthrough"
    source: String,
    target: String,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct ConsoleInfo {
    type_: String,       // e.g., "pty", "file"
    target_type: String, // e.g., "serial", "virtio"
    target_port: Option<u32>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct ChannelInfo {
    type_: String,       // e.g., "unix", "spicevmc"
    target_type: String, // e.g., "virtio", "guestfwd"
    target_name: Option<String>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct HubInfo {
    type_: String, // e.g., "usb"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct WatchdogInfo {
    model: String,  // e.g., "i6300esb"
    action: String, // e.g., "reset", "poweroff"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct MemBalloonInfo {
    model: String, // e.g., "virtio"
    period: Option<u32>,
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct NVRAMInfo {
    model: String, // e.g., "pvpanic"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct PanicDeviceInfo {
    model: String, // e.g., "hyperv", "isa"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct ShmemInfo {
    name: String,
    size: Option<u64>,
    model: String, // e.g., "ivshmem"
}

#[derive(Getters, Setters, MutGetters, Clone, Serialize, Deserialize, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct IOMMUInfo {
    model: String, // e.g., "intel"
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
    model: String, // e.g., "virtio"
    type_: String, // e.g., "qemu"
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
    pub vms: HashMap<Namespace, VmInfo>,
}

impl VmList {
    pub fn get(&self, name: &str) -> Option<&VmInfo> {
        self.vms.get(&Namespace::new(name.to_string()))
    }
}

/*
pub fn get_vminfo(
    conn: &Connect,
    domain_name: Namespace,
    state: Option<DomainState>,
    memory: MemoryInfo,

) -> Result<VmInfo, Box<dyn std::error::Error>> {
    let domain = Domain::lookup_by_name(conn, &domain_name.inner().to_string())?;
    let info = domain.get_info()?;

    let vminfo = VmInfo {
        name: domain.get_name()?,
        uuid: domain.get_uuid_string()?,
        id: domain.get_id(),
        state: info.state.into(),
        state_reason: domain.get_state()?.1,
        memory: MemoryInfo {
            current: info.memory,
            maximum: domain.get_max_memory()?,
            ..Default::default()
        },
        vcpus: VCPUInfo {
            current: info.nr_virt_cpu,
            maximum: domain.get_max_vcpus()? as u32,
            ..Default::default()
        },
        cpu_time: info.cpu_time,
        autostart: domain.get_autostart()?,
        persistent: domain.is_persistent()?,
        os_type: domain.get_os_type()?,
        os_arch: "".to_string(),
        devices: get_device_info(&domain)?,
        network_interfaces: get_network_interfaces(&domain)?,
        storage_volumes: get_storage_volumes(&domain)?,
        metadata: domain.get_metadata(virt::sys::VIR_DOMAIN_METADATA_DESCRIPTION.try_into()?, None, 0).ok(),
        creation_time: None,
        modification_time: None,
        cpu_stats: get_cpu_stats(&domain)?,
        block_stats: get_block_stats(&domain)?,
        interface_stats: get_interface_stats(&domain)?,
        security_model: get_security_info(&domain)?,
        snapshots: get_snapshots(&domain)?,
        hostname: domain.get_hostname(0).ok(),
        title: domain.get_metadata(virt::sys::VIR_DOMAIN_METADATA_TITLE.try_into()?, None, 0).ok(),
        description: domain.get_metadata(virt::sys::VIR_DOMAIN_METADATA_DESCRIPTION.try_into()?, None, 0).ok()

    };

    Ok(vminfo)
}

pub fn get_device_info(domain: &Domain) -> Result<DeviceInfo, Box<dyn std::error::Error>> {
    let xml = domain.get_xml_desc(0)?;
    let doc = Document::parse(&xml)?;
    let devices_node = doc.root_element().children().find(|n| n.has_tag_name("devices"))
        .ok_or("no devices found in domain xml")?;

    let mut device_info = DeviceInfo {
        disks: Vec::new(),
        interfaces: Vec::new(),
        graphics: Vec::new(),
        videos: Vec::new(),
        controllers: Vec::new(),
        input_devices: Vec::new(),
        sound_devices: Vec::new(),
        hostdev: Vec::new(),
        redirdev: Vec::new(),
        smart_cards: Vec::new(),
        rng: Vec::new(),
        memory_devices: Vec::new(),
        tpm: Vec::new(),
        emulator: None,
        fs: Vec::new(),
        consoles: Vec::new(),
        channels: Vec::new(),
        hubs: Vec::new(),
        watchdog: None,
        memballoon: None,
        nvram: None,
        panic_devices: Vec::new(),
        shmem: Vec::new(),
        iommu: None,
        vsock: None,
        crypto: Vec::new(),
        memory_backing: None,
    };

    for device in devices_node.children().filter(|n| n.is_element()) {
        match device.tag_name().name() {
            "disk" => device_info.disks.push(parse_disk(&device)?),
            "interface" => device_info.interfaces.push(parse_interface(&device)?),
            "graphics" => device_info.graphics.push(parse_graphics(&device)?),
            "video" => device_info.videos.push(parse_video(&device)?),
            "controller" => device_info.controllers.push(parse_controller(&device)?),
            "input" => device_info.input_devices.push(parse_input_device(&device)?),
            "sound" => device_info.sound_devices.push(parse_sound_device(&device)?),
            "hostdev" => device_info.hostdev.push(parse_hostdev(&device)?),
            "redirdev" => device_info.redirdev.push(parse_redirdev(&device)?),
            "smartcard" => device_info.smart_cards.push(parse_smartcard(&device)?),
            "rng" => device_info.rng.push(parse_rng(&device)?),
            "memory" => device_info.memory_devices.push(parse_memory_device(&device)?),
            "tpm" => device_info.tpm.push(parse_tpm(&device)?),
            "emulator" => device_info.emulator = Some(device.text().unwrap_or("").to_string()),
            "filesystem" => device_info.fs.push(parse_filesystem(&device)?),
            "console" => device_info.consoles.push(parse_console(&device)?),
            "channel" => device_info.channels.push(parse_channel(&device)?),
            "hub" => device_info.hubs.push(parse_hub(&device)?),
            "watchdog" => device_info.watchdog = Some(parse_watchdog(&device)?),
            "memballoon" => device_info.memballoon = Some(parse_memballoon(&device)?),
            "nvram" => device_info.nvram = Some(parse_nvram(&device)?),
            "panic" => device_info.panic_devices.push(parse_panic_device(&device)?),
            "shmem" => device_info.shmem.push(parse_shmem(&device)?),
            "iommu" => device_info.iommu = Some(parse_iommu(&device)?),
            "vsock" => device_info.vsock = Some(parse_vsock(&device)?),
            "crypto" => device_info.crypto.push(parse_crypto(&device)?),
            _ => {} // Ignore unknown devices
        }
    }

    // Parse memory backing, which is not under 'devices'
    device_info.memory_backing = parse_memory_backing(&doc.root_element())?;

    Ok(device_info)
}

fn parse_disk(node: &roxmltree::Node) -> Result<DiskInfo, Box<dyn std::error::Error>> {
    Ok(DiskInfo {
        device: node.attribute("device").unwrap_or("").to_string(),
        target: node.children().find(|n| n.has_tag_name("target"))
            .and_then(|n| n.attribute("dev"))
            .unwrap_or("").to_string(),
        source: node.children().find(|n| n.has_tag_name("source"))
            .and_then(|n| n.attribute("file").or(n.attribute("dev")))r
            .unwrap_or("").to_string(),
        driver: node.children().find(|n| n.has_tag_name("driver"))
            .and_then(|n| n.attribute("name"))
            .unwrap_or("").to_string(),
        bus: node.children().find(|n| n.has_tag_name("target"))
            .and_then(|n| n.attribute("bus"))
            .unwrap_or("").to_string(),
    })
}

fn parse_interface(node: &roxmltree::Node) -> Result<InterfaceInfo, Box<dyn std::error::Error>> {
    Ok(InterfaceInfo {
        type_: node.attribute("type").unwrap_or("").to_string(),
        mac: node.children().find(|n| n.has_tag_name("mac"))
            .and_then(|n| n.attribute("address"))
            .unwrap_or("").to_string(),
        model: node.children().find(|n| n.has_tag_name("model"))
            .and_then(|n| n.attribute("type"))
            .unwrap_or("").to_string(),
        source: node.children().find(|n| n.has_tag_name("source"))
            .and_then(|n| n.attribute("network").or(n.attribute("bridge")))
            .unwrap_or("").to_string(),
    })
}

fn parse_graphics(node: &roxmltree::Node) -> Result<GraphicsInfo, Box<dyn std::error::Error>> {
    Ok(GraphicsInfo {
        type_: node.attribute("type").unwrap_or("").to_string(),
        port: node.attribute("port").and_then(|p| p.parse().ok()).unwrap_or(-1),
        listen: node.children().find(|n| n.has_tag_name("listen"))
            .and_then(|n| n.attribute("address"))
            .unwrap_or("").to_string(),
        passwd: node.attribute("passwd").map(|s| s.to_string()),
    })
}

fn parse_video(node: &roxmltree::Node) -> Result<VideoInfo, Box<dyn std::error::Error>> {
    let model = node.children().find(|n| n.has_tag_name("model"));
    Ok(VideoInfo {
        model: model.and_then(|n| n.attribute("type")).unwrap_or("").to_string(),
        vram: model.and_then(|n| n.attribute("vram"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        heads: model.and_then(|n| n.attribute("heads"))
            .and_then(|h| h.parse().ok())
            .unwrap_or(1),
    })
}

fn parse_controller(node: &roxmltree::Node) -> Result<ControllerInfo, Box<dyn std::error::Error>> {
    Ok(ControllerInfo {
        type_: node.attribute("type").unwrap_or("").to_string(),
        model: node.attribute("model").unwrap_or("").to_string(),
    })
}

fn parse_memory_backing(root: &roxmltree::Node) -> Result<Option<MemoryBackingInfo>, Box<dyn std::error::Error>> {
    root.children().find(|n| n.has_tag_name("memoryBacking")).map(|node| {
        Ok(MemoryBackingInfo {
            hugepages: node.children().find(|n| n.has_tag_name("hugepages"))
                .map(|hp| HugepagesInfo {
                    page_size: hp.children().find(|n| n.has_tag_name("page"))
                        .and_then(|p| p.attribute("size"))
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0),
                    nodeset: hp.children().find(|n| n.has_tag_name("page"))
                        .and_then(|p| p.attribute("nodeset"))
                        .map(|s| s.to_string()),
                }),
            nosharepages: node.children().find(|n| n.has_tag_name("nosharepages")),
            locked: node.children().find(|n| n.has_tag_name("locked")),
            source_type: node.children().find(|n| n.has_tag_name("source"))
                .and_then(|s| s.attribute("type"))
                .map(|s| s.to_string()),
            access_mode: node.children().find(|n| n.has_tag_name("access"))
                .and_then(|a| a.attribute("mode"))
                .map(|s| s.to_string()),
        })
    }).transpose()
}

fn parse_input_device(node: &Node) -> Result<InputDeviceInfo, Box<dyn std::error::Error>> {
    Ok(InputDeviceInfo {
        type_: node.attribute("type").unwrap_or("").to_string(),
        bus: node.attribute("bus").unwrap_or("").to_string(),
        model: node.attribute("model").map(String::from),
    })
}

fn parse_sound_device(node: &Node) -> Result<SoundDeviceInfo, Box<dyn std::error::Error>> {
    Ok(SoundDeviceInfo {
        model: node.attribute("model").unwrap_or("").to_string(),
        codec: node.children().find(|n| n.has_tag_name("codec"))
            .and_then(|n| n.attribute("type"))
            .map(String::from),
    })
}

fn parse_hostdev(node: &Node) -> Result<HostDevInfo, Box<dyn std::error::Error>> {
    let source = node.children().find(|n| n.has_tag_name("source")).unwrap();
    Ok(HostDevInfo {
        type_: node.attribute("type").unwrap_or("").to_string(),
        mode: node.attribute("mode").unwrap_or("").to_string(),
        source: HostDevSource {
            vendor_id: source.children().find(|n| n.has_tag_name("vendor"))
                .and_then(|n| n.attribute("id"))
                .map(String::from),
            product_id: source.children().find(|n| n.has_tag_name("product"))
                .and_then(|n| n.attribute("id"))
                .map(String::from),
            address: source.children().find(|n| n.has_tag_name("address"))
                .and_then(|n| n.attribute("bus"))
                .map(String::from),
        },
    })
}

fn parse_redirdev(node: &Node) -> Result<RedirDevInfo, Box<dyn std::error::Error>> {
    Ok(RedirDevInfo {
        bus: node.attribute("bus").unwrap_or("").to_string(),
        type_: node.attribute("type").unwrap_or("").to_string(),
        server: node.children().find(|n| n.has_tag_name("source"))
            .map(|n| RedirDevServer {
                address: n.attribute("host").map(String::from),
                port: n.attribute("service").map(String::from),
            }),
        boot_order: node.children().find(|n| n.has_tag_name("boot"))
            .and_then(|n| n.attribute("order"))
            .and_then(|o| o.parse().ok()),
        address: node.children().find(|n| n.has_tag_name("address"))
            .map(|n| RedirDevAddress {
                type_: n.attribute("type").unwrap_or("").to_string(),
                bus: n.attribute("bus").and_then(|b| b.parse().ok()),
                port: n.attribute("port").and_then(|p| p.parse().ok()),
            }),
    })
}

fn parse_smartcard(node: &Node) -> Result<SmartCardInfo, Box<dyn std::error::Error>> {
    Ok(SmartCardInfo {
        mode: node.attribute("mode").unwrap_or("").to_string(),
        type_: node.children().find(|n| n.has_tag_name("source"))
            .and_then(|n| n.attribute("mode"))
            .unwrap_or("").to_string(),
    })
}

fn parse_rng(node: &Node) -> Result<RNGInfo, Box<dyn std::error::Error>> {
    Ok(RNGInfo {
        model: node.attribute("model").unwrap_or("").to_string(),
        backend_model: node.children().find(|n| n.has_tag_name("backend"))
            .and_then(|n| n.attribute("model"))
            .unwrap_or("").to_string(),
        rate_bytes: node.children().find(|n| n.has_tag_name("rate"))
            .and_then(|n| n.attribute("bytes"))
            .and_then(|b| b.parse().ok()),
        rate_period: node.children().find(|n| n.has_tag_name("rate"))
            .and_then(|n| n.attribute("period"))
            .and_then(|p| p.parse().ok()),
    })
}

fn parse_memory_device(node: &Node) -> Result<MemoryDeviceInfo, Box<dyn std::error::Error>> {
    Ok(MemoryDeviceInfo {
        model: node.attribute("model").unwrap_or("").to_string(),
        size: node.children().find(|n| n.has_tag_name("target"))
            .and_then(|n| n.children().find(|c| c.has_tag_name("size")))
            .and_then(|n| n.attribute("value"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        target_node: node.children().find(|n| n.has_tag_name("target"))
            .and_then(|n| n.children().find(|c| c.has_tag_name("node")))
            .and_then(|n| n.attribute("value"))
            .and_then(|v| v.parse().ok()),
        label_size: node.children().find(|n| n.has_tag_name("target"))
            .and_then(|n| n.children().find(|c| c.has_tag_name("label")))
            .and_then(|n| n.attribute("size"))
            .and_then(|v| v.parse().ok()),
    })
}

fn parse_tpm(node: &Node) -> Result<TPMInfo, Box<dyn std::error::Error>> {
    Ok(TPMInfo {
        model: node.attribute("model").unwrap_or("").to_string(),
        backend_type: node.children().find(|n| n.has_tag_name("backend"))
            .and_then(|n| n.attribute("type"))
            .unwrap_or("").to_string(),
        version: node.children().find(|n| n.has_tag_name("backend"))
            .and_then(|n| n.attribute("version"))
            .unwrap_or("").to_string(),
    })
}

fn parse_filesystem(node: &Node) -> Result<FilesystemInfo, Box<dyn std::error::Error>> {
    Ok(FilesystemInfo {
        type_: node.attribute("type").unwrap_or("").to_string(),
        access_mode: node.attribute("accessmode").unwrap_or("").to_string(),
        source: node.children().find(|n| n.has_tag_name("source"))
            .and_then(|n| n.attribute("dir"))
            .unwrap_or("").to_string(),
        target: node.children().find(|n| n.has_tag_name("target"))
            .and_then(|n| n.attribute("dir"))
            .unwrap_or("").to_string(),
    })
}

fn parse_console(node: &Node) -> Result<ConsoleInfo, Box<dyn std::error::Error>> {
    Ok(ConsoleInfo {
        type_: node.attribute("type").unwrap_or("").to_string(),
        target_type: node.children().find(|n| n.has_tag_name("target"))
            .and_then(|n| n.attribute("type"))
            .unwrap_or("").to_string(),
        target_port: node.children().find(|n| n.has_tag_name("target"))
            .and_then(|n| n.attribute("port"))
            .and_then(|p| p.parse().ok()),
    })
}

fn parse_channel(node: &Node) -> Result<ChannelInfo, Box<dyn std::error::Error>> {
    Ok(ChannelInfo {
        type_: node.attribute("type").unwrap_or("").to_string(),
        target_type: node.children().find(|n| n.has_tag_name("target"))
            .and_then(|n| n.attribute("type"))
            .unwrap_or("").to_string(),
        target_name: node.children().find(|n| n.has_tag_name("target"))
            .and_then(|n| n.attribute("name"))
            .map(String::from),
    })
}

fn parse_hub(node: &Node) -> Result<HubInfo, Box<dyn std::error::Error>> {
    Ok(HubInfo {
        type_: node.attribute("type").unwrap_or("").to_string(),
    })
}

fn parse_watchdog(node: &Node) -> Result<WatchdogInfo, Box<dyn std::error::Error>> {
    Ok(WatchdogInfo {
        model: node.attribute("model").unwrap_or("").to_string(),
        action: node.attribute("action").unwrap_or("").to_string(),
    })
}

fn parse_memballoon(node: &Node) -> Result<MemBalloonInfo, Box<dyn std::error::Error>> {
    Ok(MemBalloonInfo {
        model: node.attribute("model").unwrap_or("").to_string(),
        period: node.children().find(|n| n.has_tag_name("stats"))
            .and_then(|n| n.attribute("period"))
            .and_then(|p| p.parse().ok()),
    })
}

fn parse_nvram(node: &Node) -> Result<NVRAMInfo, Box<dyn std::error::Error>> {
    Ok(NVRAMInfo {
        model: node.attribute("model").unwrap_or("").to_string(),
    })
}

fn parse_panic_device(node: &Node) -> Result<PanicDeviceInfo, Box<dyn std::error::Error>> {
    Ok(PanicDeviceInfo {
        model: node.attribute("model").unwrap_or("").to_string(),
    })
}

fn parse_shmem(node: &Node) -> Result<ShmemInfo, Box<dyn std::error::Error>> {
    Ok(ShmemInfo {
        name: node.attribute("name").unwrap_or("").to_string(),
        size: node.children().find(|n| n.has_tag_name("size"))
            .and_then(|n| n.attribute("value"))
            .and_then(|v| v.parse().ok()),
        model: node.attribute("model").unwrap_or("").to_string(),
    })
}

fn parse_iommu(node: &Node) -> Result<IOMMUInfo, Box<dyn std::error::Error>> {
    Ok(IOMMUInfo {
        model: node.attribute("model").unwrap_or("").to_string(),
    })
}

fn parse_vsock(node: &Node) -> Result<VSockInfo, Box<dyn std::error::Error>> {
    Ok(VSockInfo {
        cid: node.children().find(|n| n.has_tag_name("cid"))
            .and_then(|n| n.attribute("value"))
            .unwrap_or("").to_string(),
        auto_cid: node.children().find(|n| n.has_tag_name("cid"))
            .and_then(|n| n.attribute("auto"))
            .and_then(|a| a.parse().ok()),
    })
}

fn parse_crypto(node: &Node) -> Result<CryptoInfo, Box<dyn std::error::Error>> {
    Ok(CryptoInfo {
        model: node.attribute("model").unwrap_or("").to_string(),
        type_: node.children().find(|n| n.has_tag_name("backend"))
            .and_then(|n| n.attribute("model"))
            .unwrap_or("").to_string(),
    })
}

pub fn get_network_interfaces(domain: &Domain) -> Result<Vec<NetworkInterfaceInfo>, Box<dyn std::error::Error>> {
    let flags = virt::sys::VIR_DOMAIN_INTERFACE_ADDRESSES_SRC_AGENT;
    let interfaces = match domain.interface_addresses(flags, 0) {
        Ok(ifaces) => ifaces,
        Err(_) => {
            // Fall back to lease information if guest agent is not responsive
            domain.interface_addresses(virt::sys::VIR_DOMAIN_INTERFACE_ADDRESSES_SRC_LEASE, 0)?
        }
    };

    let mut network_interfaces = Vec::new();

    for iface in interfaces {
        let mac_address = iface.hwaddr;

        // Get additional interface details from XML
        let xml = domain.get_xml_desc(0)?;
        let doc = roxmltree::Document::parse(&xml)?;
        let iface_node = doc.descendants()
            .find(|n| n.has_tag_name("interface") && n.attribute("type") == Some("network"))
            .and_then(|n| n.children().find(|c| c.has_tag_name("mac") && c.attribute("address") == Some(&mac_address)));

        let model = iface_node
            .and_then(|n| n.parent())
            .and_then(|n| n.children().find(|c| c.has_tag_name("model")))
            .and_then(|n| n.attribute("type"))
            .unwrap_or("unknown")
            .to_string();

        let ip_addresses = iface.addrs.into_iter().map(|addr| {
            IPAddress {
                address: addr.addr,
                prefix: addr.prefix as u8,
                type_: "ipv4".to_string(),
            }
        }).collect();

        network_interfaces.push(NetworkInterfaceInfo {
            name: iface.name,
            mac_address,
            model,
            ip_addresses,
        });
    }

    Ok(network_interfaces)
}
*/

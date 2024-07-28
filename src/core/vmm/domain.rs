use getset::{Getters, MutGetters, Setters};
use serde::{Serialize, Deserialize};
use crate::cloud_init::CloudConfig;

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
#[serde(rename = "domain")]
pub struct Domain {
    #[serde(rename = "@type")]
    domain_type: String,
    name: String,
    uuid: Option<String>,
    memory: Memory,
    current_memory: Option<Memory>,
    vcpu: VCPU,
    cpu: Option<CPU>,
    os: OS,
    features: Option<Features>,
    clock: Option<Clock>,
    on_poweroff: Option<String>,
    on_reboot: Option<String>,
    on_crash: Option<String>,
    devices: Devices,
    seclabel: Option<Seclabel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cloud_init: Option<CloudConfig>
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Memory {
    #[serde(rename = "@unit")]
    unit: String,
    #[serde(rename = "$text")]
    value: u64,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct VCPU {
    #[serde(rename = "@placement")]
    placement: String,
    #[serde(rename = "$text")]
    value: u32,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct CPU {
    #[serde(rename = "@mode")]
    mode: Option<String>,
    model: Option<String>,
    topology: Option<CPUTopology>,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct CPUTopology {
    #[serde(rename = "@sockets")]
    sockets: u32,
    #[serde(rename = "@cores")]
    cores: u32,
    #[serde(rename = "@threads")]
    threads: u32,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct OS {
    #[serde(rename = "type")]
    os_type: OSType,
    boot: Vec<Boot>,
    kernel: Option<String>,
    initrd: Option<String>,
    cmdline: Option<String>,
    image: Option<OSImage>,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
pub struct OSImage {
    source: ImageSource,
    format: ImageFormat
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ImageSource {
    File(String),
    Volume(String),
    Network(String)
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ImageFormat {
    Raw,
    Qcow2,
    Vmdk,
    Vdi,
    Other(String),
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct OSType {
    #[serde(rename = "@arch")]
    arch: String,
    #[serde(rename = "@machine")]
    machine: String,
    #[serde(rename = "$text")]
    value: String,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Boot {
    #[serde(rename = "@dev")]
    dev: String,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Features {
    acpi: Option<Empty>,
    apic: Option<Empty>,
    pae: Option<Empty>,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Empty {}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Clock {
    #[serde(rename = "@offset")]
    offset: String,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Devices {
    emulator: String,
    disks: Vec<Disk>,
    interfaces: Vec<Interface>,
    serial: Option<Serial>,
    console: Option<Console>,
    input: Vec<Input>,
    graphics: Option<Graphics>,
    video: Option<Video>,
    filesystems: Vec<Filesystem>,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Disk {
    #[serde(rename = "@type")]
    disk_type: String,
    #[serde(rename = "@device")]
    device: String,
    driver: DiskDriver,
    source: DiskSource,
    target: DiskTarget,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct DiskDriver {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@type")]
    driver_type: String,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct DiskSource {
    #[serde(rename = "@file")]
    file: String,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct DiskTarget {
    #[serde(rename = "@dev")]
    dev: String,
    #[serde(rename = "@bus")]
    bus: String,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Interface {
    #[serde(rename = "@type")]
    interface_type: String,
    source: InterfaceSource,
    model: InterfaceModel,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct InterfaceSource {
    #[serde(rename = "@bridge")]
    bridge: String,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct InterfaceModel {
    #[serde(rename = "@type")]
    model_type: String,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Serial {
    #[serde(rename = "@type")]
    serial_type: String,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Console {
    #[serde(rename = "@type")]
    console_type: String,
    target: ConsoleTarget,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct ConsoleTarget {
    #[serde(rename = "@type")]
    target_type: String,
    #[serde(rename = "@port")]
    port: String,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Input {
    #[serde(rename = "@type")]
    input_type: String,
    #[serde(rename = "@bus")]
    bus: String,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Graphics {
    #[serde(rename = "@type")]
    graphics_type: String,
    #[serde(rename = "@port")]
    port: i32,
    #[serde(rename = "@autoport")]
    autoport: String,
    listen: GraphicsListen,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct GraphicsListen {
    #[serde(rename = "@type")]
    listen_type: String,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Video {
    model: VideoModel,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct VideoModel {
    #[serde(rename = "@type")]
    model_type: String,
    #[serde(rename = "@vram")]
    vram: u64,
    #[serde(rename = "@heads")]
    heads: u32,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Filesystem {
    #[serde(rename = "@type")]
    fs_type: String,
    #[serde(rename = "@accessmode")]
    accessmode: String,
    source: FilesystemSource,
    target: FilesystemTarget,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct FilesystemSource {
    #[serde(rename = "@dir")]
    dir: String,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct FilesystemTarget {
    #[serde(rename = "@dir")]
    dir: String,
}

#[derive(Getters, MutGetters, Setters, Serialize, Deserialize)]
struct Seclabel {
    #[serde(rename = "@type")]
    label_type: String,
    #[serde(rename = "@model")]
    model: String,
    #[serde(rename = "@relabel")]
    relabel: String,
    label: String,
}



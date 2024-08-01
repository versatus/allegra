use crate::allegra_rpc::{CloudInit as ProtoCloudInit, InstanceCreateParams};
use getset::{Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path, process::Command};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct VirtInstall {
    name: String,
    memory: Option<String>,
    vcpus: Option<String>,
    cpu: Option<String>,
    metadata: Option<String>,
    os_variant: Option<String>,
    host_device: Vec<String>,
    network: Vec<String>,
    disk: Vec<String>,
    filesystem: Vec<String>,
    controller: Vec<String>,
    input: Vec<String>,
    graphics: Option<String>,
    sound: Option<String>,
    video: Option<String>,
    smartcard: Option<String>,
    redirdev: Vec<String>,
    memballoon: Option<String>,
    tpm: Option<String>,
    rng: Option<String>,
    panic: Option<String>,
    shmem: Option<String>,
    memdev: Vec<String>,
    vsock: Option<String>,
    iommu: Option<String>,
    watchdog: Option<String>,
    serial: Vec<String>,
    parallel: Vec<String>,
    channel: Vec<String>,
    console: Vec<String>,
    install: Option<String>,
    cdrom: Option<String>,
    location: Option<String>,
    pxe: bool,
    import: bool,
    boot: Option<String>,
    idmap: Option<String>,
    features: HashMap<String, String>,
    clock: Option<String>,
    launch_security: Option<String>,
    numatune: Option<String>,
    boot_dev: Vec<String>,
    unattended: bool,
    print_xml: Option<String>,
    dry_run: bool,
    connect: Option<String>,
    virt_type: Option<String>,
    cloud_init: Option<CloudInit>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CloudInit {
    pub root_password_generate: bool,
    pub disable: bool,
    pub root_password_file: Option<String>,
    pub meta_data: Option<String>,
    pub user_data: Option<String>,
    pub root_ssh_key: Option<String>,
    pub clouduser_ssh_key: Option<String>,
    pub network_config: Option<String>,
}

#[derive(Getters, MutGetters, Setters, Debug, Serialize, Deserialize)]
pub struct UserData {
    users: Vec<User>,
    packages: Vec<String>,
    write_files: Vec<WriteFile>,
    mounts: Vec<String>,
    runcmd: Vec<String>,
    bootcmd: Vec<String>,
}

#[derive(Clone, Getters, MutGetters, Setters, Debug, Serialize, Deserialize)]
pub struct User {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ssh_authorized_keys: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sudo: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    groups: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shell: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    passwd: Option<String>,
    #[serde(rename = "lock-passwd", skip_serializing_if = "Option::is_none")]
    lock_passwd: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chpasswd: Option<Chpasswd>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ssh_pwauth: Option<bool>,
}

#[derive(Clone, Getters, MutGetters, Setters, Debug, Serialize, Deserialize)]
pub struct Chpasswd {
    expire: bool,
}

#[derive(Clone, Getters, MutGetters, Setters, Debug, Serialize, Deserialize)]
pub struct WriteFile {
    path: String,
    content: String,
    permissions: Option<String>,
}

#[derive(Clone, Getters, MutGetters, Setters, Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    version: u8,
    ethernets: std::collections::HashMap<String, EthernetConfig>,
}

#[derive(Clone, Getters, MutGetters, Setters, Debug, Serialize, Deserialize)]
pub struct EthernetConfig {
    dhcp4: bool,
    addresses: Vec<String>,
    nameservers: Nameservers,
    routes: Vec<Route>,
}

#[derive(Clone, Getters, MutGetters, Setters, Debug, Serialize, Deserialize)]
pub struct Nameservers {
    addresses: Vec<String>,
}

#[derive(Clone, Getters, MutGetters, Setters, Debug, Serialize, Deserialize)]
pub struct Route {
    to: String,
    via: String,
}

#[derive(Clone, Getters, MutGetters, Setters, Debug, Serialize, Deserialize)]
pub struct MetaData {
    instance_id: String,
    local_hostname: String,
}

impl From<ProtoCloudInit> for CloudInit {
    fn from(value: ProtoCloudInit) -> Self {
        Self {
            root_password_generate: value.root_password_generate,
            disable: value.disable,
            root_password_file: value.root_password_file,
            meta_data: value.meta_data,
            user_data: value.user_data,
            root_ssh_key: value.root_ssh_key,
            clouduser_ssh_key: value.clouduser_ssh_key,
            network_config: value.network_config,
        }
    }
}

impl CloudInit {
    pub fn new() -> Self {
        CloudInit::default()
    }

    pub fn set_root_password_generate(mut self, value: bool) -> Self {
        self.root_password_generate = value;
        self
    }

    pub fn set_disable(mut self, value: bool) -> Self {
        self.disable = value;
        self
    }

    pub fn set_root_password_file(mut self, file: String) -> Self {
        self.root_password_file = Some(file);
        self
    }

    pub fn set_meta_data(mut self, data: String) -> Self {
        self.meta_data = Some(data);
        self
    }

    pub fn set_user_data(mut self, data: String) -> Self {
        self.user_data = Some(data);
        self
    }

    pub fn set_root_ssh_key(mut self, key: String) -> Self {
        self.root_ssh_key = Some(key);
        self
    }

    pub fn set_clouduser_ssh_key(mut self, key: String) -> Self {
        self.clouduser_ssh_key = Some(key);
        self
    }

    pub fn set_network_config(mut self, config: String) -> Self {
        self.network_config = Some(config);
        self
    }
}

impl VirtInstall {
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    pub fn memory(mut self, memory: String) -> Self {
        self.memory = Some(memory);
        self
    }

    pub fn vcpus(mut self, vcpus: String) -> Self {
        self.vcpus = Some(vcpus);
        self
    }

    pub fn cpu(mut self, cpu: String) -> Self {
        self.cpu = Some(cpu);
        self
    }

    pub fn metadata(mut self, metadata: String) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn os_variant(mut self, os_variant: String) -> Self {
        self.os_variant = Some(os_variant);
        self
    }

    pub fn host_device(mut self, host_device: String) -> Self {
        self.host_device.push(host_device);
        self
    }

    pub fn network(mut self, network: String) -> Self {
        self.network.push(network);
        self
    }

    pub fn disk(mut self, disk: String) -> Self {
        self.disk.push(disk);
        self
    }

    pub fn filesystem(mut self, filesystem: String) -> Self {
        self.filesystem.push(filesystem);
        self
    }

    pub fn controller(mut self, controller: String) -> Self {
        self.controller.push(controller);
        self
    }

    pub fn input(mut self, input: String) -> Self {
        self.input.push(input);
        self
    }

    pub fn graphics(mut self, graphics: String) -> Self {
        self.graphics = Some(graphics);
        self
    }

    pub fn sound(mut self, sound: String) -> Self {
        self.sound = Some(sound);
        self
    }

    pub fn video(mut self, video: String) -> Self {
        self.video = Some(video);
        self
    }

    pub fn smartcard(mut self, smartcard: String) -> Self {
        self.smartcard = Some(smartcard);
        self
    }

    pub fn redirdev(mut self, redirdev: String) -> Self {
        self.redirdev.push(redirdev);
        self
    }

    pub fn memballoon(mut self, memballoon: String) -> Self {
        self.memballoon = Some(memballoon);
        self
    }

    pub fn tpm(mut self, tpm: String) -> Self {
        self.tpm = Some(tpm);
        self
    }

    pub fn rng(mut self, rng: String) -> Self {
        self.rng = Some(rng);
        self
    }

    pub fn panic(mut self, panic: String) -> Self {
        self.panic = Some(panic);
        self
    }

    pub fn shmem(mut self, shmem: String) -> Self {
        self.shmem = Some(shmem);
        self
    }

    pub fn memdev(mut self, memdev: String) -> Self {
        self.memdev.push(memdev);
        self
    }

    pub fn vsock(mut self, vsock: String) -> Self {
        self.vsock = Some(vsock);
        self
    }

    pub fn iommu(mut self, iommu: String) -> Self {
        self.iommu = Some(iommu);
        self
    }

    pub fn watchdog(mut self, watchdog: String) -> Self {
        self.watchdog = Some(watchdog);
        self
    }

    pub fn serial(mut self, serial: String) -> Self {
        self.serial.push(serial);
        self
    }

    pub fn parallel(mut self, parallel: String) -> Self {
        self.parallel.push(parallel);
        self
    }

    pub fn channel(mut self, channel: String) -> Self {
        self.channel.push(channel);
        self
    }

    pub fn console(mut self, console: String) -> Self {
        self.console.push(console);
        self
    }

    pub fn install(mut self, install: String) -> Self {
        self.install = Some(install);
        self
    }

    pub fn cdrom(mut self, cdrom: String) -> Self {
        self.cdrom = Some(cdrom);
        self
    }

    pub fn location(mut self, location: String) -> Self {
        self.location = Some(location);
        self
    }

    pub fn pxe(mut self) -> Self {
        self.pxe = true;
        self
    }

    pub fn import(mut self) -> Self {
        self.import = true;
        self
    }

    pub fn boot(mut self, boot: String) -> Self {
        self.boot = Some(boot);
        self
    }

    pub fn idmap(mut self, idmap: String) -> Self {
        self.idmap = Some(idmap);
        self
    }

    pub fn feature(mut self, key: String, value: String) -> Self {
        self.features.insert(key, value);
        self
    }

    pub fn clock(mut self, clock: String) -> Self {
        self.clock = Some(clock);
        self
    }

    pub fn launch_security(mut self, launch_security: String) -> Self {
        self.launch_security = Some(launch_security);
        self
    }

    pub fn numatune(mut self, numatune: String) -> Self {
        self.numatune = Some(numatune);
        self
    }

    pub fn boot_dev(mut self, boot_dev: String) -> Self {
        self.boot_dev.push(boot_dev);
        self
    }

    pub fn unattended(mut self) -> Self {
        self.unattended = true;
        self
    }

    pub fn print_xml(mut self, step: String) -> Self {
        self.print_xml = Some(step);
        self
    }

    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    pub fn connect(mut self, connect: String) -> Self {
        self.connect = Some(connect);
        self
    }

    pub fn virt_type(mut self, virt_type: String) -> Self {
        self.virt_type = Some(virt_type);
        self
    }

    pub fn execute(&self) -> std::io::Result<std::process::Output> {
        let mut command = Command::new("virt-install");

        command.arg("--name").arg(&self.name);

        if let Some(memory) = &self.memory {
            command.arg("--memory").arg(memory);
        }

        if let Some(vcpus) = &self.vcpus {
            command.arg("--vcpus").arg(vcpus);
        }

        if let Some(cpu) = &self.cpu {
            command.arg("--cpu").arg(cpu);
        }

        if let Some(metadata) = &self.metadata {
            command.arg("--metadata").arg(metadata);
        }

        if let Some(os_variant) = &self.os_variant {
            command.arg("--os-variant").arg(os_variant);
        }

        for host_device in &self.host_device {
            command.arg("--host-device").arg(host_device);
        }

        for network in &self.network {
            command.arg("--network").arg(network);
        }

        for disk in &self.disk {
            command.arg("--disk").arg(disk);
        }

        for filesystem in &self.filesystem {
            command.arg("--filesystem").arg(filesystem);
        }

        for controller in &self.controller {
            command.arg("--controller").arg(controller);
        }

        for input in &self.input {
            command.arg("--input").arg(input);
        }

        if let Some(graphics) = &self.graphics {
            command.arg("--graphics").arg(graphics);
        }

        if let Some(sound) = &self.sound {
            command.arg("--sound").arg(sound);
        }

        if let Some(video) = &self.video {
            command.arg("--video").arg(video);
        }

        if let Some(smartcard) = &self.smartcard {
            command.arg("--smartcard").arg(smartcard);
        }

        for redirdev in &self.redirdev {
            command.arg("--redirdev").arg(redirdev);
        }

        if let Some(memballoon) = &self.memballoon {
            command.arg("--memballoon").arg(memballoon);
        }

        if let Some(tpm) = &self.tpm {
            command.arg("--tpm").arg(tpm);
        }

        if let Some(rng) = &self.rng {
            command.arg("--rng").arg(rng);
        }

        if let Some(panic) = &self.panic {
            command.arg("--panic").arg(panic);
        }

        if let Some(shmem) = &self.shmem {
            command.arg("--shmem").arg(shmem);
        }

        for memdev in &self.memdev {
            command.arg("--memdev").arg(memdev);
        }

        if let Some(vsock) = &self.vsock {
            command.arg("--vsock").arg(vsock);
        }

        if let Some(iommu) = &self.iommu {
            command.arg("--iommu").arg(iommu);
        }

        if let Some(watchdog) = &self.watchdog {
            command.arg("--watchdog").arg(watchdog);
        }

        for serial in &self.serial {
            command.arg("--serial").arg(serial);
        }

        for parallel in &self.parallel {
            command.arg("--parallel").arg(parallel);
        }

        for channel in &self.channel {
            command.arg("--channel").arg(channel);
        }

        for console in &self.console {
            command.arg("--console").arg(console);
        }

        if let Some(install) = &self.install {
            command.arg("--install").arg(install);
        }

        if let Some(cdrom) = &self.cdrom {
            command.arg("--cdrom").arg(cdrom);
        }

        if let Some(location) = &self.location {
            command.arg("--location").arg(location);
        }

        if self.pxe {
            command.arg("--pxe");
        }

        if self.import {
            command.arg("--import");
        }

        if let Some(boot) = &self.boot {
            command.arg("--boot").arg(boot);
        }

        if let Some(idmap) = &self.idmap {
            command.arg("--idmap").arg(idmap);
        }

        for (key, value) in &self.features {
            command.arg("--features").arg(format!("{}={}", key, value));
        }

        if let Some(clock) = &self.clock {
            command.arg("--clock").arg(clock);
        }

        if let Some(launch_security) = &self.launch_security {
            command.arg("--launchSecurity").arg(launch_security);
        }

        if let Some(numatune) = &self.numatune {
            command.arg("--numatune").arg(numatune);
        }

        for boot_dev in &self.boot_dev {
            command.arg("--boot").arg(boot_dev);
        }

        if self.unattended {
            command.arg("--unattended");
        }

        if let Some(print_xml) = &self.print_xml {
            command.arg("--print-xml").arg(print_xml);
        }

        if self.dry_run {
            command.arg("--dry-run");
        }

        if let Some(connect) = &self.connect {
            command.arg("--connect").arg(connect);
        }

        if let Some(virt_type) = &self.virt_type {
            command.arg("--virt-type").arg(virt_type);
        }

        // Make directory for profile
        // use template and update
        if let Some(cloud_init) = &self.cloud_init {
            let mut cloud_init_args = Vec::new();

            if cloud_init.root_password_generate {
                cloud_init_args.push("root-password-generate=on".to_string());
            }
            if cloud_init.disable {
                cloud_init_args.push("disable=on".to_string());
            }
            if let Some(file) = &cloud_init.root_password_file {
                cloud_init_args.push(format!("root-password-file={}", file.clone()));
            }
            if let Some(meta_data) = &cloud_init.meta_data {
                cloud_init_args.push(format!("meta-data={}", meta_data.clone()));
            }
            if let Some(user_data) = &cloud_init.user_data {
                cloud_init_args.push(format!("user-data={}", user_data.clone()));
            }
            if let Some(root_ssh_key) = &cloud_init.root_ssh_key {
                cloud_init_args.push(format!("root-ssh-key={}", root_ssh_key.clone()));
            }
            if let Some(clouduser_ssh_key) = &cloud_init.clouduser_ssh_key {
                cloud_init_args.push(format!("clouduser-ssh-key={}", clouduser_ssh_key.clone()));
            }
            if let Some(network_config) = &cloud_init.network_config {
                cloud_init_args.push(format!("network-config={}", network_config.clone()));
            }

            if !cloud_init_args.is_empty() {
                command.arg("--cloud-init");
                command.arg(cloud_init_args.join(","));
            } else {
                command.arg("--cloud-init");
            }
        }

        command.output()
    }
}

impl From<InstanceCreateParams> for VirtInstall {
    fn from(value: InstanceCreateParams) -> Self {
        Self {
            name: value.name,
            memory: value.memory,
            vcpus: value.vcpus,
            cpu: value.cpu,
            metadata: value.metadata,
            os_variant: value.os_variant,
            host_device: value.host_device,
            network: value.network,
            disk: value.disk,
            filesystem: value.filesystem,
            controller: value.controller,
            input: value.input,
            graphics: value.graphics,
            sound: value.sound,
            video: value.video,
            smartcard: value.smartcard,
            redirdev: value.redirdev,
            memballoon: value.memballoon,
            tpm: value.tpm,
            rng: value.rng,
            panic: value.panic,
            shmem: value.shmem,
            memdev: value.memdev,
            vsock: value.vsock,
            iommu: value.iommu,
            watchdog: value.watchdog,
            serial: value.serial,
            parallel: value.parallel,
            channel: value.channel,
            console: value.console,
            install: value.install,
            cdrom: value.cdrom,
            location: value.location,
            pxe: value.pxe,
            import: value.import,
            boot: value.boot,
            idmap: value.idmap,
            features: value
                .features
                .iter()
                .map(|f| (f.name.clone(), f.feature.clone()))
                .collect(),
            clock: value.clock,
            launch_security: value.launch_security,
            numatune: value.numatune,
            boot_dev: value.boot_dev,
            unattended: value.unattended,
            print_xml: value.print_xml,
            dry_run: value.dry_run,
            connect: value.connect,
            virt_type: value.virt_type,
            cloud_init: match value.cloud_init {
                Some(ci) => Some(ci.into()),
                None => None,
            },
        }
    }
}

pub fn merge_user_data(default: &mut UserData, user_provided: Option<UserData>) {
    if let Some(user) = user_provided {
        if !user.users.is_empty() {
            default.users = user.users.clone();
        } else {
            for (default_user, provided_user) in default.users.iter_mut().zip(user.users.iter()) {
                if let Some(keys) = &provided_user.ssh_authorized_keys {
                    default_user.ssh_authorized_keys = Some(keys.clone());
                }
                if let Some(sudo) = &provided_user.sudo {
                    default_user.sudo = Some(sudo.clone());
                }
                if let Some(groups) = &provided_user.groups {
                    default_user.groups = Some(groups.clone());
                }
                if let Some(shell) = &provided_user.shell {
                    default_user.shell = Some(shell.clone());
                }
                if let Some(passwd) = &provided_user.passwd {
                    default_user.passwd = Some(passwd.clone());
                }
                if let Some(lock_passwd) = provided_user.lock_passwd {
                    default_user.lock_passwd = Some(lock_passwd);
                }
                if let Some(chpasswd) = &provided_user.chpasswd {
                    default_user.chpasswd = Some(chpasswd.clone());
                }
                if let Some(ssh_pwauth) = provided_user.ssh_pwauth {
                    default_user.ssh_pwauth = Some(ssh_pwauth);
                }
            }
        }

        default.packages.extend(user.packages.iter().cloned());
        default.write_files.extend(user.write_files.iter().cloned());
        default.mounts.extend(user.mounts.iter().cloned());
        default.runcmd.extend(user.runcmd.iter().cloned());
    }
}

pub fn generate_cloud_init_files(
    instance_id: &str,
    hostname: &str,
    user_provided: Option<UserData>,
    ip_address: &str,
) -> std::io::Result<()> {
    let mut default_user_data = UserData {
        users: vec![User {
            name: "ubuntu".to_string(),
            ssh_authorized_keys: None,
            sudo: Some(vec!["ALL=(ALL) NOPASSWD:ALL".to_string()]),
            groups: Some(vec!["sudo".to_string()]), 
            shell: Some("/bin/bash".to_string()),
            passwd: Some("$6$ZP1jxGi3aYL9Cs2m$hh5hIXjvztsXarenhMWiLZNqXM.J/djgkuB3trkqi9fkDNCAANDrWZgFWymG0rUONkMXavx3kIFyqc5eEsnuV1".to_string()),
            lock_passwd: Some(false),
            chpasswd: Some(Chpasswd { expire: false }),
            ssh_pwauth: Some(true)
        }],
        packages: vec![
            "glusterfs-client".to_string(),
            "rsync".to_string(),
            "inotify-tools".to_string()
        ],
        write_files: vec![
            WriteFile {
                path: "/etc/systemd/system/glusterfs-sync.service".to_string(),
                content: include_str!("templates/glusterfs-sync.service").to_string(),
                permissions: None
            },
            WriteFile {
                path: "/usr/local/bin/glusterfs-sync.sh".to_string(),
                content: include_str!("templates/glusterfs-sync.sh").to_string(),
                permissions: Some("0755".to_string()),
            }
        ],
        mounts: vec!["[ \"localhost:/gv0\", \"/mnt/glusterfs/\", \"glusterfs\", \"defaults,_netdev\", \"0\", \"0\" ]".to_string()],
        runcmd: vec![
            "systemctl daemon-reload".to_string(),
            "systemctl enable glusterfs-sync.service".to_string(),
            "systemctl start glusterfs-sync.service".to_string()
        ],
        bootcmd: vec![]
    };

    merge_user_data(&mut default_user_data, user_provided);

    let network_config = NetworkConfig {
        version: 2,
        ethernets: [(
            "enp1s0".to_string(),
            EthernetConfig {
                dhcp4: false,
                addresses: vec![format!("{}/24", ip_address)],
                nameservers: Nameservers {
                    addresses: vec!["192.168.122.1".to_string()],
                },
                routes: vec![Route {
                    to: "0.0.0.0/0".to_string(),
                    via: "192.168.122.1".to_string(),
                }],
            },
        )]
        .iter()
        .cloned()
        .collect(),
    };

    let metadata = MetaData {
        instance_id: instance_id.to_string(),
        local_hostname: hostname.to_string(),
    };

    let profile_dir = Path::new("/var/lib/libvirt/profiles").join(instance_id);
    fs::create_dir_all(&profile_dir)?;
    fs::write(
        profile_dir.join("user-data.yaml"),
        serde_yml::to_string(&default_user_data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
    )?;
    fs::write(
        profile_dir.join("network-config.yaml"),
        serde_yml::to_string(&network_config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
    )?;

    fs::write(
        profile_dir.join("meta-data.yaml"),
        serde_yml::to_string(&metadata)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
    )?;

    Ok(())
}

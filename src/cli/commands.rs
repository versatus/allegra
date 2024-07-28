use clap::Subcommand;

use crate::params::ServiceType;
use crate::vm_types::VmType;


#[derive(Clone, Subcommand)]
pub enum AllegraCommands {
    #[command(name = "ssh")]
    Ssh {
        #[arg(long, short)]
        owner: String,
        #[arg(long, short)]
        name: String,
        #[arg(long, short, default_value="~/.ssh/id_rsa")]
        keypath: String,
        #[arg(long, short, default_value="root")]
        username: String, 
        #[arg(long, short)]
        endpoint: Option<String>,
    },
    #[command(name = "wallet")]
    Wallet {
        #[arg(long, short)]
        new: bool,
        #[arg(long, short)]
        display: bool,
        #[arg(long, short)]
        save: bool,
        #[arg(long, short, default_value = "")]
        path: String,
    },
    #[command(name = "create")]
    Create {
        #[arg(long, short)]
        name: String,
        #[arg(long, short)]
        distro: String,
        #[arg(long, short)]
        version: String,
        #[arg(long, short='t')]
        vmtype: VmType,
        #[arg(long)]
        memory: Option<String>,
        #[arg(long)]
        vcpus: Option<String>,
        #[arg(long)]
        cpu: Option<String>,
        #[arg(long)]
        metadata: Option<String>,
        #[arg(long)]
        os_variant: Option<String>,
        #[arg(long)]
        host_device: Vec<String>,
        #[arg(long)]
        network: Vec<String>,
        #[arg(long)]
        disk: Vec<String>,
        #[arg(long)]
        filesystem: Vec<String>,
        #[arg(long)]
        controller: Vec<String>,
        #[arg(long)]
        input: Vec<String>,
        #[arg(long)]
        graphics: Option<String>,
        #[arg(long)]
        sound: Option<String>,
        #[arg(long)]
        video: Option<String>,
        #[arg(long)]
        smartcard: Option<String>,
        #[arg(long)]
        redirdev: Vec<String>,
        #[arg(long)]
        memballoon: Option<String>,
        #[arg(long)]
        tpm: Option<String>,
        #[arg(long)]
        rng: Option<String>,
        #[arg(long)]
        panic: Option<String>,
        #[arg(long)]
        shmem: Option<String>,
        #[arg(long)]
        memdev: Vec<String>,
        #[arg(long)]
        vsock: Option<String>,
        #[arg(long)]
        iommu: Option<String>,
        #[arg(long)]
        watchdog: Option<String>,
        #[arg(long)]
        serial: Vec<String>,
        #[arg(long)]
        parallel: Vec<String>,
        #[arg(long)]
        channel: Vec<String>,
        #[arg(long)]
        console: Vec<String>,
        #[arg(long)]
        install: Option<String>,
        #[arg(long)]
        cdrom: Option<String>,
        #[arg(long)]
        location: Option<String>,
        #[arg(long)]
        pxe: bool,
        #[arg(long)]
        import: bool,
        #[arg(long)]
        boot: Option<String>,
        #[arg(long)]
        idmap: Option<String>,
        #[arg(long)]
        features: Vec<String>,
        #[arg(long)]
        clock: Option<String>,
        #[arg(long)]
        launch_security: Option<String>,
        #[arg(long)]
        numatune: Option<String>,
        #[arg(long)]
        boot_dev: Vec<String>,
        #[arg(long)]
        unattended: bool,
        #[arg(long)]
        print_xml: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        connect: Option<String>,
        #[arg(long)]
        virt_type: Option<String>,
        #[arg(long)]
        cloud_init: Option<String>,
        #[arg(long, short)]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short='f')]
        from_file: Option<bool>,
        #[arg(long, short='p')]
        path: Option<String>,
        #[arg(long, short='i')]
        kp_index: Option<usize>,
        #[arg(long, short='e')]
        endpoint: Option<String>,
    },
    #[command(name = "start")]
    Start {
        #[arg(long, short)]
        name: String,
        #[arg(long, short)]
        console: bool,
        #[arg(long, short)]
        stateless: bool,
        #[arg(long, short='k')]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short)]
        from_file: Option<bool>,
        #[arg(long, short)]
        path: Option<String>,
        #[arg(long, short)]
        kp_index: Option<usize>,
        #[arg(long, short)]
        endpoint: Option<String>,
    },
    #[command(name = "stop")]
    Stop {
        #[arg(long, short)]
        name: String,
        #[arg(long, short)]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short)]
        from_file: Option<bool>,
        #[arg(long, short)]
        path: Option<String>,
        #[arg(long, short)]
        kp_index: Option<usize>,
        #[arg(long, short)]
        endpoint: Option<String>,
    },
    #[command(name = "add-pubkey")]
    AddPubkey {
        #[arg(long, short)]
        name: String,
        #[arg(long, short='k')]
        pubkey: String,
        #[arg(long, short)]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short='f')]
        from_file: Option<bool>,
        #[arg(long, short='p')]
        path: Option<String>,
        #[arg(long, short='i')]
        kp_index: Option<usize>,
        #[arg(long, short)]
        endpoint: Option<String>,
    },
    #[command(name = "delete")]
    Delete {
        #[arg(long, short)]
        name: String,
        #[arg(long)]
        force: bool,
        #[arg(long, short)]
        interactive: bool,
        #[arg(long, short)]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short)]
        from_file: Option<bool>,
        #[arg(long, short)]
        path: Option<String>,
        #[arg(long, short)]
        kp_index: Option<usize>,
        #[arg(long, short)]
        endpoint: Option<String>,
    },
    #[command(name = "expose-service")]
    ExposeService {
        #[arg(long, short)]
        name: String,
        #[arg(long, short)]
        port: Vec<u16>,
        #[arg(long, short='t')]
        service_type: Vec<ServiceType>,
        #[arg(long, short)]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short)]
        from_file: Option<bool>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long, short)]
        kp_index: Option<usize>,
        #[arg(long, short)]
        endpoint: Option<String>,
    },
    #[command(name = "get-ssh")]
    GetSshDetails {
        #[arg(long, short)]
        name: String,
        #[arg(long, short)]
        owner: String,
        #[arg(long, short)]
        keypath: Option<String>,
        #[arg(long, short)]
        username: Option<String>,
        #[arg(long, short)]
        endpoint: Option<String>,
    },
    #[command(name = "poll-task")]
    PollTask {
        #[arg(long, short)]
        owner: String,
        #[arg(long, short)]
        task_id: String,
        #[arg(long, short)]
        endpoint: Option<String>,
    }
}

impl AllegraCommands {
    pub fn from_file(&self) -> Option<bool> {
        match self {
            Self::Create { from_file, .. } => from_file.clone(),
            Self::Stop { from_file, .. } => from_file.clone(),
            Self::Start { from_file, .. } => from_file.clone(),
            Self::Delete { from_file, .. } => from_file.clone(),
            Self::AddPubkey { from_file, .. } => from_file.clone(),
            Self::ExposeService { from_file, .. } => from_file.clone(),
            Self::GetSshDetails { .. } => None,
            Self::PollTask { .. } => None,
            Self::Wallet { .. } => None,
            Self::Ssh { .. } => None,
        }
    }

    pub fn sk(&self) -> Option<String> {
        match self {
            Self::Create { sk, .. } => sk.clone(),
            Self::Stop { sk, .. } => sk.clone(),
            Self::Start { sk, .. } => sk.clone(),
            Self::Delete { sk, .. } => sk.clone(),
            Self::AddPubkey { sk, .. } => sk.clone(),
            Self::ExposeService { sk, .. } => sk.clone(),
            Self::GetSshDetails { .. } => None,
            Self::PollTask { .. } => None,
            Self::Wallet { .. } => None,
            Self::Ssh { .. } => None,
        }

    }

    pub fn mnemonic(&self) -> Option<String> {
        match self {
            Self::Create { mnemonic, .. } => mnemonic.clone(),
            Self::Stop { mnemonic, .. } => mnemonic.clone(),
            Self::Start { mnemonic, .. } => mnemonic.clone(),
            Self::Delete { mnemonic, .. } => mnemonic.clone(),
            Self::AddPubkey { mnemonic, .. } => mnemonic.clone(),
            Self::ExposeService { mnemonic, .. } => mnemonic.clone(),
            Self::GetSshDetails { .. } => None,
            Self::PollTask { .. } => None,
            Self::Wallet { .. } => None,
            Self::Ssh { .. } => None,
        }
    }

    pub fn path(&self) -> Option<String> {
        match self {
            Self::Create { path, .. } => path.clone(),
            Self::Stop { path, .. } => path.clone(),
            Self::Start { path, .. } => path.clone(),
            Self::Delete { path, .. } => path.clone(),
            Self::AddPubkey { path, .. } => path.clone(),
            Self::ExposeService { path, .. } => path.clone(),
            Self::GetSshDetails { .. } => None,
            Self::PollTask { .. } => None,
            Self::Wallet { .. } => None,
            Self::Ssh { .. } => None,
        }
    }

    pub fn kp_index(&self) -> Option<usize> {
        match self {
            Self::Create { kp_index, .. } => kp_index.clone(),
            Self::Stop { kp_index, .. } => kp_index.clone(),
            Self::Start { kp_index, .. } => kp_index.clone(),
            Self::Delete { kp_index, .. } => kp_index.clone(),
            Self::AddPubkey { kp_index, .. } => kp_index.clone(),
            Self::ExposeService { kp_index, .. } => kp_index.clone(),
            Self::GetSshDetails { .. } => None,
            Self::PollTask { .. } => None,
            Self::Wallet { .. } => None,
            Self::Ssh { .. } => None 
        }
    }
}

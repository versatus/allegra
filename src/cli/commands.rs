use clap::Subcommand;
use crate::vm_types::VmType;

#[derive(Clone, Subcommand)]
pub enum AllegraCommands {
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
        #[arg(long, short)]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short)]
        from_file: Option<bool>,
        #[arg(long, short)]
        path: Option<String>,
        #[arg(long, short)]
        kp_index: Option<usize>
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
        kp_index: Option<usize>
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
        kp_index: Option<usize>
    },
    #[command(name = "add-pubkey")]
    AddPubkey {
        #[arg(long, short)]
        name: String,
        #[arg(long, short)]
        pubkey: String,
        #[arg(long, short)]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short)]
        from_file: Option<bool>,
        #[arg(long, short)]
        path: Option<String>,
        #[arg(long, short)]
        kp_index: Option<usize>
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
        kp_index: Option<usize>
    },
    #[command(name = "expose-ports")]
    ExposePorts {
        #[arg(long, short)]
        name: String,
        #[arg(long, short)]
        port: Vec<u16>,
        #[arg(long, short)]
        sk: Option<String>,
        #[arg(long, short)]
        mnemonic: Option<String>,
        #[arg(long, short)]
        from_file: Option<bool>,
        #[arg(long, short)]
        path: Option<String>,
        #[arg(long, short)]
        kp_index: Option<usize>
    },
    #[command(name = "get-ssh")]
    GetSshDetails {
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
        kp_index: Option<usize>
    },
    #[command(name = "poll-task")]
    PollTask {
        #[arg(long, short)]
        owner: String,
        #[arg(long, short)]
        task_id: String
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
            Self::ExposePorts { from_file, .. } => from_file.clone(),
            Self::GetSshDetails { from_file, .. } => from_file.clone(),
            Self::PollTask { .. } => None,
            Self::Wallet { .. } => None
        }
    }

    pub fn sk(&self) -> Option<String> {
        match self {
            Self::Create { sk, .. } => sk.clone(),
            Self::Stop { sk, .. } => sk.clone(),
            Self::Start { sk, .. } => sk.clone(),
            Self::Delete { sk, .. } => sk.clone(),
            Self::AddPubkey { sk, .. } => sk.clone(),
            Self::ExposePorts { sk, .. } => sk.clone(),
            Self::GetSshDetails { sk, .. } => sk.clone(),
            Self::PollTask { .. } => None,
            Self::Wallet { .. } => None
        }

    }

    pub fn mnemonic(&self) -> Option<String> {
        match self {
            Self::Create { mnemonic, .. } => mnemonic.clone(),
            Self::Stop { mnemonic, .. } => mnemonic.clone(),
            Self::Start { mnemonic, .. } => mnemonic.clone(),
            Self::Delete { mnemonic, .. } => mnemonic.clone(),
            Self::AddPubkey { mnemonic, .. } => mnemonic.clone(),
            Self::ExposePorts { mnemonic, .. } => mnemonic.clone(),
            Self::GetSshDetails { mnemonic, .. } => mnemonic.clone(),
            Self::PollTask { .. } => None,
            Self::Wallet { .. } => None
        }
    }

    pub fn path(&self) -> Option<String> {
        match self {
            Self::Create { path, .. } => path.clone(),
            Self::Stop { path, .. } => path.clone(),
            Self::Start { path, .. } => path.clone(),
            Self::Delete { path, .. } => path.clone(),
            Self::AddPubkey { path, .. } => path.clone(),
            Self::ExposePorts { path, .. } => path.clone(),
            Self::GetSshDetails { path, .. } => path.clone(),
            Self::PollTask { .. } => None,
            Self::Wallet { .. } => None
        }
    }

    pub fn kp_index(&self) -> Option<usize> {
        match self {
            Self::Create { kp_index, .. } => kp_index.clone(),
            Self::Stop { kp_index, .. } => kp_index.clone(),
            Self::Start { kp_index, .. } => kp_index.clone(),
            Self::Delete { kp_index, .. } => kp_index.clone(),
            Self::AddPubkey { kp_index, .. } => kp_index.clone(),
            Self::ExposePorts { kp_index, .. } => kp_index.clone(),
            Self::GetSshDetails { kp_index, .. } => kp_index.clone(),
            Self::PollTask { .. } => None,
            Self::Wallet { .. } => None
        }
    }
}


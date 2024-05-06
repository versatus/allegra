use crate::vm_types::VmType;
use clap::Args;
use serde::{Serialize, Deserialize};

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

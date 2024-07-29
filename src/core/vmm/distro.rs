use std::{path::PathBuf, str::FromStr};
use crate::allegra_rpc::Distro as ProtoDistro;
use clap::ValueEnum;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, ValueEnum)]
pub enum Distro {
    Ubuntu,
    CentOS,
    Fedora,
    Debian,
    Arch,
    Alpine,
}

impl From<Distro> for PathBuf {
    fn from(value: Distro) -> Self {
        match value {
            Distro::Ubuntu => {
                PathBuf::from("/mnt/glusterfs/images/ubuntu/ubuntu-22.04.qcow2")
            }
            Distro::CentOS => {
                PathBuf::from("/mnt/glusterfs/images/centos/centos-8.qcow2")
            }
            Distro::Fedora => {
                PathBuf::from("/mnt/glusterfs/images/fedora/fedora-40.qcow2")
            }
            Distro::Debian => {
                PathBuf::from("/mnt/glusterfs/images/debian/debian-11.qcow2")
            }
            Distro::Arch => {
                PathBuf::from("/mnt/glusterfs/images/arch/arch-linux-x86_64.qcow2")
            }
            Distro::Alpine => {
                PathBuf::from("/mnt/glusterfs/images/alpine/alpine-3.20.qcow2")
            }
        }
    }
}

impl FromStr for Distro {
    type Err = std::io::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ubuntu" => Ok(Distro::Ubuntu),
            "centos" => Ok(Distro::CentOS),
            "fedora" => Ok(Distro::Fedora),
            "debian" => Ok(Distro::Debian),
            "arch" => Ok(Distro::Arch),
            "alpine" => Ok(Distro::Alpine),
            _ => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unsupported distro {s}")
                ))
            }
        }
    }
}

impl From<ProtoDistro> for Distro {
    fn from(value: ProtoDistro) -> Self {
        match value {
            ProtoDistro::Arch => Distro::Arch,
            ProtoDistro::Ubuntu => Distro::Ubuntu,
            ProtoDistro::Centos => Distro::CentOS,
            ProtoDistro::Fedora => Distro::Fedora,
            ProtoDistro::Debian => Distro::Debian,
            ProtoDistro::Alpine => Distro::Alpine
        }
    }
}

impl From<Distro> for ProtoDistro {
    fn from(value: Distro) -> Self {
        match value {
            Distro::Alpine => ProtoDistro::Alpine,
            Distro::Arch => ProtoDistro::Arch,
            Distro::CentOS => ProtoDistro::Centos,
            Distro::Debian => ProtoDistro::Debian,
            Distro::Fedora => ProtoDistro::Fedora,
            Distro::Ubuntu => ProtoDistro::Ubuntu
        }
    }
}

impl From<Distro> for i32 {
    fn from(value: Distro) -> Self {
        match value {
            Distro::Ubuntu => 0,
            Distro::CentOS => 1,
            Distro::Fedora => 2,
            Distro::Debian => 3,
            Distro::Arch => 4,
            Distro::Alpine => 5
        }
    }
}

impl From<&Distro> for i32 {
    fn from(value: &Distro) -> Self {
        match value {
            Distro::Ubuntu => 0,
            Distro::CentOS => 1,
            Distro::Fedora => 2,
            Distro::Debian => 3,
            Distro::Arch => 4,
            Distro::Alpine => 5
        }
    }
}


impl TryFrom<i32> for Distro {
    type Error = std::io::Error;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Distro::Ubuntu),
            1 => Ok(Distro::CentOS),
            2 => Ok(Distro::Fedora),
            3 => Ok(Distro::Debian),
            4 => Ok(Distro::Arch),
            5 => Ok(Distro::Alpine),
            _ => Err(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid value"
                )
            )
        }
    }
}

impl TryFrom<&i32> for Distro {
    type Error = std::io::Error;
    fn try_from(value: &i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Distro::Ubuntu),
            1 => Ok(Distro::CentOS),
            2 => Ok(Distro::Fedora),
            3 => Ok(Distro::Debian),
            4 => Ok(Distro::Arch),
            5 => Ok(Distro::Alpine),
            _ => Err(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid value"
                )
            )
        }
    }
}

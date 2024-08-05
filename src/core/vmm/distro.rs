use crate::allegra_rpc::Distro as ProtoDistro;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::PathBuf, str::FromStr};
use derive_more::Display;
use sha3::{Sha3_512, Digest};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{Rng, thread_rng};

pub trait DistroType: Display + Default + std::fmt::Debug {
    fn default_username() -> String {
        Self::default().to_string()
    }
    fn default_password() -> String {
        generate_password_hash(
            &Self::default().to_string(), 
            &generate_salt()
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Display, Default)]
pub struct Ubuntu;
#[derive(Clone, Debug, Serialize, Deserialize, Display, Default)]
pub struct Centos;
#[derive(Clone, Debug, Serialize, Deserialize, Display, Default)]
pub struct Fedora;
#[derive(Clone, Debug, Serialize, Deserialize, Display, Default)]
pub struct Debian;
#[derive(Clone, Debug, Serialize, Deserialize, Display, Default)]
pub struct Arch;
#[derive(Clone, Debug, Serialize, Deserialize, Display, Default)]
pub struct Alpine;
#[derive(Clone, Debug, Serialize, Deserialize, Display, Default)]
pub struct Other;

impl DistroType for Ubuntu {}
impl DistroType for Centos {}
impl DistroType for Fedora {}
impl DistroType for Debian {}
impl DistroType for Arch {}
impl DistroType for Alpine {}
impl DistroType for Other {}


#[derive(Clone, Debug, Serialize, Deserialize, ValueEnum)]
pub enum Distro {
    Ubuntu,
    CentOS,
    Fedora,
    Debian,
    Arch,
    Alpine,
    Other,
}

impl Display for Distro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Distro::Ubuntu => write!(f, "ubuntu"),
            Distro::CentOS => write!(f, "centos"),
            Distro::Fedora => write!(f, "fedora"),
            Distro::Debian => write!(f, "debian"),
            Distro::Arch => write!(f, "arch"),
            Distro::Alpine => write!(f, "alpine"),
            Distro::Other => write!(f, "linux")
        }
    }
}

impl From<Distro> for PathBuf {
    fn from(value: Distro) -> Self {
        match value {
            Distro::Ubuntu => PathBuf::from("/var/lib/libvirt/images/ubuntu/ubuntu-22.04.qcow2"),
            Distro::CentOS => PathBuf::from("/var/lib/libvirt/images/centos/centos-8.qcow2"),
            Distro::Fedora => PathBuf::from("/var/lib/libvirt/images/fedora/fedora-40.qcow2"),
            Distro::Debian => PathBuf::from("/var/lib/libvirt/images/debian/debian-11.qcow2"),
            Distro::Arch => PathBuf::from("/var/lib/libvirt/images/arch/arch-linux-x86_64.qcow2"),
            Distro::Alpine => PathBuf::from("/var/lib/libvirt//images/alpine/alpine-3.20.qcow2"),
            Distro::Other => PathBuf::from("/var/lib/libvirt/images/ubuntu/ubuntu-22.04.qcow2"),
        }
    }
}

impl From<i32> for Distro {
    fn from(value: i32) -> Self {
        match value {
            0 => Distro::Ubuntu,
            1 => Distro::CentOS,
            2 => Distro::Fedora,
            3 => Distro::Debian,
            4 => Distro::Arch,
            5 => Distro::Alpine,
            _ => Distro::Other,
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
            "other" => Ok(Distro::Other),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unsupported distro {s}"),
            )),
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
            ProtoDistro::Alpine => Distro::Alpine,
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
            Distro::Ubuntu => ProtoDistro::Ubuntu,
            Distro::Other => ProtoDistro::Ubuntu,
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
            Distro::Alpine => 5,
            Distro::Other => 0,
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
            Distro::Alpine => 5,
            Distro::Other => 0,
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
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid value",
            )),
        }
    }
}

fn generate_salt() -> String {
    let mut rng = thread_rng();
    let salt: [u8; 16] = rng.gen();
    URL_SAFE_NO_PAD.encode(&salt)
}

fn generate_password_hash(password: &str, salt: &str) -> String {
    let mut hasher = Sha3_512::new();
    hasher.update(format!("{}{}", salt, password));
    let hash = hasher.finalize();
    format!("$6${}${}", salt, URL_SAFE_NO_PAD.encode(&hash))
}

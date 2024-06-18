use std::any::Any;
use clap::ValueEnum;
use serde::{Serialize, Deserialize};
use crate::vm_types::VmType;
use crate::allegra_rpc::{
    GetTaskStatusRequest, InstanceAddPubkeyParams as ProtoAddPubkey, InstanceCreateParams as ProtoCreate, InstanceDeleteParams as ProtoDelete, InstanceExposeServiceParams as ProtoExpose, InstanceGetSshDetails as ProtoGetSsh, InstanceStartParams as ProtoStart, InstanceStopParams as ProtoStop, ServiceType as ProtoServiceType
};

#[derive(Clone, Debug, Serialize, Deserialize, ValueEnum)]
pub enum ServiceType {
    Ssh,
    NodeJs,
    Postgres,
    MySQL,
    Redis,
    MongoDB,
    RabbitMQ,
    Kafka,
    Custom
}

impl From<ServiceType> for i32 {
    fn from(value: ServiceType) -> Self {
        match value {
            ServiceType::Ssh => 0,
            ServiceType::NodeJs => 1,
            ServiceType::Postgres => 2,
            ServiceType::MySQL => 3,
            ServiceType::Redis => 4,
            ServiceType::MongoDB => 5,
            ServiceType::RabbitMQ => 6,
            ServiceType::Kafka => 7,
            ServiceType::Custom => 8,
        }
    }
}


impl From<i32> for ServiceType {
    fn from(val: i32) -> Self {
        match ProtoServiceType::try_from(val) {
            Ok(ProtoServiceType::Ssh) => ServiceType::Ssh,
            Ok(ProtoServiceType::NodeJs) => ServiceType::NodeJs,
            Ok(ProtoServiceType::Postgres) => ServiceType::Postgres,
            Ok(ProtoServiceType::Mysql) => ServiceType::MySQL,
            Ok(ProtoServiceType::Redis) => ServiceType::Redis,
            Ok(ProtoServiceType::MongoDb) => ServiceType::MongoDB,
            Ok(ProtoServiceType::RabbitMq) => ServiceType::RabbitMQ,
            Ok(ProtoServiceType::Kafka) => ServiceType::Kafka,
            Ok(ProtoServiceType::Custom) => ServiceType::Custom,
            Err(_) => ServiceType::Custom
        }
    }
}

impl From<ProtoServiceType> for ServiceType {
    fn from(val: ProtoServiceType) -> Self {
        match val {
            ProtoServiceType::Ssh => ServiceType::Ssh,
            ProtoServiceType::NodeJs => ServiceType::NodeJs,
            ProtoServiceType::Postgres => ServiceType::Postgres,
            ProtoServiceType::Mysql => ServiceType::MySQL,
            ProtoServiceType::Redis => ServiceType::Redis,
            ProtoServiceType::MongoDb => ServiceType::MongoDB,
            ProtoServiceType::RabbitMq => ServiceType::RabbitMQ,
            ProtoServiceType::Kafka => ServiceType::Kafka,
            ProtoServiceType::Custom => ServiceType::Custom,
        }
    }
}


impl ServiceType {
    pub fn default_port(&self) -> Option<u16> {
        match *self {
            ServiceType::Ssh => Some(22),
            ServiceType::NodeJs => Some(3000),
            ServiceType::Postgres => Some(5432),
            ServiceType::MySQL => Some(3306),
            ServiceType::Redis => Some(6379),
            ServiceType::MongoDB => Some(27017),
            ServiceType::RabbitMQ => Some(5672),
            ServiceType::Kafka => Some(9092),
            ServiceType::Custom => None,
        }
    }
}

impl TryFrom<ProtoStart> for InstanceStartParams {
    type Error = std::io::Error;
    fn try_from(value: ProtoStart) -> Result<Self, Self::Error> {
        Ok(InstanceStartParams {
            name: value.name,
            console: value.console,
            stateless: value.stateless,
            sig: value.sig,
            recovery_id: value.recovery_id.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        })
    }
}

impl TryFrom<ProtoCreate> for InstanceCreateParams {
    type Error = std::io::Error;
    fn try_from(value: ProtoCreate) -> Result<Self, Self::Error> {
        Ok(InstanceCreateParams {
            name: value.name,
            distro: value.distro,
            version: value.version,
            vmtype: serde_json::from_str(&value.vmtype).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?,
            sig: value.sig,
            recovery_id: value.recovery_id.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        })
    }
}

impl TryFrom<ProtoStop> for InstanceStopParams {
    type Error = std::io::Error;
    fn try_from(value: ProtoStop) -> Result<Self, Self::Error> {
        Ok(InstanceStopParams { 
            name: value.name,
            sig: value.sig,
            recovery_id: value.recovery_id.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        })
    }
}

impl TryFrom<ProtoDelete> for InstanceDeleteParams {
    type Error = std::io::Error;
    fn try_from(value: ProtoDelete) -> Result<Self, Self::Error> {
        Ok(InstanceDeleteParams { 
            name: value.name, 
            force: value.force, 
            interactive: value.interactive, 
            sig: value.sig, 
            recovery_id: value.recovery_id.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        })
    }
}

impl TryFrom<ProtoExpose> for InstanceExposeServiceParams {
    type Error = std::io::Error;
    fn try_from(value: ProtoExpose) -> Result<Self, Self::Error> {
        Ok(InstanceExposeServiceParams { 
            name: value.name, 
            port: value.port.iter().filter_map(|n| {
                let n = *n;
                n.try_into().ok()
            }).collect(), 
            service_type: value.service_type.iter().map(|s| {
                let s = *s;
                s.into()
            }).collect(), 
            sig: value.sig, 
            recovery_id: value.recovery_id.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        })
    }
}

impl TryFrom<ProtoGetSsh> for InstanceGetSshDetails {
    type Error = std::io::Error;
    fn try_from(value: ProtoGetSsh) -> Result<Self, Self::Error> {
        Ok(InstanceGetSshDetails { 
            owner: value.owner, 
            name: value.name, 
            keypath: value.keypath, 
            username: value.username, 
        })
    }
}

impl TryFrom<ProtoAddPubkey> for InstanceAddPubkeyParams {
    type Error = std::io::Error;
    fn try_from(value: ProtoAddPubkey) -> Result<Self, Self::Error> {
        Ok(InstanceAddPubkeyParams { 
            name: value.name, 
            pubkey: value.pubkey, 
            sig: value.sig, 
            recovery_id: value.recovery_id.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        })
    }
}


impl TryFrom<ProtoStart> for Params {
    type Error = std::io::Error;
    fn try_from(value: ProtoStart) -> Result<Self, Self::Error> {
        Ok(Params::Start(InstanceStartParams {
            name: value.name,
            console: value.console,
            stateless: value.stateless,
            sig: value.sig,
            recovery_id: value.recovery_id.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        }))
    }
}

impl TryFrom<ProtoCreate> for Params {
    type Error = std::io::Error;
    fn try_from(value: ProtoCreate) -> Result<Self, Self::Error> {
        Ok(Params::Create(InstanceCreateParams {
            name: value.name,
            distro: value.distro,
            version: value.version,
            vmtype: serde_json::from_str(&value.vmtype).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?,
            sig: value.sig,
            recovery_id: value.recovery_id.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        }))
    }
}

impl TryFrom<ProtoStop> for Params {
    type Error = std::io::Error;
    fn try_from(value: ProtoStop) -> Result<Self, Self::Error> {
        Ok(Params::Stop(InstanceStopParams { 
            name: value.name,
            sig: value.sig,
            recovery_id: value.recovery_id.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        }))
    }
}

impl TryFrom<ProtoDelete> for Params {
    type Error = std::io::Error;
    fn try_from(value: ProtoDelete) -> Result<Self, Self::Error> {
        Ok(Params::Delete(InstanceDeleteParams { 
            name: value.name, 
            force: value.force, 
            interactive: value.interactive, 
            sig: value.sig, 
            recovery_id: value.recovery_id.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        }))
    }
}

impl TryFrom<ProtoExpose> for Params {
    type Error = std::io::Error;
    fn try_from(value: ProtoExpose) -> Result<Self, Self::Error> {
        Ok(Params::ExposeService(InstanceExposeServiceParams { 
            name: value.name, 
            port: value.port.iter().filter_map(|n| {
                let n = *n;
                n.try_into().ok()
            }).collect(), 
            service_type: value.service_type.iter().map(|s| {
                let s = *s;
                s.into()
            }).collect(), 
            sig: value.sig, 
            recovery_id: value.recovery_id.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        }))
    }
}

impl TryFrom<ProtoGetSsh> for  Params {
    type Error = std::io::Error;
    fn try_from(value: ProtoGetSsh) -> Result<Self, Self::Error> {
        Ok(Params::GetSshDetails(InstanceGetSshDetails { 
            owner: value.owner, 
            name: value.name, 
            keypath: value.keypath, 
            username: value.username, 
        }))
    }
}

impl TryFrom<ProtoAddPubkey> for Params {
    type Error = std::io::Error;
    fn try_from(value: ProtoAddPubkey) -> Result<Self, Self::Error> {
        Ok(Params::AddPubkey(InstanceAddPubkeyParams { 
            name: value.name, 
            pubkey: value.pubkey, 
            sig: value.sig, 
            recovery_id: value.recovery_id.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        }))
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Params {
    Create(InstanceCreateParams),
    Start(InstanceStartParams),
    Stop(InstanceStopParams),
    AddPubkey(InstanceAddPubkeyParams),
    Delete(InstanceDeleteParams),
    ExposeService(InstanceExposeServiceParams),
    GetSshDetails(InstanceGetSshDetails)
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceCreateParams {
    pub name: String,
    pub distro: String,
    pub version: String,
    pub vmtype: VmType,
    pub sig: String,
    pub recovery_id: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceStartParams {
    pub name: String,
    pub console: bool,
    pub stateless: bool,
    pub sig: String,
    pub recovery_id: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceStopParams {
    pub name: String, 
    pub sig: String,
    pub recovery_id: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceAddPubkeyParams {
    pub name: String,
    pub pubkey: String,
    pub sig: String,
    pub recovery_id: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceDeleteParams {
    pub name: String,
    pub force: bool,
    pub interactive: bool,
    pub sig: String,
    pub recovery_id: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceExposeServiceParams {
    pub name: String,
    pub port: Vec<u16>,
    pub service_type: Vec<ServiceType>,
    pub sig: String,
    pub recovery_id: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceGetSshDetails {
    pub owner: String,
    pub name: String,
    pub keypath: Option<String>,
    pub username: Option<String>
}

pub struct InstanceSshSession {
    pub name: String,
}

pub trait HasOwner {
    fn owner(&self) -> std::io::Result<[u8; 20]>;
}

impl HasOwner for InstanceGetSshDetails {
    fn owner(&self) -> std::io::Result<[u8; 20]> {
        let mut buffer = [0u8; 20];
        if self.owner.starts_with("0x") {
            let bytes = hex::decode(&self.owner[2..]).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            buffer.copy_from_slice(&bytes[..]);
        } else {
            let bytes = hex::decode(&self.owner).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            buffer.copy_from_slice(&bytes[..]);
        }

        return Ok(buffer)
    }
}

impl HasOwner for ProtoGetSsh {
    fn owner(&self) -> std::io::Result<[u8; 20]> {
        let mut buffer = [0u8; 20];
        if self.owner.starts_with("0x") {
            let bytes = hex::decode(&self.owner[2..]).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            buffer.copy_from_slice(&bytes[..]);
        } else {
            let bytes = hex::decode(&self.owner).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            buffer.copy_from_slice(&bytes[..]);
        }

        return Ok(buffer)
    }
}

impl HasOwner for GetTaskStatusRequest {
    fn owner(&self) -> std::io::Result<[u8; 20]> {
        let mut buffer = [0u8; 20];
        if self.owner.starts_with("0x") {
            let owner_string = &self.owner[2..];
            let bytes = hex::decode(owner_string).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            buffer.copy_from_slice(&bytes[..]);
        } else {
            let bytes = hex::decode(&self.owner).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            buffer.copy_from_slice(&bytes[..]);
        }

        Ok(buffer)
    }
}

pub trait Payload: Any {
    fn into_payload(&self) -> String;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

impl Payload for InstanceCreateParams {
    fn into_payload(&self) -> String {
        serde_json::json!({
            "command": "launch",
            "name": &self.name,
            "distro": &self.distro,
            "version": &self.version,
            "vmtype": &self.vmtype,
        }).to_string()
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl Payload for InstanceStopParams {
    fn into_payload(&self) -> String {
        serde_json::json!({
            "command": "stop",
            "name": &self.name, 
        }).to_string()
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl Payload for InstanceStartParams {
    fn into_payload(&self) -> String {
        serde_json::json!({
            "command": "start",
            "name": &self.name,
            "console": self.console,
            "stateless": self.stateless
        }).to_string()

    }
    
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl Payload for InstanceDeleteParams {
    fn into_payload(&self) -> String {
        serde_json::json!({
            "command": "delete",
            "name": self.name
        }).to_string()
    }
    
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl Payload for InstanceAddPubkeyParams {
    fn into_payload(&self) -> String {
        serde_json::json!({
            "command": "injectAuth",
            "name": &self.name,
            "pubkey": &self.pubkey
        }).to_string()
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl Payload for InstanceExposeServiceParams {
    fn into_payload(&self) -> String {
        serde_json::json!({
            "command": "exposePort",
            "name": &self.name,
            "ports": &self.port,
            "services": &self.service_type
        }).to_string()
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl Payload for InstanceGetSshDetails {
    fn into_payload(&self) -> String {
        serde_json::json!({
            "command": "getSshDetails",
            "name": &self.name
        }).to_string()
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

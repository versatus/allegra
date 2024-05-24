use std::any::Any;
use clap::ValueEnum;
use serde::{Serialize, Deserialize};
use crate::vm_types::VmType;

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

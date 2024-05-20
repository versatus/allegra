use std::any::Any;

use serde::{Serialize, Deserialize};
use crate::vm_types::VmType;

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
pub struct InstanceExposePortParams {
    pub name: String,
    pub port: Vec<u16>,
    pub sig: String,
    pub recovery_id: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceGetSshDetails {
    pub name: String,
    pub sig: String,
    pub recovery_id: u8,
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

impl Payload for InstanceExposePortParams {
    fn into_payload(&self) -> String {
        serde_json::json!({
            "command": "exposePort",
            "name": &self.name,
            "ports": &self.port,
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

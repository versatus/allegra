use crate::{
    params::Payload,
    allegra_rpc::{
        InstanceCreateParams,
        InstanceStopParams,
        InstanceDeleteParams,
        InstanceExposeServiceParams,
        InstanceGetSshDetails,
        InstanceAddPubkeyParams,
        InstanceStartParams, GetTaskStatusRequest
    }
};

use std::any::Any;


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

impl Payload for GetTaskStatusRequest {
    fn into_payload(&self) -> String {
        serde_json::json!({
            "owner": self.owner,
            "id": self.id,
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

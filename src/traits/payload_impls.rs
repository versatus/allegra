use crate::allegra_rpc::{
    GetTaskStatusRequest, InstanceAddPubkeyParams, InstanceCreateParams, InstanceDeleteParams,
    InstanceExposeServiceParams, InstanceGetSshDetails, InstanceStartParams, InstanceStopParams,
    NewPeerMessage, NodeCertMessage, ServerConfigMessage,
};

use std::any::Any;

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
        })
        .to_string()
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
        })
        .to_string()
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
        })
        .to_string()
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
        })
        .to_string()
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
        })
        .to_string()
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
        })
        .to_string()
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
        })
        .to_string()
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
        })
        .to_string()
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl Payload for NewPeerMessage {
    fn into_payload(&self) -> String {
        serde_json::json!({
            "command": "newPeerMessage",
            "newPeerId": self.new_peer_id,
            "newPeerAddress": self.new_peer_address,
        })
        .to_string()
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl Payload for NodeCertMessage {
    fn into_payload(&self) -> String {
        serde_json::json!({
            "command": "newCertificate",
            "peerId": self.peer_id,
            "peerAddress": self.peer_address,
            "certificate": self.cert
        })
        .to_string()
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl Payload for ServerConfigMessage {
    fn into_payload(&self) -> String {
        serde_json::json!({
            "command": "syncServerConfig",
            "serverConfig": self.server_config
        })
        .to_string()
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

use std::collections::VecDeque;

use serde::{Serialize, Deserialize};
use tokio::sync::broadcast::{Sender, Receiver};
use crate::{params::ServiceType, vm_types::VmType, account::Namespace, allegra_rpc::SshDetails};

#[derive(Clone, Debug)]
pub struct Topic {
    name: String,
    queue: VecDeque<Event>,
    subscribers: Sender<Event>
}

impl Topic {
    pub fn new(name: String, subscribers: Sender<Event>) -> Self {
        Self {
            name,
            queue: VecDeque::new(),
            subscribers
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn publish(&mut self, event: Event) {
        self.queue.push_back(event.clone());
        let _ = self.subscribers.send(event.clone());
    }

    pub fn subscribe(&mut self) -> Receiver<Event> {
        self.subscribers.subscribe()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    VmmEvent(VmmEvent),
    NetworkEvent(NetworkEvent),
    DnsEvent(DnsEvent),
    StateEvent(StateEvent),
    QuorumEvent(QuorumEvent),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VmmEvent {
    Create {
        name: String,
        distro: String,
        version: String,
        vmtype: VmType,
        sig: String,
        recovery_id: u8,
    },
    Start {
        name: String, 
        console: bool, 
        stateless: bool,
        sig: String, 
        recovery_id: u8
    },
    Stop {
        name: String,
        sig: String,
        recovery_id: u8
    },
    Delete {
        name: String,
        sig: String,
        recovery_id: u8,
        force: bool,
        interactive: bool,
    },
    ExposeService {
        name: String,
        sig: String,
        recovery_id: u8,
        port: Vec<u16>,
        service_type: Vec<ServiceType>
    },
    AddPubkey {
        name: String,
        sig: String,
        recovery_id: u8,
        pubkey: String,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkEvent {
    NewPeer {
        peer_id: String,
        peer_address: String,
        peer_number: u32,
        dst: String,
    },
    Create {
        name: String,
        distro: String,
        version: String,
        vmtype: String,
        sig: String,
        recovery_id: u8,
        dst: String,
    },
    Sync {
        namespace: String,
        path: String,
        target: String,
        last_update: Option<u64>,
        dst: String,
    }, //lxc copy --refresh
    Migrate {
        namespace: String,
        path: String,
        target: String,
        last_update: Option<u64>,
        new_quorum: String,
        dst: String,
    }, //lxc move
    ExposeService {
        name: String,
        sig: String, 
        recovery_id: u8,
        port: Vec<u16>,
        service_type: Vec<ServiceType>,
        dst: String,
    },
    Start {
        name: String,
        sig: String,
        recovery_id: u8,
        console: bool,
        stateless: bool,
        dst: String,
    },
    Stop {
        name: String, 
        sig: String,
        recovery_id: u8,
        dst: String,
    },
    Delete {
        name: String,
        force: bool,
        interactive: bool,
        sig: String,
        recovery_id: u8,
        dst: String,
    },
    AddPubkey {
        name: String,
        sig: String,
        recovery_id: u8,
        pubkey: String,
        dst: String,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DnsEvent {
    Register { 
        name: String,
        sig: String,
        recovery_id: u8,
        domain_name: String,
        // Add proof mechanism
    },
    Deregister {
        name: String,
        sig: String,
        recovery_id: u8,
        domain_name: String
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StateEvent {
    Put {
        key: Vec<u8>,
        value: Vec<u8> 
    },
    Get(Vec<u8>),
    Post {
        key: Vec<u8>,
        value: Vec<u8>
    },
    Delete(Vec<u8>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QuorumEvent {
    Expand {
        id: String,
        address: String
    },
    Consolidate(String),
    RequestSshDetails(Namespace) 
}


pub trait BrokerEvent {}

impl BrokerEvent for Event {} 
impl BrokerEvent for VmmEvent {} 
impl BrokerEvent for NetworkEvent {} 
impl BrokerEvent for DnsEvent {} 
impl BrokerEvent for StateEvent {} 
impl BrokerEvent for QuorumEvent {} 

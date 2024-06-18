use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use sha3::{Digest, Sha3_256};
use crate::{
    account::{ExposedPort, Namespace, TaskId, TaskStatus}, allegra_rpc::{
        InstanceAddPubkeyParams, InstanceCreateParams, InstanceDeleteParams, InstanceExposeServiceParams, InstanceGetSshDetails, InstanceStartParams, InstanceStopParams
    }, dht::Peer, helpers::{recover_namespace, recover_owner_address}, params::{HasOwner, Params, ServiceType}, vm_info::{VmInfo, VmList}, vm_types::VmType
};
use crate::params::Payload;

macro_rules! impl_into_event {
    ($($t:ty => $variant:ident),*) => {
        $(
            impl IntoEvent for $t {
                fn into_event(&self) -> Event {
                    Event::$variant(self.clone())
                }
                fn to_inner(self: Box<Self>) -> Event {
                    Event::$variant(*self)
                }
            }
        )*
    };
}

macro_rules! impl_send {
    ($($t:ty),*) => {
        $(
            unsafe impl Send for $t {}
        )*
    };
}

impl_into_event!(
    VmmEvent => VmmEvent,
    NetworkEvent => NetworkEvent,
    DnsEvent => DnsEvent,
    StateEvent => StateEvent,
    QuorumEvent => QuorumEvent,
    TaskStatusEvent => TaskStatusEvent,
    SyncEvent => SyncEvent,
    RpcResponseEvent => RpcResponseEvent,
    GeneralResponseEvent => GeneralResponseEvent
);

impl_send!(
    VmmEvent,
    NetworkEvent,
    DnsEvent,
    StateEvent,
    QuorumEvent,
    TaskStatusEvent,
    SyncEvent,
    RpcResponseEvent,
    GeneralResponseEvent
);

pub trait IntoEvent {
    fn into_event(&self) -> Event;
    fn to_inner(self: Box<Self>) -> Event;
}

pub trait SerializeIntoInner: Serialize {
    fn inner_to_string(&self) -> std::io::Result<String>;
}

impl SerializeIntoInner for Event {
    fn inner_to_string(&self) -> std::io::Result<String> {
        match self {
            Self::VmmEvent(event) => {
                serde_json::to_string(&event).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })
            }
            Self::NetworkEvent(event) => {
                serde_json::to_string(&event).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })
            }
            Self::DnsEvent(event) => {
                serde_json::to_string(&event).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })
            }
            Self::StateEvent(event) => {
                serde_json::to_string(&event).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })
            }
            Self::QuorumEvent(event) => {
                serde_json::to_string(&event).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })
            }
            Self::TaskStatusEvent(event) => {
                serde_json::to_string(&event).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })
            }
            Self::SyncEvent(event) => {
                serde_json::to_string(&event).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })
            }
            Self::RpcResponseEvent(event) => {
                serde_json::to_string(&event).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })
            }
            Self::GeneralResponseEvent(event) => {
                serde_json::to_string(&event).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    VmmEvent(VmmEvent),
    NetworkEvent(NetworkEvent),
    DnsEvent(DnsEvent),
    StateEvent(StateEvent),
    QuorumEvent(QuorumEvent),
    TaskStatusEvent(TaskStatusEvent),
    SyncEvent(SyncEvent),
    RpcResponseEvent(RpcResponseEvent),
    GeneralResponseEvent(GeneralResponseEvent)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TaskStatusEvent {
    Update {
        owner: [u8; 20],
        task_id: TaskId,
        task_status: TaskStatus,
        event_id: String,
    },
    Get { 
        owner: [u8; 20],
        task_id: TaskId,
        event_id: String,
        response_topics: Vec<String>
    }, 
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
    },
    DistributeCerts {
        certs: HashMap<Peer, String>,
    },
    ShareCert {
        peer: Peer,
        cert: String, 
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SyncEvent {
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
        event_id: String,
        task_id: TaskId,
        key: Vec<u8>,
        value: Vec<u8> 
    },
    Get{
        event_id: String,
        task_id: TaskId,
        key: Vec<u8>,
    },
    Post {
        event_id: String,
        task_id: TaskId,
        key: Vec<u8>,
        value: Vec<u8>
    },
    Delete {
        event_id: String,
        task_id: TaskId,
        key: Vec<u8>,
    },
    PutAccount {
        event_id: String,
        task_id: TaskId,
        task_status: TaskStatus,
        owner: [u8; 20],
        vmlist: VmList,
        namespace: Namespace,
        exposed_ports: Option<Vec<ExposedPort>>,
    },
    PutInstance {
        event_id: String,
        task_id: TaskId,
        task_status: TaskStatus,
        namespace: Namespace,
        vm_info: VmInfo,
        port_map: HashMap<u16, (u16, ServiceType)>,
    },
    GetAccount {
        event_id: String,
        task_id: TaskId,
        task_status: TaskStatus,
        owner: [u8; 20]
    },
    GetInstance {
        event_id: String,
        task_id: TaskId,
        task_status: TaskStatus,
        owner: [u8; 20]
    },
    PostAccount {
        event_id: String,
        task_id: TaskId,
        task_status: TaskStatus,
        owner: [u8; 20],
        vmlist: VmList,
        namespace: Namespace,
        exposed_ports: Vec<ExposedPort>,
    },
    PostInstance {
        event_id: String,
        task_id: TaskId,
        task_status: TaskStatus,
        namespace: Namespace,
        vm_info: VmInfo,
        port_map: HashMap<u16, (u16, ServiceType)>,
    },
    DeleteInstance {
        event_id: String,
        task_id: TaskId,
        task_status: TaskStatus,
        namespace: Namespace
    },
    PutTaskStatus {
        event_id: String,
        task_id: TaskId,
        task_status: TaskStatus,
    },
    PostTaskStatus {
        event_id: String,
        task_id: TaskId, 
        task_status: TaskStatus,
    },
    GetTaskStatus {
        event_id: String,
        task_id: TaskId,
    },
    DeleteTaskStatus {
        event_id: String,
        task_id: TaskId,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QuorumEvent {
    Expand {
        id: String,
        address: String
    },
    Consolidate(String),
    RequestSshDetails(Namespace),
    CheckResponsibility {
        namespace: Namespace,
        task_id: TaskId,
        payload: Params,
        event_id: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneralResponseEvent {
    event_id: String,
    original_event_id: String,
    response: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcResponseEvent {
    original_event_id: String,
    event_id: String,
    response: String, 
}

pub trait BrokerEvent {}

impl BrokerEvent for Event {} 
impl BrokerEvent for VmmEvent {} 
impl BrokerEvent for NetworkEvent {} 
impl BrokerEvent for DnsEvent {} 
impl BrokerEvent for StateEvent {} 
impl BrokerEvent for QuorumEvent {} 
impl BrokerEvent for SyncEvent {} 
impl BrokerEvent for TaskStatusEvent {} 
impl BrokerEvent for RpcResponseEvent {} 
impl BrokerEvent for GeneralResponseEvent {} 

impl TryFrom<(Peer, InstanceCreateParams)> for NetworkEvent {
    type Error = std::io::Error;
    fn try_from(value: (Peer, InstanceCreateParams)) -> Result<Self, Self::Error> {
        let recovery_id = value.1.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        Ok(NetworkEvent::Create { 
            name: value.1.name.clone(), 
            distro: value.1.distro.clone(), 
            version: value.1.version.clone(), 
            vmtype: value.1.vmtype.clone(), 
            sig: value.1.sig.clone(), 
            recovery_id,
            dst: value.0.address().to_string()
        })
    }
}


impl TryFrom<(Peer, InstanceStopParams)> for NetworkEvent {
    type Error = std::io::Error;
    fn try_from(value: (Peer, InstanceStopParams)) -> Result<Self, Self::Error> {
        let recovery_id = value.1.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        Ok(NetworkEvent::Stop { 
            name: value.1.name,
            sig: value.1.sig,
            recovery_id, 
            dst: value.0.address().to_string() 
        })
    }
}

impl TryFrom<(Peer, InstanceStartParams)> for NetworkEvent {
    type Error = std::io::Error;
    fn try_from(value: (Peer, InstanceStartParams)) -> Result<Self, Self::Error> {
        let recovery_id = value.1.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        Ok(NetworkEvent::Start { 
            name: value.1.name.clone(), 
            sig: value.1.sig.clone(), 
            recovery_id,
            console: value.1.console, 
            stateless: value.1.stateless, 
            dst: value.0.address().to_string(), 
        })
    }
}

impl TryFrom<(Peer, InstanceAddPubkeyParams)> for NetworkEvent {
    type Error = std::io::Error;
    fn try_from(value: (Peer, InstanceAddPubkeyParams)) -> Result<Self, Self::Error> {
        let recovery_id = value.1.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        Ok(NetworkEvent::AddPubkey { 
            name: value.1.name.clone(), 
            sig: value.1.sig.clone(), 
            recovery_id, 
            pubkey: value.1.pubkey, 
            dst: value.0.address().to_string() 
        })  
    }
}

impl TryFrom<(Peer, InstanceDeleteParams)> for NetworkEvent {
    type Error = std::io::Error;
    fn try_from(value: (Peer, InstanceDeleteParams)) -> Result<Self, Self::Error> {
        let recovery_id = value.1.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        Ok(NetworkEvent::Delete { 
            name: value.1.name.clone(), 
            force: value.1.force, 
            interactive: value.1.interactive, 
            sig: value.1.sig, 
            recovery_id, 
            dst: value.0.address().to_string() 
        }) 
    }
}

impl TryFrom<(Peer, InstanceExposeServiceParams)> for NetworkEvent {
    type Error = std::io::Error;
    fn try_from(value: (Peer, InstanceExposeServiceParams)) -> Result<Self, Self::Error> {
        let recovery_id = value.1.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        Ok(NetworkEvent::ExposeService { 
            name: value.1.name.clone(), 
            sig: value.1.sig.clone(), 
            recovery_id, 
            port: value.1.port.iter().filter_map(|p| {
                p.to_owned().try_into().ok()
            }).collect::<Vec<u16>>().clone(), 
            service_type: value.1.service_type.iter().map(|st| {
                st.to_owned().into()
            }).collect::<Vec<ServiceType>>().clone(), 
            dst: value.0.address().to_string() 
        }) 
    }
}

impl TryFrom<InstanceCreateParams> for Namespace {
    type Error = std::io::Error;

    fn try_from(params: InstanceCreateParams) -> Result<Self, Self::Error> {
        let payload = params.into_payload(); 
        log::info!("converted params into payload...");
        let mut hasher = Sha3_256::new();
        hasher.update(
            payload.as_bytes()
        );
        let hash = hasher.finalize().to_vec();
        log::info!("hashed params payload...");
        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        log::info!("converted recovery_id into u32...");
        let owner = recover_owner_address(hash, params.sig.clone(), recovery_id)?;
        let namespace = recover_namespace(owner, &params.name);

        Ok(namespace)
    }
}


impl TryFrom<InstanceStopParams> for Namespace {
    type Error = std::io::Error;

    fn try_from(params: InstanceStopParams) -> Result<Self, Self::Error> {
        let payload = params.into_payload(); 
        log::info!("converted params into payload...");
        let mut hasher = Sha3_256::new();
        hasher.update(
            payload.as_bytes()
        );
        let hash = hasher.finalize().to_vec();
        log::info!("hashed params payload...");
        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        log::info!("converted recovery_id into u32...");
        let owner = recover_owner_address(hash, params.sig.clone(), recovery_id)?;
        let namespace = recover_namespace(owner, &params.name);

        Ok(namespace)
    }
}


impl TryFrom<InstanceStartParams> for Namespace {
    type Error = std::io::Error;

    fn try_from(params: InstanceStartParams) -> Result<Self, Self::Error> {
        let payload = params.into_payload(); 
        log::info!("converted params into payload...");
        let mut hasher = Sha3_256::new();
        hasher.update(
            payload.as_bytes()
        );
        let hash = hasher.finalize().to_vec();
        log::info!("hashed params payload...");
        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        log::info!("converted recovery_id into u32...");
        let owner = recover_owner_address(hash, params.sig.clone(), recovery_id)?;
        let namespace = recover_namespace(owner, &params.name);

        Ok(namespace)
    }
}


impl TryFrom<InstanceDeleteParams> for Namespace {
    type Error = std::io::Error;

    fn try_from(params: InstanceDeleteParams) -> Result<Self, Self::Error> {
        let payload = params.into_payload(); 
        log::info!("converted params into payload...");
        let mut hasher = Sha3_256::new();
        hasher.update(
            payload.as_bytes()
        );
        let hash = hasher.finalize().to_vec();
        log::info!("hashed params payload...");
        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        log::info!("converted recovery_id into u32...");
        let owner = recover_owner_address(hash, params.sig.clone(), recovery_id)?;
        let namespace = recover_namespace(owner, &params.name);

        Ok(namespace)
    }
}

impl TryFrom<InstanceExposeServiceParams> for Namespace {
    type Error = std::io::Error;

    fn try_from(params: InstanceExposeServiceParams) -> Result<Self, Self::Error> {
        let payload = params.into_payload(); 
        log::info!("converted params into payload...");
        let mut hasher = Sha3_256::new();
        hasher.update(
            payload.as_bytes()
        );
        let hash = hasher.finalize().to_vec();
        log::info!("hashed params payload...");
        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        log::info!("converted recovery_id into u32...");
        let owner = recover_owner_address(hash, params.sig.clone(), recovery_id)?;
        let namespace = recover_namespace(owner, &params.name);

        Ok(namespace)
    }
}

impl TryFrom<InstanceAddPubkeyParams> for Namespace {
    type Error = std::io::Error;

    fn try_from(params: InstanceAddPubkeyParams) -> Result<Self, Self::Error> {
        let payload = params.into_payload(); 
        log::info!("converted params into payload...");
        let mut hasher = Sha3_256::new();
        hasher.update(
            payload.as_bytes()
        );
        let hash = hasher.finalize().to_vec();
        log::info!("hashed params payload...");
        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        log::info!("converted recovery_id into u32...");
        let owner = recover_owner_address(hash, params.sig.clone(), recovery_id)?;
        let namespace = recover_namespace(owner, &params.name);

        Ok(namespace)
    }
}


impl TryFrom<InstanceGetSshDetails> for Namespace {
    type Error = std::io::Error;

    fn try_from(params: InstanceGetSshDetails) -> Result<Self, Self::Error> {
        let owner = params.owner()?; 
        let namespace = recover_namespace(owner, &params.name);
        Ok(namespace)
    }
}

use std::{collections::{HashMap, HashSet}, net::SocketAddr};
use rayon::iter::{ParallelIterator, IntoParallelRefIterator};
use serde::{Serialize, Deserialize};
use sha3::{Digest, Sha3_256};
use uuid::Uuid;
use crate::{
    account::{
        ExposedPort,
        Namespace,
        TaskId,
        TaskStatus
    }, allegra_rpc::{
        InstanceAddPubkeyParams, 
        InstanceCreateParams, 
        InstanceDeleteParams, 
        InstanceExposeServiceParams, 
        InstanceGetSshDetails, 
        InstanceStartParams, 
        InstanceStopParams
    }, dht::{Peer, Quorum}, grpc_light::generate_task_id, helpers::{
            recover_namespace, 
            recover_owner_address
        }, params::{
            HasOwner, Params, ServiceType
        }, publish::GeneralResponseTopic, vm_info::{
            VmInfo, 
            VmList
        }, vm_types::VmType, vmm::Instance, voting::Vote
};
use crate::params::Payload;
use getset::Getters;

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
pub enum StateValueType {
    Account,
    TaskStatus,
    Instance
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
        original_task_id: TaskId,
        current_task_id: TaskId,
        event_id: String,
        response_topics: Vec<GeneralResponseTopic>
    }, 
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VmmEvent {
    Create {
        event_id: String,
        task_id: TaskId,
        name: String,
        distro: String,
        version: String,
        vmtype: VmType,
        sig: String,
        recovery_id: u8,
    },
    Start {
        event_id: String,
        task_id: TaskId,
        name: String, 
        console: bool, 
        stateless: bool,
        sig: String, 
        recovery_id: u8
    },
    Stop {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u8
    },
    Delete {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u8,
        force: bool,
        interactive: bool,
    },
    ExposeService {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u8,
        port: Vec<u16>,
        service_type: Vec<ServiceType>
    },
    AddPubkey {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u8,
        pubkey: String,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkEvent {
    NewPeer {
        event_id: String,
        task_id: TaskId,
        received_from: Peer,
        peer_id: String,
        peer_address: String,
        dst: String,
    },
    Create {
        event_id: String,
        task_id: TaskId,
        name: String,
        distro: String,
        version: String,
        vmtype: String,
        sig: String,
        recovery_id: u8,
        dst: String,
    },
    ExposeService {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String, 
        recovery_id: u8,
        port: Vec<u16>,
        service_type: Vec<ServiceType>,
        dst: String,
    },
    Start {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u8,
        console: bool,
        stateless: bool,
        dst: String,
    },
    Stop {
        event_id: String,
        task_id: TaskId,
        name: String, 
        sig: String,
        recovery_id: u8,
        dst: String,
    },
    Delete {
        event_id: String,
        task_id: TaskId,
        name: String,
        force: bool,
        interactive: bool,
        sig: String,
        recovery_id: u8,
        dst: String,
    },
    AddPubkey {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u8,
        pubkey: String,
        dst: String,
    },
    DistributeCerts {
        event_id: String,
        task_id: TaskId,
        certs: HashMap<Peer, String>,
        peer: Peer,
        quorum_id: String,
    },
    ShareCert {
        event_id: String,
        task_id: TaskId,
        peer: Peer,
        cert: String,
        quorum_id: String,
        dst: Peer
    },
    CastLeaderElectionVote {
        event_id: String,
        task_id: TaskId,
        vote: Vote,
        peers: Vec<Peer>
    },
    BootstrapNewPeer {
        // This event ensures a new peer is fully bootstrapped into the network
        // before it starts participating.
        // This means this event should trigger the peer being added to a quourum
        // the peer should receive, from it's quorum leader, the current makeup
        // of it's quuorum and all other quorums
        // the peer should be synced with it's quorum
        //
        // in the event this process leads to a quorum reshuffling,
        // the new peer should receive all of the network's peer information
        // such as the existing peers and the quorums they are members of
        // after the quorum reshuffling and leader elections occur, and the 
        // peer is firmly a member of a quorum
        //
        // in the event this process is interrupted by a new leader election
        // the leader election should complete without the new peer
        // and then following the leader election, the new leader will
        // be responsible for bootstrapping the new peer into the network
        event_id: String,
        task_id: TaskId,
        peer: Peer,
        dst: Peer
    },
    BootstrapResponse {
        event_id: String,
        original_event_id: String,
        task_id: TaskId,
        quorums: Vec<Quorum>,
        instances: Vec<Instance>,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SyncEvent {
    Sync {
        event_id: String,
        task_id: TaskId,
        namespace: String,
        path: String,
        target: String,
        last_update: Option<u64>,
        dst: String,
    }, //lxc copy --refresh
    Migrate {
        event_id: String,
        task_id: TaskId,
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
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u8,
        domain_name: String,
        // Add proof mechanism
    },
    Deregister {
        event_id: String,
        task_id: TaskId,
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
        response_topics: Vec<GeneralResponseTopic>,
        expected_type: StateValueType,
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
        last_snapshot: Option<u64>,
        last_sync: Option<u64>,
    },
    GetAccount {
        event_id: String,
        task_id: TaskId,
        task_status: TaskStatus,
        owner: [u8; 20],
        response_topics: Vec<GeneralResponseTopic>,
    },
    GetInstance {
        event_id: String,
        task_id: TaskId,
        task_status: TaskStatus,
        namespace: Namespace,
        response_topics: Vec<GeneralResponseTopic>,
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
        last_snapshot: Option<u64>,
        last_sync: Option<u64>,
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
        response_topics: Vec<GeneralResponseTopic>,
    },
    DeleteTaskStatus {
        event_id: String,
        task_id: TaskId,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QuorumEvent {
    Expand {
        event_id: String,
        task_id: TaskId,
        quorum_id: String,
        address: String
    },
    Consolidate{
        event_id: String,
        task_id: TaskId,
        quorum_id: String,
    },
    RequestSshDetails {
        event_id: String,
        task_id: TaskId,
        namespace: Namespace,
        requestor_addr: Option<SocketAddr>
    },
    CheckResponsibility {
        event_id: String,
        task_id: TaskId,
        namespace: Namespace,
        payload: Params,
    },
    NewPeer {
        event_id: String,
        task_id: TaskId,
        peer: Peer,
        received_from: Peer,
    },
    CheckAcceptCert {
        event_id: String,
        task_id: TaskId,
        peer: Peer,
        cert: String,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneralResponseEvent {
    event_id: String,
    original_event_id: String,
    response: String,
}

impl GeneralResponseEvent {
    pub fn new(
        event_id: String,
        original_event_id: String,
        response: String,
    ) -> Self {
        Self { event_id, original_event_id, response }
    }
}

#[derive(Clone, Debug, Getters, Serialize, Deserialize)]
#[getset(get = "pub")]
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
        let event_id = uuid::Uuid::new_v4().to_string();
        let task_id = generate_task_id(value.1.clone()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;
        Ok(NetworkEvent::Create { 
            name: value.1.name.clone(), 
            distro: value.1.distro.clone(), 
            version: value.1.version.clone(), 
            vmtype: value.1.vmtype.clone(), 
            sig: value.1.sig.clone(), 
            recovery_id,
            dst: value.0.ip_address().to_string(),
            event_id,
            task_id
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
        let event_id = uuid::Uuid::new_v4().to_string();
        let task_id = generate_task_id(value.1.clone()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;
        Ok(NetworkEvent::Stop { 
            name: value.1.name,
            sig: value.1.sig,
            recovery_id, 
            dst: value.0.ip_address().to_string(), 
            event_id,
            task_id
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
        let event_id = uuid::Uuid::new_v4().to_string();
        let task_id = generate_task_id(value.1.clone()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;
        Ok(NetworkEvent::Start { 
            name: value.1.name.clone(), 
            sig: value.1.sig.clone(), 
            recovery_id,
            console: value.1.console, 
            stateless: value.1.stateless, 
            dst: value.0.ip_address().to_string(), 
            event_id,
            task_id
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
        let event_id = uuid::Uuid::new_v4().to_string();
        let task_id = generate_task_id(value.1.clone()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;
        Ok(NetworkEvent::AddPubkey { 
            name: value.1.name.clone(), 
            sig: value.1.sig.clone(), 
            recovery_id, 
            pubkey: value.1.pubkey, 
            dst: value.0.ip_address().to_string(), 
            event_id,
            task_id
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
        let event_id = uuid::Uuid::new_v4().to_string();
        let task_id = generate_task_id(value.1.clone()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;
        Ok(NetworkEvent::Delete { 
            name: value.1.name.clone(), 
            force: value.1.force, 
            interactive: value.1.interactive, 
            sig: value.1.sig, 
            recovery_id, 
            dst: value.0.ip_address().to_string(), 
            event_id,
            task_id
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
        let event_id = uuid::Uuid::new_v4().to_string();
        let task_id = generate_task_id(value.1.clone()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;
        Ok(NetworkEvent::ExposeService { 
            name: value.1.name.clone(), 
            sig: value.1.sig.clone(), 
            recovery_id, 
            port: value.1.port.par_iter().filter_map(|p| {
                p.to_owned().try_into().ok()
            }).collect::<Vec<u16>>().clone(), 
            service_type: value.1.service_type.par_iter().map(|st| {
                st.to_owned().into()
            }).collect::<Vec<ServiceType>>().clone(), 
            dst: value.0.ip_address().to_string(),
            event_id,
            task_id
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

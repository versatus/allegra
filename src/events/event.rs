use std::{collections::{HashMap, HashSet}, net::SocketAddr};
use libretto::pubsub::LibrettoEvent;
use rayon::iter::{ParallelIterator, IntoParallelRefIterator};
use serde::{Serialize, Deserialize};
use sha3::{Digest, Sha3_256};
use crate::{
    account::{
        ExposedPort,
        Namespace,
        TaskId,
        TaskStatus
    }, allegra_rpc::{
        CloudInit, InstanceAddPubkeyParams, InstanceCreateParams, InstanceDeleteParams, InstanceExposeServiceParams, InstanceGetSshDetails, InstanceStartParams, InstanceStopParams
    }, network::quorum::Quorum, network::peer::Peer, helpers::{
            generate_task_id, recover_namespace, recover_owner_address
        }, params::{
            HasOwner, Params, ServiceType
        }, publish::GeneralResponseTopic, vm_info::{
            VmInfo, 
            VmList
        }, vm_types::VmType, vmm::Instance, voting::Vote
};
use crate::payload_impls::Payload;
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
        recovery_id: u32,
        sync: Option<bool>,
        memory: Option<String>,
        vcpus: Option<String>,
        cpu: Option<String>,
        metadata: Option<String>,
        os_variant: Option<String>,
        host_device: Vec<String>,
        network: Vec<String>,
        disk: Vec<String>,
        filesystem: Vec<String>,
        controller: Vec<String>,
        input: Vec<String>,
        graphics: Option<String>,
        sound: Option<String>,
        video: Option<String>,
        smartcard: Option<String>,
        redirdev: Vec<String>,
        memballoon: Option<String>,
        tpm: Option<String>,
        rng: Option<String>,
        panic: Option<String>,
        shmem: Option<String>,
        memdev: Vec<String>,
        vsock: Option<String>,
        iommu: Option<String>,
        watchdog: Option<String>,
        serial: Vec<String>,
        parallel: Vec<String>,
        channel: Vec<String>,
        console: Vec<String>,
        install: Option<String>,
        cdrom: Option<String>,
        location: Option<String>,
        pxe: bool,
        import: bool,
        boot: Option<String>,
        idmap: Option<String>,
        features: HashMap<String, String>,
        clock: Option<String>,
        launch_security: Option<String>,
        numatune: Option<String>,
        boot_dev: Vec<String>,
        unattended: bool,
        print_xml: Option<String>,
        dry_run: bool,
        connect: Option<String>,
        virt_type: Option<String>,
        cloud_init: Option<CloudInit>,
    },
    Start {
        event_id: String,
        task_id: TaskId,
        name: String, 
        console: bool, 
        stateless: bool,
        sig: String, 
        recovery_id: u32 
    },
    Stop {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u32 
    },
    Delete {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u32,
        force: bool,
        interactive: bool,
    },
    ExposeService {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u32,
        port: Vec<u16>,
        service_type: Vec<ServiceType>
    },
    AddPubkey {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u32,
        pubkey: String,
    },
    LaunchInstance {
        event_id: String,
        task_id: TaskId,
        namespace: Namespace,
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
        recovery_id: u32,
        dst: String,
        sync: Option<bool>,
        memory: Option<String>,
        vcpus: Option<String>,
        cpu: Option<String>,
        metadata: Option<String>,
        os_variant: Option<String>,
        host_device: Vec<String>,
        network: Vec<String>,
        disk: Vec<String>,
        filesystem: Vec<String>,
        controller: Vec<String>,
        input: Vec<String>,
        graphics: Option<String>,
        sound: Option<String>,
        video: Option<String>,
        smartcard: Option<String>,
        redirdev: Vec<String>,
        memballoon: Option<String>,
        tpm: Option<String>,
        rng: Option<String>,
        panic: Option<String>,
        shmem: Option<String>,
        memdev: Vec<String>,
        vsock: Option<String>,
        iommu: Option<String>,
        watchdog: Option<String>,
        serial: Vec<String>,
        parallel: Vec<String>,
        channel: Vec<String>,
        console: Vec<String>,
        install: Option<String>,
        cdrom: Option<String>,
        location: Option<String>,
        pxe: bool,
        import: bool,
        boot: Option<String>,
        idmap: Option<String>,
        features: HashMap<String, String>,
        clock: Option<String>,
        launch_security: Option<String>,
        numatune: Option<String>,
        boot_dev: Vec<String>,
        unattended: bool,
        print_xml: Option<String>,
        dry_run: bool,
        connect: Option<String>,
        virt_type: Option<String>,
        cloud_init: Option<CloudInit>,
    },
    ExposeService {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String, 
        recovery_id: u32,
        port: Vec<u16>,
        service_type: Vec<ServiceType>,
        dst: String,
    },
    Start {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u32,
        console: bool,
        stateless: bool,
        dst: String,
    },
    Stop {
        event_id: String,
        task_id: TaskId,
        name: String, 
        sig: String,
        recovery_id: u32,
        dst: String,
    },
    Delete {
        event_id: String,
        task_id: TaskId,
        name: String,
        force: bool,
        interactive: bool,
        sig: String,
        recovery_id: u32,
        dst: String,
    },
    AddPubkey {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u32,
        pubkey: String,
        dst: String,
    },
    CastLeaderElectionVote {
        event_id: String,
        task_id: TaskId,
        vote: Vote,
        peers: Vec<Peer>
    },
    ShareInstanceNamespaces {
        event_id: String,
        task_id: TaskId,
        instances: HashSet<Namespace>,
        peer: Peer
    },
    BootstrapNewPeer {
        // This event ensures a new peer is fully bootstrapped into the network
        // before it starts participating.
        // This means this event should trigger the peer being added to a quourum
        // the peer should receive, from it's quorum leader, the current makeup
        // of it's quorum and all other quorums
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
    BootstrapInstancesResponse {
        event_id: String,
        task_id: TaskId,
        requestor: Peer,
        bootstrapper: Peer
    },
    BootstrapResponse {
        event_id: String,
        original_event_id: String,
        task_id: TaskId,
        instances: Vec<Instance>,
    },
    Heartbeat {
        event_id: String,
        task_id: TaskId,
        peer: Peer,
        requestor: Peer,
    },
    PreparedForLaunch {
        event_id: String,
        task_id: TaskId,
        instance: Namespace,
        dst: Peer,
        local_peer: Peer
    },
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
    },
    Migrate {
        event_id: String,
        task_id: TaskId,
        namespace: String,
        path: String,
        target: String,
        last_update: Option<u64>,
        new_quorum: String,
        dst: String,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DnsEvent {
    Register { 
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u32,
        domain_name: String,
        // Add proof mechanism
    },
    Deregister {
        event_id: String,
        task_id: TaskId,
        name: String,
        sig: String,
        recovery_id: u32,
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
    BootstrapInstances {
        event_id: String,
        task_id: TaskId,
        instances: Vec<Namespace>,
        received_from: Peer,
    },
    BootstrapInstancesComplete {
        event_id: String,
        task_id: TaskId,
        peer: Peer,
    },
    PreparedForLaunch {
        event_id: String,
        task_id: TaskId,
        instance: Namespace,
    },
    AcceptLaunchPreparation {
        event_id: String,
        task_id: TaskId,
        instance: Namespace,
        peer: Peer,
    }
}

#[derive(Clone, Debug, Getters, Serialize, Deserialize)]
#[getset(get = "pub")]
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
            task_id,
            sync: Some(value.1.sync()),
            memory: value.1.memory,
            vcpus: value.1.vcpus,
            cpu: value.1.cpu,
            metadata: value.1.metadata,
            os_variant: value.1.os_variant,
            host_device: value.1.host_device,
            network: value.1.network,
            disk: value.1.disk,
            filesystem: value.1.filesystem,
            controller: value.1.controller,
            input: value.1.input,
            graphics: value.1.graphics,
            sound: value.1.sound,
            video: value.1.video,
            smartcard: value.1.smartcard,
            redirdev: value.1.redirdev,
            memballoon: value.1.memballoon,
            tpm: value.1.tpm,
            rng: value.1.rng,
            panic: value.1.panic,
            shmem: value.1.shmem,
            memdev: value.1.memdev,
            vsock: value.1.vsock,
            iommu: value.1.iommu,
            watchdog: value.1.watchdog,
            serial: value.1.serial,
            parallel: value.1.parallel,
            channel: value.1.channel,
            console: value.1.console,
            install: value.1.install,
            cdrom: value.1.cdrom,
            pxe: value.1.pxe,
            location: value.1.location,
            import: value.1.import,
            boot: value.1.boot,
            idmap: value.1.idmap,
            features: value.1.features.par_iter().map(|f| {
                (f.name.clone(), f.feature.clone())
            }).collect(),
            clock: value.1.clock,
            launch_security: value.1.launch_security,
            numatune: value.1.numatune,
            boot_dev: value.1.boot_dev,
            unattended: value.1.unattended,
            print_xml: value.1.print_xml,
            dry_run: value.1.dry_run,
            connect: value.1.connect,
            virt_type: value.1.virt_type,
            cloud_init: value.1.cloud_init
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

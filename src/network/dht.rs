use std::{collections::{HashMap, HashSet}, hash::RandomState, net::SocketAddr};
use alloy::primitives::Address;
use futures::stream::FuturesUnordered;
use libretto::pubsub::LibrettoEvent;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use uuid::Uuid;
use futures::StreamExt;
use anchorhash::AnchorHash;
use serde::{Serialize, Deserialize};
use regex::Regex;

use crate::{
    account::{
        Namespace,
        TaskId
    }, event::{
        NetworkEvent, 
        QuorumEvent, 
        VmmEvent
    }, network::node::Node, node::NodeState, params::{
            InstanceAddPubkeyParams, InstanceCreateParams, InstanceDeleteParams, InstanceExposeServiceParams, InstanceGetSshDetails, InstanceStartParams, InstanceStopParams, Params
        }, publish::{
            GenericPublisher, NetworkTopic, VmManagerTopic
        }, subscribe::QuorumSubscriber
};

use conductor::subscriber::SubStream;
use conductor::publisher::PubStream;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use getset::{Getters, MutGetters};
use crate::statics::BOOTSTRAP_QUORUM;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuorumSyncEvent {
    LibrettoEvent(LibrettoEvent),
    IntervalEvent(Option<u64>)
}

#[derive(Debug, Clone, Getters, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Peer {
    wallet_address: Address,
    ip_address: SocketAddr,
}

impl Peer {
    pub fn new(wallet_address: Address, ip_address: SocketAddr) -> Self {
        Self { wallet_address, ip_address }
    }

    pub fn wallet_address(&self) -> &Address {
        &self.wallet_address
    }

    pub fn ip_address(&self) -> &SocketAddr {
        &self.ip_address
    }

    pub fn wallet_address_hex(&self) -> String {
        format!("{:x}", self.wallet_address())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    #[serde(rename = "tls")]
    Tls,
    #[serde(rename = "file access")]
    FileAccess,
    #[serde(rename = "candid")]
    Candid,
    #[serde(rename = "pos")]
    Pos,
    #[serde(rename = "pki")]
    Pki,
    #[serde(rename = "rbac")]
    Rbac,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    SimpleStream,
    Lxd,
    #[serde(other)]
    Other
}

#[derive(Debug, Clone, Getters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub")]
struct RemoteFields {
    #[serde(rename = "Addr")]
    ip_addr: String,
    #[serde(rename = "AuthType")]
    auth_type: Option<AuthType>,
    #[serde(rename = "Domain")]
    domain: Option<String>,
    #[serde(rename = "Project")]
    project: Option<String>,
    #[serde(rename = "Protocol")]
    protocol: Option<Protocol>,
    #[serde(rename = "Public")]
    public: bool,
    #[serde(rename = "Global")]
    global: bool,
    #[serde(rename = "Static")]
    static_: bool
}


#[derive(Debug, Clone, Getters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct Remote {
    id: String,
    #[serde(rename = "Addr")]
    ip_addr: String,
    #[serde(rename = "AuthType")]
    auth_type: Option<AuthType>,
    #[serde(rename = "Domain")]
    domain: Option<String>,
    #[serde(rename = "Project")]
    project: Option<String>,
    #[serde(rename = "Protocol")]
    protocol: Option<Protocol>,
    #[serde(rename = "Public")]
    public: bool,
    #[serde(rename = "Global")]
    global: bool,
    #[serde(rename = "Static")]
    static_: bool
}

#[allow(private_interfaces)]
impl Remote {
    pub fn from_map(id: String, fields: RemoteFields) -> Self {
        Self {
            id,
            ip_addr: fields.ip_addr().clone(),
            auth_type: fields.auth_type().clone(),
            domain: fields.domain().clone(),
            project: fields.project().clone(),
            protocol: fields.protocol().clone(),
            public: fields.public().clone(),
            global: fields.global().clone(),
            static_: fields.static_().clone()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustType {
    Client,
    Metrics
}

#[derive(Debug, Clone, Getters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct TrustEntry {
    #[serde(rename = "name")]
    id: String,
    #[serde(rename = "type")]
    type_: TrustType,
    restricted: bool,
    projects: Vec<String>,
    certificate: String,
    fingerprint: String,
}

#[derive(Debug, Clone, Getters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct TrustToken {
    #[serde(rename = "ClientName")]
    id: String,
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "ExpiresAt")]
    expires_at: String,
}

#[derive(Debug, Clone, Getters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct TrustStore {
    remotes: HashMap<String, Remote>,
    trust_entries: HashMap<String, TrustEntry>,
    trust_tokens: HashMap<String, TrustToken>,
}

impl TrustStore {
    pub fn new() -> std::io::Result<Self> {
        let trust_entries = Self::update_trust_entries()?;
        let remotes = Self::update_remotes()?;
        let trust_tokens = Self::update_trust_tokens()?;

        Ok(Self { trust_tokens, trust_entries, remotes })
    }

    fn update_trust_entries() -> std::io::Result<HashMap<String, TrustEntry>> {
        let trust_entries_output = std::process::Command::new("lxc")
            .arg("config")
            .arg("trust")
            .arg("list")
            .arg("-f")
            .arg("json")
            .output()?;

        if trust_entries_output.status.success() {
            let trust_entries: Vec<TrustEntry> = serde_json::from_slice(
                &trust_entries_output.stdout
            ).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            let trust_entries = trust_entries.par_iter().map(|t| {
                (t.id().to_string(), t.clone()) 
            }).collect();
            return Ok(trust_entries)
        } else {
            let err = std::str::from_utf8(&trust_entries_output.stderr).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;

            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    err
                )
            )
        }
    }

    fn update_remotes() -> std::io::Result<HashMap<String, Remote>> {
        let remotes_output = std::process::Command::new("lxc")
            .arg("remote")
            .arg("list")
            .arg("-f")
            .arg("json")
            .output()?;

        if remotes_output.status.success() {
            let remotes_fields: HashMap<String, RemoteFields> = serde_json::from_slice(&remotes_output.stdout).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;

            let remotes: HashMap<String, Remote> = remotes_fields.par_iter().map(|(id, fields)| {
                (id.clone(), Remote::from_map(id.clone(), fields.clone()))
            }).collect();

            return Ok(remotes)
        } else {
            let err = std::str::from_utf8(&remotes_output.stderr).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;

            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    err
                )
            )
        }
    }

    fn update_trust_tokens() -> std::io::Result<HashMap<String, TrustToken>> {
        let trust_tokens_output = std::process::Command::new("lxc")
            .arg("config")
            .arg("trust")
            .arg("list-tokens")
            .arg("-f")
            .arg("json")
            .output()?;

        if trust_tokens_output.status.success() {
            let trust_tokens: Vec<TrustToken> = serde_json::from_slice(&trust_tokens_output.stdout)?; 
            let trust_tokens: HashMap<String, TrustToken> = trust_tokens.par_iter().map(|tt| {
                (tt.id().clone(), tt.clone())
            }).collect();

            return Ok(trust_tokens)
        } else {
            let err = std::str::from_utf8(&trust_tokens_output.stderr).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;

            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    err
                )
            )

        }
    }
}

#[derive(Debug, Clone, Getters, MutGetters, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quorum {
    #[getset(get_copy="pub", get="pub", get_mut)]
    id: Uuid,
    #[getset(get_copy="pub", get="pub", get_mut)]
    peers: HashSet<Peer>,
}

impl Quorum {
    pub fn new() -> Self {
        let id = Uuid::new_v4(); 
        Self { id, peers: HashSet::new() }
    }

    pub fn add_peer(&mut self, peer: &Peer) -> bool {
        if !self.peers.contains(peer) {
            self.peers.insert(peer.clone());
            return true
        } else {
            return false
        }
    }
    
    pub fn size(&self) -> usize {
        self.peers().len()
    }
}

pub enum QuorumResult {
    Unit(()),
    Other(String),
}

#[derive(Getters, MutGetters)]
#[getset(get = "pub", get_copy = "pub", get_mut)]
pub struct QuorumManager {
    node: Node,
    peers: HashMap<Address, Peer>,
    instances: HashMap<Namespace, Quorum>,
    quorums: HashMap<Uuid, Quorum>,
    peer_hashring: AnchorHash<Address, Quorum, RandomState>,
    instance_hashring: AnchorHash<Namespace, Quorum, RandomState>,
    subscriber: QuorumSubscriber,
    publisher: GenericPublisher,
    trust_store: TrustStore,
    futures: FuturesUnordered<JoinHandle<std::io::Result<QuorumResult>>>
}


#[allow(unused)]
impl QuorumManager {
    pub async fn new(
        subscriber_uri: &str, 
        publisher_uri: &str,
        peer_info: Peer,
    ) -> std::io::Result<Self> {
        let peer_hashring = anchorhash::Builder::default()
            .with_resources(
                vec![BOOTSTRAP_QUORUM.clone()]
            ).build(100);

        let instance_hashring = anchorhash::Builder::default()
            .with_resources(
                vec![BOOTSTRAP_QUORUM.clone()]
            ).build(100);

        let mut quorums = HashMap::new();
        quorums.insert(BOOTSTRAP_QUORUM.id().clone(), BOOTSTRAP_QUORUM.clone());
        let publisher = GenericPublisher::new(publisher_uri).await?;
        let subscriber = QuorumSubscriber::new(subscriber_uri).await?;
        let node = Node::new(peer_info);
        let trust_store = TrustStore::new()?;
        
        Ok(Self {
            node,
            peers: HashMap::new(),
            instances: HashMap::new(),
            quorums,
            peer_hashring,
            instance_hashring,
            publisher,
            subscriber,
            trust_store,
            futures: FuturesUnordered::new()
        })
    }

    pub async fn run(
        &mut self
    ) -> std::io::Result<()> {
        let mut election_interval = interval(Duration::from_secs(21600));
        let mut heartbeat_interval = interval(Duration::from_secs(20));
        let mut check_remotes_interval = interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                result = self.subscriber.receive() => {
                    match result {
                        Ok(messages) => {
                            for m in messages.clone() {
                                if let Err(e) = self.handle_quorum_message(m.clone()).await {
                                    log::error!("self.handle_quorum_message(m): {e}: message: {m:?}");
                                }
                            }
                            log::info!("handled all available {} messages", messages.len());
                        }
                        Err(e) => log::error!("self.subscriber.receive() Error: {e}")
                    }
                },
                Some(quorum_result) = self.futures.next() => {
                    match quorum_result {
                        Ok(Ok(QuorumResult::Unit(()))) => {
                            log::info!("Successfully awaited future");
                        }
                        Ok(Ok(QuorumResult::Other(details))) => {
                            log::info!("Successfully awaited future: {details}");
                        }
                        Err(e) => {
                            log::error!("Error awaiting future: {e}");
                        }
                        Ok(Err(e)) => {
                            log::error!("Error awaiting future: {e}");
                        }
                    }
                },
                leader_election = election_interval.tick() => {
                    log::info!("leader election event triggered: {:?}", leader_election);
                    let _ = self.elect_leader();
                },
                _heartbeat = heartbeat_interval.tick() => {
                    log::info!("Quorum is still alive...");
                    // Send heartbeat to quorum members...
                },
                _check_remotes = check_remotes_interval.tick() => {
                    log::info!("checking if all peers have a remote connection...");
                    if let Err(e) = self.check_remotes().await {
                        log::info!("Error attempting to check remotes: {e}");
                    }
                },
                _ = tokio::signal::ctrl_c() => {
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_quorum_message(&mut self, m: QuorumEvent) -> std::io::Result<()> {
        match m {
            QuorumEvent::Expand { .. } => todo!(),
            QuorumEvent::Consolidate { .. } => todo!(),
            QuorumEvent::RequestSshDetails { .. } => todo!(),
            QuorumEvent::NewPeer { event_id, task_id, peer, received_from } => {
                //log::info!("Received NewPeer quorum message: {event_id}: {task_id}");
                self.handle_new_peer_message(&peer, &received_from).await?;
                log::info!("Successfully completed self.handle_new_peer_message call for QuorumEvent::NewPeer message...");
            }
            QuorumEvent::CheckResponsibility { event_id, task_id, namespace, payload } => {
                log::info!("Received CheckResponsibility quorum message: {event_id}: {task_id}");
                self.handle_check_responsibility_message(
                    &namespace,
                    &payload,
                    task_id
                ).await?;
                log::info!("Successfully completed self.handle_check_responsibility_message call for QuorumEvent::CheckResponsibility message...");
            }
            QuorumEvent::CheckAcceptCert { peer, cert, event_id, task_id } => {
                log::warn!("Received CheckAcceptCert quorum message for peer {}: {}", peer.wallet_address_hex(), peer.ip_address().to_string());
                self.accept_cert(&peer, &cert).await?;
                log::info!("Successfully completed self.accept_cert call for QuorumEvent::CheckAcceptCert message...");
            }
            QuorumEvent::SyncInstanceEvent { event_id, task_id, namespace, event} => {
                log::info!("Received SyncInstanceEvent quorum message for namespace: {}", namespace.to_string());
                self.attempt_sync_instance(event_id, namespace, QuorumSyncEvent::LibrettoEvent(event)).await?;
            }
            QuorumEvent::SyncInstanceInterval { event_id, task_id, namespace, last_sync } => {
                log::info!("Received SyncInstanceInterval quorum message for namespace: {}", namespace.to_string());
                self.attempt_sync_instance(event_id, namespace, QuorumSyncEvent::IntervalEvent(last_sync)).await?;
            }
        }

        Ok(())
    }

    fn is_responsible(&mut self, namespace: &Namespace) -> Option<bool> {
        let quorum_id = self.get_instance_quorum_membership(&namespace)?; 
        let local_peer = self.node().peer_info();
        let quorum_membership = self.get_peer_quorum_membership(local_peer)?;
        if quorum_membership == quorum_id {
            return Some(true)
        } else {
            return Some(false)
        }
    }

    fn get_local_quorum_membership(&mut self) -> std::io::Result<Uuid> {
        let local_peer = self.node().peer_info();
        Ok(self.get_peer_quorum_membership(&local_peer).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Local Peer's quorum cannot be found"
            )
        )?)
    }

    fn get_local_peers(&mut self) -> Option<&HashSet<Peer>> {
        let local_peer = self.node().peer_info();
        let local_quorum_id = self.get_peer_quorum_membership(local_peer)?;
        let peers = self.get_quorum_by_id(&local_quorum_id)?.peers();
        Some(peers)
    }

    fn get_quorum_peers_by_id(&mut self, quorum_id: &Uuid) -> std::io::Result<&HashSet<Peer>> {
        Ok(self.get_quorum_by_id(quorum_id).ok_or(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Unable to acquire peers for quorum {}", quorum_id)
                )
            )?.peers())
    }

    async fn handle_create_payload(
        &mut self,
        namespace: &Namespace,
        payload: &InstanceCreateParams,
        task_id: &TaskId
    ) -> std::io::Result<()> {
        let publisher_addr = self.publisher.peer_addr()?;
        let event_id = Uuid::new_v4().to_string();
        let quorum_responsible = self.get_quorum_responsible(&namespace)?; 
        let peers = self.get_quorum_peers_by_id(&quorum_responsible)?.clone();
        if let Some(true) = self.is_responsible(&namespace) {
            log::info!("Local node quorum is responsible for instance {}", &namespace.to_string());
            self.publisher_mut().publish(
                Box::new(VmManagerTopic),
                Box::new(
                    VmmEvent::Create { 
                        event_id, 
                        task_id: task_id.clone(), 
                        name: payload.name.clone(), 
                        distro: payload.distro.clone(), 
                        version: payload.version.clone(), 
                        vmtype: payload.vmtype.clone(), 
                        sig: payload.sig.clone(), 
                        recovery_id: payload.recovery_id 
                    }
                )
            ).await?;
        }

        for p in peers {
            if p != self.node().peer_info().clone() {
                let publish_to_addr = self.publisher().peer_addr()?;
                let inner_payload = payload.clone();
                let inner_task_id = task_id.clone();
                let future = tokio::spawn(
                    async move {
                        log::info!("publishing payload for {inner_task_id} to {}", p.ip_address().to_string());
                        let mut publisher = GenericPublisher::new(&publish_to_addr).await?;
                        let event_id = Uuid::new_v4().to_string();
                        let _ = publisher.publish(
                            Box::new(NetworkTopic),
                            Box::new(
                                NetworkEvent::Create { 
                                    event_id, 
                                    task_id: inner_task_id.clone(), 
                                    name: inner_payload.name.clone(), 
                                    distro: inner_payload.distro.clone(), 
                                    version: inner_payload.version.clone(), 
                                    vmtype: inner_payload.vmtype.clone().to_string(), 
                                    sig: inner_payload.sig.clone(), 
                                    recovery_id: inner_payload.recovery_id, 
                                    dst: p.ip_address().to_string() 
                                }
                            )
                        ).await?;

                        Ok::<_, std::io::Error>(QuorumResult::Unit(()))
                    }
                );

                self.futures.push(future);
            }
        }


        Ok(())
    }

    async fn attempt_sync_instance(&mut self, original_event_id: String, namespace: Namespace, event: QuorumSyncEvent) -> std::io::Result<()> {
        let instance_quorum = self.get_instance_quorum_membership(&namespace).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Unable to acquire quuorum membership for instance {}", namespace.to_string())
            )
        )?;

        let local_quorum = self.get_local_quorum_membership()?;

        if local_quorum == instance_quorum {
            log::info!("local quorum is responsible for instance {}", namespace.to_string());
            let node_state = self.node().state().clone();
            match node_state {
                NodeState::Leader => {
                    log::info!("local node is the leader, start sync...");

                    let local_peers = self.get_local_peers().clone().ok_or(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "unable to acquire quorum peers..."
                        )
                    )?.clone();

                    let inner_namespace = namespace.clone();
                    let future = tokio::spawn(async move {
                        for peer in local_peers {
                            let output = tokio::process::Command::new("lxc")
                                .arg("copy")
                                .arg(inner_namespace.to_string())
                                .arg(format!("{}:{}", peer.wallet_address_hex(), inner_namespace.to_string()))
                                .arg("--refresh")
                                .arg("--mode")
                                .arg("pull")
                                .output().await?;
                            
                            if output.status.success() {
                                log::info!("Successfully synced: {} with {}", inner_namespace.to_string(), peer.wallet_address_hex());
                            } else {
                                let err = std::str::from_utf8(&output.stderr).map_err(|e| {
                                    std::io::Error::new(
                                        std::io::ErrorKind::Other,
                                        e
                                    )
                                })?;
                                log::error!("Error attempting to sync {} with {}: {err}", inner_namespace.to_string(), peer.wallet_address_hex());
                            }
                        }

                        Ok::<_, std::io::Error>(QuorumResult::Unit(()))
                    });

                    self.futures.push(future);
                }
                NodeState::Follower => {
                    log::info!("local node is not the leader, inform leader of sync event...");
                    let leader = self.node().current_leader().clone().ok_or(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "quorum currently has no leader"
                        )
                    )?;

                    log::info!("Discovered leader: {}, inform leader of sync event...", leader.wallet_address_hex());

                    let event_id = Uuid::new_v4().to_string();
                    let task_id = TaskId::new(Uuid::new_v4().to_string());
                    let requestor = self.node().peer_info().clone();
                    self.publisher_mut().publish(
                        Box::new(NetworkTopic),
                        Box::new(NetworkEvent::SyncInstanceToLeader {
                            event_id,
                            original_event_id,
                            requestor,
                            task_id,
                            namespace: namespace.clone(),
                            event,
                            dst: leader
                        })
                    ).await?;
                }
            }

            return Ok(())
        }

        return Ok(())
    }

    async fn handle_start_payload(
        &mut self,
        namespace: &Namespace,
        payload: &InstanceStartParams,
        task_id: &TaskId
    ) -> std::io::Result<()> {
        let publisher_addr = self.publisher.peer_addr()?;
        let event_id = Uuid::new_v4().to_string();
        let quorum_responsible = self.get_quorum_responsible(&namespace)?;
        let peers = self.get_quorum_peers_by_id(&quorum_responsible)?.clone();
        if let Some(true) = self.is_responsible(namespace) {
            self.publisher_mut().publish(
                Box::new(VmManagerTopic),
                Box::new(
                    VmmEvent::Start {
                        event_id,
                        task_id: task_id.clone(),
                        name: payload.name.clone(),
                        console: payload.console,
                        stateless: payload.stateless,
                        sig: payload.sig.clone(),
                        recovery_id: payload.recovery_id
                    }
                )
            ).await?;
        }

        let futures = peers.par_iter().map(|p| {
            let payload = payload.clone();
            let publish_to_addr = publisher_addr.clone();
            let peer = p.clone();
            let task_id = task_id.clone();
            tokio::spawn(
                async move {
                    let mut publisher = GenericPublisher::new(&publish_to_addr).await?;
                    let event_id = Uuid::new_v4().to_string();
                    publisher.publish(
                        Box::new(NetworkTopic),
                        Box::new(
                            NetworkEvent::Start {
                                event_id,
                                task_id: task_id.clone(),
                                name: payload.name.clone(),
                                console: payload.console,
                                stateless: payload.stateless,
                                sig: payload.sig.clone(),
                                recovery_id: payload.recovery_id,
                                dst: peer.ip_address().to_string()
                            }
                        )
                    ).await?;

                    Ok::<_, std::io::Error>(QuorumResult::Unit(()))
                }
            )
        }).collect::<Vec<_>>();

        self.futures.extend(futures);

        Ok(())
    }

    async fn handle_stop_payload(
        &mut self,
        namespace: &Namespace,
        payload: &InstanceStopParams,
        task_id: &TaskId
    ) -> std::io::Result<()> {
        let publisher_addr = self.publisher.peer_addr()?;
        let event_id = Uuid::new_v4().to_string();
        let quorum_responsible = self.get_quorum_responsible(&namespace)?;
        let peers = self.get_quorum_peers_by_id(&quorum_responsible)?.clone();
        if let Some(true) = self.is_responsible(namespace) {
            self.publisher_mut().publish(
                Box::new(VmManagerTopic),
                Box::new(
                    VmmEvent::Stop {
                        event_id,
                        task_id: task_id.clone(),
                        name: payload.name.clone(),
                        sig: payload.sig.clone(),
                        recovery_id: payload.recovery_id
                    }
                )
            ).await?;
        }

        let futures = peers.par_iter().map(|p| {
            let payload = payload.clone();
            let publish_to_addr = publisher_addr.clone();
            let peer = p.clone();
            let task_id = task_id.clone();
            tokio::spawn(
                async move {
                    let mut publisher = GenericPublisher::new(&publish_to_addr).await?;
                    let event_id = Uuid::new_v4().to_string();
                    publisher.publish(
                        Box::new(NetworkTopic),
                        Box::new(
                            NetworkEvent::Stop {
                                event_id,
                                task_id: task_id.clone(),
                                name: payload.name.clone(),
                                sig: payload.sig.clone(),
                                recovery_id: payload.recovery_id,
                                dst: peer.ip_address().to_string()
                            }
                        )
                    ).await?;

                    Ok::<_, std::io::Error>(QuorumResult::Unit(()))
                }
            )
        }).collect::<Vec<_>>();

        self.futures.extend(futures);

        Ok(())

    }

    async fn handle_delete_payload(
        &mut self,
        namespace: &Namespace,
        payload: &InstanceDeleteParams,
        task_id: &TaskId
    ) -> std::io::Result<()> {
        let publisher_addr = self.publisher.peer_addr()?;
        let event_id = Uuid::new_v4().to_string();
        let quorum_responsible = self.get_quorum_responsible(&namespace)?;
        let peers = self.get_quorum_peers_by_id(&quorum_responsible)?.clone();
        if let Some(true) = self.is_responsible(namespace) {
            self.publisher_mut().publish(
                Box::new(VmManagerTopic),
                Box::new(
                    VmmEvent::Delete {
                        event_id,
                        task_id: task_id.clone(),
                        name: payload.name.clone(),
                        sig: payload.sig.clone(),
                        recovery_id: payload.recovery_id,
                        force: payload.force,
                        interactive: payload.interactive
                    }
                )
            ).await?;
        }

        let futures = peers.par_iter().map(|p| {
            let payload = payload.clone();
            let publish_to_addr = publisher_addr.clone();
            let peer = p.clone();
            let task_id = task_id.clone();
            tokio::spawn(
                async move {
                    let mut publisher = GenericPublisher::new(&publish_to_addr).await?;
                    let event_id = Uuid::new_v4().to_string();
                    publisher.publish(
                        Box::new(NetworkTopic),
                        Box::new(
                            NetworkEvent::Delete {
                                event_id,
                                task_id: task_id.clone(),
                                name: payload.name.clone(),
                                sig: payload.sig.clone(),
                                recovery_id: payload.recovery_id,
                                force: payload.force,
                                interactive: payload.interactive,
                                dst: peer.ip_address().to_string()
                            }
                        )
                    ).await?;

                    Ok::<_, std::io::Error>(QuorumResult::Unit(()))
                }
            )
        }).collect::<Vec<_>>();

        self.futures.extend(futures);

        Ok(())

    }

    async fn handle_add_pubkey_payload(
        &mut self,
        namespace: &Namespace,
        payload: &InstanceAddPubkeyParams,
        task_id: &TaskId
    ) -> std::io::Result<()> {
        let publisher_addr = self.publisher.peer_addr()?;
        let event_id = Uuid::new_v4().to_string();
        let quorum_responsible = self.get_quorum_responsible(&namespace)?;
        let peers = self.get_quorum_peers_by_id(&quorum_responsible)?.clone();
        if let Some(true) = self.is_responsible(namespace) {
            self.publisher_mut().publish(
                Box::new(VmManagerTopic),
                Box::new(
                    VmmEvent::AddPubkey {
                        event_id,
                        task_id: task_id.clone(),
                        name: payload.name.clone(),
                        sig: payload.sig.clone(),
                        recovery_id: payload.recovery_id,
                        pubkey: payload.pubkey.clone()
                    }
                )
            ).await?;
        }

        let futures = peers.par_iter().map(|p| {
            let payload = payload.clone();
            let publish_to_addr = publisher_addr.clone();
            let peer = p.clone();
            let task_id = task_id.clone();
            tokio::spawn(
                async move {
                    let mut publisher = GenericPublisher::new(&publish_to_addr).await?;
                    let event_id = Uuid::new_v4().to_string();
                    publisher.publish(
                        Box::new(NetworkTopic),
                        Box::new(
                            NetworkEvent::AddPubkey {
                                event_id,
                                task_id: task_id.clone(),
                                name: payload.name.clone(),
                                sig: payload.sig.clone(),
                                recovery_id: payload.recovery_id,
                                pubkey: payload.pubkey.clone(),
                                dst: peer.ip_address().to_string()
                            }
                        )
                    ).await?;

                    Ok::<_, std::io::Error>(QuorumResult::Unit(()))
                }
            )
        }).collect::<Vec<_>>();

        self.futures.extend(futures);

        Ok(())

    }

    async fn handle_expose_service_payload(
        &mut self,
        namespace: &Namespace,
        payload: &InstanceExposeServiceParams,
        task_id: &TaskId
    ) -> std::io::Result<()> {
        let publisher_addr = self.publisher.peer_addr()?;
        let event_id = Uuid::new_v4().to_string();
        let quorum_responsible = self.get_quorum_responsible(&namespace)?;
        let peers = self.get_quorum_peers_by_id(&quorum_responsible)?.clone();
        if let Some(true) = self.is_responsible(namespace) {
            self.publisher_mut().publish(
                Box::new(VmManagerTopic),
                Box::new(
                    VmmEvent::ExposeService {
                        event_id,
                        task_id: task_id.clone(),
                        name: payload.name.clone(),
                        sig: payload.sig.clone(),
                        recovery_id: payload.recovery_id,
                        port: payload.port.clone(),
                        service_type: payload.service_type.clone()
                    }
                )
            ).await?;
        }

        let futures = peers.par_iter().map(|p| {
            let payload = payload.clone();
            let publish_to_addr = publisher_addr.clone();
            let peer = p.clone();
            let task_id = task_id.clone();
            tokio::spawn(
                async move {
                    let mut publisher = GenericPublisher::new(&publish_to_addr).await?;
                    let event_id = Uuid::new_v4().to_string();
                    publisher.publish(
                        Box::new(NetworkTopic),
                        Box::new(
                            NetworkEvent::ExposeService {
                                event_id,
                                task_id: task_id.clone(),
                                name: payload.name.clone(),
                                sig: payload.sig.clone(),
                                recovery_id: payload.recovery_id,
                                port: payload.port.clone(),
                                service_type: payload.service_type.clone(),
                                dst: peer.ip_address().to_string()
                            }
                        )
                    ).await?;

                    Ok::<_, std::io::Error>(QuorumResult::Unit(()))
                }
            )
        }).collect::<Vec<_>>();

        self.futures.extend(futures);

        Ok(())
    }

    async fn handle_get_ssh_details(
        &mut self,
        namespace: &Namespace,
        payload: &InstanceGetSshDetails,
        task_id: &TaskId,
        requestor: String,
    ) -> std::io::Result<()> {
        todo!()
    }

    async fn handle_new_peer_message(
        &mut self, 
        peer: &Peer,
        received_from: &Peer
    ) -> std::io::Result<()> {
        self.add_peer(peer, Some(received_from)).await?;

        Ok(())
    }

    async fn handle_check_responsibility_message(
        &mut self,
        namespace: &Namespace,
        payload: &Params,
        task_id: TaskId,
    ) -> std::io::Result<()> {
        match payload {
            Params::Create(p) => {
                log::info!("Handling Quorum Create message: {task_id}");
                return self.handle_create_payload(
                    namespace, p, &task_id
                ).await
            }
            Params::Start(p) => {
                log::info!("Handling Quorum Start message: {task_id}");
                return self.handle_start_payload(
                    namespace, p, &task_id
                ).await
            }
            Params::Stop(p) => {
                log::info!("Handling Quorum Stop message: {task_id}");
                return self.handle_stop_payload(
                    namespace, p, &task_id
                ).await
            }
            Params::Delete(p) => {
                log::info!("Handling Quorum Delete message: {task_id}");
                return self.handle_delete_payload(
                    namespace, p, &task_id
                ).await
            }
            Params::AddPubkey(p) => {
                log::info!("Handling Quorum AddPubkey message: {task_id}");
                return self.handle_add_pubkey_payload(
                    namespace, p, &task_id
                ).await
            }
            Params::ExposeService(p) => {
                log::info!("Handling ExposeService message: {task_id}");
                return self.handle_expose_service_payload(
                    namespace, p, &task_id
                ).await
            }
            _ => {
                return Err(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Unable to check responsibility for InstanceGetSshDetails and InstanceSshSession params"
                    )
                )
            }
        }
    }

    fn get_quorum_responsible(&mut self, namespace: &Namespace) -> std::io::Result<Uuid> {
        Ok(self.get_instance_quorum_membership(&namespace).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Unable to acquire quorum id for instance: {}", &namespace.to_string())
            )
        )?)
    }

    async fn elect_leader(&mut self) -> std::io::Result<()> {
        let quorums = self.quorums().clone();
        let quorum_peers = quorums.get(
            &self.get_peer_quorum_membership(
                self.node.peer_info()
            ).ok_or(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "unable to find membership for local node"
                )
            )?
        ).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to find peers for local node quorum"
            )
        )?;
        let uri = self.publisher().peer_addr()?;
        self.node.start_election(quorum_peers.peers(), &uri).await?;

        Ok(())
    }

    #[async_recursion::async_recursion]
    async fn reorganize_resources(&mut self) -> std::io::Result<()> {
        self.quorums.iter_mut().for_each(|(_, q)| {
            q.peers = HashSet::new();
        });

        for (_, peer) in self.peers.clone() {
            self.add_peer(&peer, None).await?;
        }

        for (namespace, _) in self.instances.clone() {
            self.add_instance(&namespace)?;
        }

        Ok(())
    }


    pub async fn add_quorum(&mut self, quorum: Quorum) -> std::io::Result<()> {
        let mut peer_hashring = self.peer_hashring.clone();
        peer_hashring.add_resource(quorum.clone()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        let mut instance_hashring = self.instance_hashring.clone();

        instance_hashring.add_resource(quorum.clone()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        self.peer_hashring = peer_hashring;
        self.instance_hashring = instance_hashring;
        self.quorums.insert(quorum.id().to_owned(), quorum);

        self.reorganize_resources().await?;

        Ok(())
    }

    fn extract_cert(cert: &str) -> std::io::Result<String> {
        let re = Regex::new(r"token:\n(.*)\n").map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        re.captures(cert).and_then(|cap| cap.get(1).map(|m| m.as_str().to_string())).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to extract jwt from cert"
            )
        )
    }

    pub async fn update_trust_store(&mut self) -> std::io::Result<()> {
        let trust_store = TrustStore::new()?;
        self.trust_store = trust_store;

        Ok(())
    }

    pub async fn share_cert(
        &mut self, 
        peer: &Peer, 
    ) -> std::io::Result<()> {
        log::info!("Attempting to share certificate with peer: {}", &peer.wallet_address_hex()); 
        let peer_id = peer.wallet_address_hex();
        let is_local_peer = if peer_id == self.node().peer_info().wallet_address_hex() {
            true
        } else {
            false
        };

        log::warn!("is_local_peer = {is_local_peer}");
        if !is_local_peer {
            let output = std::process::Command::new("lxc")
                .arg("config")
                .arg("trust")
                .arg("add")
                .arg("--name")
                .arg(&peer_id)
                .output()?;

            if output.status.success() {
                log::info!("Successfully created token for peer {}", &peer.wallet_address().to_string());
                let cert = match std::str::from_utf8(&output.stdout) {
                    Ok(res) => res.to_string(),
                    Err(e) => return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e
                        )
                    )
                };

                self.update_trust_store().await?;

                let cert = self.trust_store().trust_tokens().get(&peer_id).ok_or(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Unable to find peer_id in trust tokens despite success in call to lxc config trust add --name {}", peer_id)
                    )
                )?.token().to_string();

                let quorum_id = self.get_local_quorum_membership()?.to_string();
                let task_id = TaskId::new(uuid::Uuid::new_v4().to_string()); 
                let event_id = uuid::Uuid::new_v4().to_string();
                let event = NetworkEvent::ShareCert { 
                    peer: self.node().peer_info().clone(), 
                    cert,
                    task_id,
                    event_id,
                    quorum_id,
                    dst: peer.clone() 
                };

                self.publisher.publish(
                    Box::new(NetworkTopic),
                    Box::new(event)
                ).await?;
            } else {
                let stderr = std::str::from_utf8(&output.stderr).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?;
                return Err(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Unable to add client certificate to trust store for peer {}: {}", &peer.wallet_address_hex(), &stderr)
                    )
                )
            }
        }
        Ok(())
    }

    pub async fn check_remotes(&mut self) -> std::io::Result<()> {
        log::info!("Updating trust store...");
        self.update_trust_store().await?;
        log::info!("Checking to ensure all local quorum members are remotes...");
        let local_quorum_id = self.get_local_quorum_membership()?;
        let local_quorum = self.get_quorum_by_id(&local_quorum_id).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to find local quorum"
            )
        )?.clone(); 
        let local_quorum_members = local_quorum.peers();
        for peer in local_quorum_members {
            log::info!("checking if peer {}: {} is remote...", peer.wallet_address_hex(), peer.ip_address().to_string());
            if peer != self.node().peer_info() {
                log::info!("checking trust store for peer {}", peer.wallet_address_hex());
                if !self.trust_store.remotes().contains_key(&peer.wallet_address_hex()) {
                    log::info!("peer {} not in trust store... checking trust tokens", peer.wallet_address_hex());
                    match self.trust_store.trust_tokens().get(&peer.wallet_address_hex()) {
                        Some(token) => {
                            log::info!("found peer {} trust token, sharing...", peer.wallet_address_hex());
                            let event_id = Uuid::new_v4().to_string();
                            let task_id = TaskId::new(Uuid::new_v4().to_string());
                            let event = NetworkEvent::ShareCert { 
                                event_id,
                                task_id,
                                peer: self.node().peer_info().clone(),
                                cert: token.token().clone(),
                                quorum_id: local_quorum_id.to_string(),
                                dst: peer.clone() 
                            };
                            self.publisher_mut().publish(
                                Box::new(NetworkTopic),
                                Box::new(event)
                            ).await?;
                            log::info!("Successfully shared trust token with peer {}: {}...", peer.wallet_address_hex(), peer.ip_address().to_string());
                        }
                        None => {
                            log::info!("peer {} has no trust token, calling self.share_cert()...", peer.wallet_address_hex());
                            self.share_cert(peer).await?;
                            log::info!("successfully called self.share_cert() to create and share a trust token with peer {}...", peer.wallet_address_hex());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn accept_cert(
        &mut self,
        peer: &Peer,
        cert: &str
    ) -> std::io::Result<()> {
        //TODO(asmith): We will want to check against their stake to verify membership

        // Check if peer is member of same quorum as local node
        //log::info!("checking if certificate from peer: {:?} is for local quorum member...", peer);
        let qid = self.get_local_quorum_membership()?;
        log::info!("Local node is member of quorum {}", qid.to_string());
        let quorum_peers = self.get_quorum_peers_by_id(&qid)?;
        log::warn!("quorum_peers.contains(peer) = {}", quorum_peers.contains(peer));
        if quorum_peers.contains(peer) {
            let output = std::process::Command::new("lxc")
                .arg("remote")
                .arg("add")
                .arg(peer.wallet_address_hex())
                .arg(cert)
                .output()?;

            if output.status.success() {
                log::info!("SUCCESS! SUCCESS! Successfully added peer {} to remote", &peer.wallet_address_hex());
                let stdout = std::str::from_utf8(&output.stdout).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?;
                return Ok(())
            } else {
                let stderr = std::str::from_utf8(&output.stderr).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?;
                return Err(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to add peer {} certificate to trust store: {}", &peer.wallet_address_hex(), stderr)
                    )
                )
            }
        } else {
            log::warn!("Local quorum does not have {} as peer...", peer.wallet_address_hex());
        }

        log::info!("Completed self.accept_cert method returning...");
        Ok(())
    }

    pub async fn add_peer(&mut self, peer: &Peer, received_from: Option<&Peer>) -> std::io::Result<()> {
        let q = self.peer_hashring.get_resource(peer.wallet_address().clone()).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to map peer to resource"
            )
        )?.clone();

        let local_quorum_member = if q.id() == &self.get_local_quorum_membership()? {
            true
        } else {
            false
        };

        log::warn!("local quorum member = {}", &local_quorum_member);
        let quorum = self.quorums.get_mut(&q.id().clone()).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "quorum assigned does not exist"
            )
        )?;

        quorum.add_peer(peer);
        log::warn!("quorum.size() = {}", quorum.size());

        if quorum.clone().size() >= 50 {
            log::info!("quorum exceeds maximum size, time to reshuffle quorums");
            self.create_new_quorum().await?;
        }

        if !self.peers.contains_key(peer.wallet_address()) {
            log::info!("self.peers does not contain peer key, adding");
            self.peers.insert(*peer.wallet_address(), peer.clone());
            log::warn!("self.peers.len() = {}", self.peers.len());
            for (peer_wallet_address, dst_peer) in self.peers.clone() {
                if (&dst_peer != peer) && (&dst_peer != self.node.peer_info()) && (Some(&dst_peer.clone()) != received_from) {
                    let task_id = TaskId::new(
                        uuid::Uuid::new_v4().to_string()
                    );
                    let event_id = uuid::Uuid::new_v4().to_string();
                    let dst_event = NetworkEvent::NewPeer { 
                        peer_id: peer.wallet_address_hex(), 
                        peer_address: peer.ip_address().to_string(), 
                        dst: dst_peer.ip_address().to_string(),
                        task_id,
                        event_id,
                        received_from: self.node().peer_info().clone()
                    };


                    self.publisher_mut().publish(
                        Box::new(NetworkTopic),
                        Box::new(dst_event)
                    ).await?;

                    let task_id = TaskId::new(
                        uuid::Uuid::new_v4().to_string()
                    );

                    let event_id = uuid::Uuid::new_v4().to_string();
                    let new_peer_event = NetworkEvent::NewPeer { 
                        event_id,
                        task_id,
                        peer_id: dst_peer.wallet_address_hex(),
                        peer_address: dst_peer.ip_address().to_string(), 
                        dst: peer.ip_address().to_string(),
                        received_from: self.node().peer_info().clone()
                    };

                    self.publisher_mut().publish(
                        Box::new(NetworkTopic),
                        Box::new(new_peer_event)
                    ).await?;

                }
            }
            if local_quorum_member {
                self.share_cert(&peer).await?;
            }
        } 


        Ok(())
    }

    pub async fn create_new_quorum(&mut self) -> std::io::Result<()> {
        let new_quorum = Quorum::new();
        self.add_quorum(new_quorum).await?;

        Ok(())
    }

    pub fn get_peer_quorum_membership(&self, peer: &Peer) -> Option<Uuid> {
        let q = self.peer_hashring.get_resource(peer.wallet_address().clone())?;
        Some(q.id().clone())
    }

    pub fn get_instance_quorum_membership(&self, namespace: &Namespace) -> Option<Uuid> {
        let q = self.instance_hashring.get_resource(namespace.clone())?;
        Some(q.id().clone())
    }

    pub fn get_peer_quorum(&self, peer: &Peer) -> Option<Quorum> {
        let qid = self.get_peer_quorum_membership(peer)?;
        let q = self.quorums.get(&qid)?;

        Some(q.clone())
    }

    pub fn get_quorum_peers(&self, peer: &Peer) -> Option<HashSet<Peer>> {
        let q = self.get_peer_quorum(peer)?;

        Some(q.peers().clone())
    }

    pub fn get_quorum_by_id(&self, id: &Uuid) -> Option<&Quorum> {
        self.quorums.get(id)
    }

    pub fn add_instance(&mut self, namespace: &Namespace) -> std::io::Result<()> {
        let q = self.instance_hashring.get_resource(namespace.clone()).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to map instance to resource"
            )
        )?;

        let quorum = self.quorums.get_mut(q.id()).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "quorum assigned does not exist"
            )
        )?;

        self.instances.insert(namespace.clone(), quorum.clone());

        Ok(())
    }
}

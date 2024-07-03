use std::{collections::{HashMap, HashSet}, hash::RandomState, net::SocketAddr};
use alloy::primitives::Address;
use futures::stream::FuturesUnordered;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use uuid::Uuid;
use anchorhash::AnchorHash;
use serde::{Serialize, Deserialize};

use crate::{
    account::{
        Namespace,
        TaskId
    }, event::{
        NetworkEvent, 
        QuorumEvent, 
        VmmEvent
    }, network::node::Node, 
        params::{
            InstanceCreateParams, Params, InstanceStartParams,
            InstanceStopParams, InstanceGetSshDetails, InstanceExposeServiceParams,
            InstanceDeleteParams, InstanceAddPubkeyParams
        }, 
        publish::{
            GenericPublisher, NetworkTopic, VmManagerTopic
        }, subscribe::QuorumSubscriber
};

use conductor::subscriber::SubStream;
use conductor::publisher::PubStream;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use futures::StreamExt;
use getset::{Getters, MutGetters};
use crate::statics::BOOTSTRAP_QUORUM;

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
        
        Ok(Self {
            node,
            peers: HashMap::new(),
            instances: HashMap::new(),
            quorums,
            peer_hashring,
            instance_hashring,
            publisher,
            subscriber,
            futures: FuturesUnordered::new()
        })
    }

    pub async fn run(
        &mut self
    ) -> std::io::Result<()> {
        let mut interval = interval(Duration::from_secs(21600));
        loop {
            tokio::select! {
                result = self.subscriber.receive() => {
                    match result {
                        Ok(messages) => {
                            log::info!("Received {} messages", messages.len());
                            for m in messages {
                                log::info!("attempting to handle message: {:?}", m);
                                if let Err(e) = self.handle_quorum_message(m.clone()).await {
                                    log::error!("self.handle_quorum_message(m): {e}: message: {m:?}");
                                }
                            }
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
                leader_election = interval.tick() => {
                    log::info!("leader election event triggered: {:?}", leader_election);
                    let _ = self.elect_leader();
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
            QuorumEvent::NewPeer { event_id, task_id, peer } => {
                log::info!("Received NewPeer quorum message: {event_id}: {task_id}");
                self.handle_new_peer_message(&peer).await?;
            }
            QuorumEvent::CheckResponsibility { event_id, task_id, namespace, payload } => {
                log::info!("Received CheckResponsibility quorum message: {event_id}: {task_id}");
                self.handle_check_responsibility_message(
                    &namespace,
                    &payload,
                    task_id
                ).await?;
            }
            QuorumEvent::CheckAcceptCert { peer, cert, event_id, task_id } => {
                log::info!("Received CheckAcceptCert quorum message for peer {peer:?}: {event_id}: {task_id}");
                self.accept_cert(&peer, &cert).await?;
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

        let futures = peers.par_iter().map(|p| {
            let payload = payload.clone();
            let publish_to_addr = publisher_addr.clone();
            let peer = p.clone();
            let task_id = task_id.clone();
            tokio::spawn(
                async move {
                    log::info!("publishing payload for {task_id} to {}", peer.ip_address().to_string());
                    let mut publisher = GenericPublisher::new(&publish_to_addr).await?;
                    let event_id = Uuid::new_v4().to_string();
                    let _ = publisher.publish(
                        Box::new(NetworkTopic),
                        Box::new(
                            NetworkEvent::Create { 
                                event_id, 
                                task_id: task_id.clone(), 
                                name: payload.name.clone(), 
                                distro: payload.distro.clone(), 
                                version: payload.version.clone(), 
                                vmtype: payload.vmtype.clone().to_string(), 
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
        peer: &Peer
    ) -> std::io::Result<()> {

        log::info!("Attempting to handle NewPeer event...");
        self.add_peer(peer).await?;

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
            self.add_peer(&peer).await?;
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

    pub async fn share_cert(
        &mut self, 
        local_id: &str, 
        peer: &Peer, 
        quorum_id: &Uuid, 
    ) -> std::io::Result<()> {
        log::info!("Attempting to share certificate with peer: {}", &peer.wallet_address_hex()); 
        let peer_id = peer.wallet_address_hex();

        let output = std::process::Command::new("sudo")
            .arg("lxc")
            .arg("config")
            .arg("trust")
            .arg("add")
            .arg("--name")
            .arg(&peer_id)
            .output()?;

        if output.status.success() {
            log::info!("Successfully added client certificate to trust store for peer {}", &peer.wallet_address().to_string());

            let cert = match std::str::from_utf8(&output.stdout) {
                Ok(res) => res.to_string(),
                Err(e) => return Err(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                )
            };

            let task_id = TaskId::new(uuid::Uuid::new_v4().to_string()); 
            let event_id = uuid::Uuid::new_v4().to_string();
            let event = NetworkEvent::ShareCert { 
                peer: self.node().peer_info().clone(), 
                cert,
                task_id,
                event_id,
                quorum_id: quorum_id.to_string(),
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


        Ok(())
    }

    pub async fn accept_cert(
        &mut self,
        peer: &Peer,
        cert: &str
    ) -> std::io::Result<()> {
        //TODO(asmith): We will want to check against their stake to verify membership

        // Check if peer is member of same quorum as local node
        log::info!("checking if certificate from peer: {:?} is for local quorum member...", peer);
        let qid = self.get_local_quorum_membership()?;
        let quorum_peers = self.get_quorum_peers_by_id(&qid)?;
        log::info!("Quorum peers: {:?}", quorum_peers);
        if quorum_peers.contains(peer) {
            log::info!("peer is member of local quorum, add certificate...");
            let output = std::process::Command::new("sudo")
                .arg("lxc")
                .arg("remote")
                .arg("add")
                .arg(peer.wallet_address_hex())
                .arg(cert)
                .output()?;

            if output.status.success() {
                log::info!("Successfully added peer {} certificate to trust store", &peer.wallet_address_hex());
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
        }

        Ok(())
    }

    pub async fn add_peer(&mut self, peer: &Peer) -> std::io::Result<()> {
        log::info!("Attempting to add peer: {:?} to DHT", peer);
        let q = self.peer_hashring.get_resource(peer.wallet_address().clone()).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to map peer to resource"
            )
        )?.clone();

        log::info!("checking if new peer is member in local quorum");
        let local_quorum_member = if q.id() == &self.get_local_quorum_membership()? {
            true
        } else {
            false
        };

        log::info!("acquiring quorum that new peer is a member of");
        let quorum = self.quorums.get_mut(&q.id().clone()).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "quorum assigned does not exist"
            )
        )?;

        log::info!("quorum.size() = {}", quorum.size());
        quorum.add_peer(peer);
        log::info!("quorum.size() = {}", quorum.size());

        if quorum.clone().size() >= 50 {
            log::info!("quorum exceeds maximum size, time to reshuffle quorums");
            self.create_new_quorum().await?;
        }

        if !self.peers.contains_key(peer.wallet_address()) {
            log::info!("self.peers does not contain peer key, adding");
            self.peers.insert(*peer.wallet_address(), peer.clone());
            log::info!("added new peer to self.peers");
            for (peer_wallet_address, dst_peer) in self.peers.clone() {
                if (&dst_peer != peer) && (&dst_peer != self.node.peer_info()) {
                    log::warn!("informing: {:?} of new peer", peer_wallet_address);
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
                        dst: peer.ip_address().to_string() 
                    };

                    self.publisher_mut().publish(
                        Box::new(NetworkTopic),
                        Box::new(new_peer_event)
                    ).await?;

                }
            }
        } 

        if local_quorum_member && (self.node().peer_info().wallet_address() != peer.wallet_address()) {
            log::info!("new peer is member of same quorum as local node and is not the local peer, attempting to share certificate");
            let local_id = self.node().peer_info().wallet_address().clone();
            let local_peer = self.node.peer_info().clone();
            self.share_cert(
                &local_id.to_string(),
                &peer,
                q.id(),
            ).await?;
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

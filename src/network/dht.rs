use std::{collections::{HashMap, HashSet}, hash::RandomState, io};
use futures::stream::FuturesUnordered;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sha3::{Sha3_256, Digest};
use uuid::Uuid;
use anchorhash::AnchorHash;
use serde::{Serialize, Deserialize};

use crate::{
    account::{
        Namespace,
        TaskId
    }, event::{
        self, 
        Event, 
        NetworkEvent, 
        QuorumEvent, 
        RpcResponseEvent, 
        VmmEvent
    }, network::node::Node, 
        params::{
            InstanceCreateParams, Params, InstanceStartParams,
            InstanceStopParams, InstanceGetSshDetails, InstanceExposeServiceParams,
            InstanceDeleteParams, InstanceAddPubkeyParams
        }, 
        publish::{
            GenericPublisher, NetworkTopic, RpcResponseTopic, VmManagerTopic
        }, subscribe::QuorumSubscriber
};

use conductor::subscriber::SubStream;
use conductor::publisher::PubStream;
use tokio::task::JoinHandle;
use tokio::sync::mpsc::Receiver;
use tokio::time::{interval, Duration};
use futures::StreamExt;
use getset::{Getters, MutGetters};

lazy_static::lazy_static! {
    pub static ref BOOTSTRAP_QUORUM: Quorum = Quorum::new(); 
}

#[derive(Debug, Clone, Getters, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Peer {
    id: Uuid,
    address: String,
}

impl Peer {
    pub fn new(id: Uuid, address: String) -> Self {
        Self { id, address }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn id_hash(&self) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(self.id.as_bytes());
        let hash_vec = hasher.finalize().to_vec();
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&hash_vec[..]);
        hash_bytes
    }

    pub fn id_hex(&self) -> String {
        hex::encode(&self.id_hash())
    }
}

#[derive(Debug, Clone, Getters, MutGetters, PartialEq, Eq)]
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
    peers: HashMap<Uuid, Peer>,
    instances: HashMap<Namespace, Quorum>,
    quorums: HashMap<Uuid, Quorum>,
    peer_hashring: AnchorHash<Uuid, Quorum, RandomState>,
    instance_hashring: AnchorHash<Namespace, Quorum, RandomState>,
    subscriber: QuorumSubscriber,
    publisher: GenericPublisher,
    futures: FuturesUnordered<JoinHandle<std::io::Result<QuorumResult>>>
}


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
        &mut self,
        mut stop_rx: Receiver<()>
    ) -> std::io::Result<()> {
        let mut interval = interval(Duration::from_secs(21600));
        loop {
            tokio::select! {
                result = self.subscriber.receive() => {
                    match result {
                        Ok(messages) => {
                            for m in messages {
                                self.handle_quorum_message(m).await?;
                            }
                        }
                        Err(e) => log::error!("{e}")
                    }
                },
                quorum_result = self.futures.next() => {
                    match quorum_result {
                        Some(Ok(Ok(QuorumResult::Unit(())))) => {
                            log::info!("Successfully awaited future");
                        }
                        Some(Ok(Ok(QuorumResult::Other(details)))) => {
                            log::info!("Successfully awaited future: {details}");
                        }
                        Some(Err(e)) => {
                            log::error!("Error awaiting future: {e}");
                        }
                        Some(Ok(Err(e))) => {
                            log::error!("Error awaiting future: {e}");
                        }
                        None => {
                            log::error!("Unable to await future");
                        }
                    }
                    todo!()
                },
                leader_election = interval.tick() => {
                    log::info!("leader election event triggered: {:?}", leader_election);
                    let _ = self.elect_leader();
                },
                _ = stop_rx.recv() => {
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
            QuorumEvent::CheckResponsibility { event_id, task_id, namespace, payload } => {
                self.handle_check_responsibility_message(
                    &namespace,
                    &payload,
                    task_id
                );
                // Send to peers in quorum responsible
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
                                dst: peer.address().to_string() 
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
                                dst: peer.address().to_string()
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
                                dst: peer.address().to_string()
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
                                dst: peer.address().to_string()
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
                                dst: peer.address().to_string()
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
                                dst: peer.address().to_string()
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

    async fn handle_check_responsibility_message(
        &mut self,
        namespace: &Namespace,
        payload: &Params,
        task_id: TaskId,
    ) -> std::io::Result<()> {
        match payload {
            Params::Create(p) => {
                return self.handle_create_payload(
                    namespace, p, &task_id
                ).await
            }
            Params::Start(p) => {
                return self.handle_start_payload(
                    namespace, p, &task_id
                ).await
            }
            Params::Stop(p) => {
                return self.handle_stop_payload(
                    namespace, p, &task_id
                ).await
            }
            Params::Delete(p) => {
                return self.handle_delete_payload(
                    namespace, p, &task_id
                ).await
            }
            Params::AddPubkey(p) => {
                return self.handle_add_pubkey_payload(
                    namespace, p, &task_id
                ).await
            }
            Params::ExposeService(p) => {
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

    pub async fn share_cert(&mut self, local_id: &str, peer: &Peer) -> std::io::Result<()> {
        let peer_id_bytes = peer.id().to_string().as_bytes().to_vec();
        let local_peer_id_bytes = local_id.as_bytes();

        let peer_trust_name = {
            assert_eq!(local_peer_id_bytes.len(), peer_id_bytes.len());
            let mut result = Vec::with_capacity(local_peer_id_bytes.len());
            for i in 0..local_peer_id_bytes.len() {
                result.push(local_peer_id_bytes[i] ^ peer_id_bytes[i]);
            }

            match std::str::from_utf8(&result) {
                Ok(res) => res.to_string(),
                Err(e) => {
                    return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e
                        )
                    )
                }
            }
        };

        let output = std::process::Command::new("sudo")
            .arg("lxc")
            .arg("config")
            .arg("trust")
            .arg("add")
            .arg("--name")
            .arg(&peer_trust_name)
            .output()?;

        if output.status.success() {
            log::info!("Successfully added client certificate to trust store for peer {}", &peer.id().to_string());

            let cert = match std::str::from_utf8(&output.stdout) {
                Ok(res) => res.to_string(),
                Err(e) => return Err(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                )
            };

            //TODO(asmith): Replace with publisher 
            let task_id = TaskId::new(uuid::Uuid::new_v4().to_string()); 
            let event_id = uuid::Uuid::new_v4().to_string();
            let _event = Event::NetworkEvent(
                NetworkEvent::ShareCert { 
                    peer: peer.clone(), 
                    cert,
                    task_id,
                    event_id
                }
            );
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
                    format!("Unable to add client certificate to trust store for peer {}: {}", &peer.id().to_string(), &stderr)
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

        let output = std::process::Command::new("sudo")
            .arg("lxc")
            .arg("remote")
            .arg("add")
            .arg(peer.id().to_string())
            .arg(cert)
            .output()?;

        if output.status.success() {
            log::info!("Successfully added peer {} certificate to trust store", &peer.id().to_string());
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
                    format!("Failed to add peer {} certificate to trust store: {}", &peer.id().to_string(), stderr)
                )
            )
        }
    }

    pub async fn add_peer(&mut self, peer: &Peer) -> std::io::Result<()> {
        log::info!("Attempting to add peer: {:?} to DHT", peer);
        let q = self.peer_hashring.get_resource(peer.id().clone()).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to map peer to resource"
            )
        )?;

        let quorum = self.quorums.get_mut(q.id()).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "quorum assigned does not exist"
            )
        )?;

        log::info!("quorum.size() = {}", quorum.size());
        quorum.add_peer(peer);
        log::info!("quorum.size() = {}", quorum.size());
        if !self.peers.contains_key(&peer.id()) {
            self.peers.insert(peer.id, peer.clone());
            for (_, dst_peer) in &self.peers {
                if dst_peer != peer {
                    let task_id = TaskId::new(
                        uuid::Uuid::new_v4().to_string()
                    );
                    let event_id = uuid::Uuid::new_v4().to_string();
                    let _dst_event = Event::NetworkEvent(
                        NetworkEvent::NewPeer { 
                            peer_id: peer.id().to_string(), 
                            peer_address: peer.address().to_string(), 
                            dst: dst_peer.address().to_string(),
                            task_id,
                            event_id,
                        }
                    );

                    let task_id = TaskId::new(
                        uuid::Uuid::new_v4().to_string()
                    );
                    let event_id = uuid::Uuid::new_v4().to_string();
                    let _new_peer_event = Event::NetworkEvent(
                        NetworkEvent::NewPeer { 
                            peer_id: dst_peer.id().to_string(), 
                            peer_address: dst_peer.address().to_string(), 
                            dst: dst_peer.address().to_string(),
                            task_id,
                            event_id
                        }
                    );

                    //TODO(asmith): replace with publisher
                    //let mut guard = self.event_broker.lock().await;
                    //guard.publish("Network".to_string(), dst_event).await;
                    //guard.publish("Network".to_string(), new_peer_event).await;
                }
            }
        } 

        if quorum.size() >= 50 {
            self.create_new_quorum().await?;
        }

        Ok(())
    }

    pub async fn create_new_quorum(&mut self) -> std::io::Result<()> {
        let new_quorum = Quorum::new();
        self.add_quorum(new_quorum).await?;

        Ok(())
    }

    pub fn get_peer_quorum_membership(&self, peer: &Peer) -> Option<Uuid> {
        let q = self.peer_hashring.get_resource(peer.id().clone())?;
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

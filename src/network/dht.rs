use std::{collections::{HashMap, HashSet}, hash::RandomState, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;
use anchorhash::AnchorHash;

use crate::{account::Namespace, broker::broker::EventBroker, event::{Event, NetworkEvent}};

lazy_static::lazy_static! {
    pub static ref BOOTSTRAP_QUORUM: Quorum = Quorum::new(); 
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Peer {
    id: Uuid,
    address: String
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quorum {
    id: Uuid,
    peers: HashSet<Peer>,
}

impl Quorum {
    pub fn new() -> Self {
        let id = Uuid::new_v4(); 
        Self { id, peers: HashSet::new() }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn peers(&self) -> &HashSet<Peer> {
        &self.peers
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


#[derive(Clone, Debug)]
pub struct AllegraNetworkState {
    peers: HashMap<Uuid, Peer>,
    instances: HashMap<Namespace, Quorum>,
    quorums: HashMap<Uuid, Quorum>,
    peer_hashring: AnchorHash<Uuid, Quorum, RandomState>,
    instance_hashring: AnchorHash<Namespace, Quorum, RandomState>,
    event_broker: Arc<Mutex<EventBroker>>
}


impl AllegraNetworkState {
    pub fn new(event_broker: Arc<Mutex<EventBroker>>) -> Self {
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
        
        Self {
            peers: HashMap::new(),
            instances: HashMap::new(),
            quorums,
            peer_hashring,
            instance_hashring,
            event_broker,
        }
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
                    let event = Event::NetworkEvent(
                        NetworkEvent::NewPeer { 
                            peer_id: peer.id().to_string(), 
                            peer_address: peer.address().to_string(), 
                            peer_number: (self.peers.len() + 1) as u32, 
                            dst: dst_peer.address().to_string() 
                        }
                    );
                    let mut guard = self.event_broker.lock().await;
                    guard.publish("Network".to_string(), event).await;
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

    pub fn peers(&self) -> &HashMap<Uuid, Peer> {
        &self.peers
    }
}

use ractor::{ActorCell, ActorRef};
use tokio::task::JoinHandle;
use ractor_cluster::RactorMessage;

pub struct NodeId(String);

pub struct ActorSpawn<M: Sized> {
    actor: (ActorRef<M>, JoinHandle<()>)
}

impl<M: Sized> ActorSpawn<M> {
    pub fn new(actor: (ActorRef<M>, JoinHandle<()>)) -> Self {
        Self { actor }
    }
}

#[derive(Debug, RactorMessage)]
pub enum DhtMessage {
    Test,
}

#[derive(Debug, RactorMessage)]
pub enum VmmMessage {
    Test,
}

#[derive(Debug, RactorMessage)]
pub enum StateMessage {
    Test,
}

#[derive(Debug, RactorMessage)]
pub enum RaMessage {
    Test,
}

#[derive(Debug, RactorMessage)]
pub enum NetworkMessage {
    Test,
}

pub struct ActorManager {
    dht_actor_spawn: ActorSpawn<DhtMessage>,
    vmm_actor_spawn: ActorSpawn<VmmMessage>,
    state_actor_spawn: ActorSpawn<StateMessage>,
    ra_actor_spawn: ActorSpawn<RaMessage>,
    network_actor: ActorSpawn<NetworkMessage>
}

pub struct Node {
    id: NodeId,
    manager: ActorManager,
    stop_rx: tokio::sync::mpsc::Receiver<()>,
    panic_rx: tokio::sync::mpsc::Receiver<ActorCell>

}

impl Node {
    pub async fn run(mut self) -> std::io::Result<()> {
        loop {
            tokio::select! {
                _panicked_actor = self.panic_rx.recv() => {
                    // implement logic for panicked actor recovery
                }
                _ = self.stop_rx.recv() => {
                    break
                }
            }
        }

        Ok(())
    }
}

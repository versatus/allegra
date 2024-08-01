use serde::{Serialize, Deserialize};
use getset::{Getters, MutGetters};
use std::collections::HashSet;
use uuid::Uuid;
use conductor::publisher::PubStream;
use crate::{Peer, TaskId, GenericPublisher, Namespace, NetworkTopic, event::NetworkEvent};

#[derive(Debug, Clone, Getters, MutGetters, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quorum {
    #[getset(get_copy="pub", get="pub", get_mut)]
    pub(super) id: Uuid,
    #[getset(get_copy="pub", get="pub", get_mut)]
    pub(super) peers: HashSet<Peer>,
}


impl Quorum {
    pub fn new() -> Self {
        let id = Uuid::new_v4(); 
        Self { id, peers: HashSet::new() }
    }

    pub async fn add_peer(&mut self, peer: &Peer) -> std::io::Result<bool> {
        if !self.peers.contains(peer) {
            self.peers.insert(peer.clone());
            self.add_glusterfs_peer(peer).await?;
            return Ok(true)
        } else {
            return Ok(false)
        }
    }

    pub(super) async fn add_glusterfs_peer(&mut self, peer: &Peer) -> std::io::Result<()> {
        let output = std::process::Command::new("gluster")
            .arg("peer")
            .arg("probe")
            .arg(peer.ip_address().to_string())
            .arg("--mode=script")
            .output()?;

        if !output.status.success() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to add peer to NFS volume"))
        }

        Ok(())
    }

    pub(super) async fn create_gluster_volume(
        &mut self,
        instance: Namespace,
        peers: Vec<&Peer>
    ) -> std::io::Result<()> {
        // Simply create the volume
        let mut command = std::process::Command::new("gluster");
        command
            .arg("volume")
            .arg("create")
            .arg(instance.inner().to_string());

        let replica = peers.len();
        command.arg("replica")
            .arg(&replica.to_string());

        for peer in peers {
            command.arg(
                &format!(
                    "{}:/mnt/glusterfs/vms/{}/brick", 
                    peer.ip_address().to_string(),
                    instance.inner().to_string()
                )
            );
        }

        command.arg("force").arg("--mode=script");
        command.output()?;

        Ok(())
    }

    pub(super) fn get_gluster_volumes(&self) -> std::io::Result<Vec<String>> {
        let output = std::process::Command::new("gluster")
            .arg("volume")
            .arg("list")
            .output()?;

        if !output.status.success() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to list gluster volumes"))
        }

        let volumes = String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .map(String::from)
            .collect();

        Ok(volumes)
    }

    pub(super) async fn add_peer_to_gluster_volume(&self, peer: &Peer, instance: Namespace) -> std::io::Result<()> {
        let output = std::process::Command::new("gluster")
            .arg("volume")
            .arg("add-brick")
            .arg(&instance.inner().to_string())
            .arg(&format!("{}:/mnt/glusterfs/vms/{}/brick", peer.ip_address(), instance.inner().to_string()))
            .arg("force")
            .arg("--mode=script")
            .output()?;

        if !output.status.success() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to add peer to GlusterFS volume"));
        }

        Ok(())
    }

    pub(super) async fn increase_glusterfs_replica_factor(&self) -> std::io::Result<()> {
        let volumes = self.get_gluster_volumes()?;
        for volume in volumes {
            let output = std::process::Command::new("gluster")
                .arg("volume")
                .arg("set")
                .arg(&volume)
                .arg("replica")
                .arg(self.size().to_string())
                .arg("force")
                .arg("--mode=script")
                .output()?;
            if !output.status.success() {
                return Err(
                    std::io::Error::new(
                        std::io::ErrorKind::Other, 
                        format!("Failed to set replica for volume {}", volume)
                    )
                )
            }
        }

        Ok(())
    }

    pub(super) async fn share_instances(
        &mut self, 
        publisher: &mut GenericPublisher, 
        instances: HashSet<Namespace>,
        peer: Peer
    ) -> std::io::Result<()> {
        let event_id = Uuid::new_v4().to_string();
        let task_id = TaskId::new(Uuid::new_v4().to_string());
        let event = NetworkEvent::ShareInstanceNamespaces { event_id, task_id, instances, peer };
        publisher.publish(
            Box::new(NetworkTopic),
            Box::new(event)
        ).await?;

        Ok(())
    }
    
    pub fn size(&self) -> usize {
        self.peers().len()
    }
}

pub enum QuorumResult {
    Unit(()),
    Other(String),
}


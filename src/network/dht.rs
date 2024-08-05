use alloy::primitives::Address;
use anchorhash::AnchorHash;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;
use std::str::FromStr;
use std::{
    collections::{HashMap, HashSet},
    hash::RandomState,
};
use uuid::Uuid;

use crate::params::{
    AddPubkeyParams, CreateParams, DeleteParams, ExposeServiceParams, StartParams, StopParams,
};
use crate::{
    account::{Namespace, TaskId},
    allegra_rpc::InstanceGetSshDetails,
    consts::NGINX_CONFIG_PATH,
    event::{NetworkEvent, QuorumEvent, VmmEvent},
    network::node::Node,
    params::Params,
    publish::{GenericPublisher, NetworkTopic, VmManagerTopic},
    subscribe::QuorumSubscriber,
    VmType,
};

use crate::statics::BOOTSTRAP_QUORUM;
use crate::{Peer, Quorum, QuorumResult};
use conductor::publisher::PubStream;
use conductor::subscriber::SubStream;
use getset::{Getters, MutGetters};
use tokio::time::{interval, Duration, Instant};
use tokio::{io::AsyncWriteExt, task::JoinHandle};

#[derive(Getters, MutGetters)]
#[getset(get = "pub", get_copy = "pub", get_mut)]
pub struct QuorumManager {
    node: Node,
    peers: HashMap<Address, Peer>,
    instances: HashMap<Namespace, Quorum>,
    quorums: HashMap<Uuid, Quorum>,
    peer_hashring: AnchorHash<Address, Quorum, RandomState>,
    instance_hashring: AnchorHash<Namespace, Quorum, RandomState>,
    launch_ready: HashMap<Namespace, HashSet<Peer>>,
    subscriber: QuorumSubscriber,
    publisher: GenericPublisher,
    futures: FuturesUnordered<JoinHandle<std::io::Result<QuorumResult>>>,
    heartbeats: HashMap<Peer, Instant>,
}

#[allow(unused)]
impl QuorumManager {
    pub async fn new(
        subscriber_uri: &str,
        publisher_uri: &str,
        peer_info: Peer,
    ) -> std::io::Result<Self> {
        let peer_hashring = anchorhash::Builder::default()
            .with_resources(vec![BOOTSTRAP_QUORUM.clone()])
            .build(100);

        let instance_hashring = anchorhash::Builder::default()
            .with_resources(vec![BOOTSTRAP_QUORUM.clone()])
            .build(100);

        let mut quorums = HashMap::new();
        quorums.insert(BOOTSTRAP_QUORUM.id().clone(), BOOTSTRAP_QUORUM.clone());
        let publisher = GenericPublisher::new(publisher_uri).await?;
        log::info!("Publisher publishing to {}", publisher_uri);
        let subscriber = QuorumSubscriber::new(subscriber_uri).await?;
        log::info!("subscriber listening on {}", subscriber_uri);
        let node = Node::new(peer_info);

        Ok(Self {
            node,
            peers: HashMap::new(),
            instances: HashMap::new(),
            quorums,
            peer_hashring,
            instance_hashring,
            launch_ready: HashMap::new(),
            publisher,
            subscriber,
            futures: FuturesUnordered::new(),
            heartbeats: HashMap::new(),
        })
    }

    pub async fn run(&mut self) -> std::io::Result<()> {
        let mut election_interval = interval(Duration::from_secs(21600));
        let mut heartbeat_interval = interval(Duration::from_secs(60));
        let mut check_remotes_interval = interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                result = self.subscriber.receive() => {
                    log::info!("Quorum subscriber received message");
                    match result {
                        Ok(messages) => {
                            for m in messages.clone() {
                                log::info!("Received message: {:?}", m);
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
                    self.heartbeat().await?;
                    self.check_heartbeats().await?;
                },
                _ = tokio::signal::ctrl_c() => {
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_quorum_message(&mut self, m: QuorumEvent) -> std::io::Result<()> {
        log::info!("Received message: {:?}", m);
        match m {
            QuorumEvent::Expand { .. } => todo!(),
            QuorumEvent::Consolidate { .. } => todo!(),
            QuorumEvent::RequestSshDetails { .. } => todo!(),
            QuorumEvent::NewPeer {
                event_id,
                task_id,
                peer,
                received_from,
            } => {
                //log::info!("Received NewPeer quorum message: {event_id}: {task_id}");
                self.handle_new_peer_message(&peer, &received_from).await?;
                log::info!("Successfully completed self.handle_new_peer_message call for QuorumEvent::NewPeer message...");
            }
            QuorumEvent::CheckResponsibility {
                event_id,
                task_id,
                namespace,
                payload,
            } => {
                log::info!("Received CheckResponsibility quorum message: {event_id}: {task_id}");
                self.handle_check_responsibility_message(&namespace, &payload, task_id)
                    .await?;
                log::info!("Successfully completed self.handle_check_responsibility_message call for QuorumEvent::CheckResponsibility message...");
            }
            QuorumEvent::BootstrapInstances {
                instances,
                received_from,
                event_id,
                task_id,
            } => {
                self.bootstrap_instances(instances, &received_from).await?;
            }
            QuorumEvent::BootstrapInstancesComplete {
                event_id,
                peer,
                task_id,
            } => {
                // Here we need to add the brick to the gluster volume
                self.complete_bootstrap(&peer).await?;
            }
            QuorumEvent::PreparedForLaunch {
                event_id,
                task_id,
                instance,
            } => {
                self.prepared_for_launch(instance).await?;
            }
            QuorumEvent::AcceptLaunchPreparation { peer, instance, .. } => {
                log::info!("QuorumManager received AcceptLaunchPreparation Event...");
                self.accept_launch_preparation(peer, instance).await;
            }
        }

        Ok(())
    }

    async fn accept_launch_preparation(
        &mut self,
        peer: Peer,
        instance: Namespace,
    ) -> std::io::Result<()> {
        let quorum_id = self.get_quorum_responsible(&instance)?;
        let mut quorum = self
            .get_quorum_by_id(&quorum_id)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to find quorum",
            ))?
            .clone();

        log::info!("Checking if entry exists or inserting new set");
        let mut entry = self
            .launch_ready
            .entry(instance.clone())
            .or_insert(HashSet::new());
        entry.insert(peer);
        log::info!("Added prepared peer to set...");

        let peers = quorum.peers().clone();
        let mut prepared = true;
        for peer in peers {
            if !entry.contains(&peer) {
                prepared = false;
            }
        }

        if prepared {
            log::info!("attempting to create gluster volume for {}", &instance.inner().to_string());
            quorum
                .create_gluster_volume(instance.clone(), quorum.peers().clone().iter().collect())
                .await?;
        }

        let event_id = Uuid::new_v4().to_string();
        let task_id = TaskId::new(Uuid::new_v4().to_string());
        let event = VmmEvent::LaunchInstance {
            event_id,
            task_id,
            namespace: instance.clone(),
        };

        self.publisher_mut()
            .publish(Box::new(VmManagerTopic), Box::new(event))
            .await?;

        Ok(())
    }

    async fn prepared_for_launch(&mut self, instance: Namespace) -> std::io::Result<()> {
        // Create and send network event to each peer
        let local_peer = self.node().peer_info().clone();
        let peers = self
            .get_quorum_peers(&local_peer)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to find peers",
            ))?;

        for peer in peers {
            if peer != local_peer {
                let event_id = Uuid::new_v4().to_string();
                let task_id = TaskId::new(Uuid::new_v4().to_string());
                let event = NetworkEvent::PreparedForLaunch {
                    event_id,
                    task_id,
                    instance: instance.clone(),
                    dst: peer.clone(),
                    local_peer: local_peer.clone(),
                };

                self.publisher_mut()
                    .publish(Box::new(NetworkTopic), Box::new(event))
                    .await?;
            }
        }

        self.accept_launch_preparation(local_peer, instance).await?;

        Ok(())
    }

    fn is_responsible(&mut self, namespace: &Namespace) -> Option<bool> {
        let quorum_id = self.get_instance_quorum_membership(&namespace)?;
        let local_peer = self.node().peer_info();
        let quorum_membership = self.get_peer_quorum_membership(local_peer)?;
        if quorum_membership == quorum_id {
            return Some(true);
        } else {
            return Some(false);
        }
    }

    fn get_local_quorum_membership(&mut self) -> std::io::Result<Uuid> {
        let local_peer = self.node().peer_info();
        Ok(self
            .get_peer_quorum_membership(local_peer)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Local Peer's quorum cannot be found",
            ))?)
    }

    fn get_local_peers(&mut self) -> Option<&HashSet<Peer>> {
        let local_peer = self.node().peer_info();
        let local_quorum_id = self.get_peer_quorum_membership(local_peer)?;
        let peers = self.get_quorum_by_id(&local_quorum_id)?.peers();
        Some(peers)
    }

    fn get_quorum_peers_by_id(&mut self, quorum_id: &Uuid) -> std::io::Result<&HashSet<Peer>> {
        Ok(self
            .get_quorum_by_id(quorum_id)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Unable to acquire peers for quorum {}", quorum_id),
            ))?
            .peers())
    }

    async fn handle_create_payload(
        &mut self,
        namespace: &Namespace,
        payload: &CreateParams,
        task_id: &TaskId,
    ) -> std::io::Result<()> {
        let publisher_addr = self.publisher.peer_addr()?;
        let event_id = Uuid::new_v4().to_string();
        let quorum_responsible = self.get_quorum_responsible(&namespace)?;
        let peers = self.get_quorum_peers_by_id(&quorum_responsible)?.clone();
        if let Some(true) = self.is_responsible(&namespace) {
            log::info!(
                "Local node quorum is responsible for instance {}",
                &namespace.to_string()
            );
            self.publisher_mut()
                .publish(
                    Box::new(VmManagerTopic),
                    Box::new(VmmEvent::Create {
                        event_id,
                        task_id: task_id.clone(),
                        name: payload.name.clone(),
                        distro: payload.distro.clone(),
                        version: payload.version.clone(),
                        vmtype: VmType::from_str(&payload.vmtype)?.clone(),
                        sig: payload.sig.clone(),
                        recovery_id: payload.recovery_id.clone(),
                        sync: payload.sync.clone(),
                        memory: payload.memory.clone(),
                        vcpus: payload.vcpus.clone(),
                        cpu: payload.cpu.clone(),
                        metadata: payload.metadata.clone(),
                        os_variant: payload.os_variant.clone(),
                        host_device: payload.host_device.clone(),
                        network: payload.network.clone(),
                        disk: payload.disk.clone(),
                        filesystem: payload.filesystem.clone(),
                        controller: payload.controller.clone(),
                        input: payload.input.clone(),
                        graphics: payload.graphics.clone(),
                        sound: payload.sound.clone(),
                        video: payload.video.clone(),
                        smartcard: payload.smartcard.clone(),
                        redirdev: payload.redirdev.clone(),
                        memballoon: payload.memballoon.clone(),
                        tpm: payload.tpm.clone(),
                        rng: payload.rng.clone(),
                        panic: payload.panic.clone(),
                        shmem: payload.shmem.clone(),
                        memdev: payload.memdev.clone(),
                        vsock: payload.vsock.clone(),
                        iommu: payload.iommu.clone(),
                        watchdog: payload.watchdog.clone(),
                        serial: payload.serial.clone(),
                        parallel: payload.parallel.clone(),
                        channel: payload.channel.clone(),
                        console: payload.console.clone(),
                        install: payload.install.clone(),
                        cdrom: payload.cdrom.clone(),
                        location: payload.location.clone(),
                        pxe: payload.pxe.clone(),
                        import: payload.import.clone(),
                        boot: payload.boot.clone(),
                        idmap: payload.idmap.clone(),
                        features: payload
                            .features
                            .par_iter()
                            .map(|f| (f.name.clone(), f.feature.clone()))
                            .collect(),
                        clock: payload.clock.clone(),
                        launch_security: payload.launch_security.clone(),
                        numatune: payload.numatune.clone(),
                        boot_dev: payload.boot_dev.clone(),
                        unattended: payload.unattended.clone(),
                        print_xml: payload.print_xml.clone(),
                        dry_run: payload.dry_run.clone(),
                        connect: payload.connect.clone(),
                        virt_type: payload.virt_type.clone(),
                        cloud_init: match payload.cloud_init.clone() {
                            Some(ci) => Some(ci.into()),
                            None => None,
                        },
                    }),
                )
                .await?;
        }

        for p in peers {
            if p != self.node().peer_info().clone() {
                let publish_to_addr = self.publisher().peer_addr()?;
                let inner_payload = payload.clone();
                let inner_task_id = task_id.clone();
                let future = tokio::spawn(async move {
                    log::info!(
                        "publishing payload for {inner_task_id} to {}",
                        p.ip_address().to_string()
                    );
                    let mut publisher = GenericPublisher::new(&publish_to_addr).await?;
                    let event_id = Uuid::new_v4().to_string();
                    let _ = publisher
                        .publish(
                            Box::new(NetworkTopic),
                            Box::new(NetworkEvent::Create {
                                event_id,
                                task_id: inner_task_id.clone(),
                                name: inner_payload.name.clone(),
                                distro: inner_payload.distro.clone(),
                                version: inner_payload.version.clone(),
                                vmtype: inner_payload.vmtype.clone().to_string(),
                                sig: inner_payload.sig.clone(),
                                recovery_id: inner_payload.recovery_id,
                                dst: p.ip_address().to_string(),
                                sync: inner_payload.sync,
                                memory: inner_payload.memory,
                                vcpus: inner_payload.vcpus,
                                cpu: inner_payload.cpu,
                                metadata: inner_payload.metadata,
                                os_variant: inner_payload.os_variant,
                                host_device: inner_payload.host_device,
                                network: inner_payload.network,
                                disk: inner_payload.disk,
                                filesystem: inner_payload.filesystem,
                                controller: inner_payload.controller,
                                input: inner_payload.input,
                                graphics: inner_payload.graphics,
                                sound: inner_payload.sound,
                                video: inner_payload.video,
                                smartcard: inner_payload.smartcard,
                                redirdev: inner_payload.redirdev,
                                memballoon: inner_payload.memballoon,
                                tpm: inner_payload.tpm,
                                rng: inner_payload.rng,
                                panic: inner_payload.panic,
                                shmem: inner_payload.shmem,
                                memdev: inner_payload.memdev,
                                vsock: inner_payload.vsock,
                                iommu: inner_payload.iommu,
                                watchdog: inner_payload.watchdog,
                                serial: inner_payload.serial,
                                parallel: inner_payload.parallel,
                                channel: inner_payload.channel,
                                console: inner_payload.console,
                                install: inner_payload.install,
                                cdrom: inner_payload.cdrom,
                                location: inner_payload.location,
                                pxe: inner_payload.pxe,
                                import: inner_payload.import,
                                boot: inner_payload.boot,
                                idmap: inner_payload.idmap,
                                features: inner_payload
                                    .features
                                    .par_iter()
                                    .map(|f| (f.name.clone(), f.feature.clone()))
                                    .collect(),
                                clock: inner_payload.clock,
                                launch_security: inner_payload.launch_security,
                                numatune: inner_payload.numatune,
                                boot_dev: inner_payload.boot_dev,
                                unattended: inner_payload.unattended,
                                print_xml: inner_payload.print_xml,
                                dry_run: inner_payload.dry_run,
                                connect: inner_payload.connect,
                                virt_type: inner_payload.virt_type,
                                cloud_init: match inner_payload.cloud_init {
                                    Some(ci) => Some(ci.into()),
                                    None => None,
                                },
                            }),
                        )
                        .await?;

                    Ok::<_, std::io::Error>(QuorumResult::Unit(()))
                });

                self.futures.push(future);
            }
        }

        Ok(())
    }

    async fn attempt_share_server_config(
        dst: Peer,
        requestor: Peer,
        publisher_uri: &str,
    ) -> std::io::Result<()> {
        todo!()
    }

    async fn handle_start_payload(
        &mut self,
        namespace: &Namespace,
        payload: &StartParams,
        task_id: &TaskId,
    ) -> std::io::Result<()> {
        let publisher_addr = self.publisher.peer_addr()?;
        let event_id = Uuid::new_v4().to_string();
        let quorum_responsible = self.get_quorum_responsible(&namespace)?;
        let peers = self.get_quorum_peers_by_id(&quorum_responsible)?.clone();
        if let Some(true) = self.is_responsible(namespace) {
            self.publisher_mut()
                .publish(
                    Box::new(VmManagerTopic),
                    Box::new(VmmEvent::Start {
                        event_id,
                        task_id: task_id.clone(),
                        name: payload.name.clone(),
                        console: payload.console,
                        stateless: payload.stateless,
                        sig: payload.sig.clone(),
                        recovery_id: payload.recovery_id,
                    }),
                )
                .await?;
        }

        let futures = peers
            .par_iter()
            .map(|p| {
                let payload = payload.clone();
                let publish_to_addr = publisher_addr.clone();
                let peer = p.clone();
                let task_id = task_id.clone();
                tokio::spawn(async move {
                    let mut publisher = GenericPublisher::new(&publish_to_addr).await?;
                    let event_id = Uuid::new_v4().to_string();
                    publisher
                        .publish(
                            Box::new(NetworkTopic),
                            Box::new(NetworkEvent::Start {
                                event_id,
                                task_id: task_id.clone(),
                                name: payload.name.clone(),
                                console: payload.console,
                                stateless: payload.stateless,
                                sig: payload.sig.clone(),
                                recovery_id: payload.recovery_id,
                                dst: peer.ip_address().to_string(),
                            }),
                        )
                        .await?;

                    Ok::<_, std::io::Error>(QuorumResult::Unit(()))
                })
            })
            .collect::<Vec<_>>();

        self.futures.extend(futures);

        Ok(())
    }

    async fn handle_stop_payload(
        &mut self,
        namespace: &Namespace,
        payload: &StopParams,
        task_id: &TaskId,
    ) -> std::io::Result<()> {
        let publisher_addr = self.publisher.peer_addr()?;
        let event_id = Uuid::new_v4().to_string();
        let quorum_responsible = self.get_quorum_responsible(&namespace)?;
        let peers = self.get_quorum_peers_by_id(&quorum_responsible)?.clone();
        if let Some(true) = self.is_responsible(namespace) {
            self.publisher_mut()
                .publish(
                    Box::new(VmManagerTopic),
                    Box::new(VmmEvent::Stop {
                        event_id,
                        task_id: task_id.clone(),
                        name: payload.name.clone(),
                        sig: payload.sig.clone(),
                        recovery_id: payload.recovery_id,
                    }),
                )
                .await?;
        }

        let futures = peers
            .par_iter()
            .map(|p| {
                let payload = payload.clone();
                let publish_to_addr = publisher_addr.clone();
                let peer = p.clone();
                let task_id = task_id.clone();
                tokio::spawn(async move {
                    let mut publisher = GenericPublisher::new(&publish_to_addr).await?;
                    let event_id = Uuid::new_v4().to_string();
                    publisher
                        .publish(
                            Box::new(NetworkTopic),
                            Box::new(NetworkEvent::Stop {
                                event_id,
                                task_id: task_id.clone(),
                                name: payload.name.clone(),
                                sig: payload.sig.clone(),
                                recovery_id: payload.recovery_id,
                                dst: peer.ip_address().to_string(),
                            }),
                        )
                        .await?;

                    Ok::<_, std::io::Error>(QuorumResult::Unit(()))
                })
            })
            .collect::<Vec<_>>();

        self.futures.extend(futures);

        Ok(())
    }

    async fn handle_delete_payload(
        &mut self,
        namespace: &Namespace,
        payload: &DeleteParams,
        task_id: &TaskId,
    ) -> std::io::Result<()> {
        let publisher_addr = self.publisher.peer_addr()?;
        let event_id = Uuid::new_v4().to_string();
        let quorum_responsible = self.get_quorum_responsible(&namespace)?;
        let peers = self.get_quorum_peers_by_id(&quorum_responsible)?.clone();
        if let Some(true) = self.is_responsible(namespace) {
            self.publisher_mut()
                .publish(
                    Box::new(VmManagerTopic),
                    Box::new(VmmEvent::Delete {
                        event_id,
                        task_id: task_id.clone(),
                        name: payload.name.clone(),
                        sig: payload.sig.clone(),
                        recovery_id: payload.recovery_id,
                        force: payload.force,
                        interactive: payload.interactive,
                    }),
                )
                .await?;
        }

        let futures = peers
            .par_iter()
            .map(|p| {
                let payload = payload.clone();
                let publish_to_addr = publisher_addr.clone();
                let peer = p.clone();
                let task_id = task_id.clone();
                tokio::spawn(async move {
                    let mut publisher = GenericPublisher::new(&publish_to_addr).await?;
                    let event_id = Uuid::new_v4().to_string();
                    publisher
                        .publish(
                            Box::new(NetworkTopic),
                            Box::new(NetworkEvent::Delete {
                                event_id,
                                task_id: task_id.clone(),
                                name: payload.name.clone(),
                                sig: payload.sig.clone(),
                                recovery_id: payload.recovery_id,
                                force: payload.force,
                                interactive: payload.interactive,
                                dst: peer.ip_address().to_string(),
                            }),
                        )
                        .await?;

                    Ok::<_, std::io::Error>(QuorumResult::Unit(()))
                })
            })
            .collect::<Vec<_>>();

        self.futures.extend(futures);

        Ok(())
    }

    async fn handle_add_pubkey_payload(
        &mut self,
        namespace: &Namespace,
        payload: &AddPubkeyParams,
        task_id: &TaskId,
    ) -> std::io::Result<()> {
        let publisher_addr = self.publisher.peer_addr()?;
        let event_id = Uuid::new_v4().to_string();
        let quorum_responsible = self.get_quorum_responsible(&namespace)?;
        let peers = self.get_quorum_peers_by_id(&quorum_responsible)?.clone();
        if let Some(true) = self.is_responsible(namespace) {
            self.publisher_mut()
                .publish(
                    Box::new(VmManagerTopic),
                    Box::new(VmmEvent::AddPubkey {
                        event_id,
                        task_id: task_id.clone(),
                        name: payload.name.clone(),
                        sig: payload.sig.clone(),
                        recovery_id: payload.recovery_id,
                        pubkey: payload.pubkey.clone(),
                    }),
                )
                .await?;
        }

        let futures = peers
            .par_iter()
            .map(|p| {
                let payload = payload.clone();
                let publish_to_addr = publisher_addr.clone();
                let peer = p.clone();
                let task_id = task_id.clone();
                tokio::spawn(async move {
                    let mut publisher = GenericPublisher::new(&publish_to_addr).await?;
                    let event_id = Uuid::new_v4().to_string();
                    publisher
                        .publish(
                            Box::new(NetworkTopic),
                            Box::new(NetworkEvent::AddPubkey {
                                event_id,
                                task_id: task_id.clone(),
                                name: payload.name.clone(),
                                sig: payload.sig.clone(),
                                recovery_id: payload.recovery_id,
                                pubkey: payload.pubkey.clone(),
                                dst: peer.ip_address().to_string(),
                            }),
                        )
                        .await?;

                    Ok::<_, std::io::Error>(QuorumResult::Unit(()))
                })
            })
            .collect::<Vec<_>>();

        self.futures.extend(futures);

        Ok(())
    }

    async fn handle_expose_service_payload(
        &mut self,
        namespace: &Namespace,
        payload: &ExposeServiceParams,
        task_id: &TaskId,
    ) -> std::io::Result<()> {
        let publisher_addr = self.publisher.peer_addr()?;
        let event_id = Uuid::new_v4().to_string();
        let quorum_responsible = self.get_quorum_responsible(&namespace)?;
        let peers = self.get_quorum_peers_by_id(&quorum_responsible)?.clone();
        if let Some(true) = self.is_responsible(namespace) {
            self.publisher_mut()
                .publish(
                    Box::new(VmManagerTopic),
                    Box::new(VmmEvent::ExposeService {
                        event_id,
                        task_id: task_id.clone(),
                        name: payload.name.clone(),
                        sig: payload.sig.clone(),
                        recovery_id: payload.recovery_id,
                        port: payload
                            .port
                            .iter()
                            .filter_map(|i| {
                                let i = *i;
                                i.try_into().ok().clone()
                            })
                            .collect::<Vec<u16>>()
                            .clone(),
                        service_type: payload
                            .service_type
                            .iter()
                            .map(|i| crate::params::ServiceType::from(i.clone()))
                            .collect::<Vec<crate::params::ServiceType>>()
                            .clone(),
                    }),
                )
                .await?;
        }

        let futures = peers
            .par_iter()
            .map(|p| {
                let payload = payload.clone();
                let publish_to_addr = publisher_addr.clone();
                let peer = p.clone();
                let task_id = task_id.clone();
                tokio::spawn(async move {
                    let mut publisher = GenericPublisher::new(&publish_to_addr).await?;
                    let event_id = Uuid::new_v4().to_string();
                    publisher
                        .publish(
                            Box::new(NetworkTopic),
                            Box::new(NetworkEvent::ExposeService {
                                event_id,
                                task_id: task_id.clone(),
                                name: payload.name.clone(),
                                sig: payload.sig.clone(),
                                recovery_id: payload.recovery_id,
                                port: payload
                                    .port
                                    .iter()
                                    .filter_map(|i| {
                                        let i = *i;
                                        i.try_into().ok().clone()
                                    })
                                    .collect(),
                                service_type: payload
                                    .service_type
                                    .iter()
                                    .map(|i| crate::params::ServiceType::from(i.clone()))
                                    .collect::<Vec<crate::params::ServiceType>>()
                                    .clone(),
                                dst: peer.ip_address().to_string(),
                            }),
                        )
                        .await?;

                    Ok::<_, std::io::Error>(QuorumResult::Unit(()))
                })
            })
            .collect::<Vec<_>>();

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
        received_from: &Peer,
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
        Ok(self
            .get_instance_quorum_membership(&namespace)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Unable to acquire quorum id for instance: {}",
                    &namespace.to_string()
                ),
            ))?)
    }

    async fn elect_leader(&mut self) -> std::io::Result<()> {
        let quorums = self.quorums().clone();
        let quorum_peers = quorums
            .get(
                &self
                    .get_peer_quorum_membership(self.node.peer_info())
                    .ok_or(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "unable to find membership for local node",
                    ))?,
            )
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to find peers for local node quorum",
            ))?;
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
        peer_hashring
            .add_resource(quorum.clone())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let mut instance_hashring = self.instance_hashring.clone();

        instance_hashring
            .add_resource(quorum.clone())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        self.peer_hashring = peer_hashring;
        self.instance_hashring = instance_hashring;
        self.quorums.insert(quorum.id().to_owned(), quorum);

        self.reorganize_resources().await?;

        Ok(())
    }

    fn extract_cert(cert: &str) -> std::io::Result<String> {
        let re = Regex::new(r"token:\n(.*)\n")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        re.captures(cert)
            .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to extract jwt from cert",
            ))
    }

    pub async fn accept_server_config(
        &mut self,
        peer: &Peer,
        server_config: &str,
    ) -> std::io::Result<()> {
        let leader = self
            .node()
            .current_leader()
            .clone()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "quorum currently has no leader",
            ))?;

        if peer == &leader {
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(NGINX_CONFIG_PATH)
                .await?;

            file.write_all(server_config.as_bytes()).await?;

            log::info!("wrote to nginx config file");
            let output = std::process::Command::new("sudo")
                .args(["nginx", "-t"])
                .output()?;

            if !output.status.success() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "nginx config has a syntax error",
                ));
            }
            log::info!("confirmed nginx config file free of syntax errors...");

            let output = std::process::Command::new("sudo")
                .args(["systemctl", "reload", "nginx"])
                .output()?;

            if !output.status.success() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "could not reload nginx after updating config",
                ));
            }

            log::info!("reloaded nginx...");
        }

        Ok(())
    }

    pub async fn heartbeat(&mut self) -> std::io::Result<()> {
        let local_peers = self
            .get_local_peers()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to acquire local peers",
            ))?
            .clone();
        let publisher_uri = self.publisher.peer_addr()?.clone();
        for peer in local_peers {
            let requestor = self.node().peer_info().clone();
            let inner_publisher_uri = publisher_uri.clone();
            let event_id = Uuid::new_v4().to_string();
            let task_id = TaskId::new(Uuid::new_v4().to_string());
            let event = NetworkEvent::Heartbeat {
                event_id,
                task_id,
                peer: peer.clone(),
                requestor: requestor.clone(),
            };

            let mut publisher = GenericPublisher::new(&inner_publisher_uri).await?;

            publisher
                .publish(Box::new(NetworkTopic), Box::new(event))
                .await?;
        }

        Ok(())
    }

    pub async fn accept_heartbeat(&mut self, peer: &Peer) -> std::io::Result<()> {
        self.heartbeats.insert(peer.clone(), Instant::now());

        Ok(())
    }

    pub async fn check_heartbeats(&mut self) -> std::io::Result<()> {
        let systime = Instant::now();
        let heartbeats = self.heartbeats.clone();
        let dead_peers: Vec<&Peer> = heartbeats
            .par_iter()
            .filter_map(|(p, hb)| {
                let since = systime.duration_since(*hb);
                if since > Duration::from_secs(60) {
                    Some(p)
                } else {
                    None
                }
            })
            .collect();

        for peer in dead_peers {
            self.remove_peer(peer).await?;
        }

        Ok(())
    }

    pub async fn remove_peer(&mut self, peer: &Peer) -> std::io::Result<()> {
        let current_leader = self
            .node()
            .current_leader()
            .clone()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to find current leader",
            ))?;

        if peer == &current_leader {
            self.peers
                .remove(peer.wallet_address())
                .ok_or(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "unable to remove peer",
                ))?;
        }

        Ok(())
    }

    pub async fn add_peer(
        &mut self,
        peer: &Peer,
        received_from: Option<&Peer>,
    ) -> std::io::Result<()> {
        let q = self
            .peer_hashring
            .get_resource(peer.wallet_address().clone())
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to map peer to resource",
            ))?
            .clone();

        let local_quorum_member = if q.id() == &self.get_local_quorum_membership()? {
            true
        } else {
            false
        };

        log::warn!("local quorum member = {}", &local_quorum_member);
        let quorum = self
            .quorums
            .get_mut(&q.id().clone())
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "quorum assigned does not exist",
            ))?;

        quorum.add_peer(peer).await?;
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
                if (&dst_peer != peer)
                    && (&dst_peer != self.node.peer_info())
                    && (Some(&dst_peer.clone()) != received_from)
                {
                    let task_id = TaskId::new(uuid::Uuid::new_v4().to_string());
                    let event_id = uuid::Uuid::new_v4().to_string();
                    let dst_event = NetworkEvent::NewPeer {
                        peer_id: peer.wallet_address_hex(),
                        peer_address: peer.ip_address().to_string(),
                        dst: dst_peer.ip_address().to_string(),
                        task_id,
                        event_id,
                        received_from: self.node().peer_info().clone(),
                    };

                    self.publisher_mut()
                        .publish(Box::new(NetworkTopic), Box::new(dst_event))
                        .await?;

                    let task_id = TaskId::new(uuid::Uuid::new_v4().to_string());

                    let event_id = uuid::Uuid::new_v4().to_string();
                    let new_peer_event = NetworkEvent::NewPeer {
                        event_id,
                        task_id,
                        peer_id: dst_peer.wallet_address_hex(),
                        peer_address: dst_peer.ip_address().to_string(),
                        dst: peer.ip_address().to_string(),
                        received_from: self.node().peer_info().clone(),
                    };

                    self.publisher_mut()
                        .publish(Box::new(NetworkTopic), Box::new(new_peer_event))
                        .await?;
                }
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
        let q = self
            .peer_hashring
            .get_resource(peer.wallet_address().clone())?;
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

    pub fn get_quorum_by_id_mut(&mut self, id: &Uuid) -> Option<&mut Quorum> {
        self.quorums.get_mut(id)
    }

    pub fn add_instance(&mut self, namespace: &Namespace) -> std::io::Result<()> {
        let q = self
            .instance_hashring
            .get_resource(namespace.clone())
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to map instance to resource",
            ))?;

        let quorum = self.quorums.get_mut(q.id()).ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "quorum assigned does not exist",
        ))?;

        self.instances.insert(namespace.clone(), quorum.clone());

        Ok(())
    }

    pub async fn bootstrap_instances(
        &mut self,
        instances: Vec<Namespace>,
        received_from: &Peer,
    ) -> std::io::Result<()> {
        self.node_mut().setup_instance_bricks(instances).await?;

        let event_id = Uuid::new_v4().to_string();
        let task_id = TaskId::new(Uuid::new_v4().to_string());
        let event = NetworkEvent::BootstrapInstancesResponse {
            event_id,
            task_id,
            requestor: self.node().peer_info().clone(),
            bootstrapper: received_from.clone(),
        };

        self.publisher_mut()
            .publish(Box::new(NetworkTopic), Box::new(event))
            .await?;

        Ok(())
    }

    pub async fn complete_bootstrap(&mut self, peer: &Peer) -> std::io::Result<()> {
        let local_quorum_id = self.get_local_quorum_membership()?;
        let instances = self.instances.clone();
        let local_quorum =
            self.get_quorum_by_id_mut(&local_quorum_id)
                .ok_or(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Unable to find quorum",
                ))?;

        let instances: Vec<Namespace> = instances
            .iter()
            .filter_map(|(n, q)| {
                if q.id() == &local_quorum_id {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .collect();

        for instance in instances {
            local_quorum.increase_glusterfs_replica_factor().await?;
            local_quorum
                .add_peer_to_gluster_volume(peer, instance)
                .await?;
        }

        Ok(())
    }
}

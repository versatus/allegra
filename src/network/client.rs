use crate::{
    allegra_rpc::{
        BootstrapCompleteMessage, BootstrapInstancesMessage, Features, InstanceAddPubkeyParams,
        InstanceCreateParams, InstanceDeleteParams, InstanceExposeServiceParams,
        InstanceStartParams, InstanceStopParams, MessageHeader, NewPeerMessage,
        PreparedForLaunchMessage, ServerConfigMessage, SyncEvent, SyncMessage,
    },
    create_allegra_rpc_client_to_addr,
    event::NetworkEvent,
    network::peer::Peer,
    publish::GenericPublisher,
    subscribe::NetworkSubscriber,
};
use base64::Engine as _;
use conductor::subscriber::SubStream;
use tokio::time::{interval, Duration};
use uuid::Uuid;

pub struct NetworkClient {
    local_peer: Peer,
    subscriber: NetworkSubscriber,
    #[allow(unused)]
    publisher: GenericPublisher,
}

impl NetworkClient {
    pub async fn new(
        local_peer: Peer,
        subscriber_uri: &str,
        publisher_uri: &str,
    ) -> std::io::Result<Self> {
        let subscriber = NetworkSubscriber::new(subscriber_uri).await?;
        let publisher = GenericPublisher::new(publisher_uri).await?;

        Ok(Self {
            local_peer,
            subscriber,
            publisher,
        })
    }

    pub async fn run(mut self) -> std::io::Result<()> {
        let mut heartbeat_interval = interval(Duration::from_secs(20));
        loop {
            tokio::select! {
                Ok(messages) = self.subscriber.receive() => {
                    log::info!("received {} messages", messages.len());
                    for message in messages {
                        //log::info!("Attempting to handle message: {:?}", message);
                        if let Err(e) = self.handle_networking_event(message.clone()).await {
                            log::error!("self.handle_networking_event(message): {e}: message: {message:?}");
                        }
                        //log::info!("Completed message handling");
                    }
                    //log::info!("handled all messages awaiting next batch...");
                },
                _heartbeat = heartbeat_interval.tick() => {
                    log::info!("Network client is still alive");
                },
                _ = tokio::signal::ctrl_c() => {
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_networking_event(&mut self, event: NetworkEvent) -> std::io::Result<()> {
        match event {
            NetworkEvent::Create {
                name,
                distro,
                version,
                vmtype,
                sig,
                recovery_id,
                dst,
                sync,
                memory,
                vcpus,
                cpu,
                metadata,
                os_variant,
                host_device,
                network,
                disk,
                filesystem,
                controller,
                input,
                graphics,
                sound,
                video,
                smartcard,
                redirdev,
                memballoon,
                tpm,
                rng,
                panic,
                shmem,
                memdev,
                vsock,
                iommu,
                watchdog,
                serial,
                parallel,
                channel,
                console,
                install,
                cdrom,
                location,
                pxe,
                import,
                boot,
                idmap,
                features,
                clock,
                launch_security,
                numatune,
                boot_dev,
                unattended,
                print_xml,
                dry_run,
                connect,
                virt_type,
                cloud_init,
                ..
            } => {
                log::info!("Received Create event, sending to {dst}");
                let create_instance_message = InstanceCreateParams {
                    name,
                    distro,
                    version,
                    vmtype,
                    sig,
                    recovery_id: recovery_id.into(),
                    sync,
                    memory,
                    vcpus,
                    cpu,
                    metadata,
                    os_variant,
                    host_device,
                    network,
                    disk,
                    filesystem,
                    controller,
                    input,
                    graphics,
                    sound,
                    video,
                    smartcard,
                    redirdev,
                    memballoon,
                    tpm,
                    rng,
                    panic,
                    shmem,
                    memdev,
                    vsock,
                    iommu,
                    watchdog,
                    serial,
                    parallel,
                    channel,
                    console,
                    install,
                    cdrom,
                    location,
                    pxe,
                    import,
                    boot,
                    idmap,
                    features: features
                        .into_iter()
                        .map(|(name, feature)| Features {
                            name: name.clone(),
                            feature: feature.clone(),
                        })
                        .collect(),
                    clock,
                    launch_security,
                    numatune,
                    boot_dev,
                    unattended,
                    print_xml,
                    dry_run,
                    connect,
                    virt_type,
                    cloud_init,
                };

                let mut client = create_allegra_rpc_client_to_addr(&dst).await?;
                let resp = client
                    .create_vm(tonic::Request::new(create_instance_message))
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));

                log::info!(
                    "Received valid response to Create message sent to {dst}: {}",
                    resp.is_ok()
                );
            }
            NetworkEvent::NewPeer {
                peer_id,
                peer_address,
                dst,
                ..
            } => {
                log::info!("Received NewPeer event, sending to {dst}");
                let header = MessageHeader {
                    peer_id: self.local_peer.wallet_address_hex(),
                    peer_address: self.local_peer.ip_address().to_string().clone(),
                    message_id: uuid::Uuid::new_v4().to_string(),
                };

                //log::info!("constructed message header");
                let new_peer_message = NewPeerMessage {
                    header: Some(header),
                    new_peer_id: peer_id,
                    new_peer_address: peer_address,
                };

                //log::info!("constructed new_peer message");
                let mut client = create_allegra_rpc_client_to_addr(&dst).await?;
                let resp = client
                    .register(new_peer_message)
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
                log::info!(
                    "Received valid response to NewPeer message sent to {dst}: {}",
                    resp.is_ok()
                );
            }
            NetworkEvent::ExposeService {
                name,
                sig,
                recovery_id,
                port,
                service_type,
                dst,
                ..
            } => {
                let port: Vec<u32> = port.iter().map(|n| *n as u32).collect();
                let service_type: Vec<i32> =
                    service_type.iter().map(|s| s.clone().into()).collect();
                let recovery_id = recovery_id as u32;
                let expose_service_message = InstanceExposeServiceParams {
                    name,
                    sig,
                    recovery_id,
                    port,
                    service_type,
                };

                let mut client = create_allegra_rpc_client_to_addr(&dst).await?;
                let _resp = client
                    .expose_vm_ports(tonic::Request::new(expose_service_message))
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
                    .into_inner();
            }
            NetworkEvent::Stop {
                name,
                sig,
                recovery_id,
                dst,
                ..
            } => {
                let stop_message = InstanceStopParams {
                    name,
                    sig,
                    recovery_id: recovery_id as u32,
                };

                let mut client = create_allegra_rpc_client_to_addr(&dst).await?;
                let _resp = client
                    .shutdown_vm(tonic::Request::new(stop_message))
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
                    .into_inner();
            }
            NetworkEvent::Start {
                name,
                sig,
                recovery_id,
                console,
                stateless,
                dst,
                ..
            } => {
                let start_message = InstanceStartParams {
                    name,
                    sig,
                    recovery_id: recovery_id as u32,
                    console,
                    stateless,
                };

                let mut client = create_allegra_rpc_client_to_addr(&dst).await?;
                let _resp = client
                    .start_vm(tonic::Request::new(start_message))
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
                    .into_inner();
            }
            NetworkEvent::Delete {
                name,
                force,
                interactive,
                sig,
                recovery_id,
                dst,
                ..
            } => {
                let delete_message = InstanceDeleteParams {
                    name,
                    force,
                    interactive,
                    sig,
                    recovery_id: recovery_id as u32,
                };

                let mut client = create_allegra_rpc_client_to_addr(&dst).await?;
                let _resp = client
                    .delete_vm(tonic::Request::new(delete_message))
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
                    .into_inner();
            }
            NetworkEvent::AddPubkey {
                name,
                sig,
                recovery_id,
                pubkey,
                dst,
                ..
            } => {
                let add_pubkey_message = InstanceAddPubkeyParams {
                    name,
                    sig,
                    recovery_id: recovery_id as u32,
                    pubkey,
                };

                let mut client = create_allegra_rpc_client_to_addr(&dst).await?;
                let _resp = client
                    .set_ssh_pubkey(tonic::Request::new(add_pubkey_message))
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
                    .into_inner();
            }
            NetworkEvent::CastLeaderElectionVote { .. } => {
                todo!()
            }
            NetworkEvent::BootstrapNewPeer { .. } => {
                log::info!("Received Bootstrap New Peer event");
            }
            NetworkEvent::BootstrapResponse { .. } => {
                log::info!("Received Bootstrap Response event");
            }
            NetworkEvent::Heartbeat { .. } => {
                /*
                let mut client = create_allegra_rpc_client_to_addr(&peer.ip_address().to_string()).await?;
                let header = MessageHeader {
                    peer_id
                }
                */
                todo!()
            }
            NetworkEvent::ShareInstanceNamespaces {
                task_id,
                instances,
                peer,
                ..
            } => {
                let mut client =
                    create_allegra_rpc_client_to_addr(&peer.ip_address().to_string()).await?;

                let message = BootstrapInstancesMessage {
                    header: None,
                    request_id: task_id.to_string(),
                    instances: instances
                        .iter()
                        .map(|namespace| namespace.inner().to_string())
                        .collect(),
                };

                let resp = client
                    .bootstrap_instances(tonic::Request::new(message))
                    .await;
                log::info!(
                    "Sent BootstrapInstancesMessage request to {}: {}: response: {}",
                    peer.wallet_address_hex(),
                    peer.ip_address(),
                    resp.is_ok()
                );
            }
            NetworkEvent::BootstrapInstancesResponse {
                task_id, requestor, ..
            } => {
                let mut client =
                    create_allegra_rpc_client_to_addr(&requestor.ip_address().to_string()).await?;
                let message = BootstrapCompleteMessage {
                    header: None,
                    original_request_id: task_id.to_string(),
                };

                let resp = client
                    .bootstrap_complete(tonic::Request::new(message))
                    .await;
                log::info!(
                    "Sent BootstrapInstancesMessage request to {}: {}: response: {}",
                    requestor.wallet_address_hex(),
                    requestor.ip_address(),
                    resp.is_ok()
                );
            }
            NetworkEvent::PreparedForLaunch {
                instance,
                dst,
                local_peer,
                ..
            } => {
                let mut client =
                    create_allegra_rpc_client_to_addr(&dst.ip_address().to_string()).await?;
                let message = PreparedForLaunchMessage {
                    peer_id: local_peer.wallet_address_hex(),
                    peer_address: local_peer.ip_address().to_string(),
                    instance: instance.inner().to_string(),
                };

                let resp = client
                    .prepared_for_launch(tonic::Request::new(message))
                    .await;

                log::info!(
                    "Sent PreparedForLaunch request to {}: {}: response: {}",
                    dst.wallet_address_hex(),
                    dst.ip_address(),
                    resp.is_ok()
                );
            }
        }

        Ok(())
    }
}

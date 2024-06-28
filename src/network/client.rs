use conductor::subscriber::SubStream;
use uuid::Uuid;

use crate::{
    allegra_rpc::{
        InstanceAddPubkeyParams, InstanceCreateParams, InstanceDeleteParams, InstanceExposeServiceParams, InstanceStartParams, InstanceStopParams, MessageHeader, NewPeerMessage, NodeCertMessage
    }, create_allegra_rpc_client_to_addr, event::NetworkEvent, publish::GenericPublisher, subscribe::NetworkSubscriber
};

pub struct NetworkClient {
    local_peer_id: String,
    local_peer_address: String,
    subscriber: NetworkSubscriber,
    #[allow(unused)]
    publisher: GenericPublisher
}

impl NetworkClient {
    pub async fn new(
        local_peer_id: String,
        local_peer_address: String,
        subscriber_uri: &str,
        publisher_uri: &str,
    ) -> std::io::Result<Self> {

        let subscriber = NetworkSubscriber::new(subscriber_uri).await?;
        let publisher = GenericPublisher::new(publisher_uri).await?;

        Ok(Self { local_peer_id, local_peer_address, subscriber, publisher })
    }

    pub async fn run(mut self) -> std::io::Result<()> {
        loop {
            tokio::select! {
                Ok(messages) = self.subscriber.receive() => {
                    for message in messages {
                        self.handle_networking_event(message).await?;
                    }
                },
                _ = tokio::signal::ctrl_c() => {
                    break;
                }
            }
        }

        Ok(())
    }

    #[allow(unused)]
    async fn handle_networking_event(&mut self, event: NetworkEvent) -> std::io::Result<()> {
        match event {
            NetworkEvent::Create { name, distro, version, vmtype, sig, recovery_id, dst, .. } => {
                let create_instance_message = InstanceCreateParams {
                    name, distro, version, vmtype, sig, recovery_id: recovery_id.into()
                };

                let mut client = create_allegra_rpc_client_to_addr(&dst).await?;
                let resp = client.create_vm(
                    tonic::Request::new(
                        create_instance_message
                    )
                ).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string()
                    )
                })?.into_inner();
            }
            NetworkEvent::NewPeer { 
                peer_id, peer_address, dst, ..
            } => {
                let header = MessageHeader {
                    peer_id: self.local_peer_id.clone(),
                    peer_address: self.local_peer_address.clone(),
                    message_id: uuid::Uuid::new_v4().to_string(),
                };

                let new_peer_message = NewPeerMessage {
                    header: Some(header),
                    new_peer_id: peer_id,
                    new_peer_address: peer_address,
                };

                let mut client = create_allegra_rpc_client_to_addr(&dst).await?;
                let resp = client.register(new_peer_message).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string()
                    )
                })?.into_inner();
            }
            NetworkEvent::ExposeService { 
                name, sig, recovery_id, port, service_type, dst, ..
            } => {
                let port: Vec<u32> = port.iter().map(|n| *n as u32).collect();
                let service_type: Vec<i32> = service_type.iter().map(|s| s.clone().into()).collect();
                let recovery_id = recovery_id as u32;
                let expose_service_message = InstanceExposeServiceParams {
                    name, sig, recovery_id, port, service_type
                };

                let mut client = create_allegra_rpc_client_to_addr(&dst).await?;
                let resp = client.expose_vm_ports(
                    tonic::Request::new(
                        expose_service_message
                    )
                ).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string()
                    )
                })?.into_inner();
            }
            NetworkEvent::Stop { 
                name, sig, recovery_id, dst, ..
            } => {
                let stop_message = InstanceStopParams { name, sig, recovery_id: recovery_id as u32 };
                
                let mut client = create_allegra_rpc_client_to_addr(&dst).await?;
                let resp = client.shutdown_vm(
                    tonic::Request::new(
                        stop_message
                    )
                ).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string()
                    )
                })?.into_inner();
            }
            NetworkEvent::Start { 
                name, sig, recovery_id, console, stateless, dst, ..
            } => {
                let start_message = InstanceStartParams { name, sig, recovery_id: recovery_id as u32, console, stateless };

                let mut client = create_allegra_rpc_client_to_addr(&dst).await?;
                let resp = client.start_vm(
                    tonic::Request::new(
                        start_message
                    )
                ).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string()
                    )
                })?.into_inner();
            }
            NetworkEvent::Delete { 
                name, force, interactive, sig, recovery_id, dst, ..
            }=> {
                let delete_message = InstanceDeleteParams { name, force, interactive, sig, recovery_id: recovery_id as u32 };

                let mut client = create_allegra_rpc_client_to_addr(&dst).await?;
                let resp = client.delete_vm(
                    tonic::Request::new(
                        delete_message
                    )
                ).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string()
                    )
                })?.into_inner();
            }
            NetworkEvent::AddPubkey { 
                name, sig, recovery_id, pubkey, dst, ..
            } => {
                let add_pubkey_message = InstanceAddPubkeyParams { name, sig, recovery_id: recovery_id as u32, pubkey };

                let mut client = create_allegra_rpc_client_to_addr(&dst).await?;
                let resp = client.set_ssh_pubkey(
                    tonic::Request::new(
                        add_pubkey_message
                    )
                ).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string()
                    )
                })?.into_inner();
            }
            NetworkEvent::DistributeCerts { certs, peer, quorum_id, .. } => {
                for (peer, cert) in certs {
                    let mut client = create_allegra_rpc_client_to_addr(&peer.address()).await?;
                    let request_id = Uuid::new_v4().to_string();
                    let node_cert_message = NodeCertMessage {
                        peer_id: peer.id().to_string(),
                        peer_address: peer.address().to_string(),
                        quorum_id: quorum_id.clone(),
                        cert,
                        request_id
                    };

                    let resp = client.node_certificate(
                        tonic::Request::new(
                            node_cert_message
                        )
                    ).await.map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e
                        )
                    })?;
                }
            }
            NetworkEvent::ShareCert { peer, cert, quorum_id, dst, .. } => {
                let request_id = Uuid::new_v4().to_string();
                let node_cert_message = NodeCertMessage {
                    peer_id: peer.id().to_string(),
                    peer_address: peer.address().to_string(),
                    quorum_id,
                    cert,
                    request_id
                };

                let mut client = create_allegra_rpc_client_to_addr(
                    &dst.address()
                ).await?;
                let resp = client.node_certificate(
                    tonic::Request::new(
                        node_cert_message
                    )
                ).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?.into_inner();
            }
            NetworkEvent::CastLeaderElectionVote { .. } => {
                todo!()
            }
        }

        Ok(())
    }
}

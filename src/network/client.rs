use crate::{
    allegra_rpc::{
        vmm_client::VmmClient, SyncMessage, MessageHeader, NewPeerMessage, MigrateMessage, InstanceExposeServiceParams, InstanceStopParams, InstanceStartParams, InstanceDeleteParams, InstanceAddPubkeyParams, FileChunk
    }, create_allegra_rpc_client, event::NetworkEvent
};
use tokio::{sync::broadcast::{Receiver, error::RecvError}, io::AsyncReadExt};
use crate::event::Event;
use tonic::transport::Channel;

#[derive(Debug)]
pub struct NetworkClient {
    local_peer_id: String,
    local_peer_address: String,
    events: Receiver<Event>,
    client: VmmClient<Channel>
}

impl NetworkClient {
    pub async fn new(
        events: Receiver<Event>,
        local_peer_id: String,
        local_peer_address: String
    ) -> std::io::Result<Self> {
        let client = create_allegra_rpc_client().await?;

        Ok(Self { local_peer_id, local_peer_address, events, client })
    }

    pub async fn run(&mut self) -> std::io::Result<()> {
        loop {
            match self.events.recv().await {
                Ok(event) => {
                    match self.handle_networking_event(event).await {
                        Err(e) => log::error!("{e}"),
                        _ => {}
                    };
                }
                Err(e) => {
                    match e {
                        RecvError::Closed => break,
                        _ => log::error!("{e}")
                    }
                }
            }
        }

        Ok(())
    }

    #[allow(unused)]
    async fn handle_networking_event(&mut self, event: Event) -> std::io::Result<()> {
        match event {
            Event::NetworkEvent(ne) => {
                match ne {
                    NetworkEvent::Sync { 
                        namespace, path, last_update, target
                    } => {
                        let mut file = tokio::fs::File::open(&path).await?;
                        let mut buffer = Vec::new();
                        file.read_to_end(&mut buffer).await?;

                        let request_namespace = namespace.clone();
                        let request = tonic::Request::new(futures::stream::once(async move {
                            FileChunk {
                                namespace: request_namespace,
                                content: buffer,
                                new_quorum: None
                            }
                        }));

                        let resp = self.client.sync(request).await.map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e
                            )
                        })?.into_inner();

                        if resp.success {
                            crate::vmm::remove_temp_instance(&namespace, &path).await?;
                            return Ok(())
                        } else {
                            return Err(
                                std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    format!("{} failed", &resp.message)
                                )
                            )
                        }
                    }
                    NetworkEvent::NewPeer { 
                        peer_id, peer_address, peer_number 
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
                            peer_number
                        };

                        let resp = self.client.register(new_peer_message).await.map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e.to_string()
                            )
                        })?.into_inner();
                    }
                    NetworkEvent::Migrate { 
                        namespace, path, new_quorum, target, last_update 
                    } => {
                        let mut file = tokio::fs::File::open(&path).await?;
                        let mut buffer = Vec::new();
                        file.read_to_end(&mut buffer).await?;

                        let request_namespace = namespace.clone();
                        let request = tonic::Request::new(futures::stream::once(async move {
                            FileChunk {
                                namespace: request_namespace,
                                content: buffer,
                                new_quorum: None,
                            }
                        }));

                        let resp = self.client.sync(request).await.map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e
                            )
                        })?.into_inner();

                        if resp.success {
                            crate::vmm::remove_temp_instance(&namespace, &path).await?;
                            return Ok(())
                        } else {
                            return Err(
                                std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    format!("{} failed", &resp.message)
                                )
                            )
                        }
                    }
                    NetworkEvent::ExposeService { 
                        name, sig, recovery_id, port, service_type 
                    } => {
                        let port: Vec<u32> = port.iter().map(|n| *n as u32).collect();
                        let service_type: Vec<i32> = service_type.iter().map(|s| s.clone().into()).collect();
                        let recovery_id = recovery_id as u32;
                        let expose_service_message = InstanceExposeServiceParams {
                            name, sig, recovery_id, port, service_type
                        };

                        let resp = self.client.expose_vm_ports(
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
                        name, sig, recovery_id 
                    } => {
                        let stop_message = InstanceStopParams { name, sig, recovery_id: recovery_id as u32 };
                        
                        let resp = self.client.shutdown_vm(
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
                        name, sig, recovery_id, console, stateless 
                    } => {
                        let start_message = InstanceStartParams { name, sig, recovery_id: recovery_id as u32, console, stateless };

                        let resp = self.client.start_vm(
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
                        name, force, interactive, sig, recovery_id 
                    }=> {
                        let delete_message = InstanceDeleteParams { name, force, interactive, sig, recovery_id: recovery_id as u32 };

                        let resp = self.client.delete_vm(
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
                        name, sig, recovery_id, pubkey 
                    } => {
                        let add_pubkey_message = InstanceAddPubkeyParams { name, sig, recovery_id: recovery_id as u32, pubkey };

                        let resp = self.client.set_ssh_pubkey(
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
                }
            }
            _ => return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "invalid event for networking"
                )
            )
        }

        Ok(())
    }
}

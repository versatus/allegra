use crate::{
    allegra_rpc::{
        MessageHeader, NewPeerMessage, InstanceCreateParams, InstanceExposeServiceParams, InstanceStopParams, InstanceStartParams, InstanceDeleteParams, InstanceAddPubkeyParams
    }, event::NetworkEvent, create_allegra_rpc_client_to_addr
};
use crate::event::Event;

#[derive(Debug)]
pub struct NetworkClient {
    local_peer_id: String,
    local_peer_address: String,
    //TODO(asmith): Replace with subscriber
    //events: Receiver<Event>,
}

impl NetworkClient {
    pub async fn new(
        //TODO(asmith): Replace with subscriber
        //events: Receiver<Event>,
        local_peer_id: String,
        local_peer_address: String
    ) -> std::io::Result<Self> {
        Ok(Self { local_peer_id, local_peer_address, /*events*/ })
    }

    pub async fn run(&mut self) -> std::io::Result<()> {
        /*
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
        */

        Ok(())
    }

    #[allow(unused)]
    async fn handle_networking_event(&mut self, event: Event) -> std::io::Result<()> {
        match event {
            Event::NetworkEvent(ne) => {
                match ne {
                    NetworkEvent::Create { name, distro, version, vmtype, sig, recovery_id, dst } => {
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
                        peer_id, peer_address, dst
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
                        name, sig, recovery_id, port, service_type, dst 
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
                        name, sig, recovery_id, dst 
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
                        name, sig, recovery_id, console, stateless, dst 
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
                        name, force, interactive, sig, recovery_id, dst 
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
                        name, sig, recovery_id, pubkey, dst 
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
                    NetworkEvent::DistributeCerts { certs } => {
                        todo!()
                    }
                    NetworkEvent::ShareCert { peer, cert } => {
                        todo!()
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

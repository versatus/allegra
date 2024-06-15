use crate::{
    allegra_rpc::{
        vmm_server::Vmm,
        VmResponse,
        InstanceCreateParams,
        InstanceStartParams,
        InstanceStopParams,
        InstanceDeleteParams,
        InstanceExposeServiceParams,
        InstanceAddPubkeyParams, 
        GetTaskStatusRequest,
        InstanceGetSshDetails,
        NewPeerMessage,
        PingMessage,
        PongMessage, 
        Ack, 
        SshDetails, 
        ServiceType,
        GetPortMessage, 
        PortResponse, 
        MessageHeader, 
        TransferStatus
    }, 
    vmm::{
        VmManagerMessage,
        SUCCESS,
        Instance, 
        TEMP_PATH
    }, 
    account::{
        TaskId, 
        TaskStatus,
        Account, Namespace
    }, 
    params::Payload,
    helpers::{
        recover_namespace,
        recover_owner_address
    }, dht::{
        AllegraNetworkState,
        Peer
    }, create_allegra_rpc_client_to_addr, 
        broker::broker::EventBroker, 
        event::{
            NetworkEvent, 
            Event
        }
};

use tokio::{
    sync::{
        mpsc::Sender, 
        Mutex
    }, 
    fs::File, 
    io::AsyncWriteExt
};
use tokio::sync::RwLock;

use sha3::{Digest, Sha3_256};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use std::result::Result;
use lru::LruCache;
use rand::Rng;
use futures::StreamExt;


#[derive(Clone)]
pub struct VmmService {
    pub network: String,
    pub port: u16,
    pub local_peer: Peer,
    pub vmm_sender: Sender<VmManagerMessage>,
    pub tikv_client: tikv_client::RawClient,
    pub task_cache: Arc<RwLock<LruCache<TaskId, TaskStatus>>>, 
    pub network_state: Arc<RwLock<AllegraNetworkState>>,
    pub event_broker: Arc<Mutex<EventBroker>>
}

#[tonic::async_trait]
impl Vmm for VmmService {
    async fn create_vm(
        &self,
        request: Request<InstanceCreateParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let namespace: Namespace = params.clone().try_into()?;
        let task_id = generate_task_id(params.clone())?;

        let inner_task_id = task_id.clone();
        let inner_vmm_sender = self.vmm_sender.clone();
        let inner_network_state = self.network_state.clone();
        let inner_local_peer = self.local_peer.clone();
        let inner_event_broker = self.event_broker.clone();

        tokio::spawn(async move {
            let guard = inner_network_state.read().await;
            let instance_quorum_id = guard.get_instance_quorum_membership(&namespace).ok_or(
                Status::failed_precondition(
                    "Unable to allocate instance to a quorum"
                )
            )?;

            let local_peer_quorum_id = guard.get_peer_quorum_membership(&inner_local_peer).ok_or(
                Status::failed_precondition(
                    "Local peer not a member of a quorum"
                )
            )?;

            if instance_quorum_id == local_peer_quorum_id {
                let message = VmManagerMessage::NewInstance { params: params.clone(), task_id: inner_task_id.clone() };
                let vmm_sender = inner_vmm_sender.clone();
                match vmm_sender.send(message).await {
                    Ok(_) => {

                        let quorum_members = guard.get_quorum_by_id(&instance_quorum_id).ok_or(
                            Status::failed_precondition(
                                "Unable to acquire quorum members"
                            )
                        )?.clone();

                        drop(guard);

                        let mut guard = inner_event_broker.lock().await;
                        for peer in quorum_members.peers() {
                            let event = Event::NetworkEvent(
                                (peer.clone(), params.clone()).try_into().map_err(|e| {
                                    Status::from_error(Box::new(e))
                                })?
                            ); 

                            guard.publish("Network".to_string(), event).await;
                        }

                        drop(guard);
                    },
                    _ => log::error!("Error sending vm manager message to vm manager")
                }
            } else {
                let quorum_members = guard.get_quorum_by_id(&instance_quorum_id).ok_or(
                    Status::failed_precondition(
                        "Unable to acquire quorum members"
                    )
                )?.clone();

                drop(guard);

                let mut guard = inner_event_broker.lock().await;
                for peer in quorum_members.peers() {
                    let event = Event::NetworkEvent(
                        (peer.clone(), params.clone()).try_into().map_err(|e| {
                            Status::from_error(Box::new(e))
                        })?
                    ); 

                    guard.publish("Network".to_string(), event).await;
                }
                drop(guard);
            }

            return Ok::<(), Status>(())
        });
        return Ok(
            Response::new(
                VmResponse {
                    status: "PENDING".to_string(),
                    details: format!("TaskId: {}", task_id.task_id()),
                    ssh_details: None,
                }
            )
        )
    }

    async fn shutdown_vm(
        &self,
        request: Request<InstanceStopParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let namespace: Namespace = params.clone().try_into()?;
        let task_id = generate_task_id(params.clone())?;
        let inner_task_id = task_id.clone();
        let inner_vmm_sender = self.vmm_sender.clone();
        let inner_network_state = self.network_state.clone();
        let inner_local_peer = self.local_peer.clone();
        let inner_event_broker = self.event_broker.clone();

        tokio::spawn(async move {
            let guard = inner_network_state.read().await;
            let instance_quorum_id = guard.get_instance_quorum_membership(&namespace).ok_or(
                Status::failed_precondition(
                    "Unable to allocate instance to a quorum"
                )
            )?;

            let local_peer_quorum_id = guard.get_peer_quorum_membership(&inner_local_peer).ok_or(
                Status::failed_precondition(
                    "Local peer not a member of a quorum"
                )
            )?;

            if instance_quorum_id == local_peer_quorum_id {
                let message = VmManagerMessage::StopInstance { 
                    params: params.clone(), 
                    sig: params.sig.clone(), 
                    task_id: inner_task_id.clone() 
                };
                let vmm_sender = inner_vmm_sender.clone();
                match vmm_sender.send(message).await {
                    Ok(_) => {
                        let quorum_members = guard.get_quorum_by_id(&instance_quorum_id).ok_or(
                            Status::failed_precondition(
                                "Unable to acquire quorum members"
                            )
                        )?.clone();

                        drop(guard);

                        let mut guard = inner_event_broker.lock().await;

                        for peer in quorum_members.peers() {
                            if peer.id() != inner_local_peer.id() {
                                let event = Event::NetworkEvent(
                                    (peer.clone(), params.clone()).try_into().map_err(|e| {
                                        Status::from_error(Box::new(e))
                                    })?
                                ); 

                                guard.publish("Network".to_string(), event).await;
                            }
                        }
                    },
                    Err(e) => log::error!("Error sending vm manager message to vm manager: {e}")
                }
            } else {
                let quorum_members = guard.get_quorum_by_id(&instance_quorum_id).ok_or(
                    Status::failed_precondition(
                        "Unable to acquire quorum members"
                    )
                )?.clone();

                drop(guard);

                let mut guard = inner_event_broker.lock().await;

                for peer in quorum_members.peers() {
                    let event = Event::NetworkEvent(
                        (peer.clone(), params.clone()).try_into().map_err(|e| {
                            Status::from_error(Box::new(e))
                        })?
                    ); 

                    guard.publish("Network".to_string(), event).await;
                }
            }

            Ok::<(), Status>(())
        });
        Ok(Response::new(VmResponse {
            status: "PENDING".to_string(),
            details: format!("TaskId: {}", task_id.task_id()),
            ssh_details: None,
        }))
    }

    async fn start_vm(
        &self,
        request: Request<InstanceStartParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let namespace: Namespace = params.clone().try_into()?;
        let task_id = generate_task_id(params.clone())?;
        let inner_task_id = task_id.clone();
        let inner_vmm_sender = self.vmm_sender.clone();
        let inner_network_state = self.network_state.clone();
        let inner_local_peer = self.local_peer.clone();
        let inner_event_broker = self.event_broker.clone();


        tokio::spawn(async move {
            let guard = inner_network_state.read().await;
            let instance_quorum_id = guard.get_instance_quorum_membership(&namespace).ok_or(
                Status::failed_precondition(
                    "Unable to allocate instance to a quorum"
                )
            )?;

            let local_peer_quorum_id = guard.get_peer_quorum_membership(&inner_local_peer).ok_or(
                Status::failed_precondition(
                    "Local peer not a member of a quorum"
                )
            )?;


            if instance_quorum_id == local_peer_quorum_id {
                let message = VmManagerMessage::StartInstance { 
                    params: params.clone(), 
                    sig: params.sig.clone(), 
                    task_id: inner_task_id.clone() 
                };
                let vmm_sender = inner_vmm_sender.clone();
                match vmm_sender.send(message).await {
                    Ok(_) => {
                        let quorum_members = guard.get_quorum_by_id(&instance_quorum_id).ok_or(
                            Status::failed_precondition(
                                "Unable to acquire quorum members"
                            )
                        )?.clone();

                        drop(guard);

                        let mut guard = inner_event_broker.lock().await;

                        for peer in quorum_members.peers() {
                            let event = Event::NetworkEvent(
                                (peer.clone(), params.clone()).try_into().map_err(|e| {
                                    Status::from_error(Box::new(e))
                                })?
                            ); 

                            guard.publish("Network".to_string(), event).await;

                        }
                    },
                    Err(e) => {},
                }
            } else {
                let quorum_members = guard.get_quorum_by_id(&instance_quorum_id).ok_or(
                    Status::failed_precondition(
                        "Unable to acquire quorum members"
                    )
                )?.clone();

                drop(guard);

                let mut guard = inner_event_broker.lock().await;

                for peer in quorum_members.peers() {
                    let event = Event::NetworkEvent(
                        (peer.clone(), params.clone()).try_into().map_err(|e| {
                            Status::from_error(Box::new(e))
                        })?
                    ); 

                    guard.publish("Network".to_string(), event).await;

                }
            }

            Ok::<(), Status>(())
        });
        Ok(Response::new(VmResponse {
            status: "PENDING".to_string(),
            details: format!("TaskId: {}", task_id.task_id()),
            ssh_details: None,
        }))
    }

    async fn set_ssh_pubkey(
        &self, 
        request: Request<InstanceAddPubkeyParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let namespace: Namespace = params.clone().try_into()?;
        let task_id = generate_task_id(params.clone())?;
        let inner_task_id = task_id.clone();
        let inner_vmm_sender = self.vmm_sender.clone();
        let inner_network_state = self.network_state.clone();
        let inner_local_peer = self.local_peer.clone();
        let inner_event_broker = self.event_broker.clone();

        tokio::spawn(async move {
            let guard = inner_network_state.read().await;
            let instance_quorum_id = guard.get_instance_quorum_membership(&namespace).ok_or(
                Status::failed_precondition(
                    "Unable to allocate instance to a quorum"
                )
            )?;

            let local_peer_quorum_id = guard.get_peer_quorum_membership(&inner_local_peer).ok_or(
                Status::failed_precondition(
                    "Local peer not a member of a quorum"
                )
            )?;

            if instance_quorum_id == local_peer_quorum_id {
                let message = VmManagerMessage::InjectAuth { 
                    params: params.clone(), 
                    sig: params.sig.clone(), 
                    task_id: inner_task_id.clone() 
                };

                let vmm_sender = inner_vmm_sender.clone();
                match vmm_sender.send(message).await {
                    Ok(_) => {
                        let quorum_members = guard.get_quorum_by_id(&instance_quorum_id).ok_or(
                            Status::failed_precondition(
                                "Unable to acquire quorum members"
                            )
                        )?.clone();

                        drop(guard);

                        let mut guard = inner_event_broker.lock().await;

                        for peer in quorum_members.peers() {
                            let event = Event::NetworkEvent(
                                (peer.clone(), params.clone()).try_into().map_err(|e| {
                                    Status::from_error(Box::new(e))
                                })?
                            ); 

                            guard.publish("Network".to_string(), event).await;

                        }
                    }
                    Err(e) => {},
                }
            } else {
                let quorum_members = guard.get_quorum_by_id(&instance_quorum_id).ok_or(
                    Status::failed_precondition(
                        "Unable to acquire quorum members"
                    )
                )?.clone();

                drop(guard);

                let mut guard = inner_event_broker.lock().await;

                for peer in quorum_members.peers() {
                    let event = Event::NetworkEvent(
                        (peer.clone(), params.clone()).try_into().map_err(|e| {
                            Status::from_error(Box::new(e))
                        })?
                    ); 

                    guard.publish("Network".to_string(), event).await;

                }
            }

            Ok::<(), Status>(())
        });
        Ok(Response::new(VmResponse {
            status: "PENDING".to_string(),
            details: format!("TaskId: {}", task_id.task_id()),
            ssh_details: None,
        }))

    }

    async fn delete_vm(
        &self,
        request: Request<InstanceDeleteParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let namespace: Namespace = params.clone().try_into()?;
        let task_id = generate_task_id(params.clone())?;
        let inner_task_id = task_id.clone();
        let inner_vmm_sender = self.vmm_sender.clone();
        let inner_network_state = self.network_state.clone();
        let inner_local_peer = self.local_peer.clone();
        let inner_event_broker = self.event_broker.clone();

        tokio::spawn(async move {
            let guard = inner_network_state.read().await;
            let instance_quorum_id = guard.get_instance_quorum_membership(&namespace).ok_or(
                Status::failed_precondition(
                    "Unable to allocate instance to a quorum"
                )
            )?;

            let local_peer_quorum_id = guard.get_peer_quorum_membership(&inner_local_peer).ok_or(
                Status::failed_precondition(
                    "Local peer not a member of a quorum"
                )
            )?;


            if instance_quorum_id == local_peer_quorum_id {
                let message = VmManagerMessage::DeleteInstance { 
                    params: params.clone(), 
                    sig: params.sig.clone(), 
                    task_id: inner_task_id.clone() 
                };
                let vmm_sender = inner_vmm_sender.clone();
                match vmm_sender.send(message).await {
                    Ok(_) => {
                        let quorum_members = guard.get_quorum_by_id(&instance_quorum_id).ok_or(
                            Status::failed_precondition(
                                "Unable to acquire quorum members"
                            )
                        )?.clone();

                        drop(guard);

                        let mut guard = inner_event_broker.lock().await;

                        for peer in quorum_members.peers() {
                            let event = Event::NetworkEvent(
                                (peer.clone(), params.clone()).try_into().map_err(|e| {
                                    Status::from_error(Box::new(e))
                                })?
                            ); 

                            guard.publish("Network".to_string(), event).await;

                        }

                    }
                    Err(e) => {},
                }
            } else {
                let quorum_members = guard.get_quorum_by_id(&instance_quorum_id).ok_or(
                    Status::failed_precondition(
                        "Unable to acquire quorum members"
                    )
                )?.clone();

                drop(guard);

                let mut guard = inner_event_broker.lock().await;

                for peer in quorum_members.peers() {
                    let event = Event::NetworkEvent(
                        (peer.clone(), params.clone()).try_into().map_err(|e| {
                            Status::from_error(Box::new(e))
                        })?
                    ); 
                    guard.publish("Network".to_string(), event).await;
                }
            }

            Ok::<(), Status>(())

        });
        Ok(Response::new(VmResponse {
            status: "PENDING".to_string(),
            details: format!("TaskId: {}", task_id.task_id()),
            ssh_details: None,
        }))
    }

    async fn expose_vm_ports(
        &self, 
        request: Request<InstanceExposeServiceParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let namespace: Namespace = params.clone().try_into()?;
        let task_id = generate_task_id(params.clone())?;
        let inner_task_id = task_id.clone();
        let inner_vmm_sender = self.vmm_sender.clone();
        let inner_network_state = self.network_state.clone();
        let inner_local_peer = self.local_peer.clone();
        let inner_event_broker = self.event_broker.clone();

        tokio::spawn(async move {
            let guard = inner_network_state.read().await;
            let instance_quorum_id = guard.get_instance_quorum_membership(&namespace).ok_or(
                Status::failed_precondition(
                    "Unable to allocate instance to a quorum"
                )
            )?;

            let local_peer_quorum_id = guard.get_peer_quorum_membership(&inner_local_peer).ok_or(
                Status::failed_precondition(
                    "Local peer not a member of a quorum"
                )
            )?;

            if instance_quorum_id == local_peer_quorum_id {
                let message = VmManagerMessage::ExposeService { 
                    params: params.clone(), 
                    sig: params.sig.clone(), 
                    task_id: inner_task_id.clone() 
                };
                let vmm_sender = inner_vmm_sender.clone();
                match vmm_sender.send(message).await {
                    Ok(_) => {
                        let quorum_members = guard.get_quorum_by_id(&instance_quorum_id).ok_or(
                            Status::failed_precondition(
                                "Unable to acquire quorum members"
                            )
                        )?.clone();

                        drop(guard);

                        let mut guard = inner_event_broker.lock().await;

                        for peer in quorum_members.peers() {
                            let event = Event::NetworkEvent(
                                (peer.clone(), params.clone()).try_into().map_err(|e| {
                                    Status::from_error(Box::new(e))
                                })?
                            ); 
                            guard.publish("Network".to_string(), event).await;
                        }
                    }
                    Err(e) => {},
                }
            } else {
                let quorum_members = guard.get_quorum_by_id(&instance_quorum_id).ok_or(
                    Status::failed_precondition(
                        "Unable to acquire quorum members"
                    )
                )?.clone();

                drop(guard);

                let mut guard = inner_event_broker.lock().await;

                for peer in quorum_members.peers() {
                    let event = Event::NetworkEvent(
                        (peer.clone(), params.clone()).try_into().map_err(|e| {
                            Status::from_error(Box::new(e))
                        })?
                    ); 
                    guard.publish("Network".to_string(), event).await;
                }
            }

            Ok::<(), Status>(())
        });
        Ok(Response::new(VmResponse {
            status: "PENDING".to_string(),
            details: format!("TaskId: {}", task_id.task_id()),
            ssh_details: None,
        }))
    }

    async fn get_task_status(
        &self,
        request: Request<GetTaskStatusRequest>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let address_bytes = if params.owner.starts_with("0x") {
            let owner_string = &params.owner[2..];
            hex::decode(owner_string).map_err(|e| Status::internal(e.to_string()))?
        } else {
            hex::decode(&params.owner).map_err(|e| Status::internal(e.to_string()))?
        };

        let mut guard = self.task_cache.write().await; 
        if let Some(task_status) = guard.get(&TaskId::new(params.id.clone())) {
            return Ok(
                Response::new(
                    VmResponse {
                        status: "SUCCESS".to_string(),
                        details: task_status.to_string(),
                        ssh_details: None,
                    }
                )
            );
        }

        match self.tikv_client.get(address_bytes.to_vec()).await {
            Ok(Some(account_bytes)) => {
                match serde_json::from_slice::<Account>(&account_bytes) {
                    Ok(account) => {
                        let task_id = TaskId::new(params.id.clone());
                        if let Some(ts) = account.get_task_status(&task_id) {
                            Ok(Response::new(VmResponse {
                                status: "SUCCESS".to_string(),
                                details: format!("Task {} status: {:?}", &params.id, ts),
                                ssh_details: None,
                            }))
                        } else {
                            Ok(Response::new(VmResponse {
                                status: "FAILURE".to_string(),
                                details: format!("Unable to find Task {}", &params.id),
                                ssh_details: None,
                            }))
                        }
                    }
                    Err(e) => Ok(Response::new(VmResponse {
                        status: "FAILURE".to_string(),
                        details: format!("Unable to deserialize bytes for Account for owner {}: {}", params.owner, e),
                        ssh_details: None,
                    })),
                }
            }
            Ok(None) => {
                Ok(Response::new(VmResponse {
                    status: "FAILURE".to_string(),
                    details: format!("Unable to find account for owner {}", params.owner),
                    ssh_details: None,
                }))
            }
            Err(e) => Ok(Response::new(VmResponse {
                status: "FAILURE".to_string(),
                details: format!("Error acquiring account for owner {} from state: {}", params.owner, e),
                ssh_details: None,
            })),
        }
    }

    async fn get_ssh_details(
        &self,
        request: Request<InstanceGetSshDetails>
    ) -> Result<Response<VmResponse>, Status> {
        let remote_addr = request.remote_addr();
        let inner = request.into_inner().clone();
        let owner_bytes = if inner.owner.starts_with("0x") {
            let bytes = hex::decode(&inner.owner[2..]).map_err(|e| {
                Status::from_error(Box::new(e))
            })?;
            let mut owner_bytes = [0u8; 20];
            owner_bytes.copy_from_slice(&bytes[..]);
            owner_bytes
        } else {
            let bytes = hex::decode(&inner.owner).map_err(|e| {
                Status::from_error(Box::new(e))
            })?;
            let mut owner_bytes = [0u8; 20];
            owner_bytes.copy_from_slice(&bytes[..]);
            owner_bytes
        };

        let namespace = recover_namespace(owner_bytes, &inner.name);

        let (_, quorum) = {
            let guard = self.network_state.read().await;
            let id = guard.get_instance_quorum_membership(&namespace).ok_or(
                Status::not_found(
                    format!(
                        "Unable to find quorum responsible for instance: {}", 
                        &namespace
                    )
                )
            )?.clone();

            let quorum = guard.get_quorum_by_id(&id).ok_or(
                Status::not_found(
                    format!("Unable to find quorum responsible for instance: {}", &namespace)
                )
            )?.clone();

            (id, quorum)
        };
        let peers = quorum.peers();
            
        if let Some(socket_addr) = remote_addr {
            if let Ok(location) = geolocation::find(&socket_addr.ip().to_string()) {
                
                let locations: Vec<geolocation::Locator> = peers.iter().filter_map(|p| {
                    geolocation::find(p.address()).ok()
                }).collect();

                if let Ok(latitude) = location.latitude.parse::<f64>() {
                    if let Ok(longitude) = location.longitude.parse::<f64>() {
                        let distances: Vec<(String, f64)> = locations.iter().filter_map(|loc| {
                            let node_latitude = loc.latitude.parse::<f64>().ok();
                            let node_longitude = loc.longitude.parse::<f64>().ok();

                            match (node_latitude, node_longitude) {
                                (Some(x), Some(y)) => {
                                    let requestor_location = haversine::Location {
                                        latitude, longitude
                                    };

                                    let node_location = haversine::Location {
                                        latitude: x, longitude: y
                                    };

                                    let distance = haversine::distance(
                                        requestor_location, 
                                        node_location, 
                                        haversine::Units::Kilometers
                                    );
                                    
                                    Some((loc.ip.clone(), distance))
                                }
                                _ => None
                            }
                        }).collect();
                        
                        if let Some(closest) = distances.iter().min_by(|a, b| {
                            match a.1.partial_cmp(&b.1) {
                                Some(order) => order,
                                None => std::cmp::Ordering::Equal
                            }
                        }) {

                            if let Ok(resp) = get_port(namespace.inner().clone(), &closest.0, ServiceType::Ssh).await {
                                return Ok(
                                    Response::new(
                                        VmResponse {
                                            status: SUCCESS.to_string(),
                                            details: "Successfully acquired geographically closest node".to_string(),
                                            ssh_details: Some(SshDetails {
                                                ip: closest.0.clone(),
                                                port: resp.into_inner().port
                                            })
                                        }
                                    )
                                )
                            }
                        }
                    }
                }
            }

            let random_ip = {
                let mut rng = rand::thread_rng();
                let len = peers.len();
                let random_choice = rng.gen_range(0..len);
                let peers_to_choose: Vec<&Peer> = peers.iter().collect();
                peers_to_choose[random_choice].address().to_string()
            };

            if let Ok(resp) = get_port(namespace.inner().clone(), &random_ip, ServiceType::Ssh).await {
                return Ok(
                    Response::new(
                        VmResponse {
                            status: SUCCESS.to_string(),
                            details: "Successfully acquired random node, unable to acquire geographically closest".to_string(),
                            ssh_details: Some(
                                SshDetails {
                                    ip: random_ip.to_string(),
                                    port: resp.into_inner().port
                                }
                            )
                        }
                    )
                )
            }
        } else {
            return Err(Status::failed_precondition(
                "Unable to acquire lock on network state"
            ))
        }

        return Err(
            Status::failed_precondition(
                "Unable to find a node to service the request"
            )
        )
    }

    async fn register(
        &self,
        request: Request<NewPeerMessage>
    ) -> Result<Response<Ack>, Status> {
        let new_peer = request.into_inner();
        let header = new_peer.header;
        let request_id = uuid::Uuid::new_v4();

        let mut guard = self.network_state.write().await;

        let peer = Peer::new(
            uuid::Uuid::parse_str(&new_peer.new_peer_id).map_err(|e| {
                Status::from_error(Box::new(e))
            })?,
            new_peer.new_peer_address
        );
        
        guard.add_peer(&peer).await.map_err(|e| {
            Status::from_error(Box::new(e))
        })?;

        drop(guard);


        return Ok(
            Response::new(
                Ack { 
                    header, request_id: request_id.to_string() 
                }
            )
        );
    }

    async fn ping(
        &self,
        request: Request<PingMessage>
    ) -> Result<Response<PongMessage>, Status> {
        log::info!("Received ping message");
        let header = MessageHeader {
            peer_id: self.local_peer.id().to_string(), 
            peer_address: self.local_peer.address().to_string(),
            message_id: uuid::Uuid::new_v4().to_string()
        };
        log::info!("crafted response header");
        let ping_message_id = request.into_inner().header.ok_or(
            Status::failed_precondition(
                "Ping message had no header"
            )
        )?.message_id.clone();
        let pong_message = PongMessage {
            header: Some(header),
            ping_message_id
        };

        return Ok(Response::new(pong_message))
    }

    async fn sync(
        &self,
        request: Request<tonic::Streaming<crate::allegra_rpc::FileChunk>>
    ) -> Result<Response<crate::allegra_rpc::TransferStatus>, Status> {
        let mut stream = request.into_inner();
        let mut namespace = String::new();
        let mut file = None;

        let path = format!(
            "{}/{}-temp.tar.gz", 
            TEMP_PATH.to_string(),
            namespace.to_string()
        );

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if namespace.is_empty() {
                namespace = chunk.namespace.clone();
                file = Some(
                    File::create(
                        &path
                    ).await?
                );
            }

            if let Some(ref mut file) = file {
                file.write_all(&chunk.content).await?;
            }
        }

        let response = TransferStatus {
            message: format!("File for instance '{}' received in full", namespace),
            success: true
        };
        

        let message = VmManagerMessage::SyncInstance { namespace, path };
        match self.vmm_sender.send(message).await {
            Ok(()) => {
                Ok(Response::new(response))
            }
            Err(e) => {
                Err(Status::from_error(Box::new(e)))
            }
        }
    }
    
    async fn migrate(
        &self,
        request: Request<tonic::Streaming<crate::allegra_rpc::FileChunk>>
    ) -> Result<Response<crate::allegra_rpc::TransferStatus>, Status> {
        let mut stream = request.into_inner();
        let mut namespace = String::new();
        let mut file = None;
        let mut new_quorum = None;

        let path = format!(
            "{}/{}-temp.tar.gz", 
            TEMP_PATH.to_string(),
            namespace.to_string()
        );

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if namespace.is_empty() {
                namespace = chunk.namespace.clone();
                file = Some(
                    File::create(
                        &path
                    ).await?
                );
            }

            if let Some(ref mut file) = file {
                file.write_all(&chunk.content).await?;
            }

            new_quorum = chunk.new_quorum;
        }

        let response = TransferStatus {
            message: format!("File for instance '{}' received in full", namespace),
            success: true
        };
        
        let message = VmManagerMessage::MigrateInstance { namespace, path, new_quorum };
        match self.vmm_sender.send(message).await {
            Ok(()) => {
                Ok(Response::new(response))
            }
            Err(e) => {
                Err(Status::from_error(Box::new(e)))
            }
        }
    }

    async fn get_port(
        &self,
        request: Request<GetPortMessage>
    ) -> Result<Response<PortResponse>, Status> {
        let message = request.into_inner().clone();
        let namespace = message.namespace;
        let service = message.service_type;

        match self.tikv_client.get(namespace.clone()).await {
            Ok(Some(instance_bytes)) => {
                let instance = serde_json::from_slice::<Instance>(&instance_bytes).map_err(|e| {
                    Status::from_error(Box::new(e))
                })?;

                let port = {
                    let services = instance.port_map();
                    let mut port: u16 = 0;
                    for (_, value) in services {
                        let service_int: i32 = value.1.clone().into();
                        if service_int == service {
                            port = value.0;
                        }
                    }

                    port
                };

                let port_response = PortResponse {
                    namespace,
                    service_type: service,
                    port: port.into()
                };

                return Ok(Response::new(port_response))
            }
            _ => {
                return Err(
                    Status::failed_precondition(
                        format!("Unable to find instance: {}", &namespace)
                    )
                )
            }
        }
    }
}

pub fn generate_task_id(params: impl Payload) -> Result<TaskId, Status> {
    let bytes = serde_json::to_vec(&params.into_payload()).map_err(|e| {
        Status::new(tonic::Code::FailedPrecondition, e.to_string())
    })?;
    let mut hasher = Sha3_256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    Ok(TaskId::new(
        hex::encode(&result[..])
    ))
}

pub async fn get_port(
    namespace: String,
    ip: &str,
    service_type: ServiceType
) -> Result<Response<PortResponse>, Status> {
    let mut client = create_allegra_rpc_client_to_addr(&format!("http://{}:50051", ip)).await.map_err(|e| {
        Status::from_error(Box::new(e))
    })?;

    let get_port_message = GetPortMessage {
        namespace, service_type: service_type.into()
    };

    client.get_port(Request::new(get_port_message)).await
}

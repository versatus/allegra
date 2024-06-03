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
        PongMessage, Ack, SyncMessage, MigrateMessage, SshDetails, ServiceType,
        GetPortMessage, PortResponse, MessageHeader
    }, 
    vmm::{VmManagerMessage, SUCCESS, Instance}, 
    account::{
        TaskId, 
        TaskStatus,
        Account
    }, 
    params::Payload, helpers::recover_namespace, dht::{AllegraNetworkState, Peer}, create_allegra_rpc_client_to_addr
};

use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;

use sha3::{Digest, Sha3_256};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use std::result::Result;
use lru::LruCache;
use rand::Rng;


#[derive(Clone)]
pub struct VmmService {
    pub network: String,
    pub port: u16,
    pub local_peer: Peer,
    pub vmm_sender: Sender<VmManagerMessage>,
    pub tikv_client: tikv_client::RawClient,
    pub task_cache: Arc<RwLock<LruCache<TaskId, TaskStatus>>>, 
    pub network_state: Arc<RwLock<AllegraNetworkState>>
}

#[tonic::async_trait]
impl Vmm for VmmService {
    async fn create_vm(
        &self,
        request: Request<InstanceCreateParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let task_id = generate_task_id(params.clone())?;
        let message = VmManagerMessage::NewInstance { params, task_id: task_id.clone() };
        let vmm_sender = self.vmm_sender.clone();
        match vmm_sender.send(message).await {
            Ok(_) => Ok(Response::new(VmResponse {
                status: "PENDING".to_string(),
                details: format!("TaskId: {}", task_id.task_id()),
                ssh_details: None,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn shutdown_vm(
        &self,
        request: Request<InstanceStopParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let task_id = generate_task_id(params.clone())?;
        let message = VmManagerMessage::StopInstance { params: params.clone(), sig: params.sig, task_id: task_id.clone() };
        let vmm_sender = self.vmm_sender.clone();
        match vmm_sender.send(message).await {
            Ok(_) => Ok(Response::new(VmResponse {
                status: "PENDING".to_string(),
                details: format!("TaskId: {}", task_id.task_id()),
                ssh_details: None,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn start_vm(
        &self,
        request: Request<InstanceStartParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let task_id = generate_task_id(params.clone())?;
        let message = VmManagerMessage::StartInstance { params: params.clone(), sig: params.sig, task_id: task_id.clone() };
        let vmm_sender = self.vmm_sender.clone();
        match vmm_sender.send(message).await {
            Ok(_) => Ok(Response::new(VmResponse {
                status: "PENDING".to_string(),
                details: format!("TaskId: {}", task_id.task_id()),
                ssh_details: None,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn set_ssh_pubkey(
        &self, 
        request: Request<InstanceAddPubkeyParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let task_id = generate_task_id(params.clone())?;
        let message = VmManagerMessage::InjectAuth { params: params.clone(), sig: params.sig, task_id: task_id.clone() };
        let vmm_sender = self.vmm_sender.clone();
        match vmm_sender.send(message).await {
            Ok(_) => Ok(Response::new(VmResponse {
                status: "PENDING".to_string(),
                details: format!("TaskId: {}", task_id.task_id()),
                ssh_details: None,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn delete_vm(
        &self,
        request: Request<InstanceDeleteParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let task_id = generate_task_id(params.clone())?;
        let message = VmManagerMessage::DeleteInstance { params: params.clone(), sig: params.sig, task_id: task_id.clone() };
        let vmm_sender = self.vmm_sender.clone();
        match vmm_sender.send(message).await {
            Ok(_) => Ok(Response::new(VmResponse {
                status: "PENDING".to_string(),
                details: format!("TaskId: {}", task_id.task_id()),
                ssh_details: None,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn expose_vm_ports(
        &self, 
        request: Request<InstanceExposeServiceParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let task_id = generate_task_id(params.clone())?;
        let message = VmManagerMessage::ExposeService { params: params.clone(), sig: params.sig, task_id: task_id.clone() };
        let vmm_sender = self.vmm_sender.clone();
        match vmm_sender.send(message).await {
            Ok(_) => Ok(Response::new(VmResponse {
                status: "PENDING".to_string(),
                details: format!("TaskId: {}", task_id.task_id()),
                ssh_details: None,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
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

        let (id, quorum) = {
            let guard = self.network_state.read().await;
            let id = guard.get_instance_quorum_membership(&namespace).ok_or(
                Status::not_found(
                    format!("Unable to find quorum responsible for instance: {}", &namespace)
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
                peers[random_choice].address().to_string()
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

        return Ok(Response::new(Ack { header, request_id: request_id.to_string() }));
    }

    async fn ping(
        &self,
        request: Request<PingMessage>
    ) -> Result<Response<PongMessage>, Status> {
        let header = MessageHeader {
            peer_id: self.local_peer.id().to_string(),
            peer_address: self.local_peer.address().to_string(),
            message_id: uuid::Uuid::new_v4().to_string()
        };
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
        request: Request<SyncMessage>
    ) -> Result<Response<Ack>, Status> {
        let message = request.into_inner().clone();
        let header = message.header.ok_or(
            Status::failed_precondition(
                "unable to acquire requestor address, MessageHeader missing"
            )
        )?;
        
        let requestor_address = header.peer_address.clone();
        let namespace = message.namespace.clone();

        let message = VmManagerMessage::SyncInstance {
            requestor_address,
            namespace,
        };

        let message_id = uuid::Uuid::new_v4();
        let new_header = MessageHeader {
            peer_id: self.local_peer.id().to_string(),
            peer_address: self.local_peer.address().to_string(),
            message_id: message_id.to_string()
        };

        let ack = Ack {
            header: Some(new_header),
            request_id: header.message_id
        };

        let vmm_sender = self.vmm_sender.clone();
        match vmm_sender.send(message).await {
            Ok(_) => return Ok(Response::new(ack)),
            Err(e) => return Err(Status::internal(e.to_string())),
        }
    }
    
    async fn migrate(
        &self,
        request: Request<MigrateMessage>
    ) -> Result<Response<Ack>, Status> {
        let message = request.into_inner().clone();
        let header = message.header.ok_or(
            Status::failed_precondition(
                "unable to acquire requestor address, MessageHeader missing"
            )
        )?;
        
        let requestor_address = header.peer_address.clone();
        let namespace = message.namespace.clone();

        let message = VmManagerMessage::SyncInstance {
            requestor_address,
            namespace,
        };

        let message_id = uuid::Uuid::new_v4();
        let new_header = MessageHeader {
            peer_id: self.local_peer.id().to_string(),
            peer_address: self.local_peer.address().to_string(),
            message_id: message_id.to_string()
        };

        let ack = Ack {
            header: Some(new_header),
            request_id: header.message_id
        };

        let vmm_sender = self.vmm_sender.clone();
        match vmm_sender.send(message).await {
            Ok(_) => return Ok(Response::new(ack)),
            Err(e) => return Err(Status::internal(e.to_string())),
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

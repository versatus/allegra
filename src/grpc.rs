use crate::{
    account::{
        Namespace, TaskId, TaskStatus
    }, allegra_rpc::{
        vmm_server::Vmm, Ack, GetPortMessage, GetTaskStatusRequest, InstanceAddPubkeyParams, InstanceCreateParams, InstanceDeleteParams, InstanceExposeServiceParams, InstanceGetSshDetails, InstanceStartParams, InstanceStopParams, MessageHeader, MigrateMessage, NewPeerMessage, NodeCertMessage, PingMessage, PongMessage, PortResponse, SyncMessage, VmResponse
    }, dht::Peer, event::{
        QuorumEvent, 
        TaskStatusEvent
    }, helpers::{
        generate_task_id, get_payload_hash, owner_address_from_string, recover_owner_address
    }, params::{
        HasOwner, Params, Payload
    }, publish::{
        GeneralResponseTopic, GenericPublisher, QuorumTopic, TaskStatusTopic
    }, subscribe::RpcResponseSubscriber
};

use conductor::{subscriber::SubStream, publisher::PubStream};
use alloy::primitives::Address;

use tonic::{Request, Response, Status};
use uuid::Uuid;
use std::{net::SocketAddr, result::Result};
use std::sync::Arc;
use tokio::sync::Mutex;
use hex::FromHex;


pub struct VmmService {
    pub network: String,
    pub port: u16,
    pub local_peer: Peer,
    pub publisher: Arc<Mutex<GenericPublisher>>,
    pub subscriber: Arc<Mutex<RpcResponseSubscriber>>,
}

impl VmmService {
    async fn check_responsibility<P>(
        &self,
        params: P,
        task_id: TaskId
    ) -> std::io::Result<()> 
    where 
        P: TryInto<Params, Error = std::io::Error> + 
        TryInto<Namespace, Error = std::io::Error> + 
        Clone + std::fmt::Debug 
    {
        let quorum_event = Self::create_check_responsibility_event(params, task_id)?;
        log::info!("created CheckResponsibility event");
        let mut guard = self.publisher.lock().await;
        log::info!("acquired publisher guard...");
        guard.publish(Box::new(QuorumTopic), Box::new(quorum_event)).await?;
        log::info!("published CheckResponsibility event to topic {}...", QuorumTopic);
        drop(guard);
        log::info!("dropped publisher guard...");
        Ok(())
    }

    fn create_check_responsibility_event<P>(
        params: P, 
        task_id: TaskId
    ) -> std::io::Result<QuorumEvent> 
    where 
        P: TryInto<Params, Error = std::io::Error> + 
        TryInto<Namespace, Error = std::io::Error> + 
        Clone + std::fmt::Debug 
    {
        let event_id = uuid::Uuid::new_v4().to_string();
        log::info!("created event id {} for CheckResponsibility event...", &event_id);
        Ok(QuorumEvent::CheckResponsibility {
            namespace: params.clone().try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("unable to convert params to namespace: {e}")
                )
            })?,
            payload: params.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("unable to convert params to Params enum: {e}") 
                )
            })?,
            task_id,
            event_id
        })
    }

    async fn update_task_status(&self, task_id: TaskId, owner: [u8; 20]) -> std::io::Result<()> {
        let task_status_event_id = uuid::Uuid::new_v4();
        let task_status_event = TaskStatusEvent::Update { 
            owner,
            task_id: task_id.clone(), 
            task_status: TaskStatus::Pending,
            event_id: task_status_event_id.to_string()
        };
        log::info!("created task_status_event: {}", task_status_event_id);

        let mut guard = self.publisher.lock().await;
        log::info!("acquired publish guard");
        guard.publish(Box::new(TaskStatusTopic), Box::new(task_status_event)).await?;
        log::info!("published update status event");
        drop(guard);
        log::info!("dropped publish guard");
        Ok(())
    }

    async fn check_task_status(
        &self,
        original_task_id: TaskId,
        current_task_id: TaskId,
        owner: [u8; 20],
    ) -> std::io::Result<TaskStatus> {
        let event_id = uuid::Uuid::new_v4().to_string();
        let response_topics = vec![GeneralResponseTopic::RpcResponseTopic];
        let task_status_event = TaskStatusEvent::Get { 
            owner,
            original_task_id: original_task_id.clone(),
            current_task_id: current_task_id.clone(), 
            event_id: event_id.clone(),
            response_topics,
        }; 
        log::info!("created task_status_event: {}", event_id);
        let mut guard = self.publisher.lock().await;
        log::info!("acquired publish guard");
        guard.publish(
            Box::new(TaskStatusTopic),
            Box::new(task_status_event)
        ).await?;
        log::info!("published check status event");
        drop(guard);
        self.await_task_status_response(event_id, original_task_id.to_string()).await
    }

    async fn await_task_status_response(&self, task_status_check_id: String, task_id: String) -> std::io::Result<TaskStatus> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));
        let mut guard = self.subscriber.lock().await;
        log::info!("acquired subscribe guard");
        tokio::select! {
            Ok(response) = guard.receive() => {
                let mut task_status: Option<TaskStatus> = None;
                for r in response {
                    let original_event_id = r.original_event_id();
                    if task_status_check_id.to_string() == original_event_id.to_string() {
                        log::info!(
                            "Received response for task status check for task: {}",
                            task_id.to_string()
                        );
                        let status: TaskStatus = serde_json::from_str(
                            &r.response()
                        ).map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!(
                                    "Error attempting to deserialize task status for task: {}: {e}",
                                    task_id.to_string()
                                )
                            )
                        })?;
                        task_status = Some(status);
                    }
                }
                drop(guard);
                log::info!("dropped subscribe guard");
                return Ok(
                    task_status.ok_or(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!(
                                "Unable to acquire task status for task: {}",
                                task_id.to_string()
                            )
                        )
                    )?
                );
            }
            _ = interval.tick() => {
                drop(guard);
                log::info!("dropped subscribe guard");
                return Err(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "Task status request timed out for task: {}",
                            task_id.to_string()
                        )
                    )
                )
            }
        }
    }

    async fn handle_node_certificate_message(
        &self,
        node_cert: NodeCertMessage
    ) -> std::io::Result<()> {

        log::info!("Attempting to handle node certificate message");
        let event_id = Uuid::new_v4().to_string();
        log::info!("Generated event id");
        let task_id = generate_task_id(node_cert.clone()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        log::info!("Generated task id");
        let address = Address::from_hex(node_cert.peer_id).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        log::info!("parsed address from hex");
        let event = QuorumEvent::CheckAcceptCert { 
            event_id, 
            task_id, 
            peer: Peer::new(
                address,
                node_cert.peer_address.parse().map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?
            ), 
            cert: node_cert.cert 
        };
        log::info!("created QuorumEvent::CheckAcceptCert");
        let mut guard = self.publisher.lock().await;
        log::info!("Acquired publisher guard...");
        guard.publish(
            Box::new(QuorumTopic), 
            Box::new(event)
        ).await?;

        log::info!("published QuorumEvent::CheckAcceptCert");
        drop(guard);

        Ok(())
    }

    async fn request_ssh_details(
        &self,
        _remote_addr: Option<SocketAddr>,
        _params: InstanceGetSshDetails,
        _namespace: Namespace
    ) -> std::io::Result<Response<VmResponse>> {
        // check responsibility...
        // if responsible, get port map for instance
        // if not, request details from the node responsible.
        todo!()
    }

    async fn add_peer(
        &self,
        peer: NewPeerMessage
    ) -> std::io::Result<()> {
        // Send to quorum management service
        log::info!("Generating task ID for NewPeer QuorumEvent...");
        let task_id = generate_task_id(peer.clone()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        log::info!("Generating event ID for NewPeer QuorumEvent...");
        let event_id = Uuid::new_v4().to_string();

        //TODO(asmith): Replace with node signature, and recover the address
        log::info!("acquiring peer address...");
        let peer_id = Address::from_hex(&peer.new_peer_id).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        log::info!("Constructing peer...");
        let peer = Peer::new(
            peer_id, 
            peer.new_peer_address.parse().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        );
        log::info!("Constructing QuorumEvent::NewPeer...");
        let event = QuorumEvent::NewPeer { event_id, task_id, peer };

        log::info!("Publishing event...");
        let mut guard = self.publisher.lock().await;
        guard.publish(
            Box::new(QuorumTopic), 
            Box::new(event)
        ).await?;
        drop(guard);

        Ok(())

    }
}

#[tonic::async_trait]
impl Vmm for VmmService {
    async fn create_vm(
        &self,
        request: Request<InstanceCreateParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let task_id = generate_task_id(params.clone())?;

        self.check_responsibility(params.clone(), task_id.clone()).await?;

        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            Status::from_error(Box::new(e))
        })?;
        let payload_hash = get_payload_hash(
            params.into_payload().as_bytes()
        );
        let owner = recover_owner_address(payload_hash, params.sig, recovery_id)?;

        self.update_task_status(task_id.clone(), owner).await?;

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
        let task_id = generate_task_id(params.clone())?;

        self.check_responsibility(params.clone(), task_id.clone()).await?;

        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            Status::from_error(Box::new(e))
        })?;
        let payload_hash = get_payload_hash(params.into_payload().as_bytes());
        let owner = recover_owner_address(payload_hash, params.sig, recovery_id)?;

        self.update_task_status(task_id.clone(), owner).await?;

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
        let task_id = generate_task_id(params.clone())?;

        self.check_responsibility(params.clone(), task_id.clone()).await?;

        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            Status::from_error(Box::new(e))
        })?;
        let payload_hash = get_payload_hash(params.into_payload().as_bytes());
        let owner = recover_owner_address(payload_hash, params.sig, recovery_id)?;

        self.update_task_status(task_id.clone(), owner).await?;

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
        let task_id = generate_task_id(params.clone())?;

        self.check_responsibility(params.clone(), task_id.clone()).await?;

        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            Status::from_error(Box::new(e))
        })?;
        let payload_hash = get_payload_hash(params.into_payload().as_bytes());
        let owner = recover_owner_address(payload_hash, params.sig, recovery_id)?;

        self.update_task_status(task_id.clone(), owner).await?;

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
        let task_id = generate_task_id(params.clone())?;

        self.check_responsibility(params.clone(), task_id.clone()).await?;

        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            Status::from_error(Box::new(e))
        })?;
        let payload_hash = get_payload_hash(params.into_payload().as_bytes());
        let owner = recover_owner_address(payload_hash, params.sig, recovery_id)?;

        self.update_task_status(task_id.clone(), owner).await?;

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
        let task_id = generate_task_id(params.clone())?;

        self.check_responsibility(params.clone(), task_id.clone()).await?;

        let hash = get_payload_hash(params.into_payload().as_bytes());
        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            Status::from_error(Box::new(e))
        })?;
        let owner = recover_owner_address(hash, params.sig.clone(), recovery_id)?;

        self.update_task_status(task_id.clone(), owner).await?;

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
        let _owner_address = params.owner()?;

        let task_id = generate_task_id(params.clone())?;
        
        let owner = owner_address_from_string(&params.owner)?; 

        let check_task_id = TaskId::new(params.id);
        let task_status = self.check_task_status(
            check_task_id.clone(), task_id.clone(), owner 
        ).await?;

        self.update_task_status(task_id.clone(), owner).await?;

        let details = serde_json::json!({
            "task_id": check_task_id.to_string(),
            "task_status": task_status.to_string(),
        }).to_string();

        Ok(Response::new(
                VmResponse {
                    status: TaskStatus::Success.to_string(),
                    details,
                    ssh_details: None
                }
            )
        )
    }

    async fn get_ssh_details(
        &self,
        request: Request<InstanceGetSshDetails>
    ) -> Result<Response<VmResponse>, Status> {
        let remote_addr = request.remote_addr();
        let params = request.into_inner().clone();
        let namespace: Namespace = params.clone().try_into().map_err(|_| {
            Status::internal("Unable to convert params into Namespace")
        })?; 
        Ok(self.request_ssh_details(remote_addr, params, namespace).await?)
    }

    async fn register(
        &self,
        request: Request<NewPeerMessage>
    ) -> Result<Response<Ack>, Status> {
        log::info!("Received register request...");
        let new_peer = request.into_inner();
        log::info!("converted request into NewPeerMessage...");
        let header = new_peer.clone().header;
        log::info!("extracted request header...");
        let request_id = uuid::Uuid::new_v4();
        log::info!("constructed request id...");

        log::info!("calling self.add_peer...");
        self.add_peer(new_peer).await?;
        log::info!("successfully executed request...");

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
            peer_id: self.local_peer.wallet_address_hex(), 
            peer_address: self.local_peer.ip_address().to_string(),
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
        _request: Request<tonic::Streaming<SyncMessage>>
    ) -> Result<Response<crate::allegra_rpc::TransferStatus>, Status> {
        let mut _namespace = String::new();
        todo!()
    }
    
    async fn migrate(
        &self,
        _request: Request<tonic::Streaming<MigrateMessage>>
    ) -> Result<Response<crate::allegra_rpc::TransferStatus>, Status> {
        todo!()
    }

    async fn get_port(
        &self,
        request: Request<GetPortMessage>
    ) -> Result<Response<PortResponse>, Status> {
        let message = request.into_inner().clone();
        let _namespace = message.namespace;
        let _service = message.service_type;
        todo!()
    }

    async fn node_certificate(
        &self,
        request: Request<NodeCertMessage>
    ) -> Result<Response<Ack>, Status> {
        log::info!("Received node_certificate call...");
        let message = request.into_inner().clone();
        log::info!("Converted request into inner type...");
        let request_id = message.request_id.clone();
        log::info!("Attempting to handle node certificate message...");
        self.handle_node_certificate_message(message).await?;

        log::info!("Crafting response to request...");
        let message_id = Uuid::new_v4().to_string();
        let header = MessageHeader {
            peer_id: self.local_peer.wallet_address_hex(),
            peer_address: self.local_peer.ip_address().to_string(),
            message_id
        };
        log::info!("Returning response...");
        Ok(Response::new(
                Ack {
                    header: Some(header),
                    request_id
                }
            )
        )
    }
}


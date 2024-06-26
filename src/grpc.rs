use crate::{
    account::{
        Namespace, TaskId, TaskStatus
    }, allegra_rpc::{
        vmm_server::Vmm, 
        Ack, 
        GetPortMessage, 
        GetTaskStatusRequest, 
        InstanceAddPubkeyParams, 
        InstanceCreateParams, 
        InstanceDeleteParams, 
        InstanceExposeServiceParams, 
        InstanceGetSshDetails, 
        InstanceStartParams, 
        InstanceStopParams, 
        MessageHeader, 
        MigrateMessage, 
        NewPeerMessage, 
        PingMessage, 
        PongMessage, 
        PortResponse, 
        SyncMessage, 
        VmResponse
    }, dht::Peer, event::{
        QuorumEvent, 
        TaskStatusEvent
    }, helpers::{
        recover_owner_address,
        generate_task_id,
        get_payload_hash
    }, params::{
        HasOwner, Params, Payload
    }, publish::{
        GenericPublisher, 
        QuorumTopic, 
        TaskStatusTopic
    }, subscribe::RpcResponseSubscriber
};

use conductor::{subscriber::SubStream, publisher::PubStream};

use tonic::{Request, Response, Status};
use std::{net::SocketAddr, result::Result};
use std::sync::Arc;
use tokio::sync::Mutex;


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
        let event_id = uuid::Uuid::new_v4().to_string();
        log::info!("created event id {} for CheckResponsibility event...", &event_id);
        let quorum_event = QuorumEvent::CheckResponsibility {
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
        };
        log::info!("created CheckResponsibility event");
        let mut guard = self.publisher.lock().await;
        log::info!("acquired publisher guard...");
        guard.publish(Box::new(QuorumTopic), Box::new(quorum_event)).await?;
        log::info!("published CheckResponsibility event to topic {}...", QuorumTopic);
        drop(guard);
        log::info!("dropped publisher guard...");
        Ok(())
    }

    async fn update_task_status(&self, task_id: TaskId, owner: [u8; 20]) -> std::io::Result<TaskStatus> {
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
        log::info!("published check status event");
        drop(guard);
        log::info!("dropped publish guard");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));
        let mut guard = self.subscriber.lock().await;
        tokio::select! {
            Ok(response) = guard.receive() => {
                let mut task_status: Option<TaskStatus> = None;
                for r in response {
                    let original_event_id = r.original_event_id();
                    if task_status_event_id.to_string() == original_event_id.to_string() {
                        let status: TaskStatus = serde_json::from_str(&r.response()).map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!("Error attempting to deserialize task status for task: {}: {e}", task_id.to_string())
                            )
                        })?;
                        task_status = Some(status);
                    }
                }
                drop(guard);
                return Ok(
                    task_status.ok_or(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Unable to acquire task status for task: {}", task_id.to_string())
                        )
                    )?
                );
            }
            _ = interval.tick() => {
                drop(guard);
                return Err(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Task status request timed out for task: {}", task_id.to_string())
                    )
                )
            }
        }
    }

    async fn request_ssh_details(
        &self,
        _remote_addr: Option<SocketAddr>,
        _params: InstanceGetSshDetails,
        _namespace: Namespace
    ) -> std::io::Result<Response<VmResponse>> {
        todo!()
    }

    async fn add_peer(
        &self,
        _peer: NewPeerMessage
    ) -> std::io::Result<()> {
        // Send to quorum management service
        todo!()
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

        todo!()
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

        // Check which quorum is responsible
        // if local quorum is responsible
        // check if local is closest node
        Ok(self.request_ssh_details(remote_addr, params, namespace).await?)
    }

    async fn register(
        &self,
        request: Request<NewPeerMessage>
    ) -> Result<Response<Ack>, Status> {
        let new_peer = request.into_inner();
        let header = new_peer.clone().header;
        let request_id = uuid::Uuid::new_v4();

        self.add_peer(new_peer).await?;

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
}

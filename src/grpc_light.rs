#![allow(unused)]
use crate::{
    account::{
        Namespace, TaskId
    }, allegra_rpc::{
        vmm_server::Vmm, Ack, GetPortMessage, GetTaskStatusRequest, InstanceAddPubkeyParams, InstanceCreateParams, InstanceDeleteParams, InstanceExposeServiceParams, InstanceGetSshDetails, InstanceStartParams, InstanceStopParams, MessageHeader, MigrateMessage, NewPeerMessage, NodeCertMessage, PingMessage, PongMessage, PortResponse, ServiceType, SyncMessage, VmResponse
    }, create_allegra_rpc_client_to_addr, params::Payload
};

use sha3::{Digest, Sha3_256};
use tonic::{Request, Response, Status};
use std::result::Result;


#[derive(Clone)]
pub struct LightVmmService {}

#[tonic::async_trait]
impl Vmm for LightVmmService {
    async fn create_vm(
        &self,
        request: Request<InstanceCreateParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let namespace: Namespace = params.clone().try_into()?;
        let task_id = generate_task_id(params.clone())?;
        Ok(Response::new(VmResponse {
            status: "PENDING".to_string(),
            details: format!("TaskId: {}", task_id.task_id()),
            ssh_details: None,
        }))
    }

    async fn shutdown_vm(
        &self,
        request: Request<InstanceStopParams>,
    ) -> Result<Response<VmResponse>, Status> {
        let params = request.into_inner();
        let namespace: Namespace = params.clone().try_into()?;
        let task_id = generate_task_id(params.clone())?;
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
        let task_id = generate_task_id(params.clone())?;
        Ok(Response::new(VmResponse {
            status: "PENDING".to_string(),
            details: format!("TaskId: {}", task_id.to_string()),
            ssh_details: None,
        }))
    }

    async fn get_ssh_details(
        &self,
        request: Request<InstanceGetSshDetails>
    ) -> Result<Response<VmResponse>, Status> {
        Ok(Response::new(VmResponse {
            status: "PENDING".to_string(),
            details: format!("Light server cannot fulfil this request"),
            ssh_details: None
        }))
    }

    async fn register(
        &self,
        request: Request<NewPeerMessage>
    ) -> Result<Response<Ack>, Status> {
        let new_peer = request.into_inner();
        let header = new_peer.header;
        let request_id = uuid::Uuid::new_v4();

        Ok(Response::new(Ack { header, request_id: request_id.to_string() }))
    }

    async fn ping(
        &self,
        request: Request<PingMessage>
    ) -> Result<Response<PongMessage>, Status> {
        let local_peer = uuid::Uuid::new_v4();
        let peer_address = "127.0.0.1";
        let header = MessageHeader {
            peer_id: local_peer.to_string(),
            peer_address: peer_address.to_string(),
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
    ) -> Result<Response<crate::allegra_rpc::Ack>, Status> {
        todo!()
    }
    
    async fn migrate(
        &self,
        request: Request<tonic::Streaming<MigrateMessage>>
    ) -> Result<Response<crate::allegra_rpc::TransferStatus>, Status> {
        todo!()
    }

    async fn get_port(
        &self,
        request: Request<GetPortMessage>
    ) -> Result<Response<PortResponse>, Status> {
        todo!()
    }

    async fn node_certificate(
        &self,
        request: Request<NodeCertMessage>
    ) -> Result<Response<Ack>, Status> {
        todo!()
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

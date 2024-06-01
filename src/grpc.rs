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
        PongMessage, Ack, SyncMessage, MigrateMessage
    }, 
    vmm::VmManagerMessage, 
    account::{
        TaskId, 
        TaskStatus,
        Account
    }, 
    params::Payload
};

use tokio::sync::mpsc::Sender;

use sha3::{Digest, Sha3_256};
use std::sync::{Arc, RwLock};
use tonic::{Request, Response, Status};
use std::result::Result;
use lru::LruCache;


#[derive(Clone)]
pub struct VmmService {
    pub network: String,
    pub port: u16,
    pub vmm_sender: Sender<VmManagerMessage>,
    pub tikv_client: tikv_client::RawClient,
    pub task_cache: Arc<RwLock<LruCache<TaskId, TaskStatus>>> 
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

        match self.task_cache.write() {
            Ok(mut guard) => {
                if let Some(task_status) = guard.get(&TaskId::new(params.id.clone())) {
                    return Ok(Response::new(VmResponse {
                        status: "SUCCESS".to_string(),
                        details: task_status.to_string(),
                        ssh_details: None,
                    }));
                }
            }
            Err(_) => {}
        };

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
        _request: Request<InstanceGetSshDetails>
    ) -> Result<Response<VmResponse>, Status> {
        todo!()
    }

    async fn register(
        &self,
        _request: Request<NewPeerMessage>
    ) -> Result<Response<Ack>, Status> {
        todo!()
    }

    async fn ping(
        &self,
        _request: Request<PingMessage>
    ) -> Result<Response<PongMessage>, Status> {
        todo!()
    }

    async fn sync(
        &self,
        _request: Request<SyncMessage>
    ) -> Result<Response<Ack>, Status> {
        todo!()
    }
    
    async fn migrate(
        &self,
        _request: Request<MigrateMessage>
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

#![allow(unused)]
use std::{num::NonZeroUsize, collections::HashMap};
use crate::{
    account::{
        TaskId,
        Namespace, 
        TaskStatus
    }, vm_info::{VmList, VmInfo},
    params::{Payload, ServiceType},
    helpers::{
        update_task_status,
        recover_owner_address,
        recover_namespace,
        verify_ownership,
        update_iptables,
        update_account, 
        update_instance, 
        remove_temp_instance
    }, startup::{self, PUBKEY_AUTH_STARTUP_SCRIPT}, event::{Event, NetworkEvent}
};

#[cfg(feature="grpc")]
use crate::allegra_rpc::{
    InstanceCreateParams,
    InstanceStopParams,
    InstanceStartParams,
    InstanceAddPubkeyParams,
    InstanceExposeServiceParams,
    InstanceDeleteParams,
    InstanceGetSshDetails
};

use futures::{stream::{FuturesUnordered, StreamExt}, Future};
use lazy_static::lazy_static;
use tokio::{task::JoinHandle, sync::{RwLock, Mutex}};
use lru::LruCache;
use sha3::{Digest, Sha3_256};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use std::pin::Pin;

pub const LOWEST_PORT: u16 = 2222;
pub const HIGHEST_PORT: u16 = 65535;
pub static DEFAULT_NETWORK: &'static str = "lxdbr0";
pub static SUCCESS: &'static str = "SUCCESS";
pub static FAILURE: &'static str = "FAILURE";
pub static PENDING: &'static str = "PENDING";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    namespace: Namespace,
    vminfo: VmInfo,
    port_map: HashMap<u16, (u16, ServiceType)>,
    last_snapshot: Option<u64>,
}

lazy_static! {
    //TODO: replace with ENV variable
    pub static ref TEMP_PATH: &'static str = "/var/snap/lxd/common/lxd/tmp"; 
}

impl Instance {
    pub fn new(
        namespace: Namespace,
        vminfo: VmInfo,
        port_map: impl IntoIterator<Item = (u16, (u16, ServiceType))>,
        last_snapshot: Option<u64>
    ) -> Self {
        Self {
            namespace,
            vminfo,
            port_map: port_map.into_iter().collect(),
            last_snapshot
        }
    }

    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    pub fn vminfo(&self) -> &VmInfo {
        &self.vminfo
    }

    pub fn port_map(&self) -> &HashMap<u16, (u16, ServiceType)> {
        &self.port_map
    }

    pub fn extend_port_mapping(&mut self, extend: impl Iterator<Item = (u16, (u16, ServiceType))>) {
        log::info!("extending port mapping");
        self.port_map.extend(extend);
    }

    pub fn update_vminfo(&mut self, vminfo: VmInfo) {
        self.vminfo = vminfo
    }

    pub fn insert_port_mapping(&mut self, ext: u16, dest: u16, service_type: ServiceType) {
        self.port_map.insert(ext, (dest, service_type));
    }

    pub fn port_mapping_mut(&mut self) -> &mut HashMap<u16, (u16, ServiceType)> {
        &mut self.port_map
    }
}

#[derive(Debug)]
pub enum VmManagerMessage {
    NewInstance {
        #[cfg(feature="grpc")]
        params: InstanceCreateParams,
        task_id: TaskId 
    },
    StopInstance {
        #[cfg(feature="grpc")]
        params: InstanceStopParams,
        sig: String,
        task_id: TaskId
    },
    DeleteInstance {
        #[cfg(feature="grpc")]
        params: InstanceDeleteParams,
        sig: String,
        task_id: TaskId 
    },
    InjectAuth {
        #[cfg(feature="grpc")]
        params: InstanceAddPubkeyParams,
        sig: String,
        task_id: TaskId 
    },
    StartInstance {
        #[cfg(feature="grpc")]
        params: InstanceStartParams,
        sig: String,
        task_id: TaskId 
    },
    ExposeService {
        #[cfg(feature="grpc")]
        params: InstanceExposeServiceParams,
        sig: String,
        task_id: TaskId,
    },
    SyncInstance {
        namespace: String,
        path: String,
    },
    MigrateInstance {
        namespace: String,
        path: String,
        new_quorum: Option<String>,
    }
}

pub struct VmManager {
    network: String,
    next_port: u16,
    handles: FuturesUnordered<JoinHandle<std::io::Result<([u8; 20], TaskId, TaskStatus)>>>,
    sync_futures: FuturesUnordered<Pin<Box<dyn Future<Output = std::io::Result<()>> + Send>>>,
    vmlist: VmList,
    state_client: tikv_client::RawClient,
    task_cache: Arc<RwLock<LruCache<TaskId, TaskStatus>>>,
}

impl VmManager {
    pub fn task_cache(&self) -> Arc<RwLock<LruCache<TaskId, TaskStatus>>> {
        self.task_cache.clone()
    } 
}

impl VmManager {
    pub async fn new<S: Into<String>>(
        pd_endpoints: Vec<S>,
        config: Option<tikv_client::Config>,
        next_port: u16,
    ) -> std::io::Result<Self> {
        let network = DEFAULT_NETWORK.to_string();
        let handles = FuturesUnordered::new();
        let vmlist = match std::process::Command::new("lxc")
            .args(["list", "--format", "json"])
            .output() {
            Ok(output) => {
                if output.status.success() {
                    let vmlist_str = std::str::from_utf8(
                        &output.stdout
                    ).map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string()
                        )
                    })?;
                    let vmlist = serde_json::from_str(
                        vmlist_str
                    ).map_err(|e| { std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string()
                        )
                    })?;
                    vmlist
                } else {
                    let err = std::str::from_utf8(
                        &output.stderr
                    ).map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string()
                        )
                    })?.to_string();
                    return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            err
                        )
                    )
                }
            },
            Err(e) => return Err(e)
        };


        let state_client = if let Some(conf) = config {
            tikv_client::RawClient::new_with_config(
                pd_endpoints,
                conf
            ).await.map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string()
                )
            })?
        } else {
            tikv_client::RawClient::new(
                pd_endpoints
            ).await.map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string()
                )
            })?
        };

        let task_cache = Arc::new(
            RwLock::new(
                LruCache::new(
                    NonZeroUsize::new(250).ok_or(
                        std::io::Error::new(
                            std::io::ErrorKind::Other, 
                            "Unable to generate NonZeroUsize"
                        )
                    )?
                )
            )
        );
        
        let sync_futures = FuturesUnordered::new();

        Ok(Self {
            network: network.to_string(),
            next_port,
            handles,
            vmlist,
            state_client,
            task_cache,
            sync_futures,
        })
    }

    pub async fn run(
        &mut self,
        message_rx: &mut tokio::sync::mpsc::Receiver<VmManagerMessage>,
        stop_rx: &mut tokio::sync::mpsc::Receiver<()>
    ) -> std::io::Result<()> {
        loop {
            tokio::select! {
                message = message_rx.recv() => {
                    if let Some(m) = message {
                        log::info!("message received");
                        match self.handle_vmm_message(m).await {
                            Err(e) => {
                                log::error!("Error in message handler: {e}");
                            }
                            _ => {}
                        }
                    }
                },
                stop = stop_rx.recv() => {
                    if let Some(_) = stop {
                        log::warn!("received stop token");
                        break
                    }
                },
                status = self.handles.next() => {
                    match status {
                        Some(Ok(Ok((owner, task_id, task_status)))) => {
                            log::info!("future completed");
                            match update_task_status(
                                self.state_client.clone(),
                                owner,
                                task_id,
                                task_status,
                                &mut self.task_cache
                            ).await {
                                Err(e) => log::error!("Error in updating task status {e}"),
                                _ => {} 
                            }
                        }
                        Some(Err(e)) => {
                            log::error!("error handling future {e}");
                        }
                        Some(Ok(Err(e))) => {
                            log::error!("{e}");
                        }
                        None => {}
                    }
                },
                sync_status = self.sync_futures.next() => {
                    match sync_status {
                        Some(Ok(())) => {
                        }
                        Some(Err(e)) => {
                            log::error!("error handling future {e}");
                        }
                        None => {}
                    }
                },
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(180)) => {
                    log::info!("refreshing vm list");
                    match self.refresh_vmlist().await {
                        Err(e) => log::error!("{e}"),
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(900)) => {
                    log::info!("syncing VMs that have not synced in past 15 minutes");
                    match self.refresh_vmlist().await {
                        Ok(()) => {
                        }
                        Err(e) => log::error!("{e}")
                    }
                }
            }
        }

        log::info!("loop broken, exiting vmm.run()...");

        return Ok(())
    }

    pub async fn refresh_vmlist(&mut self) -> std::io::Result<()> {
        let vmlist = match std::process::Command::new("lxc")
            .args(["list", "--format", "json"])
            .output() {
            Ok(output) => {
                if output.status.success() {
                    let vmlist_str = std::str::from_utf8(
                        &output.stdout
                    ).map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string()
                        )
                    })?;
                    let vmlist = serde_json::from_str(
                        vmlist_str
                    ).map_err(|e| { std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string()
                        )
                    })?;
                    vmlist
                } else {
                    let err = std::str::from_utf8(
                        &output.stderr
                    ).map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string()
                        )
                    })?.to_string();
                    return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            err
                        )
                    )
                }
            },
            Err(e) => return Err(e)
        };

        log::info!("vm list refreshed saving to self.vmlist");
        self.vmlist = vmlist;

        Ok(())
    }


    async fn handle_vmm_message(
        &mut self,
        message: VmManagerMessage
    ) -> std::io::Result<()> {
        match message {
            VmManagerMessage::NewInstance { 
                params, 
                task_id 
            } => {
                log::info!("received NewInstance message, attempting to launch instance.");
                return self.launch_instance(params, task_id).await
            }
            VmManagerMessage::StartInstance { 
                params, 
                sig, 
                task_id 
            } => {
                log::info!("received StartInstance message, attempting to start instance.");
                return self.start_instance(params, sig, task_id).await
            }
            VmManagerMessage::InjectAuth { 
                params, 
                sig, 
                task_id 
            } => {
                log::info!("received InjectAuth message, attempting to add auth to instance.");
                return self.inject_authorization(params, sig, task_id).await
            }
            VmManagerMessage::StopInstance { 
                params, 
                sig, 
                task_id 
            } => {
                log::info!("received StopInstance message, attempting to stop instance.");
                return self.stop_instance(params, sig, task_id).await
            }
            VmManagerMessage::DeleteInstance { 
                params, 
                sig, 
                task_id 
            } => {
                log::info!("received DeleteInstance message, attempting to delete instance.");
                return self.delete_instance(params, sig, task_id).await
            }
            VmManagerMessage::ExposeService { 
                params, 
                sig, 
                task_id 
            } => {
                log::info!("received ExposeService message, attempting to expose service on instance.");
                return self.expose_service(params, sig, task_id).await
            }
            VmManagerMessage::SyncInstance { namespace, path } => {
                log::info!("received SyncInstance message, attempting to sync instance");
                let vmlist = self.vmlist.clone();
                let future = Box::pin(Self::sync_instance(vmlist, namespace, path));
                self.sync_futures.push(future);
                return Ok(())
            }
            VmManagerMessage::MigrateInstance { namespace, new_quorum, path } => {
                log::info!("received MigrateInstance message, attempting to migrate instance");
                let vmlist = self.vmlist.clone();
                let future = Box::pin(Self::move_instance(vmlist, namespace, path, new_quorum));
                self.sync_futures.push(future);
                return Ok(())
            }
        }
    }

    async fn handle_start_output_success(
        &mut self,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        update_task_status(
            self.state_client.clone(),
            owner,
            task_id,
            TaskStatus::Success,
            &mut self.task_cache
        ).await
    }

    async fn handle_start_output_failure(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        let err_string = std::str::from_utf8(
            &output.stderr
        ).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;
        update_task_status(
            self.state_client.clone(),
            owner,
            task_id,
            TaskStatus::Failure(
                err_string.to_string()
            ),
            &mut self.task_cache
        ).await
    }

    async fn handle_start_output(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        if output.status.success() {
            self.handle_start_output_success(
                owner,
                task_id
            ).await
        } else {
            self.handle_start_output_failure(
                output,
                owner, 
                task_id
            ).await
        }
    }

    async fn start_instance(
        &mut self,
        params: InstanceStartParams,
        sig: String,
        task_id: TaskId
    ) -> std::io::Result<()> {
        let payload = params.into_payload();

        let mut hasher = sha3::Sha3_256::new();
        hasher.update(
            payload.as_bytes()
        );
        let hash = hasher.finalize().to_vec();
        let owner = recover_owner_address(
            hash,
            sig,
            params.recovery_id.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        )?;

        let namespace = recover_namespace(
            owner,
            &params.name
        );

        if let Ok(()) = verify_ownership(
            self.state_client.clone(),
            owner,
            namespace
        ).await {
            let mut command = std::process::Command::new("lxc");
            command.arg("start").arg(&params.name);

            if params.console {
                command.arg("--console");
            }

            if params.stateless {
                command.arg("--stateless");
            }

            match command.output() {
                Ok(o) => {
                    self.handle_start_output(
                        o,
                        owner,
                        task_id
                    ).await?
                },
                Err(e) => {
                    return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string(),
                        )
                    )
                }
            };

            return Ok(())
        } 

        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to verify ownership of instance"
            )
        )

    }

    async fn handle_inject_authorization_output_success(
        &mut self,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        update_task_status(
            self.state_client.clone(),
            owner,
            task_id,
            TaskStatus::Success,
            &mut self.task_cache
        ).await
    }

    async fn handle_inject_authorization_output_failure(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        let err_str = std::str::from_utf8(
            &output.stderr
        ).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?.to_string();
        log::error!("{err_str}");
        update_task_status(
            self.state_client.clone(),
            owner,
            task_id,
            TaskStatus::Failure(
                err_str
            ),
            &mut self.task_cache
        ).await
    }

    async fn handle_inject_authorization_output(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        if output.status.success() {
            log::info!("Auth Injection was successful");
            self.handle_inject_authorization_output_success(
                owner,
                task_id
            ).await
        } else {
            log::info!("Auth Injection was a Failure");
            self.handle_inject_authorization_output_failure(
                output,
                owner,
                task_id
            ).await
        }
    }

    async fn inject_authorization(
        &mut self,
        params: InstanceAddPubkeyParams,
        sig: String, 
        task_id: TaskId
    ) -> std::io::Result<()> {
        let mut hasher = Sha3_256::new();
        hasher.update(params.into_payload().as_bytes());
        let hash = hasher.finalize().to_vec();

        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        let owner = recover_owner_address(hash, sig, recovery_id)?;
        log::info!("Recovered owner address");
        let namespace = recover_namespace(owner, &params.name);
        log::info!("Recovered Instance Namespace");

        match verify_ownership(
            self.state_client.clone(),
            owner,
            namespace.clone()
        ).await {
            Ok(()) => {
                let echo = format!(
                    r#"echo '{}' >> /root/.ssh/authorized_keys"#,
                    params.pubkey
                );
                let output = std::process::Command::new("lxc")
                    .arg("exec")
                    .arg(&namespace.inner())
                    .arg("--")
                    .arg("bash")
                    .arg("-c")
                    .arg(&echo)
                    .output()?;
                
                log::info!("Executed Auth Injection");

                return self.handle_inject_authorization_output(
                    output,
                    owner,
                    task_id
                ).await
            }
            Err(e) => {
                return Err(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Unable to verify ownership: {e}")
                    )
                )
            }
        }

    }

    async fn handle_stop_success(
        &mut self,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        update_task_status(
            self.state_client.clone(),
            owner,
            task_id,
            TaskStatus::Success,
            &mut self.task_cache
        ).await?;
        Ok(())
    }

    async fn handle_stop_failure(
        &mut self,  
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        let err_str = std::str::from_utf8(
            &output.stderr
        ).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{e:#?}")
            )
        })?.to_string();

        update_task_status(
            self.state_client.clone(),
            owner,
            task_id, 
            TaskStatus::Failure(
                err_str.clone()
            ),
            &mut self.task_cache
        ).await?;
        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                err_str
            )
        )
    }

    async fn handle_stop_output_and_response(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        if output.status.success() {
            println!("Successfully shutdown vm...");
            self.handle_stop_success(
                owner,
                task_id
            ).await?;
            Ok(())
        } else {
            self.handle_stop_failure(
                output,
                owner,
                task_id
            ).await?;
            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to stop instance..."
                )
            )
        }
    }

    async fn stop_instance(
        &mut self,
        params: InstanceStopParams,
        sig: String,
        task_id: TaskId
    ) -> std::io::Result<()> {
        let payload = serde_json::json!({
            "command": "stop",
            "name": &params.name, 
        }).to_string();

        let mut hasher = Sha3_256::new();
        hasher.update(
            payload.as_bytes()
        );
        let hash = hasher.finalize().to_vec();

        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        let owner = recover_owner_address(
            hash,
            sig,
            recovery_id
        )?;

        let namespace = recover_namespace(
            owner,
            &params.name
        );

        match verify_ownership(
            self.state_client.clone(),
            owner,
            namespace.clone()
        ).await {
            Ok(()) => {
                println!("attempting to shutdown {}", &params.name);
                let output = std::process::Command::new("lxc")
                    .args(["stop", &namespace.inner()])
                    .output()?;

                println!("Retrieved output...");

                self.handle_stop_output_and_response(
                    output,
                    owner,
                    task_id
                ).await?;

                return Ok(())
            }
            Err(e) => {
                return Err(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Unable to verify ownership: {e}")
                    )
                )
            }
        }

    }


    async fn handle_delete_instance_output_success(
        &mut self,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        update_task_status(
            self.state_client.clone(),
            owner,
            task_id,
            TaskStatus::Success,
            &mut self.task_cache
        ).await
    }

    async fn handle_delete_instance_output_failure(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        let err_str = std::str::from_utf8(
            &output.stderr
        ).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?.to_string();

        update_task_status(
            self.state_client.clone(),
            owner,
            task_id,
            TaskStatus::Failure(
                err_str
            ),
            &mut self.task_cache
        ).await
    }

    async fn handle_delete_instance_output(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        if output.status.success() {
            self.handle_delete_instance_output_success(
                owner,
                task_id
            ).await
        } else {
            self.handle_delete_instance_output_failure(
                output,
                owner,
                task_id
            ).await
        }
    }

    async fn delete_instance(
        &mut self,
        params: InstanceDeleteParams,
        sig: String,
        task_id: TaskId
    ) -> std::io::Result<()> {
        let payload = serde_json::json!({
            "command": "delete",
            "name": params.name
        }).to_string();

        let mut hasher = Sha3_256::new();
        hasher.update(payload.as_bytes());
        let hash = hasher.finalize().to_vec();

        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        let owner = recover_owner_address(
            hash,
            sig, 
            recovery_id
        )?;
        let namespace = recover_namespace(owner, &params.name.clone());
        let mut command = std::process::Command::new("lxc");
        command.arg("delete").arg(&namespace.inner());

        if params.interactive {
            command.arg("--interactive");
        }

        if params.force {
            command.arg("--force");
        }

        let output = command.output()?;


        self.handle_delete_instance_output(
            output,
            owner,
            task_id
        ).await
    }

    async fn expose_service(
        &mut self,
        params: InstanceExposeServiceParams,
        sig: String,
        task_id: TaskId
    ) -> std::io::Result<()> {

        let payload = params.into_payload();

        let mut hasher = Sha3_256::new();
        hasher.update(
            payload.as_bytes()
        );
        let hash = hasher.finalize().to_vec();

        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        let owner = recover_owner_address(
            hash,
            sig,
            recovery_id
        )?;

        let namespace = recover_namespace(
            owner,
            &params.name
        );

        if let Ok(()) = verify_ownership(
            self.state_client.clone(),
            owner,
            namespace.clone()
        ).await {
            for (port, service) in params.port.into_iter().zip(params.service_type.into_iter()) {
                let port = port.try_into().map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?;
                let state_client = self.state_client.clone();
                self.refresh_vmlist().await?;
                let vmlist = self.vmlist.clone();
                let next_port = self.next_port;
                let inner_namespace = namespace.clone();
                let inner_task_id = task_id.clone();
                let service: ServiceType = service.into();
                let handle: JoinHandle<std::io::Result<([u8; 20], TaskId, TaskStatus)>> = tokio::spawn(
                    async move {
                        let (owner, task_id, task_status) = update_iptables(
                            state_client.clone(),
                            vmlist.clone(),
                            owner,
                            inner_namespace.clone(),
                            next_port,
                            service.clone(),
                            inner_task_id.clone(),
                            port 
                        ).await?;
                        Ok((owner, task_id, task_status))
                    }
                );
                self.next_port += 1;
                self.handles.push(handle);
            }

        }
        Ok(())
    }

    async fn handle_create_output_success(
        &mut self,
        task_id: TaskId,
        owner: [u8; 20],
        namespace: Namespace,
    ) -> std::io::Result<()> {
        let next_port = self.next_port;
        let state_client = self.state_client.clone();
        self.refresh_vmlist().await?;
        let vm_list = self.vmlist.clone();

        let handle: JoinHandle<std::io::Result<([u8; 20], TaskId, TaskStatus)>> = tokio::spawn(
            async move {
                log::warn!("sleeping for 2 minutes to allow instance to fully launch...");
                tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
                log::info!("updating iptables...");
                let (owner, task_id, task_status) = update_iptables(
                    state_client,
                    vm_list,
                    owner,
                    namespace,
                    next_port, 
                    ServiceType::Ssh,
                    task_id.clone(),
                    22
                ).await?;
                Ok((owner, task_id, task_status))
        });

        self.handles.push(handle);
        log::info!("added update_iptables handle to self.handles, returning...");

        return Ok(())
    }

    async fn handle_create_output_failure_and_response(
        &mut self,
        output: &std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        log::error!("Launch instance failed");
        let err_str = std::str::from_utf8(
            &output.stderr
        ).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?.to_string();
        log::error!("Error from failed launch: {err_str}");
        update_task_status(
            self.state_client.clone(),
            owner,
            task_id.clone(), 
            TaskStatus::Failure(err_str.clone()),
            &mut self.task_cache
        ).await?;
        log::info!("Updated task status from PENDING to FAILURE for task {task_id}");

        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                err_str
            )
        )
    }

    async fn handle_create_output_and_response(
        &mut self,
        output: std::process::Output,
        task_id: TaskId,
        owner: [u8; 20],
        namespace: Namespace,
    ) -> std::io::Result<()> {
        if output.status.success() {
            log::info!("successfully created instance, handling output...");
            self.handle_create_output_success(
                task_id,
                owner,
                namespace,
            ).await
        } else {
            log::error!("unable to create instance, handling output...");
            self.handle_create_output_failure_and_response(
                &output,
                owner,
                task_id
            ).await
        }
    }
    pub async fn launch_instance(
        &mut self,
        params: InstanceCreateParams,
        task_id: TaskId
    ) -> std::io::Result<()> {
        log::info!("Attempting to start instance...");

        let payload = params.into_payload(); 
        log::info!("converted params into payload...");

        let mut hasher = Sha3_256::new();

        hasher.update(
            payload.as_bytes()
        );

        let hash = hasher.finalize().to_vec();

        log::info!("hashed params payload...");
        let recovery_id = params.recovery_id.try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        log::info!("converted recovery_id into u32...");

        let owner = recover_owner_address(
            hash,
            params.sig.clone(),
            recovery_id
        )?;

        log::info!("recovered owner from signature...");
        let namespace = recover_namespace(
            owner,
            &params.name
        );

        log::info!("recovered namespace from name and owner...");

        let output = std::process::Command::new("lxc")
            .arg("launch")
            .arg(
                &format!(
                    "{}:{}",
                    params.distro,
                    params.version
                )
            )
            .arg(
                &format!(
                    "{}",
                    &namespace
                )
            )
            .arg("--vm")
            .arg("-t")
            .arg(&params.vmtype.to_string())
            .arg("--network")
            .arg(&self.network.clone())
            .output()?;

        log::info!("executed launch command...");
        self.handle_create_output_and_response(
            output,
            task_id.clone(),
            owner, 
            namespace.clone(), 
        ).await?;

        self.refresh_vmlist().await?;
        log::info!("refreshed vm list...");

        let _output = startup::run_script(namespace.clone(), PUBKEY_AUTH_STARTUP_SCRIPT)?;
        log::info!("ran pubkey auth startup script...");

        update_account(
            self.state_client.clone(),
            self.vmlist.clone(),
            owner,
            namespace.clone(),
            task_id.clone(),
            TaskStatus::Pending,
            vec![]
        ).await?;

        log::info!("successfully updated account...");

        let vminfo = self.vmlist.get(&namespace.inner()).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to find instance namespace in VM list"
            )
        )?;

        log::info!("acquired vminfo...");
        update_instance(
            namespace.clone(),
            vminfo,
            vec![(22, (self.next_port, ServiceType::Ssh))],
            self.state_client.clone()
        ).await?;
        log::info!("updated instance entry in state...");

        Ok(())
    }

    pub async fn sync_instance(
        vmlist: VmList,
        namespace: String,
        path: String 
    ) -> std::io::Result<()> {
        log::info!("Attempting to sync instance {namespace}");
        let import_output = std::process::Command::new("sudo")
            .arg("lxc")
            .arg("import")
            .arg(&path)
            .arg(&format!("{}-temp", namespace))
            .output()?;

        if import_output.status.success() {
            log::info!("Successfully imported temporary instance {namespace}");
            if let Some(vm) = vmlist.get(&namespace) {
                let copy_output = std::process::Command::new("sudo")
                    .arg("lxc")
                    .arg("copy")
                    .arg(&format!("{}-temp", namespace))
                    .arg(&namespace)
                    .arg("--refresh")
                    .output()?;

                if copy_output.status.success() {
                    log::info!("Successfully copied temporary instance to permanent instance {namespace}");
                    remove_temp_instance(&namespace, &path).await?;
                    log::info!("Successfully removed temporary instance {namespace}");
                    return Ok(())
                } else {
                    return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Unable to sync existing instance with temp instance"
                        )
                    )
                }
            }
        } else {
            log::info!("Instance does not exist, moving from temp to permanent {namespace}");
            let rename_output = std::process::Command::new("sudo")
                .arg("move")
                .arg(&format!("{}-temp", namespace))
                .arg(&namespace)
                .output()?;

            if rename_output.status.success() {
                log::info!("Successfully moved temporary instance to permanent {namespace}");
                remove_temp_instance(&namespace, &path).await?;
                log::info!("Successfully removed temporary instance {namespace}");
                return Ok(())
            } else {
                return Err(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Unable to rename instance to permanent name"
                    )
                )
            }
        }

        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to import instance from tarball"
            )
        )
    }

    pub async fn move_instance(
        vmlist: VmList,
        namespace: String,
        path: String,
        new_quorum: Option<String>,
    ) -> std::io::Result<()> {
        let import_output = std::process::Command::new("sudo")
            .arg("lxc")
            .arg("import")
            .arg(&path)
            .arg(&format!("{}-temp", namespace))
            .output()?;

        if import_output.status.success() {
            if let Some(vm) = vmlist.get(&namespace) {
                let copy_output = std::process::Command::new("sudo")
                    .arg("lxc")
                    .arg("copy")
                    .arg(&format!("{}-temp", namespace))
                    .arg(&namespace)
                    .arg("--refresh")
                    .output()?;

                if copy_output.status.success() {
                    remove_temp_instance(&namespace, &path).await?;
                    return Ok(())
                } else {
                    return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Unable to sync existing instance with temp instance"
                        )
                    )
                }
            }
        } else {
            let rename_output = std::process::Command::new("sudo")
                .arg("move")
                .arg(&format!("{}-temp", namespace))
                .arg(&namespace)
                .output()?;

            if rename_output.status.success() {
                remove_temp_instance(&namespace, &path).await?;
                return Ok(())
            } else {
                return Err(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Unable to rename instance to permanent name"
                    )
                )
            }
        }

        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to import instance from tarball"
            )
        )
    }
}


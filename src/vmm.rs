use std::collections::HashMap;
use crate::{
    account::{
        Namespace, 
        TaskId, 
        TaskStatus
    }, event::{
        StateEvent, 
        VmmEvent
    }, helpers::{
        get_payload_hash, 
        recover_namespace, 
        recover_owner_address, 
        update_iptables, 
        update_task_status
    }, params::{
        Payload, ServiceType
    }, publish::{
        GenericPublisher,
        StateTopic
    }, startup::{
        self, 
        PUBKEY_AUTH_STARTUP_SCRIPT
    }, subscribe::VmmSubscriber, vm_info::{
        VmInfo, 
        VmList
    }
};

use crate::allegra_rpc::{
    InstanceCreateParams,
    InstanceStopParams,
    InstanceStartParams,
    InstanceAddPubkeyParams,
    InstanceExposeServiceParams,
    InstanceDeleteParams,
};

use conductor::{
    publisher::PubStream, 
    subscriber::SubStream
};
use futures::{
    stream::{
        FuturesUnordered, 
        StreamExt
    }, 
    Future
};
use lazy_static::lazy_static;
use rayon::iter::{
    IndexedParallelIterator, 
    IntoParallelIterator, 
    IntoParallelRefIterator, 
    ParallelExtend, 
    ParallelIterator
};
use tokio::task::JoinHandle;
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
        port_map: impl IntoParallelIterator<Item = (u16, (u16, ServiceType))>,
        last_snapshot: Option<u64>
    ) -> Self {
        Self {
            namespace,
            vminfo,
            port_map: port_map.into_par_iter().collect(),
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

    pub fn extend_port_mapping(
        &mut self,
        extend: impl ParallelIterator<Item = (u16, (u16, ServiceType))>
    ) {
        log::info!("extending port mapping");
        self.port_map.par_extend(extend);
    }

    pub fn update_vminfo(&mut self, vminfo: VmInfo) {
        self.vminfo = vminfo
    }

    pub fn insert_port_mapping(
        &mut self,
        ext: u16,
        dest: u16, 
        service_type: ServiceType
    ) {
        self.port_map.insert(ext, (dest, service_type));
    }

    pub fn port_mapping_mut(
        &mut self
    ) -> &mut HashMap<u16, (u16, ServiceType)> {
        &mut self.port_map
    }
}

#[derive(Clone, Debug)]
pub enum VmmResult {
    UpdateIptables {
        owner: [u8; 20],
        task_id: TaskId,
        task_status: TaskStatus
    },
    Unit(()),
    Other(String)
}

#[derive(Debug)]
pub enum VmManagerMessage {
    NewInstance {
        event_id: String,
        params: InstanceCreateParams,
        task_id: TaskId 
    },
    StopInstance {
        event_id: String,
        params: InstanceStopParams,
        sig: String,
        task_id: TaskId
    },
    DeleteInstance {
        event_id: String,
        params: InstanceDeleteParams,
        sig: String,
        task_id: TaskId 
    },
    InjectAuth {
        event_id: String,
        params: InstanceAddPubkeyParams,
        sig: String,
        task_id: TaskId 
    },
    StartInstance {
        event_id: String,
        params: InstanceStartParams,
        sig: String,
        task_id: TaskId 
    },
    ExposeService {
        event_id: String,
        params: InstanceExposeServiceParams,
        sig: String,
        task_id: TaskId,
    },
    SyncInstance {
        event_id: String,
        namespace: String,
        path: String,
    },
    MigrateInstance {
        event_id: String,
        namespace: String,
        path: String,
        new_quorum: Option<String>,
    }
}

impl From<VmmEvent> for VmManagerMessage {
    fn from(value: VmmEvent) -> Self {
        match value {
            VmmEvent::Create { 
                event_id,
                task_id,
                name,
                distro,
                version,
                vmtype,
                sig,
                recovery_id
            } => {
                let params = InstanceCreateParams {
                    name, 
                    distro, 
                    version, 
                    vmtype: vmtype.to_string(),
                    sig, 
                    recovery_id: recovery_id.into()
                };
                VmManagerMessage::NewInstance { 
                    params,
                    task_id, 
                    event_id 
                }
            }
            VmmEvent::Start { 
                event_id, 
                task_id, 
                name, 
                console, 
                stateless, 
                sig, 
                recovery_id 
            } => {
                let params = InstanceStartParams {
                    name, 
                    console, 
                    stateless, 
                    sig: sig.clone(), 
                    recovery_id: recovery_id.into()
                };
                VmManagerMessage::StartInstance { 
                    event_id, 
                    params, 
                    sig, 
                    task_id 
                }
            }
            VmmEvent::Stop { event_id, task_id, name, sig, recovery_id } => {
                let params = InstanceStopParams {
                    name,
                    sig: sig.clone(),
                    recovery_id: recovery_id.into()
                };
                VmManagerMessage::StopInstance { 
                    event_id, 
                    params, 
                    sig, 
                    task_id
                }
            }
            VmmEvent::Delete { 
                event_id,
                task_id, 
                name, 
                sig, 
                recovery_id, 
                force, 
                interactive 
            } => {
                let params = InstanceDeleteParams {
                    name,
                    sig: sig.clone(),
                    force,
                    interactive,
                    recovery_id: recovery_id.into()
                };

                VmManagerMessage::DeleteInstance { 
                    event_id, 
                    params, 
                    sig, 
                    task_id
                } 
            }
            VmmEvent::AddPubkey { 
                event_id,
                task_id,
                name,
                sig, 
                recovery_id,
                pubkey 
            } => {
                let params = InstanceAddPubkeyParams {
                    name,
                    pubkey: pubkey.clone(),
                    sig: sig.clone(),
                    recovery_id: recovery_id.into()
                };
                VmManagerMessage::InjectAuth { 
                    event_id,
                    params, 
                    sig,
                    task_id
                }
            }
            VmmEvent::ExposeService { 
                event_id, 
                task_id, 
                name, 
                sig, 
                recovery_id, 
                port, 
                service_type 
            } => {
                let params = InstanceExposeServiceParams {
                    name,
                    port: port.par_iter().map(|n| {
                        let n = *n;
                        n.into()
                    }).collect(),
                    service_type: service_type.par_iter().map(|s| {
                        let s = s.clone();
                        s.into()
                    }).collect(),
                    sig: sig.clone(),
                    recovery_id: recovery_id.into()
                };

                VmManagerMessage::ExposeService { 
                    event_id, 
                    params, 
                    sig, 
                    task_id 
                }
            }
        }
    }
}

pub struct VmManager {
    network: String,
    next_port: u16,
    handles: FuturesUnordered<JoinHandle<std::io::Result<VmmResult>>>,
    sync_futures: FuturesUnordered<Pin<Box<dyn Future<Output = std::io::Result<()>> + Send>>>,
    vmlist: VmList,
    publisher: GenericPublisher,
    pub subscriber: VmmSubscriber,
}

impl VmManager {
    pub async fn new(next_port: u16) -> std::io::Result<Self> {
        let network = DEFAULT_NETWORK.to_string();
        log::info!("set lxd network interface to {}", &network);
        let handles = FuturesUnordered::new();
        log::info!("established FuturesUnordered handler");
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
        log::info!("acquired vm list");

        let sync_futures = FuturesUnordered::new();
        log::info!("established syncing handler");
        let subscriber = VmmSubscriber::new("127.0.0.1:5556").await?; 
        log::info!("instantiated VmmSubscriber, listening on 127.0.0.1:5556");
        let publisher = GenericPublisher::new("127.0.0.1:5555").await?;
        log::info!("instantiated GenericPublisher publishing to 127.0.0.1:5555");
        log::info!("Returning VmManager");
        Ok(Self {
            network: network.to_string(),
            next_port,
            handles,
            vmlist,
            sync_futures,
            subscriber,
            publisher
        })
    }

    pub async fn run(
        &mut self,
        stop_rx: &mut tokio::sync::mpsc::Receiver<()>
    ) -> std::io::Result<()> {
        log::info!("Starting VmManager");
        loop {
            tokio::select! {
                messages = self.subscriber.receive() => {
                    if let Ok(m) = messages {
                        log::info!("message received");
                        for msg in m {
                            match self.handle_vmm_message(
                                msg.into()
                            ).await {
                                Err(e) => {
                                    log::error!("Error in message handler: {e}");
                                }
                                _ => {}
                            }
                        }
                    }
                },
                stop = stop_rx.recv() => {
                    if let Some(_) = stop {
                        log::warn!("received stop token");
                        break
                    }
                },
                vmm_result = self.handles.next() => {
                    match vmm_result {
                        Some(Ok(Ok(res))) => {
                            match res {
                                VmmResult::UpdateIptables {
                                    owner, task_id, task_status
                                } => {
                                    log::info!("future completed");
                                    if let Ok(uri) = self.publisher.peer_addr() {
                                        match update_task_status(
                                            uri,
                                            owner,
                                            task_id,
                                            task_status,
                                        ).await {
                                            Err(e) => log::error!(
                                                "Error in updating task status {e}"
                                            ),
                                            _ => {} 
                                        }
                                    }
                                }
                                _ => { todo!() }
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

}

// Core Handlers
impl VmManager {
    async fn handle_vmm_message(
        &mut self,
        message: VmManagerMessage
    ) -> std::io::Result<()> {
        match message {
            VmManagerMessage::NewInstance { 
                event_id,
                params, 
                task_id 
            } => {
                log::info!("received NewInstance message, attempting to launch instance.");
                return self.launch_instance(params, task_id).await
            }
            VmManagerMessage::StartInstance { 
                event_id,
                params, 
                sig, 
                task_id 
            } => {
                log::info!("received StartInstance message, attempting to start instance.");
                return self.start_instance(params, sig, task_id).await
            }
            VmManagerMessage::InjectAuth { 
                event_id,
                params, 
                sig, 
                task_id 
            } => {
                log::info!("received InjectAuth message, attempting to add auth to instance.");
                return self.inject_authorization(params, sig, task_id).await
            }
            VmManagerMessage::StopInstance { 
                event_id,
                params, 
                sig, 
                task_id 
            } => {
                log::info!("received StopInstance message, attempting to stop instance.");
                return self.stop_instance(params, sig, task_id).await
            }
            VmManagerMessage::DeleteInstance { 
                event_id,
                params, 
                sig, 
                task_id 
            } => {
                log::info!("received DeleteInstance message, attempting to delete instance.");
                return self.delete_instance(params, sig, task_id).await
            }
            VmManagerMessage::ExposeService { 
                event_id,
                params, 
                sig, 
                task_id 
            } => {
                log::info!("received ExposeService message, attempting to expose service on instance.");
                return self.expose_service(params, sig, task_id).await
            }
            VmManagerMessage::SyncInstance { event_id, namespace, path } => {
                log::info!("received SyncInstance message, attempting to sync instance");
                let vmlist = self.vmlist.clone();
                let future = Box::pin(Self::sync_instance(vmlist, namespace, path));
                self.sync_futures.push(future);
                return Ok(())
            }
            VmManagerMessage::MigrateInstance { event_id, namespace, new_quorum, path } => {
                log::info!("received MigrateInstance message, attempting to migrate instance");
                let vmlist = self.vmlist.clone();
                let future = Box::pin(Self::move_instance(vmlist, namespace, path, new_quorum));
                self.sync_futures.push(future);
                return Ok(())
            }
        }
    }

    async fn start_instance(
        &mut self,
        params: InstanceStartParams,
        sig: String,
        task_id: TaskId
    ) -> std::io::Result<()> {
        let hash = get_payload_hash(params.into_payload().as_bytes());
        let owner = recover_owner_address(
            hash,
            sig,
            params.recovery_id.to_be_bytes()[3]
        )?;

        let namespace = recover_namespace(owner, &params.name);

        let mut command = std::process::Command::new("lxc");
        command.arg("start").arg(&namespace.inner());

        if params.console {
            command.arg("--console");
        }

        if params.stateless {
            command.arg("--stateless");
        }

        match command.output() {
            Ok(o) => {
                let uri = self.publisher.peer_addr()?;
                Self::handle_start_output(
                    o,
                    owner,
                    task_id,
                    uri
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
        }

        return Ok(())
    }

    async fn handle_inject_authorization_output_success(
        &mut self,
        owner: [u8; 20],
        task_id: TaskId,
        uri: String
    ) -> std::io::Result<()> {
        update_task_status(
            uri,
            owner,
            task_id,
            TaskStatus::Success,
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
        let uri = self.publisher.peer_addr()?;
        update_task_status(
            uri,
            owner,
            task_id,
            TaskStatus::Failure(
                err_str
            ),
        ).await
    }

    async fn handle_inject_authorization_output(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId,
        uri: String
    ) -> std::io::Result<()> {
        if output.status.success() {
            log::info!("Auth Injection was successful");
            self.handle_inject_authorization_output_success(
                owner,
                task_id,
                uri
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
        task_id: TaskId,
    ) -> std::io::Result<()> {
        let hash = get_payload_hash(params.into_payload().as_bytes());
        let recovery_id = params.recovery_id.to_be_bytes()[3];
        let owner = recover_owner_address(hash, sig, recovery_id)?;
        log::info!("Recovered owner address");
        let namespace = recover_namespace(owner, &params.name);
        log::info!("Recovered Instance Namespace");

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

        let uri = self.publisher.peer_addr()?;
        return self.handle_inject_authorization_output(
            output,
            owner,
            task_id,
            uri
        ).await
    }

    async fn handle_stop_success(
        &mut self,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        let uri = self.publisher.peer_addr()?;
        update_task_status(
            uri,
            owner,
            task_id,
            TaskStatus::Success,
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

        let uri = self.publisher.peer_addr()?;
        update_task_status(
            uri,
            owner,
            task_id, 
            TaskStatus::Failure(
                err_str.clone()
            ),
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
        let hash = get_payload_hash(params.into_payload().as_bytes());
        let recovery_id = params.recovery_id.to_be_bytes()[3];
        let owner = recover_owner_address(hash, sig, recovery_id)?;
        let namespace = recover_namespace(owner, &params.name);
        let output = std::process::Command::new("lxc")
            .args(["stop", &namespace.inner()])
            .output()?;
        self.handle_stop_output_and_response(output, owner, task_id).await?;

        return Ok(())
    }


    async fn handle_delete_instance_output_success(
        &mut self,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        let uri = self.publisher.peer_addr()?;
        update_task_status(
            uri,
            owner,
            task_id,
            TaskStatus::Success,
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

        let uri = self.publisher.peer_addr()?;
        update_task_status(
            uri,
            owner,
            task_id,
            TaskStatus::Failure(
                err_str
            ),
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
        let hash = get_payload_hash(params.into_payload().as_bytes());
        let recovery_id = params.recovery_id.to_be_bytes()[3];
        let owner = recover_owner_address(hash, sig, recovery_id)?;
        let namespace = recover_namespace(owner, &params.name.clone());
        let mut command = std::process::Command::new("lxc");
        command.arg("delete").arg(&namespace.inner());
        if params.interactive { command.arg("--interactive"); }
        if params.force { command.arg("--force"); }
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
        task_id: TaskId,
    ) -> std::io::Result<()> {
        let hash = get_payload_hash(params.into_payload().as_bytes());
        let recovery_id = params.recovery_id.to_be_bytes()[3];
        let owner = recover_owner_address(hash, sig, recovery_id)?;
        let namespace = recover_namespace(owner, &params.name);

        self.refresh_vmlist().await?;
        let vmlist = self.vmlist.clone();
        let new_next_port = params.port.into_par_iter()
            .zip(
                params.service_type.into_par_iter()
            ).map(|(port, service)| {
            let port = port.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            let mut next_port = self.next_port;
            let inner_namespace = namespace.clone();
            let inner_task_id = task_id.clone();
            let service: ServiceType = service.into();
            let publisher_uri = self.publisher.peer_addr()?;
            let inner_vmlist = vmlist.clone();
            let handle: JoinHandle<std::io::Result<VmmResult>> = tokio::spawn(
                async move {
                    let (owner, task_id, task_status) = update_iptables(
                        &publisher_uri,
                        inner_vmlist,
                        owner,
                        inner_namespace.clone(),
                        next_port,
                        service.clone(),
                        inner_task_id.clone(),
                        port 
                    ).await?;
                    Ok(VmmResult::UpdateIptables { owner, task_id, task_status })
                }
            );
            next_port += 1;
            self.handles.push(handle);
            Ok::<u16, std::io::Error>(next_port)
        }).filter_map(|res| {
            match res {
                Ok(n) => Some(n),
                _ => None
            }
        }).max().ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "all iterations failed"
            )
        )?;

        self.next_port = new_next_port;
        Ok(())
    }


    pub async fn launch_instance(
        &mut self,
        params: InstanceCreateParams,
        task_id: TaskId
    ) -> std::io::Result<()> {

        log::info!("Attempting to start instance...");
        let payload = params.into_payload(); 
        log::info!("converted params into payload...");
        let hash = get_payload_hash(payload.as_bytes());
        log::info!("hashed params payload...");
        let recovery_id = params.recovery_id.to_be_bytes()[3];
        log::info!("converted recovery_id into u32...");
        let owner = recover_owner_address(hash, params.sig.clone(), recovery_id)?;
        log::info!("recovered owner from signature...");
        let namespace = recover_namespace(owner, &params.name);
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
        let event_id = uuid::Uuid::new_v4();
        let state_event = StateEvent::PutAccount { 
            event_id: event_id.to_string(),
            task_id: task_id.clone(),
            task_status: TaskStatus::Pending,
            owner,
            vmlist: self.vmlist.clone(),
            namespace: namespace.clone(),
            exposed_ports: None 
        };

        self.publisher.publish(
            Box::new(StateTopic), 
            Box::new(state_event)
        ).await?;

        log::info!("published {} to topic {}", event_id.to_string(), StateTopic);
        let vminfo = self.vmlist.get(&namespace.inner()).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to find instance namespace in VM list"
            )
        )?;
        log::info!("acquired vminfo...");
        let event_id = uuid::Uuid::new_v4();
        let state_event = StateEvent::PutInstance { 
            event_id: event_id.to_string(), 
            task_id: task_id.clone(), 
            task_status: TaskStatus::Pending, 
            namespace: namespace.clone(), 
            vm_info: vminfo, 
            port_map: vec![(22u16, (self.next_port, ServiceType::Ssh))].into_iter().collect() 
        };

        self.publisher.publish(
            Box::new(StateTopic),
            Box::new(state_event)
        ).await?;

        log::info!("published event {} to topic {}", event_id.to_string(), StateTopic);

        Ok(())
    }

    pub async fn sync_instance(
        vmlist: VmList,
        namespace: String,
        path: String 
    ) -> std::io::Result<()> {
        log::info!("Attempting to sync instance {namespace}");
        todo!()
    }

    pub async fn move_instance(
        vmlist: VmList,
        namespace: String,
        path: String,
        _new_quorum: Option<String>,
    ) -> std::io::Result<()> {
        todo!()
    }
}

//OUTPUT HANDLERS
impl VmManager {
    async fn handle_create_output_and_response(
        &mut self,
        output: std::process::Output,
        task_id: TaskId,
        owner: [u8; 20],
        namespace: Namespace,
    ) -> std::io::Result<()> {
        self.refresh_vmlist().await?;
        let uri = self.publisher.peer_addr()?;
        let vmlist = self.vmlist.clone();
        let next_port = self.next_port;
        if output.status.success() {
            log::info!("successfully created instance, handling output...");
            let update_iptables_handle = Self::handle_create_output_success(
                task_id,
                owner,
                namespace,
                vmlist,
                next_port,
                uri
            ).await?;
            self.handles.push(update_iptables_handle);
            return Ok(())
        } else {
            log::error!("unable to create instance, handling output...");
            let launch_failure_handle = Self::handle_create_output_failure_and_response(
                output,
                owner,
                task_id,
                uri
            ).await?;
            self.handles.push(launch_failure_handle);
        }

        Ok(())
    }

    async fn handle_create_output_success(
        task_id: TaskId,
        owner: [u8; 20],
        namespace: Namespace,
        vmlist: VmList,
        next_port: u16,
        uri: String
    ) -> std::io::Result<JoinHandle<std::io::Result<VmmResult>>> {
        let handle = Self::handle_create_iptables_update(
            uri,
            vmlist,
            owner,
            namespace,
            next_port,
            task_id,
        ).await;

        return Ok(handle)
    }

    async fn handle_create_output_failure_and_response(
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId,
        uri: String
    ) -> std::io::Result<JoinHandle<std::io::Result<VmmResult>>> {
        let handle = tokio::spawn(async move {
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
                uri,
                owner,
                task_id.clone(), 
                TaskStatus::Failure(err_str.clone()),
            ).await?;
            log::info!("Updated task status from PENDING to FAILURE for task {task_id}");

            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    err_str
                )
            )
        });

        return Ok(handle)
    }


    async fn handle_start_output(
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId,
        uri: String,
    ) -> std::io::Result<()> {
        if output.status.success() {
            Self::handle_start_output_success(
                owner,
                task_id,
                uri
            ).await
        } else {
            Self::handle_start_output_failure(
                output,
                owner, 
                task_id,
                uri
            ).await
        }
    }

    async fn handle_start_output_success(
        owner: [u8; 20],
        task_id: TaskId,
        uri: String
    ) -> std::io::Result<()> {
        update_task_status(
            uri,
            owner,
            task_id,
            TaskStatus::Success,
        ).await
    }

    async fn handle_start_output_failure(
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId,
        uri: String,
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
            uri,
            owner,
            task_id,
            TaskStatus::Failure(
                err_string.to_string()
            ),
        ).await
    }


    async fn handle_expose_service_iptable_updates(
        &mut self,
        params: InstanceExposeServiceParams,
        namespace: Namespace,
        task_id: TaskId,
        owner: [u8; 20]
    ) -> std::io::Result<()> {
        for (port, service) in params.port.into_iter().zip(params.service_type.into_iter()) {
            let port = port.try_into().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            self.refresh_vmlist().await?;
            let vmlist = self.vmlist.clone();
            let next_port = self.next_port;
            let inner_namespace = namespace.clone();
            let inner_task_id = task_id.clone();
            let service: ServiceType = service.into();
            let publisher_uri = self.publisher.peer_addr()?;
            let handle: JoinHandle<std::io::Result<VmmResult>> = tokio::spawn(
                async move {
                    let (owner, task_id, task_status) = update_iptables(
                        &publisher_uri,
                        vmlist.clone(),
                        owner,
                        inner_namespace.clone(),
                        next_port,
                        service.clone(),
                        inner_task_id.clone(),
                        port 
                    ).await?;
                    Ok(VmmResult::UpdateIptables { 
                        owner,
                        task_id, 
                        task_status 
                    })
                }
            );
            self.next_port += 1;
            self.handles.push(handle);
        }

        Ok(())
    }

    async fn handle_create_iptables_update(
        publisher_uri: String,
        vmlist: VmList,
        owner: [u8; 20],
        namespace: Namespace,
        next_port: u16,
        task_id: TaskId
    ) -> JoinHandle<std::io::Result<VmmResult>> {
        tokio::spawn(
            async move {
                log::warn!("sleeping for 2 minutes to allow instance to fully launch...");
                tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
                log::info!("updating iptables...");
                let (owner, task_id, task_status) = update_iptables(
                    &publisher_uri,
                    vmlist,
                    owner,
                    namespace,
                    next_port, 
                    ServiceType::Ssh,
                    task_id.clone(),
                    22
                ).await?;
                Ok(VmmResult::UpdateIptables { owner, task_id, task_status })
        })
    }
}

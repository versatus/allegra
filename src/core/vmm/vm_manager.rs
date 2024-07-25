use std::pin::Pin;

use crate::{statics::*, LibrettoSubscriber, SyncInterval, VmmResult, VmmSubscriber};
use crate::{
    update_iptables,
    account::{
        Namespace, 
        TaskId, 
        TaskStatus
    }, event::{
        QuorumEvent, StateEvent
    }, helpers::{
        get_payload_hash, 
        recover_namespace, 
        recover_owner_address, 
        update_task_status
    }, params::{
        Payload, ServiceType
    }, publish::{
        GenericPublisher, QuorumTopic, StateTopic
    }, startup,
     vm_info::VmList, VmManagerMessage
};

use crate::allegra_rpc::{
    InstanceCreateParams,
    InstanceStopParams,
    InstanceStartParams,
    InstanceAddPubkeyParams,
    InstanceExposeServiceParams,
    InstanceDeleteParams,
};

use conductor::publisher::PubStream;
use futures::stream::{FuturesUnordered, StreamExt};
use futures::Future;
use conductor::subscriber::SubStream;
use libretto::pubsub::{LibrettoEvent, VmmAction};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use tokio::{task::JoinHandle, time::{interval,Duration}};
use uuid::Uuid;
use crate::consts::*;


pub struct VmManager {
    network: String,
    next_port: u16,
    handles: FuturesUnordered<JoinHandle<std::io::Result<VmmResult>>>,
    sync_futures: FuturesUnordered<Pin<Box<dyn Future<Output = std::io::Result<()>> + Send>>>,
    sync_intervals: FuturesUnordered<Pin<Box<dyn Future<Output = std::io::Result<SyncInterval>> + Send>>>,
    vmlist: VmList,
    publisher: GenericPublisher,
    pub subscriber: VmmSubscriber,
    pub fs_monitor: LibrettoSubscriber,
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
        let fs_monitor = LibrettoSubscriber::new("127.0.0.1:5556").await?;
        let sync_intervals = FuturesUnordered::new();

        Ok(Self {
            network: network.to_string(),
            next_port,
            handles,
            vmlist,
            sync_futures,
            subscriber,
            publisher,
            fs_monitor,
            sync_intervals
        })
    }

    pub async fn run(
        &mut self,
        stop_rx: &mut tokio::sync::mpsc::Receiver<()>
    ) -> std::io::Result<()> {
        log::info!("Starting VmManager");
        log::info!("Loading instances...");
        self.refresh_vmlist().await?;
        log::info!("Setting sync intervals...");

        let vm_list = self.vmlist.clone().vms().to_vec();

        for vm in vm_list {
            let namespace = vm.name();
            let sync_interval = SyncInterval {
                namespace: Namespace::new(namespace.clone()),
                interval: interval(Duration::from_secs(900)),
                tick_counter: 0,
                last_sync: None,
            };

            log::info!("Created sync interval for {:?}", namespace);
            self.sync_intervals.push(sync_interval.tick());
        }
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
                fs_events = self.fs_monitor.receive() => {
                    if let Ok(events) = fs_events {
                        log::info!("Filesystem event received");
                        for event in events {
                            match self.handle_fs_monitor_event(
                                event
                            ).await {
                                Err(e) => {
                                    log::error!("Error in fs_monitor: {e}")
                                }
                                _ => {}
                            }
                        }
                    }
                }
                stop = stop_rx.recv() => {
                    if let Some(_) = stop {
                        log::warn!("received stop token");
                        break
                    }
                },
                Some(vmm_result) = self.handles.next() => {
                    match vmm_result {
                        Ok(Ok(res)) => {
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
                        Err(e) => {
                            log::error!("error handling future {e}");
                        }
                        Ok(Err(e)) => {
                            log::error!("{e}");
                        }
                    }
                },
                Some(sync_status) = self.sync_futures.next() => {
                    match sync_status {
                        Ok(()) => {
                        }
                        Err(e) => {
                            log::error!("error handling future {e}");
                        }
                    }
                },
                Some(Ok(sync_interval)) = self.sync_intervals.next() => {
                    log::info!("Sync interval reached...");
                    let namespace = sync_interval.namespace();
                    let last_sync = sync_interval.last_sync();
                    let publisher_uri = self.publisher.peer_addr()?;
                    let publisher = GenericPublisher::new(&publisher_uri).await?;
                    let sync_future = Self::sync_instance_interval(
                        namespace.to_string(), publisher, last_sync.clone()
                    );
                    log::info!("Adding sync future to futures...");
                    self.sync_futures.push(Box::pin(sync_future));
                    log::info!("updating sync interval for {:?}", namespace);
                    self.sync_intervals.push(sync_interval.tick());
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
                params,
                task_id,
                ..
            } => {
                log::info!("received NewInstance message, attempting to launch instance.");
                return self.launch_instance(params, task_id).await
            }
            VmManagerMessage::StartInstance { 
                params, 
                sig, 
                task_id,
                ..
            } => {
                log::info!("received StartInstance message, attempting to start instance.");
                return self.start_instance(params, sig, task_id).await
            }
            VmManagerMessage::InjectAuth { 
                params, 
                sig, 
                task_id,
                ..
            } => {
                log::info!("received InjectAuth message, attempting to add auth to instance.");
                return self.inject_authorization(params, sig, task_id).await
            }
            VmManagerMessage::StopInstance { 
                params, 
                sig, 
                task_id,
                ..
            } => {
                log::info!("received StopInstance message, attempting to stop instance.");
                return self.stop_instance(params, sig, task_id).await
            }
            VmManagerMessage::DeleteInstance { 
                params, 
                sig, 
                task_id,
                ..
            } => {
                log::info!("received DeleteInstance message, attempting to delete instance.");
                return self.delete_instance(params, sig, task_id).await
            }
            VmManagerMessage::ExposeService { 
                params, 
                sig, 
                task_id,
                ..
            } => {
                log::info!("received ExposeService message, attempting to expose service on instance.");
                return self.expose_service(params, sig, task_id).await
            }
            VmManagerMessage::SyncInstance { namespace, .. } => {
                log::info!("received SyncInstance message, attempting to sync instance");
                let publisher_uri = self.publisher.peer_addr()?;
                let publisher = GenericPublisher::new(&publisher_uri).await?;
                let future = Box::pin(Self::sync_instance_interval(namespace, publisher, None));
                self.sync_futures.push(future);
                return Ok(())
            }
            VmManagerMessage::MigrateInstance { namespace, new_quorum, path, .. } => {
                log::info!("received MigrateInstance message, attempting to migrate instance");
                let vmlist = self.vmlist.clone();
                let future = Box::pin(Self::move_instance(vmlist, namespace, path, new_quorum));
                self.sync_futures.push(future);
                return Ok(())
            }
        }
    }

    async fn handle_fs_monitor_event(
        &mut self,
        event: LibrettoEvent,
    ) -> std::io::Result<()> {
        let action = event.action(); 
        match action {
            VmmAction::Copy => {
                self.refresh_vmlist().await?;
                let vmlist = self.vmlist.clone();
                let publisher_uri = self.publisher.peer_addr()?;
                let publisher = GenericPublisher::new(&publisher_uri).await?;
                let future = Box::pin(
                    Self::sync_instance_libretto_event(
                        vmlist,
                        event.instance_name().clone().ok_or(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Instance namespace not included in event"
                            )
                        )?,
                        event,
                        publisher
                    )
                );
                self.sync_futures.push(future);
            }
            VmmAction::Rollup => {
                log::info!("Received fs_event to rollup {:?}", event.instance_name());
            }
            VmmAction::Snapshot => {
                log::info!("Received fs_event to snapshot {:?}", event.instance_name());
            }
            _ => todo!()
        }

        Ok(())
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
        let new_next_port = self.handle_expose_service_iptable_updates(
            params, namespace, task_id, owner
        ).await?; 
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

        // lxc launch e, -n self.network
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
            port_map: vec![(22u16, (self.next_port, ServiceType::Ssh))].into_iter().collect(),
            last_snapshot: None,
            last_sync: None
        };

        self.publisher.publish(
            Box::new(StateTopic),
            Box::new(state_event)
        ).await?;

        log::info!("published event {} to topic {}", event_id.to_string(), StateTopic);

        let sync_interval = SyncInterval {
            namespace: namespace.clone(),
            interval: interval(Duration::from_secs(900)), 
            tick_counter: 0,
            last_sync: None,
        };

        self.sync_intervals.push(sync_interval.tick());

        Ok(())
    }

    pub async fn sync_instance_interval(
        namespace: String,
        mut publisher: GenericPublisher,
        last_sync: Option<u64>
    ) -> std::io::Result<()> {
        log::info!("Attempting to sync instance {namespace}");
        let event_id = Uuid::new_v4().to_string();
        let task_id = TaskId::new(Uuid::new_v4().to_string());
        let quorum_event = QuorumEvent::SyncInstanceInterval { 
            event_id,
            task_id,
            namespace: Namespace::new(namespace),
            last_sync 
        }; 
        publisher.publish(
            Box::new(QuorumTopic),
            Box::new(quorum_event)
        ).await?;

        Ok(())
    }

    pub async fn sync_instance_libretto_event(
        _vmlist: VmList,
        namespace: String,
        event: LibrettoEvent,
        mut publisher: GenericPublisher
    ) -> std::io::Result<()> {
        log::info!("Attempting to sync instance {namespace}");
        let event_id = Uuid::new_v4().to_string();
        let task_id = TaskId::new(Uuid::new_v4().to_string());
        let quorum_event = QuorumEvent::SyncInstanceEvent { event_id, task_id, namespace: Namespace::new(namespace), event }; 
        publisher.publish(
            Box::new(QuorumTopic),
            Box::new(quorum_event)
        ).await?;
        
        Ok(())
    }

    pub async fn move_instance(
        _vmlist: VmList,
        _namespace: String,
        _path: String,
        _new_quorum: Option<String>,
    ) -> std::io::Result<()> {
        todo!()
    }
}

//OUTPUT HANDLERS
impl VmManager {
    pub async fn handle_create_output_and_response(
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


    pub async fn handle_start_output(
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


    pub async fn handle_expose_service_iptable_updates(
        &mut self,
        params: InstanceExposeServiceParams,
        namespace: Namespace,
        task_id: TaskId,
        owner: [u8; 20]
    ) -> std::io::Result<u16> {
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

        Ok(new_next_port)
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

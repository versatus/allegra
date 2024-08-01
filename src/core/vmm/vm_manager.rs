use std::collections::{HashMap, HashSet};
use crate::distro::Distro;
use crate::event::QuorumEvent;
use crate::{get_image_name, get_image_path, statics::*, GeneralResponseSubscriber, GeneralResponseTopic, Instance, QuorumTopic, VirtInstall, VmInfo, VmmResult, VmmSubscriber};
use crate::{
    update_iptables,
    account::{
        Namespace, 
        TaskId, 
        TaskStatus
    }, event::StateEvent, helpers::{
        get_payload_hash, 
        recover_namespace, 
        recover_owner_address, 
        update_task_status
    }, params:: ServiceType, payload_impls::Payload, publish::{
        GenericPublisher, StateTopic
    },
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
use conductor::subscriber::SubStream;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use tokio::task::JoinHandle;
use uuid::Uuid;
use virt::connect::Connect;

pub struct VmManager {
    connection: Connect,
    // Use a struct for the virbr0
    #[allow(unused)]
    network: String,
    next_ip: [u8; 4],
    next_port: u16,
    handles: FuturesUnordered<JoinHandle<std::io::Result<VmmResult>>>,
    pending_launch: HashMap<Namespace, VirtInstall>,
    vmlist: VmList,
    publisher: GenericPublisher,
    pub subscriber: VmmSubscriber,
}

impl VmManager {
    pub async fn new(next_port: u16) -> std::io::Result<Self> {
        let network = DEFAULT_NETWORK.to_string();
        log::info!("set network interface to {}", &network);
        let mut connection = Connect::open(Some("qemu:///system"))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        log::info!("established connection to qemu");
        let handles = FuturesUnordered::new();
        log::info!("established FuturesUnordered handler");
        let mut publisher = GenericPublisher::new("127.0.0.1:5555").await?;
        let vmlist = match Self::get_vmlist(&mut connection, &mut publisher).await {
            Ok(vmlist) => vmlist,
            Err(_) => VmList { vms: HashMap::new() }
        };
        log::info!("acquired vm list");

        let subscriber = VmmSubscriber::new("127.0.0.1:5556").await?; 
        log::info!("instantiated VmmSubscriber, listening on 127.0.0.1:5556");
        log::info!("instantiated GenericPublisher publishing to 127.0.0.1:5555");
        log::info!("Returning VmManager");

        Ok(Self {
            connection,
            network: network.to_string(),
            next_ip: [192, 168, 122, 2], 
            next_port,
            pending_launch: HashMap::new(),
            handles,
            vmlist,
            subscriber,
            publisher,
        })
    }

    pub async fn run(
        &mut self,
        stop_rx: &mut tokio::sync::mpsc::Receiver<()>
    ) -> std::io::Result<()> {
        log::info!("Starting VmManager");
        log::info!("Loading instances...");
        self.refresh_vmlist().await?;

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
                                _ => { }
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
        log::info!("Attempting to refresh instances...");
        let domains = self.connection.list_all_domains(0)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        log::info!("acquired all domains: {:?}", domains);
        let vms: Vec<String> = domains.iter()
            .filter_map(|domain| {
                domain.get_name().ok()
            }).collect();

        log::info!("vms: {:?}", domains);
        let mut events = HashSet::new();

        let vm_info_vec: HashMap<Namespace, VmInfo> = {
            let mut subscriber = GeneralResponseSubscriber::new(
                &DEFAULT_SUBSCRIBER_ADDRESS,
                &GeneralResponseTopic::VmManagerResponseTopic.to_string()
            ).await?;

            for vm in vms {
                let event_id = Uuid::new_v4().to_string();
                let task_id  = TaskId::new(Uuid::new_v4().to_string());
                let task_status = TaskStatus::Pending;
                let namespace = Namespace::new(vm.clone());
                let response_topics = vec![GeneralResponseTopic::VmManagerResponseTopic]; 
                let event = StateEvent::GetInstance { event_id: event_id.clone(), task_id, task_status, namespace, response_topics };
                self.publisher.publish(Box::new(StateTopic), Box::new(event)).await?;
                events.insert(event_id);
            }


            let instances = tokio::time::timeout(
                tokio::time::Duration::from_secs(60),
                Self::batch_instance_response(&mut events, &mut subscriber)
            ).await??;

            instances
        };
        let vmlist = VmList { vms: vm_info_vec };
        log::info!("vm list refreshed saving to self.vmlist");
        self.vmlist = vmlist;

        Ok(())
    }

    pub async fn get_vmlist(
        connection: &mut Connect,
        publisher: &mut GenericPublisher,
        ) -> std::io::Result<VmList> {
        let domains = connection.list_all_domains(0)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        let vms: Vec<String> = domains.iter()
            .filter_map(|domain| {
                domain.get_name().ok()
            }).collect();

        let mut events = HashSet::new();

        let vm_info_vec: HashMap<Namespace, VmInfo> = {
            let mut subscriber = GeneralResponseSubscriber::new(
                &DEFAULT_SUBSCRIBER_ADDRESS,
                &GeneralResponseTopic::VmManagerResponseTopic.to_string()
            ).await?;

            for vm in vms {
                let event_id = Uuid::new_v4().to_string();
                let task_id  = TaskId::new(Uuid::new_v4().to_string());
                let task_status = TaskStatus::Pending;
                let namespace = Namespace::new(vm.clone());
                let response_topics = vec![
                    GeneralResponseTopic::VmManagerResponseTopic
                ]; 
                let event = StateEvent::GetInstance { 
                    event_id: event_id.clone(), 
                    task_id, 
                    task_status, 
                    namespace, 
                    response_topics 
                };
                publisher.publish(Box::new(StateTopic), Box::new(event)).await?;
                events.insert(event_id);
            }


            let instances = tokio::time::timeout(
                tokio::time::Duration::from_secs(60),
                Self::batch_instance_response(&mut events, &mut subscriber)
            ).await??;

            instances
        };
        let vmlist = VmList { vms: vm_info_vec };

        Ok(vmlist)
    }

    async fn batch_instance_response(
        event_ids: &mut HashSet<String>,
        subscriber: &mut GeneralResponseSubscriber
    ) -> std::io::Result<HashMap<Namespace, VmInfo>> {
        let mut vm_info_vec = vec![];
        while !event_ids.is_empty() {
            match subscriber.receive().await {
                Ok(messages) => {
                    for m in messages {
                        if event_ids.contains(m.original_event_id()) {
                            let instance: Instance = serde_json::from_str(
                                m.response()
                            )?;
                            vm_info_vec.push((
                                instance.namespace().clone(),
                                instance.vminfo().clone()
                            ));
                        }

                        event_ids.remove(m.original_event_id());
                    }
                }
                Err(e) => {
                    return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e
                        )
                    )
                }
            }
        }

        Ok(vm_info_vec.into_par_iter().collect())
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
                log::info!("received NewInstance message, setting up to launch instance.");
                self.refresh_vmlist().await?;
                let vmlist = self.vmlist.clone();
                let next_port = self.next_port.clone();
                let uri = self.publisher.peer_addr()?;
                let mut publisher = GenericPublisher::new(&uri).await?;
                let (namespace, virt_install) = Self::prepare_instance(
                    params,
                    task_id,
                    vmlist,
                    &mut publisher,
                    next_port
                ).await?;

                self.pending_launch.insert(namespace, virt_install);

                Ok(())
            }
            VmManagerMessage::LaunchInstance {
                namespace,
                task_id,
                ..
            } => {
                log::info!("received LaunchInstance message, attempting to launch instance.");
                self.refresh_vmlist().await?;
                let vmlist = self.vmlist.clone();
                let next_port = self.next_port.clone();
                let uri = self.publisher.peer_addr()?;
                let mut publisher = GenericPublisher::new(&uri).await?;
                let params = self.pending_launch.get(&namespace).ok_or(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Namespace unknown..."
                    )
                )?;

                return Self::launch_instance(
                    params,
                    namespace,
                    task_id,
                    vmlist, 
                    &mut publisher, 
                    next_port
                ).await
            }
            VmManagerMessage::StartInstance { 
                params, 
                sig, 
                task_id,
                ..
            } => {
                log::info!("received StartInstance message, attempting to start instance.");
                let uri = self.publisher.peer_addr()?;
                return Self::start_instance(params, sig, task_id, uri).await
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
                let uri = self.publisher.peer_addr()?;
                return Self::stop_instance(params, sig, task_id, uri).await
            }
            VmManagerMessage::DeleteInstance { 
                params, 
                sig, 
                task_id,
                ..
            } => {
                log::info!("received DeleteInstance message, attempting to delete instance.");
                let uri = self.publisher.peer_addr()?;
                return Self::delete_instance(params, sig, task_id, uri).await
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
            _ => {
                return Ok(())
            }
        }
    }

    async fn start_instance(
        params: InstanceStartParams,
        sig: String,
        task_id: TaskId,
        uri: String 
    ) -> std::io::Result<()> {
        let hash = get_payload_hash(params.into_payload().as_bytes());
        let owner = recover_owner_address(
            hash,
            sig,
            params.recovery_id
        )?;

        let namespace = recover_namespace(owner, &params.name);

        let connection = Connect::open(Some("qemu///system")).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let domain = virt::domain::Domain::lookup_by_name(&connection, &namespace.inner())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        if params.stateless {
            domain.create_with_flags(virt::sys::VIR_DOMAIN_START_PAUSED)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        } else {
            domain.create().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }

        update_task_status(uri, owner, task_id, TaskStatus::Success).await?;

        return Ok(())
    }

    async fn inject_authorization(
        &mut self,
        _params: InstanceAddPubkeyParams,
        _sig: String, 
        _task_id: TaskId,
    ) -> std::io::Result<()> {
        todo!()
    }

    async fn stop_instance(
        params: InstanceStopParams,
        sig: String,
        task_id: TaskId,
        uri: String
    ) -> std::io::Result<()> {
        let hash = get_payload_hash(params.into_payload().as_bytes());
        let owner = recover_owner_address(hash, sig, params.recovery_id)?;
        let namespace = recover_namespace(owner, &params.name);

        let connection = Connect::open(Some("qemu:///system")).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let domain = virt::domain::Domain::lookup_by_name(&connection, &namespace.inner())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        domain.shutdown().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        update_task_status(
            uri,
            owner,
            task_id,
            TaskStatus::Success
        ).await?;

        return Ok(())
    }

    async fn delete_instance(
        params: InstanceDeleteParams,
        sig: String,
        task_id: TaskId,
        uri: String
    ) -> std::io::Result<()> {
        let hash = get_payload_hash(params.into_payload().as_bytes());
        let owner = recover_owner_address(hash, sig, params.recovery_id)?;
        let namespace = recover_namespace(owner, &params.name.clone());

        let connection = Connect::open(Some("qemu:///system")).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let domain = virt::domain::Domain::lookup_by_name(&connection, &namespace.inner())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        if domain.is_active().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))? {
            if params.force {
                domain.destroy().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            } else {
                return Err(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Instance is still running. use force option to destroy it."
                    )
                )
            }
        }

        let flags = if params.force {
            virt::sys::VIR_DOMAIN_UNDEFINE_MANAGED_SAVE |
            virt::sys::VIR_DOMAIN_UNDEFINE_SNAPSHOTS_METADATA |
            virt::sys::VIR_DOMAIN_UNDEFINE_NVRAM
        } else {
            0
        };

        domain.undefine_flags(flags).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        update_task_status(
            uri, 
            owner, 
            task_id,
            TaskStatus::Success
        ).await?;

        Ok(())
    }

    async fn expose_service(
        &mut self,
        params: InstanceExposeServiceParams,
        sig: String,
        task_id: TaskId,
    ) -> std::io::Result<()> {
        let hash = get_payload_hash(params.into_payload().as_bytes());
        let owner = recover_owner_address(hash, sig, params.recovery_id)?;
        let namespace = recover_namespace(owner, &params.name);
        //TODO: replace with Nginx & HAProxy
        let new_next_port = self.handle_expose_service_iptable_updates(
            params, namespace, task_id, owner
        ).await?; 
        self.next_port = new_next_port;
        Ok(())
    }

    pub async fn prepare_instance(
        params: InstanceCreateParams,
        task_id: TaskId,
        vmlist: VmList,
        publisher: &mut GenericPublisher,
        next_port: u16
    ) -> std::io::Result<(Namespace, VirtInstall)> {
        log::info!("Attempting to start instance...");
        let payload = params.into_payload(); 
        log::info!("converted params into payload...");
        let hash = get_payload_hash(payload.as_bytes());
        log::info!("hashed params payload...");
        let owner = recover_owner_address(hash, params.sig.clone(), params.recovery_id)?;
        log::info!("recovered owner from signature...");
        let namespace = recover_namespace(owner, &params.name);
        log::info!("recovered namespace from name and owner...");

        let event_id = uuid::Uuid::new_v4();

        // Setup directory
        std::fs::create_dir_all(&format!("/mnt/glusterfs/vms/{}/brick", namespace.inner().to_string()))?;
        // Get image path
        let image_path = get_image_path(Distro::try_from(&params.distro)?, &params.version);
        let image_name = get_image_name(Distro::try_from(&params.distro)?, &params.version).await?;
        // Copy image
        let tmp_dest = format!("/mnt/tmp/images/{}-{}", params.distro, params.version);
        // Convert image
        std::process::Command::new("qemu-img")
            .arg("convert")
            .arg("-f")
            .arg("qcow2")
            .arg("-O")
            .arg("raw")
            .arg(&format!("{}", image_path.display()))
            .arg(&format!("{}/{}", tmp_dest, image_name))
            .output()?;

        // Setup loop device
        let loop_device_output = std::process::Command::new("losetup")
            .arg("-fP")
            .arg("--show")
            .arg(format!("{}{}", tmp_dest, image_name))
            .output()?;

        // Acquire loop device
        let loop_device = std::str::from_utf8(&loop_device_output.stdout).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?.to_string();

        // Create mountpoint directory
        std::fs::create_dir_all(format!("/mnt/tmp/iso/{}", namespace.inner().to_string()))?;
        // Mount loop device partition 1 to the mountpoint
        std::process::Command::new("mount")
            .arg(&format!("{}p1", loop_device)) // partition 1 of loop device
            .arg(&format!("/mnt/tmp/iso/{}", namespace.inner().to_string()))
            .output()?;

        // Move contents of copied disk image to the brick directory 
        std::process::Command::new("mv")
            .arg(&format!("/mnt/tmp/iso/{}", namespace.inner().to_string()))
            .arg(&format!("/mnt/glusterfs/vms/{}/brick/", namespace.inner().to_string()))
            .output()?;
        
        // Unmount the loop device from temporary mountpoint
        std::process::Command::new("umount")
            .arg(format!("/mnt/tmp/iso/{}", namespace.inner().to_string()))
            .output()?;

        // detach the disk image from the loop device
        std::process::Command::new("losetup")
            .arg("-d")
            .arg(&loop_device)
            .output()?;

        //TODO:(asmith) Cleanup temporary directories and mount points and disk devices
        let state_event = StateEvent::PutAccount { 
            event_id: event_id.to_string(),
            task_id: task_id.clone(),
            task_status: TaskStatus::Pending,
            owner,
            vmlist: vmlist.clone(),
            namespace: namespace.clone(),
            exposed_ports: None 
        };

        publisher.publish(
            Box::new(StateTopic), 
            Box::new(state_event)
        ).await?;

        log::info!("published {} to topic {}", event_id.to_string(), StateTopic);
        let vminfo = vmlist.get(&namespace.inner()).ok_or(
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
            vm_info: vminfo.clone(), 
            port_map: vec![(22u16, (next_port, ServiceType::Ssh))].into_iter().collect(),
            last_snapshot: None,
            last_sync: None
        };

        publisher.publish(
            Box::new(StateTopic),
            Box::new(state_event)
        ).await?;

        log::info!("published event {} to topic {}", event_id.to_string(), StateTopic);

        let virt_install: VirtInstall = params.into(); 

        // Inform peers we are prepared to setup glusterfs volume
        let event_id = uuid::Uuid::new_v4().to_string();
        let task_id = TaskId::new(uuid::Uuid::new_v4().to_string());
        let event = QuorumEvent::PreparedForLaunch {
            event_id,
            task_id,
            instance: namespace.clone(),
        };

        publisher.publish(
            Box::new(QuorumTopic),
            Box::new(event)
        ).await?;

        Ok((namespace, virt_install))
    }


    pub async fn launch_instance(
        virt_install: &VirtInstall,
        _namespace: Namespace,
        _task_id: TaskId,
        _vmlist: VmList,
        _publisher: &mut GenericPublisher,
        _next_port: u16
    ) -> std::io::Result<()> {
        virt_install.execute()?;
        //Update task status, etc. etc.
        log::info!("executed launch command...");
        Ok(())
    }
}

//OUTPUT HANDLERS
impl VmManager {
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
            let next_ip = format!("{}.{}.{}.{}", self.next_ip[0], self.next_ip[1], self.next_ip[2], self.next_ip[3]);
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
                        next_ip,
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

        self.next_ip[3] += 1;
        Ok(new_next_port)
    }
}

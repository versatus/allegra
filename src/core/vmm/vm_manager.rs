use crate::distro::{Alpine, Arch, Centos, Debian, Distro, DistroType, Fedora, Ubuntu};
use crate::event::QuorumEvent;
use crate::prepare::{alternative_prepare_disk_image, get_image_name, get_image_path, prepare_nfs_brick};
use crate::virt_install::{generate_cloud_init_files, CloudInit, UserData};
use crate::{
    account::{Namespace, TaskId, TaskStatus},
    event::StateEvent,
    helpers::{get_payload_hash, recover_namespace, recover_owner_address, update_task_status},
    params::ServiceType,
    payload_impls::Payload,
    publish::{GenericPublisher, StateTopic},
    update_iptables,
    vm_info::VmList,
    VmManagerMessage,
};
use crate::{
    statics::*, GeneralResponseSubscriber, GeneralResponseTopic,
    Instance, MemoryInfo, QuorumTopic, VCPUInfo, VirtInstall, VmInfo, VmInfoBuilder, VmmResult,
    VmmSubscriber,
};
use std::collections::{HashMap, HashSet};

use crate::allegra_rpc::{
    InstanceAddPubkeyParams, InstanceCreateParams, InstanceDeleteParams,
    InstanceExposeServiceParams, InstanceStartParams, InstanceStopParams,
};

use conductor::publisher::PubStream;
use conductor::subscriber::SubStream;
use futures::stream::{FuturesUnordered, StreamExt};
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
    pending_launch: HashMap<Namespace, (VirtInstall, String, u16)>,
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
            Err(_) => VmList {
                vms: HashMap::new(),
            },
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
        stop_rx: &mut tokio::sync::mpsc::Receiver<()>,
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

        return Ok(());
    }

    pub async fn refresh_vmlist(&mut self) -> std::io::Result<()> {
        log::info!("Attempting to refresh instances...");
        let domains = self
            .connection
            .list_all_domains(0)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        log::info!("acquired all domains: {:?}", domains);
        let vms: Vec<String> = domains
            .iter()
            .filter_map(|domain| domain.get_name().ok())
            .collect();

        log::info!("vms: {:?}", domains);
        let mut events = HashSet::new();

        let vm_info_vec: HashMap<Namespace, VmInfo> = {
            let mut subscriber = GeneralResponseSubscriber::new(
                &DEFAULT_SUBSCRIBER_ADDRESS,
                &GeneralResponseTopic::VmManagerResponseTopic.to_string(),
            )
            .await?;

            for vm in vms {
                let event_id = Uuid::new_v4().to_string();
                let task_id = TaskId::new(Uuid::new_v4().to_string());
                let task_status = TaskStatus::Pending;
                let namespace = Namespace::new(vm.clone());
                let response_topics = vec![GeneralResponseTopic::VmManagerResponseTopic];
                let event = StateEvent::GetInstance {
                    event_id: event_id.clone(),
                    task_id,
                    task_status,
                    namespace,
                    response_topics,
                };
                self.publisher
                    .publish(Box::new(StateTopic), Box::new(event))
                    .await?;
                events.insert(event_id);
            }

            let instances = tokio::time::timeout(
                tokio::time::Duration::from_secs(60),
                Self::batch_instance_response(&mut events, &mut subscriber),
            )
            .await??;

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
        let domains = connection
            .list_all_domains(0)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let vms: Vec<String> = domains
            .iter()
            .filter_map(|domain| domain.get_name().ok())
            .collect();

        let mut events = HashSet::new();

        let vm_info_vec: HashMap<Namespace, VmInfo> = {
            let mut subscriber = GeneralResponseSubscriber::new(
                &DEFAULT_SUBSCRIBER_ADDRESS,
                &GeneralResponseTopic::VmManagerResponseTopic.to_string(),
            )
            .await?;

            for vm in vms {
                let event_id = Uuid::new_v4().to_string();
                let task_id = TaskId::new(Uuid::new_v4().to_string());
                let task_status = TaskStatus::Pending;
                let namespace = Namespace::new(vm.clone());
                let response_topics = vec![GeneralResponseTopic::VmManagerResponseTopic];
                let event = StateEvent::GetInstance {
                    event_id: event_id.clone(),
                    task_id,
                    task_status,
                    namespace,
                    response_topics,
                };
                publisher
                    .publish(Box::new(StateTopic), Box::new(event))
                    .await?;
                events.insert(event_id);
            }

            let instances = tokio::time::timeout(
                tokio::time::Duration::from_secs(60),
                Self::batch_instance_response(&mut events, &mut subscriber),
            )
            .await??;

            instances
        };
        let vmlist = VmList { vms: vm_info_vec };

        Ok(vmlist)
    }

    async fn batch_instance_response(
        event_ids: &mut HashSet<String>,
        subscriber: &mut GeneralResponseSubscriber,
    ) -> std::io::Result<HashMap<Namespace, VmInfo>> {
        let mut vm_info_vec = vec![];
        while !event_ids.is_empty() {
            match subscriber.receive().await {
                Ok(messages) => {
                    for m in messages {
                        if event_ids.contains(m.original_event_id()) {
                            let instance: Instance = serde_json::from_str(m.response())?;
                            vm_info_vec
                                .push((instance.namespace().clone(), instance.vminfo().clone()));
                        }

                        event_ids.remove(m.original_event_id());
                    }
                }
                Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
            }
        }

        Ok(vm_info_vec.into_par_iter().collect())
    }
}

// Core Handlers
impl VmManager {
    async fn handle_vmm_message(&mut self, message: VmManagerMessage) -> std::io::Result<()> {
        match message {
            VmManagerMessage::NewInstance {
                params, task_id, ..
            } => {
                log::info!("received NewInstance message, setting up to launch instance.");
                match self.refresh_vmlist().await {
                    Err(e) => log::error!("Error in self.refresh_vmlist(): {e}"),
                    _ => {}
                };
                let vmlist = self.vmlist.clone();
                let next_port = self.next_port.clone();
                let uri = self.publisher.peer_addr()?;
                let mut publisher = GenericPublisher::new(&uri).await?;
                let next_ip = format!(
                    "{}.{}.{}.{}",
                    self.next_ip[0], self.next_ip[1], self.next_ip[2], self.next_ip[3]
                );
                let (namespace, virt_install, use_disk, next_port) = Self::prepare_instance(
                    params,
                    task_id,
                    vmlist,
                    &mut publisher,
                    next_ip,
                    next_port,
                )
                .await?;

                self.next_ip[3] += 1;
                self.pending_launch
                    .insert(namespace, (virt_install, use_disk, next_port));

                Ok(())
            }
            VmManagerMessage::LaunchInstance { namespace, .. } => {
                log::info!("received LaunchInstance message, attempting to launch instance.");
                self.refresh_vmlist().await?;
                let uri = self.publisher.peer_addr()?;
                let mut _publisher = GenericPublisher::new(&uri).await?;
                let (virt_install, use_disk, next_port) =
                    self.pending_launch
                        .get(&namespace)
                        .ok_or(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Namespace unknown...",
                        ))?;
                let uri = self.publisher.peer_addr()?;

                return Self::launch_instance(
                    &self.connection,
                    virt_install,
                    &namespace,
                    *next_port,
                    uri,
                    &use_disk
                )
                .await;
            }
            VmManagerMessage::StartInstance {
                params,
                sig,
                task_id,
                ..
            } => {
                log::info!("received StartInstance message, attempting to start instance.");
                let uri = self.publisher.peer_addr()?;
                return Self::start_instance(params, sig, task_id, uri).await;
            }
            VmManagerMessage::InjectAuth {
                params,
                sig,
                task_id,
                ..
            } => {
                log::info!("received InjectAuth message, attempting to add auth to instance.");
                return self.inject_authorization(params, sig, task_id).await;
            }
            VmManagerMessage::StopInstance {
                params,
                sig,
                task_id,
                ..
            } => {
                log::info!("received StopInstance message, attempting to stop instance.");
                let uri = self.publisher.peer_addr()?;
                return Self::stop_instance(params, sig, task_id, uri).await;
            }
            VmManagerMessage::DeleteInstance {
                params,
                sig,
                task_id,
                ..
            } => {
                log::info!("received DeleteInstance message, attempting to delete instance.");
                let uri = self.publisher.peer_addr()?;
                return Self::delete_instance(params, sig, task_id, uri).await;
            }
            VmManagerMessage::ExposeService {
                params,
                sig,
                task_id,
                ..
            } => {
                log::info!(
                    "received ExposeService message, attempting to expose service on instance."
                );
                return self.expose_service(params, sig, task_id).await;
            }
            _ => return Ok(()),
        }
    }

    async fn start_instance(
        params: InstanceStartParams,
        sig: String,
        task_id: TaskId,
        uri: String,
    ) -> std::io::Result<()> {
        let hash = get_payload_hash(params.into_payload().as_bytes());
        let owner = recover_owner_address(hash, sig, params.recovery_id)?;

        let namespace = recover_namespace(owner, &params.name);

        let connection = Connect::open(Some("qemu///system"))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let domain = virt::domain::Domain::lookup_by_name(&connection, &namespace.inner())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        if params.stateless {
            domain
                .create_with_flags(virt::sys::VIR_DOMAIN_START_PAUSED)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        } else {
            domain
                .create()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }

        update_task_status(uri, owner, task_id, TaskStatus::Success).await?;

        return Ok(());
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
        uri: String,
    ) -> std::io::Result<()> {
        let hash = get_payload_hash(params.into_payload().as_bytes());
        let owner = recover_owner_address(hash, sig, params.recovery_id)?;
        let namespace = recover_namespace(owner, &params.name);

        let connection = Connect::open(Some("qemu:///system"))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let domain = virt::domain::Domain::lookup_by_name(&connection, &namespace.inner())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        domain
            .shutdown()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        update_task_status(uri, owner, task_id, TaskStatus::Success).await?;

        return Ok(());
    }

    async fn delete_instance(
        params: InstanceDeleteParams,
        sig: String,
        task_id: TaskId,
        uri: String,
    ) -> std::io::Result<()> {
        let hash = get_payload_hash(params.into_payload().as_bytes());
        let owner = recover_owner_address(hash, sig, params.recovery_id)?;
        let namespace = recover_namespace(owner, &params.name.clone());

        let connection = Connect::open(Some("qemu:///system"))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let domain = virt::domain::Domain::lookup_by_name(&connection, &namespace.inner())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        if domain
            .is_active()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        {
            if params.force {
                domain
                    .destroy()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Instance is still running. use force option to destroy it.",
                ));
            }
        }

        let flags = if params.force {
            virt::sys::VIR_DOMAIN_UNDEFINE_MANAGED_SAVE
                | virt::sys::VIR_DOMAIN_UNDEFINE_SNAPSHOTS_METADATA
                | virt::sys::VIR_DOMAIN_UNDEFINE_NVRAM
        } else {
            0
        };

        domain
            .undefine_flags(flags)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        update_task_status(uri, owner, task_id, TaskStatus::Success).await?;

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
        let new_next_port = self
            .handle_expose_service_iptable_updates(params, namespace, task_id, owner)
            .await?;
        self.next_port = new_next_port;
        Ok(())
    }

    pub async fn prepare_instance(
        params: InstanceCreateParams,
        task_id: TaskId,
        vmlist: VmList,
        publisher: &mut GenericPublisher,
        next_ip: String,
        next_port: u16,
    ) -> std::io::Result<(Namespace, VirtInstall, String, u16)> {
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
        prepare_nfs_brick(&namespace)?;
        log::info!("created dir for instance...");

        // Get image path
        let image_path = get_image_path(Distro::try_from(&params.distro)?);
        log::info!("acquired image path for {}: {}...", params.distro, image_path.display());
        let image_name = get_image_name(Distro::try_from(&params.distro)?).await?;
        log::info!("acquired image name for {}: {}...", params.distro, image_name);

        let disk_source = image_path.display().to_string();
        let raw_tmp_dest = format!("/mnt/images/{}/{}", namespace.inner().to_string(), "raw_tmp.img");
        let use_from = format!("/var/lib/libvirt/images/{}-{}/{}/{}", Distro::try_from(&params.distro)?, params.version, namespace.inner().to_string(), image_name);
        log::info!("attempting to prepare disk_image...");
        log::warn!("DISK SOURCE = {}", &disk_source);
        log::warn!("RAW TMP DESTINATION = {}", &raw_tmp_dest);
        log::warn!("USE DISK FROM = {}", &use_from);
        alternative_prepare_disk_image(
            &disk_source,
            &raw_tmp_dest,
            &use_from,
            &namespace
        )?;

        //TODO:(asmith) Cleanup temporary directories and mount points and disk devices
        let state_event = StateEvent::PutAccount {
            event_id: event_id.to_string(),
            task_id: task_id.clone(),
            task_status: TaskStatus::Pending,
            owner,
            vmlist: vmlist.clone(),
            namespace: namespace.clone(),
            exposed_ports: None,
        };
        log::info!("requesting account be put in state db");

        publisher
            .publish(Box::new(StateTopic), Box::new(state_event))
            .await?;

        log::info!("published event to state topic");

        log::info!("published {} to topic {}", event_id.to_string(), StateTopic);
        let virt_install: VirtInstall = params.clone().into();

        if let Some(cloud_init) = params.cloud_init {
            let cloud_init: CloudInit = cloud_init.into();
            if let Some(user_data) = cloud_init.user_data {
                let distro: Distro = params.distro.into();
                match distro {
                    Distro::Ubuntu => {
                        if let Ok(user_provided) = serde_yml::from_str::<UserData<Ubuntu>>(&user_data) {
                            generate_cloud_init_files(
                                &namespace.inner().to_string(),
                                &namespace.inner().to_string(),
                                Some(user_provided),
                                &next_ip,
                            )?;
                        } 
                    }
                    Distro::CentOS => {
                        if let Ok(user_provided) = serde_yml::from_str::<UserData<Centos>>(&user_data) {
                            generate_cloud_init_files(
                                &namespace.inner().to_string(),
                                &namespace.inner().to_string(),
                                Some(user_provided),
                                &next_ip,
                            )?;
                        }
                    }
                    Distro::Fedora => {
                        if let Ok(user_provided) = serde_yml::from_str::<UserData<Fedora>>(&user_data) {
                            generate_cloud_init_files(
                                &namespace.inner().to_string(),
                                &namespace.inner().to_string(),
                                Some(user_provided),
                                &next_ip,
                            )?;
                        } 
                    }
                    Distro::Debian => {
                        if let Ok(user_provided) = serde_yml::from_str::<UserData<Debian>>(&user_data) {
                            generate_cloud_init_files(
                                &namespace.inner().to_string(),
                                &namespace.inner().to_string(),
                                Some(user_provided),
                                &next_ip,
                            )?;
                        }
                    }
                    Distro::Arch => {
                        if let Ok(user_provided) = serde_yml::from_str::<UserData<Arch>>(&user_data) {
                            generate_cloud_init_files(
                                &namespace.inner().to_string(),
                                &namespace.inner().to_string(),
                                Some(user_provided),
                                &next_ip,
                            )?;
                        }
                    }
                    Distro::Alpine => {
                        if let Ok(user_provided) = serde_yml::from_str::<UserData<Alpine>>(&user_data) {
                            generate_cloud_init_files(
                                &namespace.inner().to_string(),
                                &namespace.inner().to_string(),
                                Some(user_provided),
                                &next_ip,
                            )?;
                        }
                    }
                    _ => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Other distros not yet supported"))
                }
            } 
        } 

        // Inform peers we are prepared to setup glusterfs volume
        let uri = publisher.peer_addr()?;
        update_iptables(
            &uri,
            vmlist,
            owner,
            namespace.clone(),
            next_port,
            ServiceType::Ssh,
            task_id,
            next_ip,
            next_port,
        )
        .await?;

        let event_id = uuid::Uuid::new_v4().to_string();
        let task_id = TaskId::new(uuid::Uuid::new_v4().to_string());
        let event = QuorumEvent::PreparedForLaunch {
            event_id,
            task_id: task_id.clone(),
            instance: namespace.clone(),
        };

        publisher
            .publish(Box::new(QuorumTopic), Box::new(event))
            .await?;

        Ok((namespace, virt_install, use_from, next_port))
    }

    pub async fn launch_instance(
        conn: &Connect,
        virt_install: &VirtInstall,
        namespace: &Namespace,
        next_port: u16,
        uri: String,
        use_disk: &str,
    ) -> std::io::Result<()> {
        let output = virt_install.execute(namespace, use_disk)?;

        if output.status.success() {

            //Update task status, etc. etc.
            log::info!("executed launch command...");

            let domain = virt::domain::Domain::lookup_by_name(&conn, &namespace.inner().to_string())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            let info = domain
                .get_info()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            let (state, state_reason) = domain
                .get_state()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let vm_info = VmInfoBuilder::default()
                .name(namespace.inner().to_string())
                .uuid(
                    domain
                        .get_uuid_string()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
                )
                .id(domain.get_id())
                .state(state.into())
                .state_reason(state_reason)
                .memory(MemoryInfo::new(
                    info.memory,
                    domain
                        .get_max_memory()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
                    domain
                        .get_max_memory()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
                    0,
                    0,
                ))
                .vcpus(VCPUInfo::new(
                    info.nr_virt_cpu,
                    domain
                        .get_max_vcpus()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
                        .try_into()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
                ))
                .cpu_time(info.cpu_time)
                .autostart(
                    domain
                        .get_autostart()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
                )
                .persistent(true)
                .os_type(
                    domain
                        .get_os_type()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
                )
                .os_arch("x86_64".to_string())
                .network_interfaces(Vec::new())
                .storage_volumes(Vec::new())
                .block_stats(HashMap::new())
                .interface_stats(HashMap::new())
                .snapshots(Vec::new())
                .build()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            let task_id = TaskId::new(Uuid::new_v4().to_string());
            log::info!("acquired vminfo...");
            let event_id = uuid::Uuid::new_v4();
            let state_event = StateEvent::PutInstance {
                event_id: event_id.to_string(),
                task_id: task_id.clone(),
                task_status: TaskStatus::Success,
                namespace: namespace.clone(),
                vm_info: vm_info.clone(),
                port_map: vec![(22u16, (next_port, ServiceType::Ssh))]
                    .into_iter()
                    .collect(),
                last_snapshot: None,
                last_sync: None,
            };

            let mut publisher = GenericPublisher::new(&uri).await?;
            publisher
                .publish(Box::new(StateTopic), Box::new(state_event))
                .await?;

            log::info!(
                "published event {} to topic {}",
                event_id.to_string(),
                StateTopic
            );
        } else {
            let err_str = std::str::from_utf8(&output.stderr).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, e)
            })?;

            return Err(std::io::Error::new(std::io::ErrorKind::Other, err_str))
        }

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
        owner: [u8; 20],
    ) -> std::io::Result<u16> {
        self.refresh_vmlist().await?;
        let vmlist = self.vmlist.clone();
        let new_next_port = params
            .port
            .into_par_iter()
            .zip(params.service_type.into_par_iter())
            .map(|(port, service)| {
                let port = port
                    .try_into()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                let mut next_port = self.next_port;
                let inner_namespace = namespace.clone();
                let inner_task_id = task_id.clone();
                let service: ServiceType = service.into();
                let publisher_uri = self.publisher.peer_addr()?;
                let inner_vmlist = vmlist.clone();
                let next_ip = format!(
                    "{}.{}.{}.{}",
                    self.next_ip[0], self.next_ip[1], self.next_ip[2], self.next_ip[3]
                );
                let handle: JoinHandle<std::io::Result<VmmResult>> = tokio::spawn(async move {
                    let (owner, task_id, task_status) = update_iptables(
                        &publisher_uri,
                        inner_vmlist,
                        owner,
                        inner_namespace.clone(),
                        next_port,
                        service.clone(),
                        inner_task_id.clone(),
                        next_ip,
                        port,
                    )
                    .await?;
                    Ok(VmmResult::UpdateIptables {
                        owner,
                        task_id,
                        task_status,
                    })
                });
                next_port += 1;
                self.handles.push(handle);
                Ok::<u16, std::io::Error>(next_port)
            })
            .filter_map(|res| match res {
                Ok(n) => Some(n),
                _ => None,
            })
            .max()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "all iterations failed",
            ))?;

        self.next_ip[3] += 1;
        Ok(new_next_port)
    }
}

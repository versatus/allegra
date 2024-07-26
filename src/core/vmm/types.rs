use std::collections::HashMap;
use crate::{
    account::{
        Namespace, 
        TaskId, 
        TaskStatus
    },  event::VmmEvent,
    params::ServiceType,
    vm_info::VmInfo
};

use crate::allegra_rpc::{
    InstanceCreateParams,
    InstanceStopParams,
    InstanceStartParams,
    InstanceAddPubkeyParams,
    InstanceExposeServiceParams,
    InstanceDeleteParams,
};

use rayon::iter::{
    IntoParallelIterator, 
    IntoParallelRefIterator, 
    ParallelExtend, 
    ParallelIterator
};
use tokio::time::Interval;
use serde::{Serialize, Deserialize};
use getset::Getters;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    namespace: Namespace,
    vminfo: VmInfo,
    //(Internal Port, (Host Port, Service))
    port_map: HashMap<u16, (u16, ServiceType)>,
    last_snapshot: Option<u64>,
    last_sync: Option<u64>,
    // Add Nginx
    // Add other metadata like quorum owned, access trie, etc.
}

#[derive(Getters)]
#[getset(get = "pub")]
pub struct SyncInterval {
    pub interval: Interval,
    pub namespace: Namespace,
    pub tick_counter: u64,
    pub last_sync: Option<u64>,
}

impl SyncInterval {
    #[async_recursion::async_recursion]
    pub async fn tick(mut self) -> std::io::Result<Self> {
        self.interval.tick().await;
        self.tick_counter += 1;

        if self.tick_counter <= 1 {
            self = self.tick().await?;
        }

        return Ok(self)
    }
}

impl Instance {
    pub fn new(
        namespace: Namespace,
        vminfo: VmInfo,
        port_map: impl IntoParallelIterator<Item = (u16, (u16, ServiceType))>,
        last_snapshot: Option<u64>,
        last_sync: Option<u64>
    ) -> Self {
        Self {
            namespace,
            vminfo,
            port_map: port_map.into_par_iter().collect(),
            last_snapshot,
            last_sync,
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
    
    pub fn update_last_snapshot(&mut self, last_snapshot: Option<u64>) {
        self.last_snapshot = last_snapshot;
    }

    pub fn update_last_sync(&mut self, last_sync: Option<u64>) {
        self.last_sync = last_sync;
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
                recovery_id,
                sync
            } => {
                let params = InstanceCreateParams {
                    name, 
                    distro, 
                    version, 
                    vmtype: vmtype.to_string(),
                    sig, 
                    recovery_id: recovery_id.into(),
                    sync,
                    ..Default::default()
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

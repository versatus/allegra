use lru::LruCache;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use tikv_client::{
    TransactionClient,
    RawClient,
    Result,
    Config,
    Key,
    Value, 
    TransactionOptions, 
    Timestamp,
    Snapshot,
    ColumnFamily,
    Backoff,
    KvPair,
    BoundRange,
    proto::kvrpcpb::{Mutation, Op}
};
use uuid::Uuid;

use std::{collections::HashMap, sync::Arc};
use std::num::NonZeroUsize;
use tokio::sync::{Mutex, RwLock};
use crate::{
    account::{
        Account, 
        ExposedPort, 
        Namespace, 
        TaskId, 
        TaskStatus
    }, event::{
        GeneralResponseEvent, 
        StateEvent, 
        StateValueType, TaskStatusEvent
    }, params::ServiceType,
    publish::{GenericPublisher, StateTopic}, 
    subscribe::{
        StateSubscriber, 
        TaskStatusSubscriber
    }, vm_info::{
        VmInfo,
        VmList
    }, vmm::Instance
};
use conductor::{publisher::PubStream, subscriber::SubStream};
use getset::{Getters, MutGetters};

#[derive(Getters, MutGetters)]
#[getset(get = "pub", get_mut)]
pub struct StateManager {
    writer: StateWriter,
    reader: StateReader,
    task_cache: Arc<RwLock<LruCache<TaskId, TaskStatus>>>,
    account_cache: Arc<RwLock<LruCache<[u8; 20], Account>>>,
    instance_cache: Arc<RwLock<LruCache<Namespace, Instance>>>,
    state_subscriber: StateSubscriber,
    task_subscriber: TaskStatusSubscriber,
    publisher: Arc<Mutex<GenericPublisher>>
}

impl StateManager {
    pub async fn new(pd_endpoints: Vec<String>, subscriber_uri: &str, publisher_uri: &str) -> std::io::Result<Self> {
        let writer = StateWriter::new(pd_endpoints.clone()).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        log::info!("Created StateWriter...");
        let reader = StateReader::new(pd_endpoints.clone()).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        log::info!("Created StateReader...");
        let state_subscriber = StateSubscriber::new(subscriber_uri).await?;
        log::info!("Created StateSubscriber...");
        let task_subscriber = TaskStatusSubscriber::new(subscriber_uri).await?;
        log::info!("Created TaskStatusSubscriber...");
        let publisher = Arc::new(Mutex::new(GenericPublisher::new(publisher_uri).await?));
        log::info!("Created GenericPublisher...");
        let task_cache = Arc::new(
            RwLock::new(
                LruCache::new(
                    NonZeroUsize::new(1000).ok_or(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Invalid NonZeroUsize"
                        )
                    )?
                )
            )
        );
        log::info!("Created TaskCache...");
        let account_cache = Arc::new(
            RwLock::new(
                LruCache::new(
                    NonZeroUsize::new(
                        1000
                    ).ok_or(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Invalid NonZeroUsize"
                        )
                    )?
                )
            )
        );
        log::info!("Created AccountCache...");
        let instance_cache = Arc::new(
            RwLock::new(
                LruCache::new(
                    NonZeroUsize::new(
                        1000
                    ).ok_or(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Invalid NonZeroUsize"
                        )
                    )?
                )
            )
        );
        log::info!("Created InstanceCache...");

        Ok(Self {
            writer,
            reader,
            task_cache,
            account_cache,
            instance_cache,
            state_subscriber,
            task_subscriber,
            publisher
        })
    }

    pub async fn run(&mut self) -> std::io::Result<()> {
        loop {
            tokio::select! {
                Ok(message) = self.state_subscriber.receive() => {
                    for m in message {
                        log::info!("Received state event...");
                        if let Err(e) = self.handle_state_event(m).await {
                            log::info!("ERROR: self.handle_state_event: {e}");
                        }
                    }
                }
                Ok(message) = self.task_subscriber.receive() => {
                    for m in message {
                        log::info!("Received task_status event...");
                        if let Err(e) = self.handle_task_event(m).await{ 
                            log::info!("ERROR: self.handle_state_event: {e}");
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    break
                }
            }
        }

        Ok(())
    }

    async fn update_or_create_account(
        &mut self, 
        task_id: TaskId,
        task_status: TaskStatus,
        owner: [u8; 20],
        vmlist: VmList,
        namespace: Namespace,
        exposed_ports: Option<Vec<ExposedPort>>
    ) -> std::io::Result<Account> {
        if let Some(account_bytes) = self.reader_mut().get(
            owner.to_vec()
        ).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })? {
            // Account already exists, add to it
            let mut account: Account = serde_json::from_slice(
                &account_bytes
            ).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;

            let info = vmlist.get(&namespace.to_string());
            account.update_namespace(&namespace, info.clone());
            account.update_task_status(&task_id, task_status);
            if let Some(ep) = exposed_ports {
                account.update_exposed_ports(&namespace, ep);
            }
            Ok(account)
        } else {
            let exposed_ports = if let Some(ep) = exposed_ports {
                vec![(namespace.clone(), ep.clone())]
            } else {
                vec![]
            };

            let info = vmlist.get(&namespace.to_string());
            let account = Account::new(
                owner,
                vec![(namespace, info.cloned())], 
                exposed_ports, 
                vec![(task_id, task_status)]
            );
            Ok(account)
        }
    }

    async fn update_or_create_instance(
        &mut self,
        namespace: Namespace,
        vminfo: VmInfo,
        port_map: HashMap<u16, (u16, ServiceType)>,
        last_snapshot: Option<u64>,
        last_sync: Option<u64>,
    ) -> std::io::Result<Instance> {
        let port_map_iter: Vec<(u16, (u16,  ServiceType))> = port_map.par_iter()
            .map(|(x, (y, st))| {
                (*x, (*y, st.clone().into()))
            }).collect();
        if let Some(instance_bytes) = self.reader_mut().get(
            serde_json::to_vec(&namespace).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?
        ).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })? {
            let mut instance: Instance = serde_json::from_slice(&instance_bytes).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;

            instance.update_vminfo(vminfo);
            instance.extend_port_mapping(port_map_iter.par_iter().cloned());
            instance.update_last_snapshot(last_snapshot);
            instance.update_last_sync(last_sync);

            Ok(instance)
        } else {
            let instance = Instance::new(
                namespace, 
                vminfo, 
                port_map, 
                last_snapshot, 
                last_sync
            );

            Ok(instance)
        }
    }

    async fn handle_state_event(&mut self, m: StateEvent) -> std::io::Result<()> {
        #[allow(unused)]
        match m {
            StateEvent::Put { event_id, task_id, key, value } => {
                self.writer_mut().put_optimistic(key, value).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?;
            },
            StateEvent::PutAccount { 
                event_id, 
                task_id, 
                task_status, 
                owner, 
                vmlist, 
                namespace, 
                exposed_ports 
            } => {

                let account = self.update_or_create_account(
                    task_id,
                    task_status, 
                    owner, 
                    vmlist, 
                    namespace, 
                    exposed_ports
                ).await?; 

                self.writer_mut()
                    .put_optimistic(
                        owner.to_vec(),
                        serde_json::to_vec(&account)?
                    ).await.map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e
                        )
                    })?;
            }
            StateEvent::PutInstance { 
                event_id, 
                task_id, 
                task_status, 
                namespace, 
                vm_info, 
                port_map,
                last_snapshot,
                last_sync,
            } => {
                let instance = self.update_or_create_instance(
                    namespace.clone(),
                    vm_info,
                    port_map,
                    last_snapshot,
                    last_sync
                ).await?; 

                self.writer_mut()
                    .put_optimistic(
                        serde_json::to_vec(&namespace)?,
                        serde_json::to_vec(&instance)?
                    ).await.map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e
                        )
                    })?;
            }
            StateEvent::PutTaskStatus { event_id, task_id, task_status } => {
                self.writer_mut()
                    .put_optimistic(
                        serde_json::to_vec(&task_id)?, 
                        serde_json::to_vec(&task_status)? 
                    ).await.map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e
                        )
                    })?;
            }
            StateEvent::Get { event_id, task_id, key, response_topics, expected_type } => {
                let data_bytes = self.reader_mut().get(key.clone()).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?.ok_or(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("unable to find key 0x{} in state db", hex::encode(&key))
                    )
                )?;

                let response_event_id = Uuid::new_v4().to_string();
                let response_event = match expected_type {
                    StateValueType::Account => {
                        let account: Account = serde_json::from_slice(
                            &data_bytes
                        )?;
                        GeneralResponseEvent::new(
                           response_event_id,
                           event_id.clone(),
                           serde_json::to_string(&account)?
                        )
                    }
                    StateValueType::TaskStatus => {
                        let task_status: TaskStatus = serde_json::from_slice(
                            &data_bytes
                        )?;
                        GeneralResponseEvent::new(
                           response_event_id,
                           event_id.clone(),
                           serde_json::to_string(&task_status)?
                        )
                    }
                    StateValueType::Instance => {
                        let instance: Instance = serde_json::from_slice(
                            &data_bytes
                        )?;
                        GeneralResponseEvent::new(
                           response_event_id,
                           event_id.clone(),
                           serde_json::to_string(&instance)?
                        )
                    }
                };

                let mut guard = self.publisher().lock().await;
                for response_topic in response_topics {
                    guard.publish(
                        Box::new(response_topic),
                        Box::new(response_event.clone())
                    ).await?;
                }

                drop(guard);
            },
            StateEvent::GetAccount { 
                event_id, 
                task_id, 
                task_status, 
                owner, 
                response_topics 
            } => {
                let data_bytes = self.reader_mut().get(owner.to_vec()).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?.ok_or(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("unable to find key 0x{} in state db", hex::encode(&owner))
                    )
                )?;

                let account: Account = serde_json::from_slice(
                    &data_bytes
                )?;

                let response_event_id = Uuid::new_v4().to_string();

                let response_event = GeneralResponseEvent::new(
                    response_event_id,
                    event_id.clone(),
                    serde_json::to_string(&account)?
                );

                let mut guard = self.publisher().lock().await;
                for topic in response_topics {
                    guard.publish(
                        Box::new(topic),
                        Box::new(response_event.clone())
                    ).await?;
                }
            }
            StateEvent::GetInstance { event_id, task_id, task_status, namespace, response_topics } => {
                let data_bytes = self.reader_mut().get(
                    serde_json::to_vec(&namespace)?
                ).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?.ok_or(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("unable to find key 0x{} in state db", hex::encode(&serde_json::to_vec(&namespace)?))
                    )
                )?;

                let instance: Instance = serde_json::from_slice(
                    &data_bytes
                )?;
                let response_event_id = Uuid::new_v4().to_string();

                let response_event = GeneralResponseEvent::new(
                    response_event_id,
                    event_id.clone(),
                    serde_json::to_string(&instance)?
                );

                let mut guard = self.publisher().lock().await;
                for topic in response_topics {
                    guard.publish(
                        Box::new(topic),
                        Box::new(response_event.clone())
                    ).await?;
                }
            }
            StateEvent::GetTaskStatus { event_id, task_id, response_topics } => {
                let data_bytes = self.reader_mut().get(
                    serde_json::to_vec(&task_id)?
                ).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?.ok_or(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("unable to find key 0x{} in state db", hex::encode(&serde_json::to_vec(&task_id)?))
                    )
                )?;

                let task_status = serde_json::from_slice(
                    &data_bytes
                )?;
                let response_event_id = Uuid::new_v4().to_string();

                let response_event = GeneralResponseEvent::new(
                    response_event_id,
                    event_id.clone(),
                    serde_json::to_string(&task_status)?
                );

                let mut guard = self.publisher().lock().await;
                for topic in response_topics {
                    guard.publish(
                        Box::new(topic),
                        Box::new(response_event.clone())
                    ).await?;
                }
            },
            StateEvent::Post { event_id, task_id, key, value } => {
                self.writer_mut().put_optimistic(key, value).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?;
            },
            StateEvent::PostAccount { 
                event_id,
                task_id, 
                task_status, 
                owner, 
                vmlist, 
                namespace, 
                exposed_ports 
            } => {
                let account = self.update_or_create_account(
                    task_id,
                    task_status,
                    owner, 
                    vmlist, 
                    namespace, 
                    Some(exposed_ports)
                ).await?;

                self.writer_mut().put_optimistic(
                    owner.to_vec(), serde_json::to_vec(&account)?
                ).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?;
            }
            StateEvent::PostInstance { 
                event_id, 
                task_id, 
                task_status, 
                namespace, 
                vm_info, 
                port_map,
                last_snapshot,
                last_sync
            } => {
                let instance = self.update_or_create_instance(
                    namespace.clone(), vm_info, port_map, last_snapshot, last_sync
                ).await?;

                self.writer_mut().put_optimistic(
                    serde_json::to_vec(&namespace)?, serde_json::to_vec(&instance)?
                ).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?;
            }
            StateEvent::PostTaskStatus { event_id, task_id, task_status } => {
                self.writer_mut().put_optimistic(
                    serde_json::to_vec(&task_id)?, serde_json::to_vec(&task_status)?
                ).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?
            }
            StateEvent::DeleteInstance { event_id, task_id, task_status, namespace } => {
                self.writer_mut().delete(
                    serde_json::to_vec(
                        &namespace
                    )?
                ).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?;
            }
            StateEvent::DeleteTaskStatus { event_id, task_id } => {
                self.writer_mut().delete(
                    serde_json::to_vec(&task_id)?
                ).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?;
            }
            StateEvent::Delete { event_id, task_id, key } => {
                self.writer_mut().delete(key).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                })?;
            }
        }

        Ok(())
    }

    async fn handle_task_event(&mut self, task_event: TaskStatusEvent) -> std::io::Result<()> {
        match task_event {
            TaskStatusEvent::Get { original_task_id, event_id, response_topics, .. } => {
                let mut read_guard = self.task_cache().write().await;
                if let Some(task_status) = read_guard.get(&original_task_id) {
                    let response_event_id = Uuid::new_v4().to_string(); 
                    let response = GeneralResponseEvent::new(
                        response_event_id.clone(),
                        event_id.to_string(),
                        serde_json::to_string(&task_status)?
                    );
                    let mut publish_guard = self.publisher().lock().await;
                    for topic in response_topics {
                        publish_guard.publish(
                            Box::new(topic), 
                            Box::new(response.clone())
                        ).await?;
                    }
                    drop(publish_guard);
                }
                drop(read_guard);
            }
            TaskStatusEvent::Update { task_id, task_status, .. } => {
                let new_event_id = Uuid::new_v4().to_string();
                let mut read_guard = self.task_cache().write().await;
                let entry = read_guard.get_or_insert_mut(task_id.clone(), || TaskStatus::Pending);
                *entry = task_status.clone();
                let mut guard = self.publisher().lock().await;
                let event = StateEvent::PutTaskStatus { event_id: new_event_id, task_id, task_status };
                guard.publish(
                    Box::new(StateTopic),
                    Box::new(event)
                ).await?;
            }
        }

        Ok(())
    }
}

pub struct StateWriter {
    client: TransactionClient,
}

impl StateWriter {
    pub async fn new(
        pd_endpoints: Vec<String>
    ) -> Result<Self> {
        let client = TransactionClient::new(pd_endpoints).await?;
        Ok(Self { client }) 
    }

    pub async fn new_with_config<'a>(
        pd_endpoints: Vec<&'a str>,
        config: Config
    ) -> Result<Self> {
        let client = TransactionClient::new_with_config(
            pd_endpoints,
            config
        ).await?;
        Ok(Self { client })
    }


    pub async fn put_optimistic(
        &self,
        k: impl Into<Key>,
        v: impl Into<Value>
    ) -> Result<()> {
        let mut txn = self.client.begin_optimistic().await?;
        txn.put(k, v).await?;
        txn.commit().await?;
        Ok(())
    }

    pub async fn put_pessimistic(
        &self,
        k: impl Into<Key>,
        v: impl Into<Value>
    ) -> Result<()> {
        let mut txn = self.client.begin_pessimistic().await?;
        txn.put(k, v).await?;
        txn.commit().await?;
        Ok(())
    }

    pub async fn put_with_options(
        &self,
        options: TransactionOptions,
        k: impl Into<Key> + Clone,
        v: impl Into<Value>
    ) -> Result<()> {
        let mut txn = self.client.begin_with_options(options).await?;
        txn.put(k, v).await?;
        txn.commit().await?;
        Ok(())
    }

    pub async fn batch_put(
        &self,
        kv_pairs: impl ParallelIterator<Item = KvPair>
    ) -> Result<()> {
        let mut txn = self.client.begin_optimistic().await?;
        let mutations: Vec<Mutation> = kv_pairs.into_par_iter()
            .map(|kv| {
                let key: Key = kv.0.into();
                let value: Value = kv.1.into();
                Mutation {
                    op: Op::Put.into(),
                    key: key.into(),
                    value: value.into(), 
                    ..Default::default()
                }
            }).collect();

        txn.batch_mutate(mutations).await?;
        txn.commit().await?;
        Ok(())
    }

    pub fn snapshot(
        &self,
        timestamp: Timestamp,
        options: TransactionOptions
    ) -> Snapshot {
        self.client.snapshot(timestamp, options)
    }

    pub async fn current_timestamp(
        &self
    ) -> Result<Timestamp> {
        self.client.current_timestamp().await
    }

    pub async fn garbage_collect(
        &self,
        safepoint: Timestamp
    ) -> Result<bool> {
        self.client.gc(safepoint).await
    }

    pub async fn delete(
        &self,
        key: impl Into<Key> + Clone
    ) -> Result<()> {
        let mut txn = self.client.begin_optimistic().await?;
        txn.delete(key).await?;
        txn.commit().await?;
        Ok(())
    }
}

pub struct StateReader {
    client: RawClient
}

impl StateReader {
    pub async fn new(
        pd_endpoints: Vec<String>
    ) -> Result<Self> {
        let client = RawClient::new(pd_endpoints).await?;
        Ok(Self { client })
    }

    pub async fn new_with_config<'a>(
        pd_endpoints: Vec<&'a str>,
        config: Config
    ) -> Result<Self> {
        let client = RawClient::new_with_config(
            pd_endpoints,
            config
        ).await?;
        Ok(Self { client })
    }

    pub fn new_with_column_family(
        self,
        cf: ColumnFamily
    ) -> Self {
        let client = self.client.with_cf(cf);
        Self { client }
    }

    pub fn with_backoff(
        self,
        backoff: Backoff
    ) -> Self {
        let client = self.client.with_backoff(backoff);
        Self { client }
    }

    pub async fn get(
        &self,
        key: impl Into<Key>
    ) -> Result<Option<Value>> {
        self.client.get(key).await
    }

    pub async fn batch_get(
        &self,
        keys: impl IntoIterator<Item = impl Into<Key>>
    ) -> Result<Vec<KvPair>> {
        self.client.batch_get(keys).await
    }

    pub async fn batch_scan(
        &self,
        ranges: impl IntoIterator<Item = impl Into<BoundRange>>, 
        each_limit: u32
    ) -> Result<Vec<KvPair>> {
        self.client.batch_scan(
            ranges,
            each_limit
        ).await
    }

    pub async fn batch_scan_keys(
        &self,
        ranges: impl IntoIterator<Item = impl Into<BoundRange>>,
        each_limit: u32
    ) -> Result<Vec<Key>> {
        self.client.batch_scan_keys(
            ranges,
            each_limit
        ).await
    }

    pub async fn scan(
        &self,
        range: impl Into<BoundRange>,
        limit: u32
    ) -> Result<Vec<KvPair>> {
        self.client.scan(
            range,
            limit
        ).await
    }

    pub async fn scan_keys(
        &self,
        range: impl Into<BoundRange>,
        limit: u32
    ) -> Result<Vec<Key>> {
        self.client.scan_keys(
            range,
            limit
        ).await
    }
}

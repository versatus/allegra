#![allow(unused)]
use std::{
    fs::{File, OpenOptions},
    io::Write,
    process::Command,
    sync::Arc
};
use tokio::sync::Mutex;
use lazy_static::lazy_static;

use crate::{dht::Peer, event::{Event, NetworkEvent}, Ballot};
use serde::{Serialize, Deserialize};
use derive_builder::Builder;

#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
#[builder(setter(into), build_fn(error = "ClusterConfigError"))]
pub struct ClusterConfig {
    pub config: InnerConfig,
    pub storage_pools: Vec<StoragePool>,
    pub networks: Vec<Network>,
    pub profiles: Vec<Profile>,
    pub cluster: ConfiguredCluster
}

#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
#[builder(setter(into), build_fn(error = "ClusterConfigError"))]
pub struct InnerConfig {
    pub core_https_address: String,
    pub images_auto_update_interval: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
#[builder(setter(into), build_fn(error = "ClusterConfigError"))]
pub struct StoragePool {
    pub name: String,
    pub driver: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
#[builder(setter(into), build_fn(error = "ClusterConfigError"))]
pub struct Network {
    pub name: String,
    pub r#type: String
}

#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
#[builder(setter(into), build_fn(error = "ClusterConfigError"))]
pub struct Profile {
    pub name: String,
    pub devices: Devices,
}

#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
#[builder(setter(into), build_fn(error = "ClusterConfigError"))]
pub struct Devices {
    pub root: Option<Device>,
    pub eth0: Option<Device>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
#[builder(setter(into), build_fn(error = "ClusterConfigError"))]
pub struct Device {
    pub path: Option<String>,
    pub pool: Option<String>,
    pub r#type: String,
    pub name: Option<String>,
    pub nictype: Option<String>,
    pub parent: Option<String>
}

#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
#[builder(setter(into), build_fn(error = "ClusterConfigError"))]
pub struct ConfiguredCluster {
    pub server_name: String,
    pub enabled: bool
}

#[derive(Debug)]
pub struct ClusterConfigError;

impl std::fmt::Display for ClusterConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClusterConfigError")
    }
}

impl std::error::Error for ClusterConfigError {}

impl From<ClusterConfigError> for std::io::Error {
    fn from(value: ClusterConfigError) -> Self {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("{}", value)
        )
    }
}

impl From<derive_builder::UninitializedFieldError> for ClusterConfigError {
    fn from(value: derive_builder::UninitializedFieldError) -> Self {
        ClusterConfigError
    }
}

pub struct Cluster {
    local_peer: Peer,
    peers: Arc<Mutex<Vec<Peer>>>,
    cluster_leader: Peer,
    election: Vec<Ballot>,
    //TODO(asmith): Replace with publisher
    //event_broker: Arc<Mutex<EventBroker>>,
    local_join_token: Option<String>,
    join_tokens: Vec<String>
}

impl Cluster {
    pub fn new(
        local_peer: Peer, 
        cluster_leader: Peer, 
        peers: Arc<Mutex<Vec<Peer>>>, 
        election: Vec<Ballot>,
        //TODO(asmith): Replace with publisher
        //event_broker: Arc<Mutex<EventBroker>>,
    ) -> Self {
        Self { 
            local_peer, 
            peers, 
            cluster_leader, 
            election, 
            //TODO(asmith): Replace with publisher
            //event_broker, 
            local_join_token: None, 
            join_tokens: Vec::new() 
        } 
    }

    pub async fn generate_certificate(&self) -> std::io::Result<()> {
        if self.local_peer == self.cluster_leader { 
            let output = std::process::Command::new("openssl")
                .arg("req")
                .arg("-x509")
                .arg("-newkey")
                .arg("rsa:4096")
                .arg("-keyout")
                .arg("cluster.key")
                .arg("-out")
                .arg("cluster.crt")
                .arg("-days")
                .arg("365")
                .arg("-nodes")
                .arg("-subj")
                .arg("/CN=cluster")
                .output()?;

            self.distribute_certificate().await?;
            return Ok(())
        }

        Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "local node is not the cluster leader"
            )
        )
    }

    pub async fn distribute_certificate(&self) -> std::io::Result<()> {
        /*
        if self.local_peer == self.cluster_leader { 
            let cert = std::fs::read_to_string("cluster.crt")?;
            let event = Event::NetworkEvent(
                NetworkEvent::ShareCert {
                    peer, cert,
                }
            );

            let mut guard = self.event_broker.lock().await;
            guard.publish("Network".to_string(), event).await;

            drop(guard);
            return Ok(())
        } 
        */
        Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "local node is not the cluster leader"
            )
        )
    }

    pub async fn generate_join_token(&mut self, new_node: Peer) -> std::io::Result<()> {
        let output = std::process::Command::new("lxc")
            .arg("cluster")
            .arg("add")
            .arg("node")
            .arg("--target")
            .output();
        Ok(())
    }

    pub async fn distribute_join_token(&self, new_node: Peer) -> std::io::Result<()> {
        Ok(())
    }

    pub async fn cluster_init(&self) -> std::io::Result<()> {
        if self.local_peer == self.cluster_leader { 
            self.generate_certificate().await?;
            let config = ClusterConfigBuilder::default()
                .config(
                    InnerConfigBuilder::default()
                        .core_https_address(self.local_peer.ip_address().to_string())
                        .images_auto_update_interval(Some(15))
                        .build()?
                )
                .storage_pools(
                    vec![
                        StoragePoolBuilder::default()
                            .name("default".to_string())
                            .driver("dir".to_string())
                            .build()?,
                        StoragePoolBuilder::default()
                            .name("cluster-pool".to_string())
                            .driver("zfs".to_string())
                            .build()?
                    ]
                ).networks(
                    vec![
                        NetworkBuilder::default()
                            .name("lxdbr0".to_string())
                            .r#type("bridge".to_string())
                            .build()?
                    ]
                ).profiles(
                    vec![
                        ProfileBuilder::default()
                            .name("default".to_string())
                            .devices(
                                DevicesBuilder::default()
                                    .root(
                                        DeviceBuilder::default()
                                            .path(Some("/".to_string()))
                                            .pool(Some("cluster-pool".to_string()))
                                            .r#type("disk".to_string())
                                            .build()?
                                    )
                                    .eth0(
                                        DeviceBuilder::default()
                                            .name(Some("eth0".to_string()))
                                            .nictype(Some("bridged".to_string()))
                                            .parent(Some("lxdbr0".to_string()))
                                            .r#type("nic".to_string())
                                            .build()?
                                    )
                                    .build()?
                            )
                            .build()?
                    ]
                )
                .cluster(
                    ConfiguredClusterBuilder::default()
                        .server_name(self.local_peer.ip_address().to_string())
                        .enabled(true)
                        .build()?
                ).build()?;

            let yaml = serde_yml::to_string(&config).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;

            let mut file = File::create("lxd_config.yaml")?;
            file.write_all(yaml.as_bytes())?;

            std::process::Command::new("sh")
                .arg("-c")
                .arg("cat lxd config.yaml | lxd init --preseed")
                .output()?;

            let mut guard = self.peers.lock().await;
            guard.push(self.local_peer.clone());

            drop(guard);

            return Ok(())

        } else {
            let cluster = ConfiguredClusterBuilder::default();
            return Ok(())
        }
    }

    pub fn add_node(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

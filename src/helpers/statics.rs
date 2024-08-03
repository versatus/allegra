use crate::network::quorum::Quorum;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref SERVER_BLOCK_TEMPLATE: &'static str = r#"
server {
    listen {host_port};

    server_name _;

    location / {
        proxy_pass http://{instance_ip}:{instance_port};
        proxy_set_header HOST $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
"#;
    pub static ref DEFAULT_LXD_STORAGE_POOL: &'static str = "/mnt/libretto/lxd-storage-pool";
    pub static ref DEFAULT_GRPC_ADDRESS: &'static str = "0.0.0.0:50051";
    pub static ref DEFAULT_SUBSCRIBER_ADDRESS: &'static str = "127.0.0.1:5556";
    pub static ref DEFAULT_PUBLISHER_ADDRESS: &'static str = "127.0.0.1:5555";
    pub static ref TEMP_PATH: &'static str = "/var/snap/lxd/common/lxd/tmp";
    pub static ref BOOTSTRAP_QUORUM: Quorum = Quorum::new();
    pub static ref DEFAULT_NETWORK: &'static str = "virbr0";
    pub static ref SUCCESS: &'static str = "SUCCESS";
    pub static ref FAILURE: &'static str = "FAILURE";
    pub static ref PENDING: &'static str = "PENDING";
    pub static ref DEFAULT_CONFIG_FILEPATH: &'static str = "./config.toml";
    pub static ref DEFAULT_PD_ENDPOINT: &'static str = "127.0.0.1:2379";
    pub static ref IMAGES_DIR: &'static str = "/var/lib/libvirt/images/";
}

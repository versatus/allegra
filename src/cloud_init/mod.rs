use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct CloudConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub users: Option<Vec<User>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<HashMap<String, Option<Vec<String>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_authorized_keys: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_keys: Option<SshKeys>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_files: Option<Vec<WriteFile>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_update: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_upgrade: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runcmd: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootcmd: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mounts: Option<Vec<Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_default_fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_setup: Option<HashMap<String, DiskSetup>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fs_setup: Option<Vec<FsSetup>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apt: Option<AptConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yum_repos: Option<HashMap<String, YumRepo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_certs: Option<CaCerts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chef: Option<ChefConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ansible: Option<AnsibleConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_state: Option<PowerState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_root: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct User {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gecos: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sudo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_passwd: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_authorized_keys: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_import_id: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct SshKeys {
    pub rsa_private: String,
    pub rsa_public: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct WriteFile {
    pub path: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct DiskSetup {
    pub table_type: String,
    pub layout: DiskLayout,
    pub overwrite: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum DiskLayout {
    Auto(bool),
    Custom(Vec<DiskPartition>),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum DiskPartition {
    Simple(u8),
    WithType(Vec<u8>),
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct FsSetup {
    pub label: String,
    pub filesystem: String,
    pub device: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwrite: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct AptConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<Vec<AptMirror>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<HashMap<String, AptSource>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct AptMirror {
    pub arches: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct AptSource {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct YumRepo {
    pub baseurl: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpgcheck: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpgkey: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct CaCerts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_defaults: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trusted: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct ChefConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_cert: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_list: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct AnsibleConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pull: Option<AnsiblePull>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct AnsiblePull {
    pub url: String,
    pub playbook_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct PowerState {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

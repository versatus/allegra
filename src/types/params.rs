use crate::allegra_rpc::{
    Features, GetTaskStatusRequest, InstanceAddPubkeyParams, InstanceCreateParams,
    InstanceDeleteParams, InstanceExposeServiceParams, InstanceGetSshDetails, InstanceStartParams,
    InstanceStopParams, ServiceType as ProtoServiceType,
};
use crate::distro::Distro;
use crate::virt_install::CloudInit;
use clap::ValueEnum;
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{de::Deserializer, Deserialize, Serialize, Serializer};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize, ValueEnum)]
pub enum ServiceType {
    Ssh,
    NodeJs,
    Postgres,
    MySQL,
    Redis,
    MongoDB,
    RabbitMQ,
    Kafka,
    Custom,
}

impl fmt::Display for ServiceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<ServiceType> for i32 {
    fn from(value: ServiceType) -> Self {
        match value {
            ServiceType::Ssh => 0,
            ServiceType::NodeJs => 1,
            ServiceType::Postgres => 2,
            ServiceType::MySQL => 3,
            ServiceType::Redis => 4,
            ServiceType::MongoDB => 5,
            ServiceType::RabbitMQ => 6,
            ServiceType::Kafka => 7,
            ServiceType::Custom => 8,
        }
    }
}

impl From<&ServiceType> for i32 {
    fn from(value: &ServiceType) -> Self {
        match value {
            ServiceType::Ssh => 0,
            ServiceType::NodeJs => 1,
            ServiceType::Postgres => 2,
            ServiceType::MySQL => 3,
            ServiceType::Redis => 4,
            ServiceType::MongoDB => 5,
            ServiceType::RabbitMQ => 6,
            ServiceType::Kafka => 7,
            ServiceType::Custom => 8,
        }
    }
}

impl From<i32> for ServiceType {
    fn from(val: i32) -> Self {
        match ProtoServiceType::try_from(val) {
            Ok(ProtoServiceType::Ssh) => ServiceType::Ssh,
            Ok(ProtoServiceType::NodeJs) => ServiceType::NodeJs,
            Ok(ProtoServiceType::Postgres) => ServiceType::Postgres,
            Ok(ProtoServiceType::Mysql) => ServiceType::MySQL,
            Ok(ProtoServiceType::Redis) => ServiceType::Redis,
            Ok(ProtoServiceType::MongoDb) => ServiceType::MongoDB,
            Ok(ProtoServiceType::RabbitMq) => ServiceType::RabbitMQ,
            Ok(ProtoServiceType::Kafka) => ServiceType::Kafka,
            Ok(ProtoServiceType::Custom) => ServiceType::Custom,
            Err(_) => ServiceType::Custom,
        }
    }
}

impl From<ProtoServiceType> for ServiceType {
    fn from(val: ProtoServiceType) -> Self {
        match val {
            ProtoServiceType::Ssh => ServiceType::Ssh,
            ProtoServiceType::NodeJs => ServiceType::NodeJs,
            ProtoServiceType::Postgres => ServiceType::Postgres,
            ProtoServiceType::Mysql => ServiceType::MySQL,
            ProtoServiceType::Redis => ServiceType::Redis,
            ProtoServiceType::MongoDb => ServiceType::MongoDB,
            ProtoServiceType::RabbitMq => ServiceType::RabbitMQ,
            ProtoServiceType::Kafka => ServiceType::Kafka,
            ProtoServiceType::Custom => ServiceType::Custom,
        }
    }
}

impl ServiceType {
    pub fn default_port(&self) -> Option<u16> {
        match *self {
            ServiceType::Ssh => Some(22),
            ServiceType::NodeJs => Some(3000),
            ServiceType::Postgres => Some(5432),
            ServiceType::MySQL => Some(3306),
            ServiceType::Redis => Some(6379),
            ServiceType::MongoDB => Some(27017),
            ServiceType::RabbitMQ => Some(5672),
            ServiceType::Kafka => Some(9092),
            ServiceType::Custom => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateParams {
    pub name: String,
    pub distro: Distro,
    pub version: String,
    pub vmtype: String,
    pub sig: String,
    pub recovery_id: u32,
    pub sync: Option<bool>,
    pub memory: Option<String>,
    pub vcpus: Option<String>,
    pub cpu: Option<String>,
    pub metadata: Option<String>,
    pub os_variant: Option<String>,
    pub host_device: Vec<String>,
    pub network: Vec<String>,
    pub disk: Vec<String>,
    pub filesystem: Vec<String>,
    pub controller: Vec<String>,
    pub input: Vec<String>,
    pub graphics: Option<String>,
    pub sound: Option<String>,
    pub video: Option<String>,
    pub smartcard: Option<String>,
    pub redirdev: Vec<String>,
    pub memballoon: Option<String>,
    pub tpm: Option<String>,
    pub rng: Option<String>,
    pub panic: Option<String>,
    pub shmem: Option<String>,
    pub memdev: Vec<String>,
    pub vsock: Option<String>,
    pub iommu: Option<String>,
    pub watchdog: Option<String>,
    pub serial: Vec<String>,
    pub parallel: Vec<String>,
    pub channel: Vec<String>,
    pub console: Vec<String>,
    pub install: Option<String>,
    pub cdrom: Option<String>,
    pub location: Option<String>,
    pub pxe: bool,
    pub import: bool,
    pub boot: Option<String>,
    pub idmap: Option<String>,
    pub features: Vec<Features>,
    pub clock: Option<String>,
    pub launch_security: Option<String>,
    pub numatune: Option<String>,
    pub boot_dev: Vec<String>,
    pub unattended: bool,
    pub print_xml: Option<String>,
    pub dry_run: bool,
    pub connect: Option<String>,
    pub virt_type: Option<String>,
    pub cloud_init: Option<CloudInit>,
}

impl From<InstanceCreateParams> for CreateParams {
    fn from(value: InstanceCreateParams) -> Self {
        CreateParams {
            name: value.name,
            distro: value.distro.into(),
            version: value.version,
            vmtype: value.vmtype,
            sig: value.sig,
            recovery_id: value.recovery_id,
            sync: value.sync,
            memory: value.memory,
            vcpus: value.vcpus,
            cpu: value.cpu,
            metadata: value.metadata,
            os_variant: value.os_variant,
            host_device: value.host_device,
            network: value.network,
            disk: value.disk,
            filesystem: value.filesystem,
            controller: value.controller,
            input: value.input,
            graphics: value.graphics,
            sound: value.sound,
            video: value.video,
            smartcard: value.smartcard,
            redirdev: value.redirdev,
            memballoon: value.memballoon,
            tpm: value.tpm,
            rng: value.rng,
            panic: value.panic,
            shmem: value.shmem,
            memdev: value.memdev,
            vsock: value.vsock,
            iommu: value.iommu,
            watchdog: value.watchdog,
            serial: value.serial,
            parallel: value.parallel,
            channel: value.channel,
            console: value.console,
            install: value.install,
            cdrom: value.cdrom,
            location: value.location,
            pxe: value.pxe,
            import: value.import,
            boot: value.boot,
            idmap: value.idmap,
            features: value.features,
            clock: value.clock,
            launch_security: value.launch_security,
            numatune: value.numatune,
            boot_dev: value.boot_dev,
            unattended: value.unattended,
            print_xml: value.print_xml,
            dry_run: value.dry_run,
            connect: value.connect,
            virt_type: value.virt_type,
            cloud_init: match value.cloud_init {
                Some(ci) => Some(ci.into()),
                None => None,
            },
        }
    }
}

impl From<CreateParams> for InstanceCreateParams {
    fn from(value: CreateParams) -> Self {
        InstanceCreateParams {
            name: value.name,
            distro: value.distro.into(),
            version: value.version,
            vmtype: value.vmtype,
            sig: value.sig,
            recovery_id: value.recovery_id,
            sync: value.sync,
            memory: value.memory,
            vcpus: value.vcpus,
            cpu: value.cpu,
            metadata: value.metadata,
            os_variant: value.os_variant,
            host_device: value.host_device,
            network: value.network,
            disk: value.disk,
            filesystem: value.filesystem,
            controller: value.controller,
            input: value.input,
            graphics: value.graphics,
            sound: value.sound,
            video: value.video,
            smartcard: value.smartcard,
            redirdev: value.redirdev,
            memballoon: value.memballoon,
            tpm: value.tpm,
            rng: value.rng,
            panic: value.panic,
            shmem: value.shmem,
            memdev: value.memdev,
            vsock: value.vsock,
            iommu: value.iommu,
            watchdog: value.watchdog,
            serial: value.serial,
            parallel: value.parallel,
            channel: value.channel,
            console: value.console,
            install: value.install,
            cdrom: value.cdrom,
            location: value.location,
            pxe: value.pxe,
            import: value.import,
            boot: value.boot,
            idmap: value.idmap,
            features: value.features,
            clock: value.clock,
            launch_security: value.launch_security,
            numatune: value.numatune,
            boot_dev: value.boot_dev,
            unattended: value.unattended,
            print_xml: value.print_xml,
            dry_run: value.dry_run,
            connect: value.connect,
            virt_type: value.virt_type,
            cloud_init: match value.cloud_init {
                Some(ci) => Some(ci.into()),
                None => None,
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartParams {
    pub name: String,
    pub console: bool,
    pub stateless: bool,
    pub sig: String,
    pub recovery_id: u32,
}

impl From<InstanceStartParams> for StartParams {
    fn from(value: InstanceStartParams) -> Self {
        StartParams {
            name: value.name,
            console: value.console,
            stateless: value.stateless,
            sig: value.sig,
            recovery_id: value.recovery_id,
        }
    }
}

impl From<StartParams> for InstanceStartParams {
    fn from(value: StartParams) -> Self {
        InstanceStartParams {
            name: value.name,
            console: value.console,
            stateless: value.stateless,
            sig: value.sig,
            recovery_id: value.recovery_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StopParams {
    pub name: String,
    pub sig: String,
    pub recovery_id: u32,
}

impl From<InstanceStopParams> for StopParams {
    fn from(value: InstanceStopParams) -> Self {
        StopParams {
            name: value.name,
            sig: value.sig,
            recovery_id: value.recovery_id,
        }
    }
}

impl From<StopParams> for InstanceStopParams {
    fn from(value: StopParams) -> Self {
        InstanceStopParams {
            name: value.name,
            sig: value.sig,
            recovery_id: value.recovery_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeleteParams {
    pub name: String,
    pub force: bool,
    pub interactive: bool,
    pub sig: String,
    pub recovery_id: u32,
}

impl From<InstanceDeleteParams> for DeleteParams {
    fn from(value: InstanceDeleteParams) -> Self {
        DeleteParams {
            name: value.name,
            force: value.force,
            interactive: value.interactive,
            sig: value.sig,
            recovery_id: value.recovery_id,
        }
    }
}

impl From<DeleteParams> for InstanceDeleteParams {
    fn from(value: DeleteParams) -> Self {
        InstanceDeleteParams {
            name: value.name,
            force: value.force,
            interactive: value.interactive,
            sig: value.sig,
            recovery_id: value.recovery_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddPubkeyParams {
    pub name: String,
    pub pubkey: String,
    pub sig: String,
    pub recovery_id: u32,
}

impl From<InstanceAddPubkeyParams> for AddPubkeyParams {
    fn from(value: InstanceAddPubkeyParams) -> Self {
        AddPubkeyParams {
            name: value.name,
            pubkey: value.pubkey,
            sig: value.sig,
            recovery_id: value.recovery_id,
        }
    }
}

impl From<AddPubkeyParams> for InstanceAddPubkeyParams {
    fn from(value: AddPubkeyParams) -> Self {
        InstanceAddPubkeyParams {
            name: value.name,
            pubkey: value.pubkey,
            sig: value.sig,
            recovery_id: value.recovery_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExposeServiceParams {
    pub name: String,
    pub port: Vec<u32>,
    pub service_type: Vec<ServiceType>,
    pub sig: String,
    pub recovery_id: u32,
}

impl From<InstanceExposeServiceParams> for ExposeServiceParams {
    fn from(value: InstanceExposeServiceParams) -> Self {
        ExposeServiceParams {
            name: value.name,
            port: value.port,
            service_type: value
                .service_type
                .iter()
                .map(|n| {
                    let n = *n;
                    n.into()
                })
                .collect(),
            sig: value.sig,
            recovery_id: value.recovery_id,
        }
    }
}

impl From<ExposeServiceParams> for InstanceExposeServiceParams {
    fn from(value: ExposeServiceParams) -> Self {
        InstanceExposeServiceParams {
            name: value.name,
            port: value.port,
            service_type: value.service_type.iter().map(|n| n.into()).collect(),
            sig: value.sig,
            recovery_id: value.recovery_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetSshParams {
    pub owner: String,
    pub name: String,
    pub keypath: Option<String>,
    pub username: Option<String>,
}

impl From<InstanceGetSshDetails> for GetSshParams {
    fn from(value: InstanceGetSshDetails) -> Self {
        GetSshParams {
            owner: value.owner,
            name: value.name,
            keypath: value.keypath,
            username: value.username,
        }
    }
}

impl From<GetSshParams> for InstanceGetSshDetails {
    fn from(value: GetSshParams) -> Self {
        InstanceGetSshDetails {
            owner: value.owner,
            name: value.name,
            keypath: value.keypath,
            username: value.username,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Params {
    Create(CreateParams),
    Start(StartParams),
    Stop(StopParams),
    AddPubkey(AddPubkeyParams),
    Delete(DeleteParams),
    ExposeService(ExposeServiceParams),
    GetSshDetails(GetSshParams),
}

impl From<InstanceCreateParams> for Params {
    fn from(value: InstanceCreateParams) -> Self {
        Self::Create(value.into())
    }
}

impl From<InstanceStartParams> for Params {
    fn from(value: InstanceStartParams) -> Self {
        Self::Start(value.into())
    }
}

impl From<InstanceStopParams> for Params {
    fn from(value: InstanceStopParams) -> Self {
        Self::Stop(value.into())
    }
}

impl From<InstanceDeleteParams> for Params {
    fn from(value: InstanceDeleteParams) -> Self {
        Self::Delete(value.into())
    }
}

impl From<InstanceExposeServiceParams> for Params {
    fn from(value: InstanceExposeServiceParams) -> Self {
        Self::ExposeService(value.into())
    }
}

impl From<InstanceGetSshDetails> for Params {
    fn from(value: InstanceGetSshDetails) -> Self {
        Self::GetSshDetails(value.into())
    }
}

impl From<InstanceAddPubkeyParams> for Params {
    fn from(value: InstanceAddPubkeyParams) -> Self {
        Self::AddPubkey(value.into())
    }
}

pub trait HasOwner {
    fn owner(&self) -> std::io::Result<[u8; 20]>;
}

impl HasOwner for InstanceGetSshDetails {
    fn owner(&self) -> std::io::Result<[u8; 20]> {
        let mut buffer = [0u8; 20];
        if self.owner.starts_with("0x") {
            let bytes = hex::decode(&self.owner[2..])
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            buffer.copy_from_slice(&bytes[..]);
        } else {
            let bytes = hex::decode(&self.owner)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            buffer.copy_from_slice(&bytes[..]);
        }

        return Ok(buffer);
    }
}

impl HasOwner for GetTaskStatusRequest {
    fn owner(&self) -> std::io::Result<[u8; 20]> {
        let mut buffer = [0u8; 20];
        if self.owner.starts_with("0x") {
            let owner_string = &self.owner[2..];
            let bytes = hex::decode(owner_string)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            buffer.copy_from_slice(&bytes[..]);
        } else {
            let bytes = hex::decode(&self.owner)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            buffer.copy_from_slice(&bytes[..]);
        }

        Ok(buffer)
    }
}

impl Serialize for Features {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Features", 2)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("feature", &self.feature)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Features {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Name,
            Feature,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`name` or `feature`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "name" => Ok(Field::Name),
                            "feature" => Ok(Field::Feature),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct FeaturesVisitor;

        impl<'de> Visitor<'de> for FeaturesVisitor {
            type Value = Features;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Features")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Features, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut name = None;
                let mut feature = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            name = Some(map.next_value()?);
                        }
                        Field::Feature => {
                            if feature.is_some() {
                                return Err(de::Error::duplicate_field("feature"));
                            }
                            feature = Some(map.next_value()?);
                        }
                    }
                }
                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let feature = feature.ok_or_else(|| de::Error::missing_field("feature"))?;
                Ok(Features { name, feature })
            }
        }

        const FIELDS: &'static [&'static str] = &["name", "feature"];
        deserializer.deserialize_struct("Features", FIELDS, FeaturesVisitor)
    }
}

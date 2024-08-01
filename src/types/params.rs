use crate::allegra_rpc::{
    Features, GetTaskStatusRequest, InstanceAddPubkeyParams, InstanceCreateParams,
    InstanceDeleteParams, InstanceExposeServiceParams, InstanceGetSshDetails, InstanceStartParams,
    InstanceStopParams, ServiceType as ProtoServiceType,
};
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
pub enum Params {
    Create(InstanceCreateParams),
    Start(InstanceStartParams),
    Stop(InstanceStopParams),
    AddPubkey(InstanceAddPubkeyParams),
    Delete(InstanceDeleteParams),
    ExposeService(InstanceExposeServiceParams),
    GetSshDetails(InstanceGetSshDetails),
}

impl From<InstanceCreateParams> for Params {
    fn from(value: InstanceCreateParams) -> Self {
        Self::Create(value)
    }
}

impl From<InstanceStartParams> for Params {
    fn from(value: InstanceStartParams) -> Self {
        Self::Start(value)
    }
}

impl From<InstanceStopParams> for Params {
    fn from(value: InstanceStopParams) -> Self {
        Self::Stop(value)
    }
}

impl From<InstanceDeleteParams> for Params {
    fn from(value: InstanceDeleteParams) -> Self {
        Self::Delete(value)
    }
}

impl From<InstanceExposeServiceParams> for Params {
    fn from(value: InstanceExposeServiceParams) -> Self {
        Self::ExposeService(value)
    }
}

impl From<InstanceGetSshDetails> for Params {
    fn from(value: InstanceGetSshDetails) -> Self {
        Self::GetSshDetails(value)
    }
}

impl From<InstanceAddPubkeyParams> for Params {
    fn from(value: InstanceAddPubkeyParams) -> Self {
        Self::AddPubkey(value)
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

impl Serialize for crate::allegra_rpc::CloudInit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CloudInit", 8)?;
        state.serialize_field("root_password_generate", &self.root_password_generate)?;
        state.serialize_field("disable", &self.disable)?;
        state.serialize_field("root_password_file", &self.root_password_file)?;
        state.serialize_field("meta_data", &self.meta_data)?;
        state.serialize_field("user_data", &self.user_data)?;
        state.serialize_field("root_ssh_key", &self.root_ssh_key)?;
        state.serialize_field("clouduser_ssh_key", &self.clouduser_ssh_key)?;
        state.serialize_field("network_config", &self.network_config)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for crate::allegra_rpc::CloudInit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            RootPasswordGenerate,
            Disable,
            RootPasswordFile,
            MetaData,
            UserData,
            RootSshKey,
            ClouduserSshKey,
            NetworkConfig,
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
                        formatter.write_str("field identifier")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "root_password_generate" => Ok(Field::RootPasswordGenerate),
                            "disable" => Ok(Field::Disable),
                            "root_password_file" => Ok(Field::RootPasswordFile),
                            "meta_data" => Ok(Field::MetaData),
                            "user_data" => Ok(Field::UserData),
                            "root_ssh_key" => Ok(Field::RootSshKey),
                            "clouduser_ssh_key" => Ok(Field::ClouduserSshKey),
                            "network_config" => Ok(Field::NetworkConfig),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct CloudInitVisitor;

        impl<'de> Visitor<'de> for CloudInitVisitor {
            type Value = crate::allegra_rpc::CloudInit;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct CloudInit")
            }

            fn visit_map<V>(self, mut map: V) -> Result<crate::allegra_rpc::CloudInit, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut root_password_generate = None;
                let mut disable = None;
                let mut root_password_file = None;
                let mut meta_data = None;
                let mut user_data = None;
                let mut root_ssh_key = None;
                let mut clouduser_ssh_key = None;
                let mut network_config = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::RootPasswordGenerate => {
                            if root_password_generate.is_some() {
                                return Err(de::Error::duplicate_field("root_password_generate"));
                            }
                            root_password_generate = Some(map.next_value()?);
                        }
                        Field::Disable => {
                            if disable.is_some() {
                                return Err(de::Error::duplicate_field("disable"));
                            }
                            disable = Some(map.next_value()?);
                        }
                        Field::RootPasswordFile => {
                            if root_password_file.is_some() {
                                return Err(de::Error::duplicate_field("root_password_file"));
                            }
                            root_password_file = Some(map.next_value()?);
                        }
                        Field::MetaData => {
                            if meta_data.is_some() {
                                return Err(de::Error::duplicate_field("meta_data"));
                            }
                            meta_data = Some(map.next_value()?);
                        }
                        Field::UserData => {
                            if user_data.is_some() {
                                return Err(de::Error::duplicate_field("user_data"));
                            }
                            user_data = Some(map.next_value()?);
                        }
                        Field::RootSshKey => {
                            if root_ssh_key.is_some() {
                                return Err(de::Error::duplicate_field("root_ssh_key"));
                            }
                            root_ssh_key = Some(map.next_value()?);
                        }
                        Field::ClouduserSshKey => {
                            if clouduser_ssh_key.is_some() {
                                return Err(de::Error::duplicate_field("clouduser_ssh_key"));
                            }
                            clouduser_ssh_key = Some(map.next_value()?);
                        }
                        Field::NetworkConfig => {
                            if network_config.is_some() {
                                return Err(de::Error::duplicate_field("network_config"));
                            }
                            network_config = Some(map.next_value()?);
                        }
                    }
                }

                let root_password_generate = root_password_generate
                    .ok_or_else(|| de::Error::missing_field("root_password_generate"))?;
                let disable = disable.ok_or_else(|| de::Error::missing_field("disable"))?;

                Ok(crate::allegra_rpc::CloudInit {
                    root_password_generate,
                    disable,
                    root_password_file,
                    meta_data,
                    user_data,
                    root_ssh_key,
                    clouduser_ssh_key,
                    network_config,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &[
            "root_password_generate",
            "disable",
            "root_password_file",
            "meta_data",
            "user_data",
            "root_ssh_key",
            "clouduser_ssh_key",
            "network_config",
        ];
        deserializer.deserialize_struct("CloudInit", FIELDS, CloudInitVisitor)
    }
}

impl Serialize for InstanceCreateParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("InstanceCreateParams", 54)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("distro", &self.distro)?;
        state.serialize_field("version", &self.version)?;
        state.serialize_field("vmtype", &self.vmtype)?;
        state.serialize_field("sig", &self.sig)?;
        state.serialize_field("recovery_id", &self.recovery_id)?;
        state.serialize_field("sync", &self.sync)?;
        state.serialize_field("memory", &self.memory)?;
        state.serialize_field("vcpus", &self.vcpus)?;
        state.serialize_field("cpu", &self.cpu)?;
        state.serialize_field("metadata", &self.metadata)?;
        state.serialize_field("os_variant", &self.os_variant)?;
        state.serialize_field("host_device", &self.host_device)?;
        state.serialize_field("network", &self.network)?;
        state.serialize_field("disk", &self.disk)?;
        state.serialize_field("filesystem", &self.filesystem)?;
        state.serialize_field("controller", &self.controller)?;
        state.serialize_field("input", &self.input)?;
        state.serialize_field("graphics", &self.graphics)?;
        state.serialize_field("sound", &self.sound)?;
        state.serialize_field("video", &self.video)?;
        state.serialize_field("smartcard", &self.smartcard)?;
        state.serialize_field("redirdev", &self.redirdev)?;
        state.serialize_field("memballoon", &self.memballoon)?;
        state.serialize_field("tpm", &self.tpm)?;
        state.serialize_field("rng", &self.rng)?;
        state.serialize_field("panic", &self.panic)?;
        state.serialize_field("shmem", &self.shmem)?;
        state.serialize_field("memdev", &self.memdev)?;
        state.serialize_field("vsock", &self.vsock)?;
        state.serialize_field("iommu", &self.iommu)?;
        state.serialize_field("watchdog", &self.watchdog)?;
        state.serialize_field("serial", &self.serial)?;
        state.serialize_field("parallel", &self.parallel)?;
        state.serialize_field("channel", &self.channel)?;
        state.serialize_field("console", &self.console)?;
        state.serialize_field("install", &self.install)?;
        state.serialize_field("cdrom", &self.cdrom)?;
        state.serialize_field("location", &self.location)?;
        state.serialize_field("pxe", &self.pxe)?;
        state.serialize_field("import_", &self.import)?;
        state.serialize_field("boot", &self.boot)?;
        state.serialize_field("idmap", &self.idmap)?;
        state.serialize_field("features", &self.features)?;
        state.serialize_field("clock", &self.clock)?;
        state.serialize_field("launch_security", &self.launch_security)?;
        state.serialize_field("numatune", &self.numatune)?;
        state.serialize_field("boot_dev", &self.boot_dev)?;
        state.serialize_field("unattended", &self.unattended)?;
        state.serialize_field("print_xml", &self.print_xml)?;
        state.serialize_field("dry_run", &self.dry_run)?;
        state.serialize_field("connect", &self.connect)?;
        state.serialize_field("virt_type", &self.virt_type)?;
        state.serialize_field("cloud_init", &self.cloud_init)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstanceCreateParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Name,
            Distro,
            Version,
            Vmtype,
            Sig,
            RecoveryId,
            Sync,
            Memory,
            Vcpus,
            Cpu,
            Metadata,
            OsVariant,
            HostDevice,
            Network,
            Disk,
            Filesystem,
            Controller,
            Input,
            Graphics,
            Sound,
            Video,
            Smartcard,
            Redirdev,
            Memballoon,
            Tpm,
            Rng,
            Panic,
            Shmem,
            Memdev,
            Vsock,
            Iommu,
            Watchdog,
            Serial,
            Parallel,
            Channel,
            Console,
            Install,
            Cdrom,
            Location,
            Pxe,
            Import,
            Boot,
            Idmap,
            Features,
            Clock,
            LaunchSecurity,
            Numatune,
            BootDev,
            Unattended,
            PrintXml,
            DryRun,
            Connect,
            VirtType,
            CloudInit,
        }

        struct InstanceCreateParamsVisitor;

        impl<'de> Visitor<'de> for InstanceCreateParamsVisitor {
            type Value = InstanceCreateParams;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct InstanceCreateParams")
            }

            fn visit_map<V>(self, mut map: V) -> Result<InstanceCreateParams, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut name = None;
                let mut distro = None;
                let mut version = None;
                let mut vmtype = None;
                let mut sig = None;
                let mut recovery_id = None;
                let mut sync = None;
                let mut memory = None;
                let mut vcpus = None;
                let mut cpu = None;
                let mut metadata = None;
                let mut os_variant = None;
                let mut host_device = None;
                let mut network = None;
                let mut disk = None;
                let mut filesystem = None;
                let mut controller = None;
                let mut input = None;
                let mut graphics = None;
                let mut sound = None;
                let mut video = None;
                let mut smartcard = None;
                let mut redirdev = None;
                let mut memballoon = None;
                let mut tpm = None;
                let mut rng = None;
                let mut panic = None;
                let mut shmem = None;
                let mut memdev = None;
                let mut vsock = None;
                let mut iommu = None;
                let mut watchdog = None;
                let mut serial = None;
                let mut parallel = None;
                let mut channel = None;
                let mut console = None;
                let mut install = None;
                let mut cdrom = None;
                let mut location = None;
                let mut pxe = None;
                let mut import_ = None;
                let mut boot = None;
                let mut idmap = None;
                let mut features = None;
                let mut clock = None;
                let mut launch_security = None;
                let mut numatune = None;
                let mut boot_dev = None;
                let mut unattended = None;
                let mut print_xml = None;
                let mut dry_run = None;
                let mut connect = None;
                let mut virt_type = None;
                let mut cloud_init = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            name = Some(map.next_value()?);
                        }
                        Field::Distro => {
                            distro = Some(map.next_value()?);
                        }
                        Field::Version => {
                            version = Some(map.next_value()?);
                        }
                        Field::Vmtype => {
                            vmtype = Some(map.next_value()?);
                        }
                        Field::Sig => {
                            sig = Some(map.next_value()?);
                        }
                        Field::RecoveryId => {
                            recovery_id = Some(map.next_value()?);
                        }
                        Field::Sync => {
                            sync = Some(map.next_value()?);
                        }
                        Field::Memory => {
                            memory = Some(map.next_value()?);
                        }
                        Field::Vcpus => {
                            vcpus = Some(map.next_value()?);
                        }
                        Field::Cpu => {
                            cpu = Some(map.next_value()?);
                        }
                        Field::Metadata => {
                            metadata = Some(map.next_value()?);
                        }
                        Field::OsVariant => {
                            os_variant = Some(map.next_value()?);
                        }
                        Field::HostDevice => {
                            host_device = Some(map.next_value()?);
                        }
                        Field::Network => {
                            network = Some(map.next_value()?);
                        }
                        Field::Disk => {
                            disk = Some(map.next_value()?);
                        }
                        Field::Filesystem => {
                            filesystem = Some(map.next_value()?);
                        }
                        Field::Controller => {
                            controller = Some(map.next_value()?);
                        }
                        Field::Input => {
                            input = Some(map.next_value()?);
                        }
                        Field::Graphics => {
                            graphics = Some(map.next_value()?);
                        }
                        Field::Sound => {
                            sound = Some(map.next_value()?);
                        }
                        Field::Video => {
                            video = Some(map.next_value()?);
                        }
                        Field::Smartcard => {
                            smartcard = Some(map.next_value()?);
                        }
                        Field::Redirdev => {
                            redirdev = Some(map.next_value()?);
                        }
                        Field::Memballoon => {
                            memballoon = Some(map.next_value()?);
                        }
                        Field::Tpm => {
                            tpm = Some(map.next_value()?);
                        }
                        Field::Rng => {
                            rng = Some(map.next_value()?);
                        }
                        Field::Panic => {
                            panic = Some(map.next_value()?);
                        }
                        Field::Shmem => {
                            shmem = Some(map.next_value()?);
                        }
                        Field::Memdev => {
                            memdev = Some(map.next_value()?);
                        }
                        Field::Vsock => {
                            vsock = Some(map.next_value()?);
                        }
                        Field::Iommu => {
                            iommu = Some(map.next_value()?);
                        }
                        Field::Watchdog => {
                            watchdog = Some(map.next_value()?);
                        }
                        Field::Serial => {
                            serial = Some(map.next_value()?);
                        }
                        Field::Parallel => {
                            parallel = Some(map.next_value()?);
                        }
                        Field::Channel => {
                            channel = Some(map.next_value()?);
                        }
                        Field::Console => {
                            console = Some(map.next_value()?);
                        }
                        Field::Install => {
                            install = Some(map.next_value()?);
                        }
                        Field::Cdrom => {
                            cdrom = Some(map.next_value()?);
                        }
                        Field::Location => {
                            location = Some(map.next_value()?);
                        }
                        Field::Pxe => {
                            pxe = Some(map.next_value()?);
                        }
                        Field::Import => {
                            import_ = Some(map.next_value()?);
                        }
                        Field::Boot => {
                            boot = Some(map.next_value()?);
                        }
                        Field::Idmap => {
                            idmap = Some(map.next_value()?);
                        }
                        Field::Features => {
                            features = Some(map.next_value()?);
                        }
                        Field::Clock => {
                            clock = Some(map.next_value()?);
                        }
                        Field::LaunchSecurity => {
                            launch_security = Some(map.next_value()?);
                        }
                        Field::Numatune => {
                            numatune = Some(map.next_value()?);
                        }
                        Field::BootDev => {
                            boot_dev = Some(map.next_value()?);
                        }
                        Field::Unattended => {
                            unattended = Some(map.next_value()?);
                        }
                        Field::PrintXml => {
                            print_xml = Some(map.next_value()?);
                        }
                        Field::DryRun => {
                            dry_run = Some(map.next_value()?);
                        }
                        Field::Connect => {
                            connect = Some(map.next_value()?);
                        }
                        Field::VirtType => {
                            virt_type = Some(map.next_value()?);
                        }
                        Field::CloudInit => {
                            cloud_init = Some(map.next_value()?);
                        }
                    }
                }

                Ok(InstanceCreateParams {
                    name: name.unwrap_or_default(),
                    distro: distro.unwrap_or_default(),
                    version: version.unwrap_or_default(),
                    vmtype: vmtype.unwrap_or_default(),
                    sig: sig.unwrap_or_default(),
                    recovery_id: recovery_id.unwrap_or_default(),
                    sync,
                    memory,
                    vcpus,
                    cpu,
                    metadata,
                    os_variant,
                    host_device: host_device.unwrap_or_default(),
                    network: network.unwrap_or_default(),
                    disk: disk.unwrap_or_default(),
                    filesystem: filesystem.unwrap_or_default(),
                    controller: controller.unwrap_or_default(),
                    input: input.unwrap_or_default(),
                    graphics,
                    sound,
                    video,
                    smartcard,
                    redirdev: redirdev.unwrap_or_default(),
                    memballoon,
                    tpm,
                    rng,
                    panic,
                    shmem,
                    memdev: memdev.unwrap_or_default(),
                    vsock,
                    iommu,
                    watchdog,
                    serial: serial.unwrap_or_default(),
                    parallel: parallel.unwrap_or_default(),
                    channel: channel.unwrap_or_default(),
                    console: console.unwrap_or_default(),
                    install,
                    cdrom,
                    location,
                    pxe: pxe.unwrap_or_default(),
                    import: import_.unwrap_or_default(),
                    boot,
                    idmap,
                    features: features.unwrap_or_default(),
                    clock,
                    launch_security,
                    numatune,
                    boot_dev: boot_dev.unwrap_or_default(),
                    unattended: unattended.unwrap_or_default(),
                    print_xml,
                    dry_run: dry_run.unwrap_or_default(),
                    connect,
                    virt_type,
                    cloud_init,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &[
            "name",
            "distro",
            "version",
            "vmtype",
            "sig",
            "recovery_id",
            "sync",
            "memory",
            "vcpus",
            "cpu",
            "metadata",
            "os_variant",
            "host_device",
            "network",
            "disk",
            "filesystem",
            "controller",
            "input",
            "graphics",
            "sound",
            "video",
            "smartcard",
            "redirdev",
            "memballoon",
            "tpm",
            "rng",
            "panic",
            "shmem",
            "memdev",
            "vsock",
            "iommu",
            "watchdog",
            "serial",
            "parallel",
            "channel",
            "console",
            "install",
            "cdrom",
            "location",
            "pxe",
            "import_",
            "boot",
            "idmap",
            "features",
            "clock",
            "launch_security",
            "numatune",
            "boot_dev",
            "unattended",
            "print_xml",
            "dry_run",
            "connect",
            "virt_type",
            "cloud_init",
        ];

        deserializer.deserialize_struct("InstanceCreateParams", FIELDS, InstanceCreateParamsVisitor)
    }
}

impl Serialize for InstanceStartParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("InstanceStartParams", 5)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("console", &self.console)?;
        state.serialize_field("stateless", &self.stateless)?;
        state.serialize_field("sig", &self.sig)?;
        state.serialize_field("recovery_id", &self.recovery_id)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstanceStartParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Name,
            Console,
            Stateless,
            Sig,
            RecoveryId,
        }

        struct InstanceStartParamsVisitor;

        impl<'de> serde::de::Visitor<'de> for InstanceStartParamsVisitor {
            type Value = InstanceStartParams;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct InstanceStartParams")
            }

            fn visit_map<V>(self, mut map: V) -> Result<InstanceStartParams, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut name = None;
                let mut console = None;
                let mut stateless = None;
                let mut sig = None;
                let mut recovery_id = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            name = Some(map.next_value()?);
                        }
                        Field::Console => {
                            console = Some(map.next_value()?);
                        }
                        Field::Stateless => {
                            stateless = Some(map.next_value()?);
                        }
                        Field::Sig => {
                            sig = Some(map.next_value()?);
                        }
                        Field::RecoveryId => {
                            recovery_id = Some(map.next_value()?);
                        }
                    }
                }

                let name = name.ok_or_else(|| serde::de::Error::missing_field("name"))?;
                let console = console.ok_or_else(|| serde::de::Error::missing_field("console"))?;
                let stateless =
                    stateless.ok_or_else(|| serde::de::Error::missing_field("stateless"))?;
                let sig = sig.ok_or_else(|| serde::de::Error::missing_field("sig"))?;
                let recovery_id =
                    recovery_id.ok_or_else(|| serde::de::Error::missing_field("recovery_id"))?;

                Ok(InstanceStartParams {
                    name,
                    console,
                    stateless,
                    sig,
                    recovery_id,
                })
            }
        }

        const FIELDS: &'static [&'static str] =
            &["name", "console", "stateless", "sig", "recovery_id"];
        deserializer.deserialize_struct("InstanceStartParams", FIELDS, InstanceStartParamsVisitor)
    }
}

impl Serialize for InstanceStopParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("InstanceStopParams", 3)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("sig", &self.sig)?;
        state.serialize_field("recovery_id", &self.recovery_id)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstanceStopParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Name,
            Sig,
            RecoveryId,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`name`, `sig` or `recovery_id`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(Field::Name),
                            "sig" => Ok(Field::Sig),
                            "recovery_id" => Ok(Field::RecoveryId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct InstanceStopParamsVisitor;

        impl<'de> serde::de::Visitor<'de> for InstanceStopParamsVisitor {
            type Value = InstanceStopParams;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct InstanceStopParams")
            }

            fn visit_map<V>(self, mut map: V) -> Result<InstanceStopParams, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut name = None;
                let mut sig = None;
                let mut recovery_id = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            name = Some(map.next_value()?);
                        }
                        Field::Sig => {
                            sig = Some(map.next_value()?);
                        }
                        Field::RecoveryId => {
                            recovery_id = Some(map.next_value()?);
                        }
                    }
                }

                let name = name.ok_or_else(|| serde::de::Error::missing_field("name"))?;
                let sig = sig.ok_or_else(|| serde::de::Error::missing_field("sig"))?;
                let recovery_id =
                    recovery_id.ok_or_else(|| serde::de::Error::missing_field("recovery_id"))?;

                Ok(InstanceStopParams {
                    name,
                    sig,
                    recovery_id,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &["name", "sig", "recovery_id"];
        deserializer.deserialize_struct("InstanceStopParams", FIELDS, InstanceStopParamsVisitor)
    }
}

impl Serialize for InstanceDeleteParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("InstanceDeleteParams", 5)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("force", &self.force)?;
        state.serialize_field("interactive", &self.interactive)?;
        state.serialize_field("sig", &self.sig)?;
        state.serialize_field("recovery_id", &self.recovery_id)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstanceDeleteParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Name,
            Force,
            Interactive,
            Sig,
            RecoveryId,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter
                            .write_str("`name`, `force`, `interactive`, `sig` or `recovery_id`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(Field::Name),
                            "force" => Ok(Field::Force),
                            "interactive" => Ok(Field::Interactive),
                            "sig" => Ok(Field::Sig),
                            "recovery_id" => Ok(Field::RecoveryId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct InstanceDeleteParamsVisitor;

        impl<'de> serde::de::Visitor<'de> for InstanceDeleteParamsVisitor {
            type Value = InstanceDeleteParams;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct InstanceDeleteParams")
            }

            fn visit_map<V>(self, mut map: V) -> Result<InstanceDeleteParams, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut name = None;
                let mut force = None;
                let mut interactive = None;
                let mut sig = None;
                let mut recovery_id = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            name = Some(map.next_value()?);
                        }
                        Field::Force => {
                            force = Some(map.next_value()?);
                        }
                        Field::Interactive => {
                            interactive = Some(map.next_value()?);
                        }
                        Field::Sig => {
                            sig = Some(map.next_value()?);
                        }
                        Field::RecoveryId => {
                            recovery_id = Some(map.next_value()?);
                        }
                    }
                }

                let name = name.ok_or_else(|| serde::de::Error::missing_field("name"))?;
                let force = force.ok_or_else(|| serde::de::Error::missing_field("force"))?;
                let interactive =
                    interactive.ok_or_else(|| serde::de::Error::missing_field("interactive"))?;
                let sig = sig.ok_or_else(|| serde::de::Error::missing_field("sig"))?;
                let recovery_id =
                    recovery_id.ok_or_else(|| serde::de::Error::missing_field("recovery_id"))?;

                Ok(InstanceDeleteParams {
                    name,
                    force,
                    interactive,
                    sig,
                    recovery_id,
                })
            }
        }

        const FIELDS: &'static [&'static str] =
            &["name", "force", "interactive", "sig", "recovery_id"];
        deserializer.deserialize_struct("InstanceDeleteParams", FIELDS, InstanceDeleteParamsVisitor)
    }
}

impl Serialize for InstanceExposeServiceParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("InstanceExposeServiceParams", 5)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("port", &self.port)?;
        state.serialize_field("service_type", &self.service_type)?;
        state.serialize_field("sig", &self.sig)?;
        state.serialize_field("recovery_id", &self.recovery_id)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstanceExposeServiceParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Name,
            Port,
            ServiceType,
            Sig,
            RecoveryId,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter
                            .write_str("`name`, `port`, `service_type`, `sig` or `recovery_id`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(Field::Name),
                            "port" => Ok(Field::Port),
                            "service_type" => Ok(Field::ServiceType),
                            "sig" => Ok(Field::Sig),
                            "recovery_id" => Ok(Field::RecoveryId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct InstanceExposeServiceParamsVisitor;

        impl<'de> serde::de::Visitor<'de> for InstanceExposeServiceParamsVisitor {
            type Value = InstanceExposeServiceParams;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct InstanceExposeServiceParams")
            }

            fn visit_map<V>(self, mut map: V) -> Result<InstanceExposeServiceParams, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut name = None;
                let mut port = None;
                let mut service_type = None;
                let mut sig = None;
                let mut recovery_id = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            name = Some(map.next_value()?);
                        }
                        Field::Port => {
                            port = Some(map.next_value()?);
                        }
                        Field::ServiceType => {
                            service_type = Some(map.next_value()?);
                        }
                        Field::Sig => {
                            sig = Some(map.next_value()?);
                        }
                        Field::RecoveryId => {
                            recovery_id = Some(map.next_value()?);
                        }
                    }
                }

                let name = name.ok_or_else(|| serde::de::Error::missing_field("name"))?;
                let port = port.ok_or_else(|| serde::de::Error::missing_field("port"))?;
                let service_type =
                    service_type.ok_or_else(|| serde::de::Error::missing_field("service_type"))?;
                let sig = sig.ok_or_else(|| serde::de::Error::missing_field("sig"))?;
                let recovery_id =
                    recovery_id.ok_or_else(|| serde::de::Error::missing_field("recovery_id"))?;

                Ok(InstanceExposeServiceParams {
                    name,
                    port,
                    service_type,
                    sig,
                    recovery_id,
                })
            }
        }

        const FIELDS: &'static [&'static str] =
            &["name", "port", "service_type", "sig", "recovery_id"];
        deserializer.deserialize_struct(
            "InstanceExposeServiceParams",
            FIELDS,
            InstanceExposeServiceParamsVisitor,
        )
    }
}

impl Serialize for InstanceAddPubkeyParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("InstanceAddPubkeyParams", 4)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("pubkey", &self.pubkey)?;
        state.serialize_field("sig", &self.sig)?;
        state.serialize_field("recovery_id", &self.recovery_id)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstanceAddPubkeyParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Name,
            Pubkey,
            Sig,
            RecoveryId,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`name`, `pubkey`, `sig` or `recovery_id`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(Field::Name),
                            "pubkey" => Ok(Field::Pubkey),
                            "sig" => Ok(Field::Sig),
                            "recovery_id" => Ok(Field::RecoveryId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct InstanceAddPubkeyParamsVisitor;

        impl<'de> serde::de::Visitor<'de> for InstanceAddPubkeyParamsVisitor {
            type Value = InstanceAddPubkeyParams;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct InstanceAddPubkeyParams")
            }

            fn visit_map<V>(self, mut map: V) -> Result<InstanceAddPubkeyParams, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut name = None;
                let mut pubkey = None;
                let mut sig = None;
                let mut recovery_id = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            name = Some(map.next_value()?);
                        }
                        Field::Pubkey => {
                            pubkey = Some(map.next_value()?);
                        }
                        Field::Sig => {
                            sig = Some(map.next_value()?);
                        }
                        Field::RecoveryId => {
                            recovery_id = Some(map.next_value()?);
                        }
                    }
                }

                let name = name.ok_or_else(|| serde::de::Error::missing_field("name"))?;
                let pubkey = pubkey.ok_or_else(|| serde::de::Error::missing_field("pubkey"))?;
                let sig = sig.ok_or_else(|| serde::de::Error::missing_field("sig"))?;
                let recovery_id =
                    recovery_id.ok_or_else(|| serde::de::Error::missing_field("recovery_id"))?;

                Ok(InstanceAddPubkeyParams {
                    name,
                    pubkey,
                    sig,
                    recovery_id,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &["name", "pubkey", "sig", "recovery_id"];
        deserializer.deserialize_struct(
            "InstanceAddPubkeyParams",
            FIELDS,
            InstanceAddPubkeyParamsVisitor,
        )
    }
}

impl Serialize for InstanceGetSshDetails {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("InstanceGetSshDetails", 4)?;
        state.serialize_field("owner", &self.owner)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("keypath", &self.keypath)?;
        state.serialize_field("username", &self.username)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstanceGetSshDetails {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Owner,
            Name,
            Keypath,
            Username,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`owner`, `name`, `keypath` or `username`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "owner" => Ok(Field::Owner),
                            "name" => Ok(Field::Name),
                            "keypath" => Ok(Field::Keypath),
                            "username" => Ok(Field::Username),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct InstanceGetSshDetailsVisitor;

        impl<'de> serde::de::Visitor<'de> for InstanceGetSshDetailsVisitor {
            type Value = InstanceGetSshDetails;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct InstanceGetSshDetails")
            }

            fn visit_map<V>(self, mut map: V) -> Result<InstanceGetSshDetails, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut owner = None;
                let mut name = None;
                let mut keypath = None;
                let mut username = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Owner => {
                            owner = Some(map.next_value()?);
                        }
                        Field::Name => {
                            name = Some(map.next_value()?);
                        }
                        Field::Keypath => {
                            keypath = Some(map.next_value()?);
                        }
                        Field::Username => {
                            username = Some(map.next_value()?);
                        }
                    }
                }

                let owner = owner.ok_or_else(|| serde::de::Error::missing_field("owner"))?;
                let name = name.ok_or_else(|| serde::de::Error::missing_field("name"))?;
                let keypath = keypath;
                let username = username;

                Ok(InstanceGetSshDetails {
                    owner,
                    name,
                    keypath,
                    username,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &["owner", "name", "keypath", "username"];
        deserializer.deserialize_struct(
            "InstanceGetSshDetails",
            FIELDS,
            InstanceGetSshDetailsVisitor,
        )
    }
}

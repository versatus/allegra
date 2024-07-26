use clap::ValueEnum;
use serde::{Serialize, Deserialize};
use crate::allegra_rpc::{
    GetTaskStatusRequest,
    InstanceAddPubkeyParams,
    InstanceCreateParams,
    InstanceDeleteParams,
    InstanceExposeServiceParams,
    InstanceGetSshDetails,
    InstanceStartParams,
    InstanceStopParams, 
    ServiceType as ProtoServiceType
};

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
    Custom
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
            Err(_) => ServiceType::Custom
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
            let bytes = hex::decode(&self.owner[2..]).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            buffer.copy_from_slice(&bytes[..]);
        } else {
            let bytes = hex::decode(&self.owner).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            buffer.copy_from_slice(&bytes[..]);
        }

        return Ok(buffer)
    }
}

impl HasOwner for GetTaskStatusRequest {
    fn owner(&self) -> std::io::Result<[u8; 20]> {
        let mut buffer = [0u8; 20];
        if self.owner.starts_with("0x") {
            let owner_string = &self.owner[2..];
            let bytes = hex::decode(owner_string).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            buffer.copy_from_slice(&bytes[..]);
        } else {
            let bytes = hex::decode(&self.owner).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            buffer.copy_from_slice(&bytes[..]);
        }

        Ok(buffer)
    }
}

impl Serialize for InstanceCreateParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        todo!()
    }
}

impl Serialize for InstanceStartParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        todo!()
    }
}

impl Serialize for InstanceStopParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        todo!()
    }
}

impl Serialize for InstanceDeleteParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        todo!()
    }
}

impl Serialize for InstanceExposeServiceParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        todo!()
    }
}

impl Serialize for InstanceGetSshDetails {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        todo!()
    }
}


impl Serialize for InstanceAddPubkeyParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        todo!()
    }
}


impl<'de> Deserialize<'de> for InstanceCreateParams {
    fn deserialize<D>(deserializer: D) -> Result<InstanceCreateParams, D::Error>
        where
            D: serde::Deserializer<'de> 
        {

            todo!()
        
    }
}

impl<'de> Deserialize<'de> for InstanceStartParams {
    fn deserialize<D>(deserializer: D) -> Result<InstanceStartParams, D::Error>
        where
            D: serde::Deserializer<'de> 
        {

            todo!()
        
    }
}

impl<'de> Deserialize<'de> for InstanceStopParams  {
    fn deserialize<D>(deserializer: D) -> Result<InstanceStopParams, D::Error>
        where
            D: serde::Deserializer<'de> 
        {

            todo!()
        
    }
}

impl<'de> Deserialize<'de> for InstanceDeleteParams {
    fn deserialize<D>(deserializer: D) -> Result<InstanceDeleteParams, D::Error>
        where
            D: serde::Deserializer<'de> 
        {

            todo!()
        
    }
}

impl<'de> Deserialize<'de> for InstanceGetSshDetails {
    fn deserialize<D>(deserializer: D) -> Result<InstanceGetSshDetails, D::Error>
        where
            D: serde::Deserializer<'de> 
        {

            todo!()
        
    }
}

impl<'de> Deserialize<'de> for InstanceExposeServiceParams {
    fn deserialize<D>(deserializer: D) -> Result<InstanceExposeServiceParams, D::Error>
        where
            D: serde::Deserializer<'de> 
        {

            todo!()
        
    }
}

impl<'de> Deserialize<'de> for InstanceAddPubkeyParams {
    fn deserialize<D>(deserializer: D) -> Result<InstanceAddPubkeyParams, D::Error>
        where
            D: serde::Deserializer<'de> 
        {

            todo!()
        
    }
}


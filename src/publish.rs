use std::str::FromStr;
use conductor::publisher::PubStream;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use derive_more::Display;
use crate::event::{DnsEvent, Event, GeneralResponseEvent, IntoEvent, NetworkEvent, QuorumEvent, RpcResponseEvent, SerializeIntoInner, StateEvent, SyncEvent, TaskStatusEvent, VmmEvent};

macro_rules! impl_topic {
    ($($t:ty),*) => {
        $(
            impl Topic for $t {}
        )*
    };
}

impl_topic!(
    NetworkTopic,
    QuorumTopic,
    StateTopic,
    TaskStatusTopic,
    VmManagerTopic,
    SyncTopic, 
    DnsTopic
);

macro_rules! impl_from_str {
    ($($t:ty),*) => {
        $(
            impl FromStr for $t {
                type Err = std::io::Error;
                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    if s == Self::default().to_string().as_str() {
                        return Ok(Self::default())
                    } else {
                        Err(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "pattern does not match required"
                            )
                        )
                    }
                }
            }
        )*
    };
}

impl_from_str!(
    NetworkTopic,
    QuorumTopic,
    StateTopic,
    TaskStatusTopic,
    VmManagerTopic,
    SyncTopic,
    DnsTopic,
    RpcResponseTopic
);

pub trait Topic: std::fmt::Display {}

#[derive(Display, Default)]
#[display(fmt = "network")]
pub struct NetworkTopic;
#[derive(Display, Default)]
#[display(fmt = "quorum")]
pub struct QuorumTopic;
#[derive(Display, Default)]
#[display(fmt = "state")]
pub struct StateTopic;
#[derive(Display, Default)]
#[display(fmt = "task_status")]
pub struct TaskStatusTopic;
#[derive(Display, Default)]
#[display(fmt = "vmm")]
pub struct VmManagerTopic;
#[derive(Display, Default)]
#[display(fmt = "sync")]
pub struct SyncTopic;
#[derive(Display, Default)]
#[display(fmt = "dns")]
pub struct DnsTopic;
#[derive(Display, Default)]
#[display(fmt = "rpc_respose")]
pub struct RpcResponseTopic;

pub struct GenericPublisher {
    stream: TcpStream
}

impl GenericPublisher {
    pub async fn new(uri: &str) -> std::io::Result<Self> {
        Ok(Self { stream: TcpStream::connect(uri).await? })
    }

    pub fn peer_addr(&self) -> std::io::Result<String> {
        let socket_addr = self.stream.peer_addr()?;
        Ok(socket_addr.to_string())
    }
}

pub struct NetworkPublisher {
    stream: TcpStream
}

pub struct QuorumPublisher {
    stream: TcpStream
}

pub struct StatePublisher {
    stream: TcpStream
}

pub struct TaskStatusPublisher {
    stream: TcpStream
}

pub struct VmManagerPublisher {
    stream: TcpStream
}

pub struct SyncPublisher {
    stream: TcpStream
}

pub struct DnsPublisher {
    stream: TcpStream
}

pub struct RpcResponsePublisher {
    stream: TcpStream
}

pub struct GeneralResponsePublisher {
    stream: TcpStream
}

#[async_trait::async_trait]
impl PubStream for NetworkPublisher {
    type Topic = NetworkTopic;
    type Message<'async_trait> = NetworkEvent where Self: 'async_trait;

    async fn publish(&mut self, topic: Self::Topic, msg: Self::Message<'async_trait>) -> std::io::Result<()> {
        let topic_len = topic.to_string().len();
        let topic_len_bytes = topic_len.to_be_bytes();
        let message_str = serde_json::to_string(&msg).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        let message_len = message_str.len();
        let message_len_bytes = message_len.to_be_bytes();
        let total_len = conductor::HEADER_SIZE + conductor::TOPIC_SIZE_OFFSET + topic_len + message_len;
        let mut full_message = Vec::with_capacity(total_len);
        full_message.extend_from_slice(&message_len_bytes);
        full_message.extend_from_slice(&topic_len_bytes);
        full_message.extend_from_slice(&topic.to_string().as_bytes());
        full_message.extend_from_slice(message_str.as_bytes());
        self.stream.write_all(&full_message).await?;
        Ok(())
    }

}

#[async_trait::async_trait]
impl PubStream for QuorumPublisher {
    type Topic = QuorumTopic;
    type Message<'async_trait> = QuorumEvent where Self: 'async_trait;

    async fn publish(&mut self, topic: Self::Topic, msg: Self::Message<'async_trait>) -> std::io::Result<()> {
        let topic_len = topic.to_string().len();
        let topic_len_bytes = topic_len.to_be_bytes();
        let message_str = serde_json::to_string(&msg).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        let message_len = message_str.len();
        let message_len_bytes = message_len.to_be_bytes();
        let total_len = conductor::HEADER_SIZE + conductor::TOPIC_SIZE_OFFSET + topic_len + message_len;
        let mut full_message = Vec::with_capacity(total_len);
        full_message.extend_from_slice(&message_len_bytes);
        full_message.extend_from_slice(&topic_len_bytes);
        full_message.extend_from_slice(&topic.to_string().as_bytes());
        full_message.extend_from_slice(message_str.as_bytes());
        self.stream.write_all(&full_message).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl PubStream for StatePublisher {
    type Topic = StateTopic;
    type Message<'async_trait> = StateEvent where Self: 'async_trait;

    async fn publish(&mut self, topic: Self::Topic, msg: Self::Message<'async_trait>) -> std::io::Result<()> {
        let topic_len = topic.to_string().len();
        let topic_len_bytes = topic_len.to_be_bytes();
        let message_str = serde_json::to_string(&msg).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        let message_len = message_str.len();
        let message_len_bytes = message_len.to_be_bytes();
        let total_len = conductor::HEADER_SIZE + conductor::TOPIC_SIZE_OFFSET + topic_len + message_len;
        let mut full_message = Vec::with_capacity(total_len);
        full_message.extend_from_slice(&message_len_bytes);
        full_message.extend_from_slice(&topic_len_bytes);
        full_message.extend_from_slice(&topic.to_string().as_bytes());
        full_message.extend_from_slice(message_str.as_bytes());
        self.stream.write_all(&full_message).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl PubStream for TaskStatusPublisher {
    type Topic = TaskStatusTopic;
    type Message<'async_trait> = TaskStatusEvent where Self: 'async_trait;

    async fn publish(&mut self, topic: Self::Topic, msg: Self::Message<'async_trait>) -> std::io::Result<()> {
        let topic_len = topic.to_string().len();
        let topic_len_bytes = topic_len.to_be_bytes();
        let message_str = serde_json::to_string(&msg).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        let message_len = message_str.len();
        let message_len_bytes = message_len.to_be_bytes();
        let total_len = conductor::HEADER_SIZE + conductor::TOPIC_SIZE_OFFSET + topic_len + message_len;
        let mut full_message = Vec::with_capacity(total_len);
        full_message.extend_from_slice(&message_len_bytes);
        full_message.extend_from_slice(&topic_len_bytes);
        full_message.extend_from_slice(&topic.to_string().as_bytes());
        full_message.extend_from_slice(message_str.as_bytes());
        self.stream.write_all(&full_message).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl PubStream for VmManagerPublisher {
    type Topic = VmManagerTopic;
    type Message<'async_trait> = VmmEvent where Self: 'async_trait;

    async fn publish(&mut self, topic: Self::Topic, msg: Self::Message<'async_trait>) -> std::io::Result<()> {
        let topic_len = topic.to_string().len();
        let topic_len_bytes = topic_len.to_be_bytes();
        let message_str = serde_json::to_string(&msg).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        let message_len = message_str.len();
        let message_len_bytes = message_len.to_be_bytes();
        let total_len = conductor::HEADER_SIZE + conductor::TOPIC_SIZE_OFFSET + topic_len + message_len;
        let mut full_message = Vec::with_capacity(total_len);
        full_message.extend_from_slice(&message_len_bytes);
        full_message.extend_from_slice(&topic_len_bytes);
        full_message.extend_from_slice(&topic.to_string().as_bytes());
        full_message.extend_from_slice(message_str.as_bytes());
        self.stream.write_all(&full_message).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl PubStream for SyncPublisher {
    type Topic = SyncTopic;
    type Message<'async_trait> = SyncEvent where Self: 'async_trait;

    async fn publish(&mut self, topic: Self::Topic, msg: Self::Message<'async_trait>) -> std::io::Result<()> {
        let topic_len = topic.to_string().len();
        let topic_len_bytes = topic_len.to_be_bytes();
        let message_str = serde_json::to_string(&msg).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        let message_len = message_str.len();
        let message_len_bytes = message_len.to_be_bytes();
        let total_len = conductor::HEADER_SIZE + conductor::TOPIC_SIZE_OFFSET + topic_len + message_len;
        let mut full_message = Vec::with_capacity(total_len);
        full_message.extend_from_slice(&message_len_bytes);
        full_message.extend_from_slice(&topic_len_bytes);
        full_message.extend_from_slice(&topic.to_string().as_bytes());
        full_message.extend_from_slice(message_str.as_bytes());
        self.stream.write_all(&full_message).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl PubStream for DnsPublisher {
    type Topic = DnsTopic;
    type Message<'async_trait> = DnsEvent where Self: 'async_trait;

    async fn publish(&mut self, topic: Self::Topic, msg: Self::Message<'async_trait>) -> std::io::Result<()> {
        let topic_len = topic.to_string().len();
        let topic_len_bytes = topic_len.to_be_bytes();
        let message_str = serde_json::to_string(&msg).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        let message_len = message_str.len();
        let message_len_bytes = message_len.to_be_bytes();
        let total_len = conductor::HEADER_SIZE + conductor::TOPIC_SIZE_OFFSET + topic_len + message_len;
        let mut full_message = Vec::with_capacity(total_len);
        full_message.extend_from_slice(&message_len_bytes);
        full_message.extend_from_slice(&topic_len_bytes);
        full_message.extend_from_slice(&topic.to_string().as_bytes());
        full_message.extend_from_slice(message_str.as_bytes());
        self.stream.write_all(&full_message).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl PubStream for RpcResponsePublisher {
    type Topic = RpcResponseTopic;
    type Message<'async_trait> = RpcResponseEvent where Self: 'async_trait;

    async fn publish(&mut self, topic: Self::Topic, msg: Self::Message<'async_trait>) -> std::io::Result<()> {
        let topic_len = topic.to_string().len();
        let topic_len_bytes = topic_len.to_be_bytes();
        let message_str = serde_json::to_string(&msg).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        let message_len = message_str.len();
        let message_len_bytes = message_len.to_be_bytes();
        let total_len = conductor::HEADER_SIZE + conductor::TOPIC_SIZE_OFFSET + topic_len + message_len;
        let mut full_message = Vec::with_capacity(total_len);
        full_message.extend_from_slice(&message_len_bytes);
        full_message.extend_from_slice(&topic_len_bytes);
        full_message.extend_from_slice(&topic.to_string().as_bytes());
        full_message.extend_from_slice(message_str.as_bytes());
        self.stream.write_all(&full_message).await?;
        Ok(())
    }
}


#[async_trait::async_trait]
impl PubStream for GenericPublisher {
    type Topic = Box<dyn Topic + Send>;
    type Message<'async_trait> = Box<dyn IntoEvent + Send>; 

    async fn publish(&mut self, topic: Self::Topic, msg: Self::Message<'async_trait>) -> std::io::Result<()> {
        let topic_len = topic.to_string().len();
        let topic_len_bytes = topic_len.to_be_bytes();
        let event: Event = msg.to_inner().into(); 
        let message_str = event.inner_to_string()?;
        let message_len = message_str.len();
        let message_len_bytes = message_len.to_be_bytes();
        let total_len = conductor::HEADER_SIZE + conductor::TOPIC_SIZE_OFFSET + topic_len + message_len;
        let mut full_message = Vec::with_capacity(total_len);
        full_message.extend_from_slice(&message_len_bytes);
        full_message.extend_from_slice(&topic_len_bytes);
        full_message.extend_from_slice(&topic.to_string().as_bytes());
        full_message.extend_from_slice(message_str.as_bytes());

        Ok(())
    }
}

#[async_trait::async_trait]
impl PubStream for GeneralResponsePublisher {
    type Topic = String;
    type Message<'async_trait> =  GeneralResponseEvent where Self: 'async_trait;

    async fn publish(&mut self, topic: Self::Topic, msg: Self::Message<'async_trait>) -> std::io::Result<()> {
        let topic_len = topic.to_string().len();
        let topic_len_bytes = topic_len.to_be_bytes();
        let message_str = serde_json::to_string(&msg).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        let message_len = message_str.len();
        let message_len_bytes = message_len.to_be_bytes();
        let total_len = conductor::HEADER_SIZE + conductor::TOPIC_SIZE_OFFSET + topic_len + message_len;
        let mut full_message = Vec::with_capacity(total_len);
        full_message.extend_from_slice(&message_len_bytes);
        full_message.extend_from_slice(&topic_len_bytes);
        full_message.extend_from_slice(&topic.to_string().as_bytes());
        full_message.extend_from_slice(message_str.as_bytes());
        self.stream.write_all(&full_message).await?;
        Ok(())
    }

}

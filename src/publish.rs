use conductor::publisher::PubStream;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use derive_more::Display;
use crate::event::{DnsEvent, NetworkEvent, QuorumEvent, StateEvent, SyncEvent, TaskStatusEvent, VmmEvent, IntoEvent, Event, SerializeIntoInner};

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

pub trait Topic: std::fmt::Display {}

#[derive(Display)]
#[display(fmt = "network")]
pub struct NetworkTopic;
#[derive(Display)]
#[display(fmt = "quorum")]
pub struct QuorumTopic;
#[derive(Display)]
#[display(fmt = "state")]
pub struct StateTopic;
#[derive(Display)]
#[display(fmt = "task_status")]
pub struct TaskStatusTopic;
#[derive(Display)]
#[display(fmt = "vmm")]
pub struct VmManagerTopic;
#[derive(Display)]
#[display(fmt = "sync")]
pub struct SyncTopic;
#[derive(Display)]
#[display(fmt = "dns")]
pub struct DnsTopic;

pub struct GenericPublisher {
    stream: TcpStream
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

use tokio::net::TcpStream;
use conductor::subscriber::SubStream;
use tokio::io::AsyncReadExt;
use conductor::util::{try_get_topic_len, try_get_message_len, parse_next_message};
use conductor::{HEADER_SIZE, TOPIC_SIZE_OFFSET};
use crate::event::{DnsEvent, NetworkEvent, QuorumEvent, StateEvent, SyncEvent, TaskStatusEvent, VmmEvent};


pub struct NetworkSubscriber {
    stream: TcpStream
}

pub struct QuorumSubscriber {
    stream: TcpStream
}

pub struct StateSubscriber {
    stream: TcpStream
}

pub struct VmmSubscriber {
    stream: TcpStream
}

pub struct DnsSubscriber {
    stream: TcpStream
}

pub struct SyncSubscriber {
    stream: TcpStream
}

pub struct TaskStatusSubscriber {
    stream: TcpStream
}

#[async_trait::async_trait]
impl SubStream for NetworkSubscriber {
    type Message = Vec<NetworkEvent>;

    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 1024]; 
            let n = self.stream.read(&mut read_buffer).await.expect("unable to read stream to buffer");
            if n == 0 {
                break;
            }

            buffer.extend_from_slice(&read_buffer[..n]);
            let results = Self::parse_messages(&mut buffer).await?;
            if !results.is_empty() {
                return Ok(results)
            }
        }
        Err(
            std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "No complete messages received"
            )
        )
    }

    async fn parse_messages(msg: &mut Vec<u8>) -> std::io::Result<Self::Message> {
        let mut results = Vec::new();
        while msg.len() >= HEADER_SIZE {
            let total_len = try_get_message_len(msg)?;
            if msg.len() >= total_len {
                let topic_len = try_get_topic_len(msg)?;
                let (_, message) = parse_next_message(total_len, topic_len, msg).await;
                let message_offset = TOPIC_SIZE_OFFSET + topic_len;
                let msg = &message[message_offset..message_offset + total_len];
                results.push(msg.to_vec());
            }
        }

        let msg_results = results.iter().filter_map(|m| {
            serde_json::from_slice(
                &m
            ).ok()
        }).collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for QuorumSubscriber {
    type Message = Vec<QuorumEvent>;
    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 1024]; 
            let n = self.stream.read(&mut read_buffer).await.expect("unable to read stream to buffer");
            if n == 0 {
                break;
            }

            buffer.extend_from_slice(&read_buffer[..n]);
            let results = Self::parse_messages(&mut buffer).await?;
            if !results.is_empty() {
                return Ok(results)
            }
        }
        Err(
            std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "No complete messages received"
            )
        )
    }

    async fn parse_messages(msg: &mut Vec<u8>) -> std::io::Result<Self::Message> {
        let mut results = Vec::new();
        while msg.len() >= HEADER_SIZE {
            let total_len = try_get_message_len(msg)?;
            if msg.len() >= total_len {
                let topic_len = try_get_topic_len(msg)?;
                let (_, message) = parse_next_message(total_len, topic_len, msg).await;
                let message_offset = TOPIC_SIZE_OFFSET + topic_len;
                let msg = &message[message_offset..message_offset + total_len];
                results.push(msg.to_vec());
            }
        }

        let msg_results = results.iter().filter_map(|m| {
            serde_json::from_slice(
                &m
            ).ok()
        }).collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for StateSubscriber {
    type Message = Vec<StateEvent>;
    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 1024]; 
            let n = self.stream.read(&mut read_buffer).await.expect("unable to read stream to buffer");
            if n == 0 {
                break;
            }

            buffer.extend_from_slice(&read_buffer[..n]);
            let results = Self::parse_messages(&mut buffer).await?;
            if !results.is_empty() {
                return Ok(results)
            }
        }
        Err(
            std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "No complete messages received"
            )
        )
    }

    async fn parse_messages(msg: &mut Vec<u8>) -> std::io::Result<Self::Message> {
        let mut results = Vec::new();
        while msg.len() >= HEADER_SIZE {
            let total_len = try_get_message_len(msg)?;
            if msg.len() >= total_len {
                let topic_len = try_get_topic_len(msg)?;
                let (_, message) = parse_next_message(total_len, topic_len, msg).await;
                let message_offset = TOPIC_SIZE_OFFSET + topic_len;
                let msg = &message[message_offset..message_offset + total_len];
                results.push(msg.to_vec());
            }
        }

        let msg_results = results.iter().filter_map(|m| {
            serde_json::from_slice(
                &m
            ).ok()
        }).collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for VmmSubscriber {
    type Message = Vec<VmmEvent>;
    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 1024]; 
            let n = self.stream.read(&mut read_buffer).await.expect("unable to read stream to buffer");
            if n == 0 {
                break;
            }

            buffer.extend_from_slice(&read_buffer[..n]);
            let results = Self::parse_messages(&mut buffer).await?;
            if !results.is_empty() {
                return Ok(results)
            }
        }
        Err(
            std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "No complete messages received"
            )
        )
    }

    async fn parse_messages(msg: &mut Vec<u8>) -> std::io::Result<Self::Message> {
        let mut results = Vec::new();
        while msg.len() >= HEADER_SIZE {
            let total_len = try_get_message_len(msg)?;
            if msg.len() >= total_len {
                let topic_len = try_get_topic_len(msg)?;
                let (_, message) = parse_next_message(total_len, topic_len, msg).await;
                let message_offset = TOPIC_SIZE_OFFSET + topic_len;
                let msg = &message[message_offset..message_offset + total_len];
                results.push(msg.to_vec());
            }
        }

        let msg_results = results.iter().filter_map(|m| {
            serde_json::from_slice(
                &m
            ).ok()
        }).collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for DnsSubscriber {
    type Message = Vec<DnsEvent>;
    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 1024]; 
            let n = self.stream.read(&mut read_buffer).await.expect("unable to read stream to buffer");
            if n == 0 {
                break;
            }

            buffer.extend_from_slice(&read_buffer[..n]);
            let results = Self::parse_messages(&mut buffer).await?;
            if !results.is_empty() {
                return Ok(results)
            }
        }
        Err(
            std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "No complete messages received"
            )
        )
    }

    async fn parse_messages(msg: &mut Vec<u8>) -> std::io::Result<Self::Message> {
        let mut results = Vec::new();
        while msg.len() >= HEADER_SIZE {
            let total_len = try_get_message_len(msg)?;
            if msg.len() >= total_len {
                let topic_len = try_get_topic_len(msg)?;
                let (_, message) = parse_next_message(total_len, topic_len, msg).await;
                let message_offset = TOPIC_SIZE_OFFSET + topic_len;
                let msg = &message[message_offset..message_offset + total_len];
                results.push(msg.to_vec());
            }
        }

        let msg_results = results.iter().filter_map(|m| {
            serde_json::from_slice(
                &m
            ).ok()
        }).collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for SyncSubscriber {
    type Message = Vec<SyncEvent>;
    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 1024]; 
            let n = self.stream.read(&mut read_buffer).await.expect("unable to read stream to buffer");
            if n == 0 {
                break;
            }

            buffer.extend_from_slice(&read_buffer[..n]);
            let results = Self::parse_messages(&mut buffer).await?;
            if !results.is_empty() {
                return Ok(results)
            }
        }
        Err(
            std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "No complete messages received"
            )
        )
    }

    async fn parse_messages(msg: &mut Vec<u8>) -> std::io::Result<Self::Message> {
        let mut results = Vec::new();
        while msg.len() >= HEADER_SIZE {
            let total_len = try_get_message_len(msg)?;
            if msg.len() >= total_len {
                let topic_len = try_get_topic_len(msg)?;
                let (_, message) = parse_next_message(total_len, topic_len, msg).await;
                let message_offset = TOPIC_SIZE_OFFSET + topic_len;
                let msg = &message[message_offset..message_offset + total_len];
                results.push(msg.to_vec());
            }
        }

        let msg_results = results.iter().filter_map(|m| {
            serde_json::from_slice(
                &m
            ).ok()
        }).collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for TaskStatusSubscriber {
    type Message = Vec<TaskStatusEvent>;
    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 1024]; 
            let n = self.stream.read(&mut read_buffer).await.expect("unable to read stream to buffer");
            if n == 0 {
                break;
            }

            buffer.extend_from_slice(&read_buffer[..n]);
            let results = Self::parse_messages(&mut buffer).await?;
            if !results.is_empty() {
                return Ok(results)
            }
        }
        Err(
            std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "No complete messages received"
            )
        )
    }

    async fn parse_messages(msg: &mut Vec<u8>) -> std::io::Result<Self::Message> {
        let mut results = Vec::new();
        while msg.len() >= HEADER_SIZE {
            let total_len = try_get_message_len(msg)?;
            if msg.len() >= total_len {
                let topic_len = try_get_topic_len(msg)?;
                let (_, message) = parse_next_message(total_len, topic_len, msg).await;
                let message_offset = TOPIC_SIZE_OFFSET + topic_len;
                let msg = &message[message_offset..message_offset + total_len];
                results.push(msg.to_vec());
            }
        }

        let msg_results = results.iter().filter_map(|m| {
            serde_json::from_slice(
                &m
            ).ok()
        }).collect();

        Ok(msg_results)
    }
}

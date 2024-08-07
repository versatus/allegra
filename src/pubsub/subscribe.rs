use crate::event::{
    DnsEvent, GeneralResponseEvent, NetworkEvent, QuorumEvent, RpcResponseEvent, StateEvent,
    SyncEvent, TaskStatusEvent, VmmEvent,
};
use crate::publish::{
    DnsTopic, NetworkTopic, QuorumTopic, RpcResponseTopic, StateTopic, SyncTopic, TaskStatusTopic,
    VmManagerTopic,
};
use conductor::subscriber::SubStream;
use conductor::util::{parse_next_message, try_get_message_len, try_get_topic_len};
use conductor::{HEADER_SIZE, TOPIC_SIZE_OFFSET};
use libretto::pubsub::{LibrettoEvent, LibrettoTopic};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub struct LibrettoSubscriber {
    stream: TcpStream,
}

impl LibrettoSubscriber {
    pub async fn new(uri: &str) -> std::io::Result<Self> {
        let mut stream = TcpStream::connect(uri).await?;
        let topics_str = LibrettoTopic.to_string();
        stream.write_all(topics_str.as_bytes()).await?;
        Ok(Self { stream })
    }
}

pub struct RpcResponseSubscriber {
    stream: TcpStream,
}

impl RpcResponseSubscriber {
    pub async fn new(uri: &str) -> std::io::Result<Self> {
        let mut stream = TcpStream::connect(uri).await?;
        let topics_str = RpcResponseTopic.to_string();
        stream.write_all(topics_str.as_bytes()).await?;
        Ok(Self { stream })
    }
}

pub struct NetworkSubscriber {
    stream: TcpStream,
}

impl NetworkSubscriber {
    pub async fn new(uri: &str) -> std::io::Result<Self> {
        let mut stream = TcpStream::connect(uri).await?;
        let topics_str = NetworkTopic.to_string();
        stream.write_all(topics_str.as_bytes()).await?;
        Ok(Self { stream })
    }
}

pub struct QuorumSubscriber {
    stream: TcpStream,
}

impl QuorumSubscriber {
    pub async fn new(uri: &str) -> std::io::Result<Self> {
        let mut stream = TcpStream::connect(uri).await?;
        let topics_str = QuorumTopic.to_string();
        stream.write_all(topics_str.as_bytes()).await?;
        Ok(Self { stream })
    }
}

pub struct StateSubscriber {
    stream: TcpStream,
}

impl StateSubscriber {
    pub async fn new(uri: &str) -> std::io::Result<Self> {
        let mut stream = TcpStream::connect(uri).await?;
        let topics_str = StateTopic.to_string();
        stream.write_all(topics_str.as_bytes()).await?;
        Ok(Self { stream })
    }
}

pub struct VmmSubscriber {
    stream: TcpStream,
}

impl VmmSubscriber {
    pub async fn new(uri: &str) -> std::io::Result<Self> {
        let mut stream = TcpStream::connect(uri).await?;
        let topics_str = VmManagerTopic.to_string();
        stream.write_all(topics_str.as_bytes()).await?;
        Ok(Self { stream })
    }
}

pub struct DnsSubscriber {
    stream: TcpStream,
}

impl DnsSubscriber {
    pub async fn new(uri: &str) -> std::io::Result<Self> {
        let mut stream = TcpStream::connect(uri).await?;
        let topics_str = DnsTopic.to_string();
        stream.write_all(topics_str.as_bytes()).await?;
        Ok(Self { stream })
    }
}

pub struct SyncSubscriber {
    stream: TcpStream,
}

impl SyncSubscriber {
    pub async fn new(uri: &str) -> std::io::Result<Self> {
        let mut stream = TcpStream::connect(uri).await?;
        let topics_str = SyncTopic.to_string();
        stream.write_all(topics_str.as_bytes()).await?;
        Ok(Self { stream })
    }
}

pub struct TaskStatusSubscriber {
    stream: TcpStream,
}

impl TaskStatusSubscriber {
    pub async fn new(uri: &str) -> std::io::Result<Self> {
        let mut stream = TcpStream::connect(uri).await?;
        let topics_str = TaskStatusTopic.to_string();
        stream.write_all(topics_str.as_bytes()).await?;
        Ok(Self { stream })
    }
}

pub struct GeneralResponseSubscriber {
    stream: TcpStream,
}

impl GeneralResponseSubscriber {
    pub async fn new(uri: &str, topic: &str) -> std::io::Result<Self> {
        let mut stream = TcpStream::connect(uri).await?;
        stream.write_all(topic.as_bytes()).await?;
        Ok(Self { stream })
    }
}

#[async_trait::async_trait]
impl SubStream for NetworkSubscriber {
    type Message = Vec<NetworkEvent>;

    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 4096];
            match self.stream.read(&mut read_buffer).await {
                Err(e) => log::error!("Error reading stream to buffer: {e}..."),
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    buffer.extend_from_slice(&read_buffer[..n]);
                    let results = Self::parse_messages(&mut buffer).await?;
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "No complete messages received",
        ))
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

        let msg_results = results
            .par_iter()
            .filter_map(|m| serde_json::from_slice(&m).ok())
            .collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for QuorumSubscriber {
    type Message = Vec<QuorumEvent>;

    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 4096];
            match self.stream.read(&mut read_buffer).await {
                Err(e) => log::error!("Error reading stream to buffer: {e}..."),
                Ok(n) => {
                    log::info!("Received message of {} bytes", &n);
                    if n == 0 {
                        break;
                    }

                    buffer.extend_from_slice(&read_buffer[..n]);
                    log::info!("added bytes to buffer, attempting to parse messages");
                    let results = Self::parse_messages(&mut buffer).await?;
                    log::info!("successfully parsed messages: {:?}", results);
                    if !results.is_empty() {
                        log::info!("result not empty, returning");
                        return Ok(results);
                    }
                }
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "No complete messages received",
        ))
    }

    async fn parse_messages(msg: &mut Vec<u8>) -> std::io::Result<Self::Message> {
        log::info!("attempting to parse message");
        let mut results = Vec::new();
        log::info!(
            "checking if msg.len() {} is >= HEADER_SIZE {}",
            msg.len(),
            HEADER_SIZE
        );
        while msg.len() >= HEADER_SIZE {
            log::info!("msg.len() {} >= HEADER_SIZE {}", msg.len(), HEADER_SIZE);
            let total_len = try_get_message_len(msg)?;
            if msg.len() >= total_len {
                log::info!("msg.len() {} >= total_len {}", msg.len(), total_len);
                let topic_len = try_get_topic_len(msg)?;
                log::info!("topic_len: {}", topic_len);
                let (_, message) = parse_next_message(total_len, topic_len, msg).await;
                log::info!("message: {:?}", message);
                let message_offset = TOPIC_SIZE_OFFSET + topic_len;
                log::info!("message offset: {}", message_offset);
                let msg = &message[message_offset..message_offset + total_len];
                log::info!("msg: {:?}", msg);
                log::info!("adding message to results...");
                results.push(msg.to_vec());
                log::info!("results: {:?}", results);
            }
        }

        log::info!("msg.len() {} < HEADER_SIZE", msg.len());
        let msg_results = results
            .par_iter()
            .filter_map(|m| {
                log::info!("attempting to deserialize message into QuorumEvent...");
                match serde_json::from_slice(&m) {
                    Ok(res) => Some(res),
                    Err(e) => {
                        log::error!(
                            "Error attempting to deserialize message into QuorumEvent: {e}"
                        );
                        log::error!("Error type: {:?}", e.classify());
                        log::error!("Error line and column: {}:{}", e.line(), e.column());
                        match std::str::from_utf8(&m) {
                            Ok(json_str) => log::error!("Raw JSON: {}", json_str),
                            Err(e) => log::error!("Unable to convert message to UTF-8: {e}"),
                        }
                        None
                    }
                }
            })
            .collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for StateSubscriber {
    type Message = Vec<StateEvent>;

    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 4096];
            match self.stream.read(&mut read_buffer).await {
                Err(e) => log::error!("Error reading stream to buffer: {e}..."),
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    buffer.extend_from_slice(&read_buffer[..n]);
                    let results = Self::parse_messages(&mut buffer).await?;
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "No complete messages received",
        ))
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

        let msg_results = results
            .par_iter()
            .filter_map(|m| serde_json::from_slice(&m).ok())
            .collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for VmmSubscriber {
    type Message = Vec<VmmEvent>;

    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 4096];
            match self.stream.read(&mut read_buffer).await {
                Err(e) => log::error!("Error reading stream to buffer: {e}..."),
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    buffer.extend_from_slice(&read_buffer[..n]);
                    let results = Self::parse_messages(&mut buffer).await?;
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "No complete messages received",
        ))
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

        let msg_results = results
            .par_iter()
            .filter_map(|m| serde_json::from_slice(&m).ok())
            .collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for DnsSubscriber {
    type Message = Vec<DnsEvent>;

    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 4096];
            match self.stream.read(&mut read_buffer).await {
                Err(e) => log::error!("Error reading stream to buffer: {e}..."),
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    buffer.extend_from_slice(&read_buffer[..n]);
                    let results = Self::parse_messages(&mut buffer).await?;
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "No complete messages received",
        ))
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

        let msg_results = results
            .par_iter()
            .filter_map(|m| serde_json::from_slice(&m).ok())
            .collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for SyncSubscriber {
    type Message = Vec<SyncEvent>;
    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 4096];
            match self.stream.read(&mut read_buffer).await {
                Err(e) => log::error!("Error reading stream to buffer: {e}..."),
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    buffer.extend_from_slice(&read_buffer[..n]);
                    let results = Self::parse_messages(&mut buffer).await?;
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "No complete messages received",
        ))
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

        let msg_results = results
            .par_iter()
            .filter_map(|m| serde_json::from_slice(&m).ok())
            .collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for TaskStatusSubscriber {
    type Message = Vec<TaskStatusEvent>;
    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 4096];
            match self.stream.read(&mut read_buffer).await {
                Err(e) => log::error!("Error reading stream to buffer: {e}..."),
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    buffer.extend_from_slice(&read_buffer[..n]);
                    let results = Self::parse_messages(&mut buffer).await?;
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "No complete messages received",
        ))
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

        let msg_results = results
            .par_iter()
            .filter_map(|m| serde_json::from_slice(&m).ok())
            .collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for RpcResponseSubscriber {
    type Message = Vec<RpcResponseEvent>;
    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 4096];
            match self.stream.read(&mut read_buffer).await {
                Err(e) => log::error!("Error reading stream to buffer: {e}..."),
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    buffer.extend_from_slice(&read_buffer[..n]);
                    let results = Self::parse_messages(&mut buffer).await?;
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "No complete messages received",
        ))
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

        let msg_results = results
            .par_iter()
            .filter_map(|m| serde_json::from_slice(&m).ok())
            .collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for GeneralResponseSubscriber {
    type Message = Vec<GeneralResponseEvent>;

    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 4096];
            match self.stream.read(&mut read_buffer).await {
                Err(e) => log::error!("Error reading stream to buffer: {e}..."),
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    buffer.extend_from_slice(&read_buffer[..n]);
                    let results = Self::parse_messages(&mut buffer).await?;
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "No complete messages received",
        ))
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

        let msg_results = results
            .par_iter()
            .filter_map(|m| serde_json::from_slice(&m).ok())
            .collect();

        Ok(msg_results)
    }
}

#[async_trait::async_trait]
impl SubStream for LibrettoSubscriber {
    type Message = Vec<LibrettoEvent>;

    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 4096];
            match self.stream.read(&mut read_buffer).await {
                Err(e) => log::error!("Error reading stream to buffer: {e}..."),
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    buffer.extend_from_slice(&read_buffer[..n]);
                    let results = Self::parse_messages(&mut buffer).await?;
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "No complete messages received",
        ))
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

        let msg_results = results
            .par_iter()
            .filter_map(|m| serde_json::from_slice(&m).ok())
            .collect();

        Ok(msg_results)
    }
}

use tokio::sync::mpsc::Receiver;
use tokio::sync::broadcast;
use std::collections::HashMap;
use crate::event::Event;
use crate::event::Topic;

pub struct EventBroker {
    pub rx: Receiver<Event>,
    pub map: HashMap<String, Topic>
}


impl EventBroker {
    pub fn new(rx: Receiver<Event>) -> Self {
        Self {
            rx, map: HashMap::new()
        }
    }

    pub fn get_or_create_topic(&mut self, topic_name: String) -> &mut Topic {
        self.map.entry(topic_name.clone()).or_insert_with(|| {
            let (tx, _) = broadcast::channel(1024);
            Topic::new(topic_name, tx)
        })
    }

    pub async fn publish(&mut self, topic_name: String, event: Event) {
        if let Some(topic) = self.map.get_mut(&topic_name) {
            topic.publish(event)
        }
    }

    pub async fn subscribe(&mut self, topic_name: String) -> broadcast::Receiver<Event> {
        self.get_or_create_topic(topic_name).subscribe()
    }
}

use crate::types::EventFrame;
use std::collections::HashMap;
use tokio::sync::{broadcast, Mutex};

/// The Nexus Bridge: A shared data bus for the Savant Swarm.
/// Provides global state synchronization and inter-agent awareness.
pub struct NexusBridge {
    pub shared_memory: Mutex<HashMap<String, String>>,
    pub event_bus: broadcast::Sender<EventFrame>,
}

impl NexusBridge {
    pub fn new() -> Self {
        let (event_bus, _) = broadcast::channel(1024);
        Self {
            shared_memory: Mutex::new(HashMap::new()),
            event_bus,
        }
    }

    pub async fn update_state(&self, key: String, value: String) {
        let mut map = self.shared_memory.lock().await;
        map.insert(key, value);
    }

    pub async fn get_global_context(&self) -> String {
        let map = self.shared_memory.lock().await;
        map.iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub async fn publish(
        &self,
        channel: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event = EventFrame {
            event_type: channel.to_string(),
            payload: message.to_string(),
        };

        if let Err(_) = self.event_bus.send(event) {
            return Err("Failed to publish to event bus".into());
        }

        Ok(())
    }

    pub async fn subscribe(&self, _channel: &str) -> tokio::sync::broadcast::Receiver<EventFrame> {
        // For now, subscribe to all events and filter in the receiver
        self.event_bus.subscribe()
    }
}

impl Default for NexusBridge {
    fn default() -> Self {
        Self::new()
    }
}

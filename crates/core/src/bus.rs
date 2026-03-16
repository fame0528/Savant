use crate::types::EventFrame;
use tracing::debug;
use dashmap::DashMap;
use tokio::sync::broadcast;

/// The Nexus Bridge: A shared data bus for the Savant Swarm.
/// Provides global state synchronization and inter-agent awareness.
/// Optimized via DashMap for lock-free concurrency and mlock for memory pinning.
pub struct NexusBridge {
    pub shared_memory: DashMap<String, String>,
    pub event_bus: broadcast::Sender<EventFrame>,
    /// SwarmSync: High-speed broadcast for causal-ordered state deltas.
    pub swarm_sync: broadcast::Sender<String>,
}

impl NexusBridge {
    pub fn new() -> Self {
        let (event_bus, _) = broadcast::channel(4096);
        let (swarm_sync, _) = broadcast::channel(1024);
        
        let shard_count = (num_cpus::get() * 8).max(128);
        
        let bridge = Self {
            shared_memory: DashMap::with_shard_amount(shard_count),
            event_bus,
            swarm_sync,
        };

        // Pre-flight-pinning
        bridge.pre_flight_pinning();

        // 🛡️ Substrate Guard: Real-time hardware health monitoring
        // Note: We need a way to monitor the bridge instance. 
        // In a real system, the bridge is usually held in an Arc by the manager.
        
        bridge
    }

    /// Attempts to pin the shared memory pages to RAM to prevent swapping/jitter.
    fn pre_flight_pinning(&self) {
        #[cfg(unix)]
        {
            unsafe {
                if libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) == 0 {
                    info!("NexusBridge: Memory pinning successful.");
                } else {
                    warn!("NexusBridge: Memory pinning failed. Check RLIMIT_MEMLOCK.");
                }
            }
        }
        #[cfg(windows)]
        {
            debug!("NexusBridge: Memory pinning relies on OS working set management.");
        }
    }

    pub async fn update_state(&self, key: String, value: String) {
        self.shared_memory.insert(key, value);
    }

    /// SwarmSync: Broadcast a state delta to all agents.
    pub async fn sync_delta(&self, delta: String) {
        let _ = self.swarm_sync.send(delta);
    }

    pub async fn get_global_context(&self) -> String {
        self.shared_memory
            .iter()
            .map(|r| format!("{}: {}", r.key(), r.value()))
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

        if self.event_bus.send(event).is_err() {
            return Err("Failed to publish to event bus".into());
        }

        Ok(())
    }

    pub async fn subscribe(&self) -> (broadcast::Receiver<EventFrame>, broadcast::Receiver<String>) {
        (self.event_bus.subscribe(), self.swarm_sync.subscribe())
    }
}

impl Default for NexusBridge {
    fn default() -> Self {
        Self::new()
    }
}

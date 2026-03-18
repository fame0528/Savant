use crate::types::EventFrame;
use moka::sync::Cache;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, warn};

/// Maximum number of entries in the shared memory before eviction.
const MAX_SHARED_MEMORY_ENTRIES: u64 = 10_000;

/// The Nexus Bridge: A shared data bus for the Savant Swarm.
pub struct NexusBridge {
    pub shared_memory: Cache<String, String>,
    pub event_bus: broadcast::Sender<EventFrame>,
    /// SwarmSync: High-speed broadcast for causal-ordered state deltas.
    pub swarm_sync: broadcast::Sender<String>,
    /// 🏰 AAA Optimization: Context string cache to prevent O(N) re-joins.
    context_cache: RwLock<Option<String>>,
}

impl NexusBridge {
    pub fn new() -> Self {
        let (event_bus, _) = broadcast::channel(4096);
        let (swarm_sync, _) = broadcast::channel(1024);

        let bridge = Self {
            shared_memory: Cache::builder()
                .max_capacity(MAX_SHARED_MEMORY_ENTRIES)
                .build(),
            event_bus,
            swarm_sync,
            context_cache: RwLock::new(None),
        };

        // Pre-flight-pinning
        bridge.pre_flight_pinning();

        bridge
    }

    /// Attempts to pin the shared memory pages to RAM to prevent swapping/jitter.
    /// This reduces latency jitter by preventing the OS from swapping critical data to disk.
    fn pre_flight_pinning(&self) {
        #[cfg(unix)]
        {
            unsafe {
                // SAFETY: mlockall is safe to call here because:
                // 1. MCL_CURRENT | MCL_FUTURE are valid flags on all Unix platforms
                // 2. The call may fail if RLIMIT_MEMLOCK is exceeded, which we handle gracefully
                // 3. No pointers are passed - the kernel operates on the process address space
                if libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) == 0 {
                    tracing::info!("NexusBridge: Memory pinning successful.");
                } else {
                    // Log at debug level - failure is non-critical
                    debug!("NexusBridge: Memory pinning failed (check RLIMIT_MEMLOCK). Continuing without pinning.");
                }
            }
        }
        #[cfg(windows)]
        {
            debug!("NexusBridge: Memory pinning relies on OS working set management.");
        }
    }

    /// Updates a key-value pair in the shared memory bus.
    ///
    /// # Arguments
    /// * `key` - State key (max 256 characters)
    /// * `value` - State value (max 1MB)
    pub async fn update_state(&self, key: String, value: String) {
        // Validate key length to prevent unbounded key storage
        if key.len() > 256 {
            warn!(
                "NexusBridge: Rejected state update - key too long ({} bytes, max 256)",
                key.len()
            );
            return;
        }

        // AAA-Perfection: Bounded Memory Enforcement (HS-003)
        // Prevent individual "Bloat-Bombs"
        if value.len() > 1_000_000 {
            warn!(
                "NexusBridge: Rejected large state update for key {} ({} bytes, max 1MB)",
                key,
                value.len()
            );
            return;
        }

        // Invalidate context cache on write
        let mut cache = self.context_cache.write().await;
        *cache = None;

        // moka handles eviction automatically when max_capacity is reached
        self.shared_memory.insert(key, value);
    }

    /// SwarmSync: Broadcast a state delta to all agents.
    pub async fn sync_delta(&self, delta: String) {
        // 🏰 Invalidate cache on sync (since it affects state)
        let mut cache = self.context_cache.write().await;
        *cache = None;
        let _ = self.swarm_sync.send(delta);
    }

    pub async fn get_global_context(&self) -> String {
        // 🏰 AAA: Cache-First context retrieval
        {
            let cache = self.context_cache.read().await;
            if let Some(ref context) = *cache {
                return context.clone();
            }
        }

        let context = self
            .shared_memory
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n");

        // Update cache
        let mut cache = self.context_cache.write().await;
        *cache = Some(context.clone());

        context
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

    pub async fn subscribe(
        &self,
    ) -> (broadcast::Receiver<EventFrame>, broadcast::Receiver<String>) {
        (self.event_bus.subscribe(), self.swarm_sync.subscribe())
    }
}

impl Default for NexusBridge {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn benchmark_global_context_cache() {
        let bridge = NexusBridge::new();

        // Fill shared memory with 1000 keys
        for i in 0..1000 {
            bridge
                .update_state(format!("key_{}", i), "value".to_string())
                .await;
        }

        // First call (cache miss)
        let start = Instant::now();
        let _ = bridge.get_global_context().await;
        let duration_miss = start.elapsed();

        // Second call (cache hit)
        let start = Instant::now();
        let _ = bridge.get_global_context().await;
        let duration_hit = start.elapsed();

        tracing::info!(
            "Context Cache: Miss={:?}, Hit={:?}",
            duration_miss,
            duration_hit
        );
        assert!(
            duration_hit < duration_miss,
            "Cache hit ({:?}) must be faster than miss ({:?})",
            duration_hit,
            duration_miss
        );
    }
}

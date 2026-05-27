//! Resource Governor — CPU/memory-aware agent spawning with adaptive concurrency.

mod adaptive;
mod monitor;
pub mod pressure;

pub use adaptive::AdaptiveSemaphore;
pub use monitor::ResourceMonitor;
pub use pressure::PressureLevel;

use savant_core::config::ResourceGovernorConfig;
use savant_core::types::AgentConfig;
use std::sync::Arc;
use tokio::sync::{Mutex, SemaphorePermit};
use tokio_util::sync::CancellationToken;

/// Orchestrates resource-aware agent spawning.
pub struct SwarmGovernor {
    pub monitor: Arc<ResourceMonitor>,
    semaphore: AdaptiveSemaphore,
    config: ResourceGovernorConfig,
    deferred_agents: Arc<Mutex<Vec<(AgentConfig, u32)>>>,
    shutdown: CancellationToken,
}

impl SwarmGovernor {
    pub fn new(config: ResourceGovernorConfig, shutdown: CancellationToken) -> Arc<Self> {
        let monitor = ResourceMonitor::new(config.clone(), shutdown.clone());
        let semaphore = AdaptiveSemaphore::new(monitor.clone(), config.clone());
        Arc::new(Self {
            monitor,
            semaphore,
            config,
            deferred_agents: Arc::new(Mutex::new(Vec::new())),
            shutdown,
        })
    }

    /// Start background tasks (monitor + adaptive adjuster).
    pub fn start(self: &Arc<Self>) -> Vec<tokio::task::JoinHandle<()>> {
        let mut handles = Vec::new();

        // Start resource monitor
        handles.push(self.monitor.start());

        // Start adaptive permit adjuster
        let gov = self.clone();
        handles.push(tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(
                gov.config.monitor_interval_secs.max(1),
            );
            loop {
                tokio::select! {
                    _ = gov.shutdown.cancelled() => break,
                    _ = tokio::time::sleep(interval) => {
                        gov.semaphore.adjust_permits().await;
                    }
                }
            }
        }));

        handles
    }

    /// Try to acquire a spawn permit. Returns None if pressure is too high.
    pub fn try_spawn(&self) -> Option<SemaphorePermit<'_>> {
        self.semaphore.try_acquire()
    }

    /// Queue an agent for deferred spawning.
    /// Uses `.lock().await` for backpressure — never silently drops.
    pub async fn defer_agent(&self, agent: AgentConfig) {
        tracing::warn!(
            "[governor] Deferring agent '{}' — {} pressure, {} permits available",
            agent.agent_name,
            self.current_pressure(),
            self.available_permits()
        );
        let mut deferred = self.deferred_agents.lock().await;
        deferred.push((agent, 0));
    }

    /// Pop next deferred agent if retries not exhausted.
    /// Uses `.lock().await` for backpressure — never silently drops.
    pub async fn pop_deferred(&self) -> Option<AgentConfig> {
        let mut deferred = self.deferred_agents.lock().await;
        if deferred.is_empty() {
            return None;
        }
        let (agent, retries) = deferred.remove(0);
        if retries >= self.config.max_deferral_retries {
            tracing::error!(
                "[governor] Dropping deferred agent '{}' — max retries ({}) exceeded",
                agent.agent_name,
                self.config.max_deferral_retries
            );
            None
        } else {
            deferred.push((agent.clone(), retries + 1));
            Some(agent)
        }
    }

    /// Current pressure level.
    pub fn current_pressure(&self) -> PressureLevel {
        self.monitor.current_pressure()
    }

    /// Current CPU and memory percentages.
    pub fn current_metrics(&self) -> (f64, f64) {
        self.monitor.current_metrics()
    }

    /// Available permits.
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Whether governor is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    fn test_config() -> ResourceGovernorConfig {
        ResourceGovernorConfig {
            enabled: true,
            monitor_interval_secs: 5,
            memory_medium_pct: 60.0,
            memory_high_pct: 80.0,
            memory_critical_pct: 92.0,
            cpu_medium_pct: 70.0,
            cpu_high_pct: 85.0,
            cpu_critical_pct: 95.0,
            max_agents_low: 16,
            max_agents_medium: 8,
            max_agents_high: 4,
            max_agents_critical: 1,
            max_deferral_retries: 3,
        }
    }

    #[test]
    fn test_pressure_ordering() {
        assert!(PressureLevel::Low < PressureLevel::Medium);
        assert!(PressureLevel::Medium < PressureLevel::High);
        assert!(PressureLevel::High < PressureLevel::Critical);
    }

    #[test]
    fn test_governor_creation() {
        let shutdown = CancellationToken::new();
        let gov = SwarmGovernor::new(test_config(), shutdown);
        assert!(gov.is_enabled());
        assert_eq!(gov.current_pressure(), PressureLevel::Low);
    }

    #[tokio::test]
    async fn test_deferred_agent_queue() {
        let shutdown = CancellationToken::new();
        let gov = SwarmGovernor::new(test_config(), shutdown);

        let agent = AgentConfig {
            agent_id: "test".into(),
            agent_name: "test".into(),
            model_provider: savant_core::types::ModelProvider::Ollama,
            api_key: None,
            env_vars: Default::default(),
            system_prompt: String::new(),
            model: None,
            heartbeat_interval: 60,
            allowed_skills: Vec::new(),
            workspace_path: std::path::PathBuf::new(),
            identity: None,
            parent_id: None,
            session_id: None,
            proactive: savant_core::config::ProactiveConfig::default(),
            llm_params: savant_core::types::LlmParams::default(),
            personality_traits: None,
            evolution_state: None,
            orchestrator_enabled: true,
        };

        gov.defer_agent(agent).await;
        let popped = gov.pop_deferred().await;
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().agent_name, "test");
    }

    #[tokio::test]
    async fn test_adaptive_semaphore_permits() {
        let shutdown = CancellationToken::new();
        let gov = SwarmGovernor::new(test_config(), shutdown);
        let _handles = gov.start();

        assert!(gov.available_permits() >= 1);
    }
}

use std::path::PathBuf;
use savant_core::error::SavantError;
use savant_core::types::AgentConfig;
use savant_core::config::Config;
use savant_core::fs::registry::AgentRegistry;

pub struct AgentManager {
    pub _config: Config,
    pub registry: AgentRegistry,
}

impl AgentManager {
    pub fn new(config: Config) -> Self {
        let workspaces_path = PathBuf::from("."); // Project root for discovery
        Self { 
            _config: config.clone(),
            registry: AgentRegistry::new(workspaces_path, config.agent_defaults),
        }
    }

    /// Boots an agent, performing setup if necessary.
    pub async fn boot_agent(&self, agent: AgentConfig) -> Result<AgentConfig, SavantError> {
        tracing::info!("Booting agent: {}", agent.agent_name);
        // Additional boot logic (e.g. WASM provisioning) can go here
        Ok(agent)
    }

    /// Discovers all agents using the unified registry.
    pub async fn discover_agents(&self) -> Result<Vec<AgentConfig>, SavantError> {
        self.registry.discover_agents()
    }
}

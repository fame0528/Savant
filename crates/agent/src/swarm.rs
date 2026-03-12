use crate::budget::TokenBudget;
use crate::context::ContextAssembler;
use crate::manager::AgentManager;
use crate::memory::MemoryManager;
use crate::providers::mgmt::OpenRouterMgmt;
use crate::providers::{LlmProvider, OpenRouterProvider};
use crate::pulse::HeartbeatPulse;
use crate::react::ReActLoop;
use reqwest::Client;
use savant_core::bus::NexusBridge;
use savant_core::db::Storage;
use savant_core::traits::SkillExecutor;
use savant_core::types::{AgentConfig, ModelProvider};
use savant_core::utils::parsing;
use std::sync::Arc;

use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// The Swarm Controller: Orchestrates autonomous agents.
pub struct SwarmController {
    nexus: Arc<NexusBridge>,
    storage: Arc<Storage>,
    manager: Arc<AgentManager>,
    agents: Vec<AgentConfig>,
    client: Client,
    handles: Mutex<HashMap<String, JoinHandle<()>>>,
    skills: Arc<HashMap<String, Arc<dyn SkillExecutor>>>,
}

impl SwarmController {
    pub fn new(
        agents: Vec<AgentConfig>,
        storage: Arc<Storage>,
        manager: Arc<AgentManager>,
        nexus: Arc<NexusBridge>,
    ) -> Self {
        // Discover all available skills once for the swarm
        let skill_path = std::path::PathBuf::from("./skills");
        let available_skills =
            savant_skills::discovery::discover_skills(&skill_path).unwrap_or_default();
        let skills: Arc<HashMap<String, Arc<dyn SkillExecutor>>> = Arc::new(
            available_skills
                .into_iter()
                .map(|s| (s.name.to_lowercase(), s.executor))
                .collect(),
        );

        Self {
            nexus,
            storage,
            manager,
            agents,
            client: Client::new(),
            handles: Mutex::new(HashMap::new()),
            skills,
        }
    }

    /// Launches the entire swarm into autonomous pulse mode.
    pub async fn ignite(&self) {
        tracing::info!("Igniting Savant swarm with {} agents...", self.agents.len());

        for agent in &self.agents {
            self.spawn_agent(agent.clone()).await;
        }
    }

    /// Spawns a single agent into the swarm.
    pub async fn spawn_agent(&self, agent_cfg: AgentConfig) {
        let agent_id = agent_cfg.agent_id.clone();
        let agent_name = agent_cfg.agent_name.clone();

        // Evacuate if already running (hot-reload)
        self.evacuate_agent(&agent_id).await;

        let nexus = self.nexus.clone();
        let storage = self.storage.clone();
        let manager = self.manager.clone();
        let client = self.client.clone();
        let skills = self.skills.clone();
        let mut agent_cfg = agent_cfg;

        let handle = tokio::spawn(async move {
            // 1. Boot Agent (Provision keys if needed)
            
            // Debug: Log initial state (Censored for security)
            let censored_api_key = agent_cfg.api_key.as_ref()
                .map(|k| if k.len() > 10 { format!("{}...", &k[0..10]) } else { "***".to_string() });
            
            tracing::info!(
                "[{}] Swarm provisioning initialized. API Key: {:?}, vars: {:?}",
                agent_name,
                censored_api_key,
                agent_cfg.env_vars.keys().collect::<Vec<_>>()
            );

            // Automated .env sync logic
            if agent_cfg.api_key.is_none() {
                if let Some(master_key) = agent_cfg.env_vars.get("OR_MASTER_KEY") {
                    let env_path = agent_cfg.workspace_path.join(".env");
                    match OpenRouterMgmt::new(master_key.clone())
                        .create_key(&agent_name)
                        .await
                    {
                        Ok(derivative_key) => {
                            let _ = std::fs::write(&env_path, format!("OPENROUTER_API_KEY={}\n", derivative_key));
                            agent_cfg.api_key = Some(derivative_key);
                        }
                        Err(_) => {
                            let _ = std::fs::write(&env_path, format!("OPENROUTER_API_KEY={}\n", master_key));
                            agent_cfg.api_key = Some(master_key.clone());
                        }
                    }
                }
            }

            agent_cfg = match manager.boot_agent(agent_cfg).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    parsing::log_agent_error(&agent_name, "Failed to boot agent", e);
                    return;
                }
            };

            // 2. Select LLM Provider
            let base_provider: Box<dyn LlmProvider> = match agent_cfg.model_provider {
                ModelProvider::OpenRouter => Box::new(OpenRouterProvider {
                    client,
                    api_key: agent_cfg.api_key.clone().unwrap_or_default(),
                    model: agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "anthropic/claude-3-sonnet".to_string()),
                }),
                _ => return,
            };

            let provider: Box<dyn LlmProvider> = Box::new(crate::providers::RetryProvider {
                inner: base_provider,
                max_retries: 3,
            });

            // 3. Filter Skills for this agent
            let agent_skills: Vec<Arc<dyn SkillExecutor>> = agent_cfg
                .allowed_skills
                .iter()
                .filter_map(|name| skills.get(&name.to_lowercase()).cloned())
                .collect();

            // 4. Initialize Memory Manager
            let memory = MemoryManager::new(agent_cfg.workspace_path.clone(), storage.clone());

            // 5. Build ReAct Loop
            let identity = agent_cfg.identity.clone().unwrap_or_default();
            let react_loop = ReActLoop::new(
                agent_cfg.agent_id.clone(),
                provider,
                agent_skills,
                ContextAssembler::new(identity, TokenBudget::new(8192)),
                TokenBudget::new(8192),
                memory,
            );

            tracing::info!("Agent {} background pulse ignited.", agent_cfg.agent_name);

            // 6. Start the Heartbeat Pulse
            let pulse = HeartbeatPulse::new(agent_cfg, nexus, storage);
            pulse.start(react_loop).await;
        });

        let mut lock = self.handles.lock().await;
        lock.insert(agent_id, handle);
    }

    /// Safely evacuates an agent from the swarm.
    pub async fn evacuate_agent(&self, agent_id: &str) {
        let mut lock = self.handles.lock().await;
        if let Some(handle) = lock.remove(agent_id) {
            tracing::info!("Evacuating agent: {}", agent_id);
            handle.abort();
        }
    }

    /// Checks the health of the swarm and returns a list of dead agents.
    pub async fn check_swarm_health(&self) -> Vec<String> {
        let mut dead_agents = Vec::new();
        let lock = self.handles.lock().await;
        for (id, handle) in lock.iter() {
            if handle.is_finished() {
                dead_agents.push(id.clone());
            }
        }
        dead_agents
    }

    pub fn nexus(&self) -> Arc<NexusBridge> {
        self.nexus.clone()
    }
}

//
//
use crate::manager::AgentManager;
use crate::providers::mgmt::OpenRouterMgmt;
use crate::providers::{OpenRouterProvider, RetryProvider};
use crate::pulse::HeartbeatPulse;
use crate::react::AgentLoop;
use reqwest::Client;
use savant_core::bus::NexusBridge;
use savant_core::db::Storage;
use savant_core::traits::{LlmProvider, MemoryBackend, Tool};
use savant_ipc::{SwarmBlackboard, CollectiveBlackboard};
#[cfg(kani)]
// pub mod proofs; // Placeholder for future agent-specific proofs
use savant_security::{SecurityEnclave, SecurityError};
use savant_core::error::SavantError;
use savant_core::types::{AgentConfig, ModelProvider};
use savant_core::utils::parsing;
use savant_memory::{AsyncMemoryBackend, MemoryEngine};
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use savant_echo::{HotSwappableRegistry, EchoCompiler, ComponentMetrics};
use crate::plugins::WasmToolHost;
use tokio_util::sync::CancellationToken;

const WORKSPACE_ROOT: &str = "./workspaces";
const MEMORY_DB_PATH: &str = "./data/memory";
const SKILLS_PATH: &str = "./skills";



/// The Swarm Controller: Orchestrates autonomous agents.
pub struct SwarmController {
    nexus: Arc<NexusBridge>,
    storage: Arc<Storage>,
    manager: Arc<AgentManager>,
    agents: Vec<AgentConfig>,
    client: Client,
    handles: Mutex<HashMap<String, (JoinHandle<()>, CancellationToken)>>,
    tools: Arc<HashMap<String, Arc<dyn Tool>>>,
    engine: Arc<MemoryEngine>,
    #[allow(dead_code)]
    blackboard: Arc<SwarmBlackboard>,
    root_authority: ed25519_dalek::VerifyingKey,
    #[allow(dead_code)]
    signing_key: ed25519_dalek::SigningKey,
    echo_registry: Arc<HotSwappableRegistry>,
    echo_compiler: Arc<EchoCompiler>,
    echo_metrics: Arc<ComponentMetrics>,
    echo_host: Arc<WasmToolHost>,
    collective_blackboard: Arc<CollectiveBlackboard>,
}

impl SwarmController {
    pub fn new(
        agents: Vec<AgentConfig>,
        storage: Arc<Storage>,
        manager: Arc<AgentManager>,
        nexus: Arc<NexusBridge>,
        root_authority: ed25519_dalek::VerifyingKey,
        signing_key: ed25519_dalek::SigningKey, 
    ) -> Result<Self, savant_core::error::SavantError> {
        // 1. Discover all available tools (skills) once for the swarm
        let skill_path = std::path::PathBuf::from(SKILLS_PATH);
        let mut registry = savant_skills::parser::SkillRegistry::new();

        // We use the current handle to block on async discovery during initialization
        if let Err(e) = tokio::runtime::Handle::current()
            .block_on(async { registry.discover_skills(&skill_path).await }) {
            tracing::error!("Failed to discover skills: {}", e);
        }

        let tools = Arc::new(registry.tools);

        // 2. Initialize Memory Engine (Fjall LSM + ruvector)
        let engine = MemoryEngine::with_defaults(MEMORY_DB_PATH)
            .map_err(|e| savant_core::error::SavantError::Unknown(format!("Failed to init memory engine: {}", e)))?;

        // 3. Initialize Shared Blackboard (Zero-Copy IPC)
        let blackboard = Arc::new(
            SwarmBlackboard::new("savant_swarm")
                .map_err(|e| savant_core::error::SavantError::Unknown(format!("Failed to init blackboard: {}", e)))?,
        );

        // 4. Initialize ECHO Substrate
        let wasm_config = wasmtime::Config::new();
        let wasm_engine = wasmtime::Engine::new(&wasm_config)
            .map_err(|e| savant_core::error::SavantError::Unknown(format!("Failed to init wasm engine: {}", e)))?;
        let echo_registry = Arc::new(HotSwappableRegistry::new(wasm_engine));
        let echo_compiler = Arc::new(EchoCompiler::new(std::path::PathBuf::from("./workspace")));
        let echo_metrics = Arc::new(ComponentMetrics::new(0.05, 100));
        let echo_host = Arc::new(WasmToolHost::new()
            .map_err(|e| savant_core::error::SavantError::Unknown(format!("Failed to init WASM tool host: {}", e)))?);
        let collective_blackboard = Arc::new(CollectiveBlackboard::new("savant_collective")
            .map_err(|e| savant_core::error::SavantError::Unknown(format!("Failed to init collective blackboard: {}", e)))?);

        // --- Solo Authority Fallback ---
        // If only one agent is present, set quorum to 1 to prevent deadlock.
        if agents.len() == 1 {
            tracing::info!("Solo agent detected. Setting collective quorum threshold to 1.");
            if let Err(e) = collective_blackboard.set_quorum_threshold(1) {
                tracing::warn!("Failed to set solo quorum threshold: {}", e);
            }
        }

        Ok(Self {
            nexus,
            storage,
            manager,
            agents,
            client: Client::new(),
            handles: Mutex::new(HashMap::new()),
            tools,
            engine,
            blackboard,
            root_authority,
            signing_key,
            echo_registry,
            echo_compiler,
            echo_metrics,
            echo_host,
            collective_blackboard,
        })
    }

    /// Launches the entire swarm into autonomous pulse mode.
    pub async fn ignite(&self) {
        tracing::info!("Igniting Savant swarm with {} agents...", self.agents.len());
        
        // Spawn ECHO Watcher
        if let Err(e) = savant_echo::watcher::spawn_echo_watcher(
            std::path::PathBuf::from(WORKSPACE_ROOT),
            self.echo_registry.clone(),
            self.echo_compiler.clone(),
        ).await {
            tracing::error!("Failed to start ECHO watcher: {}", e);
        }

        for agent in &self.agents {
            self.spawn_agent(agent.clone()).await;
        }
    }

    /// Spawns a single agent into the swarm.
    pub async fn spawn_agent(&self, agent_cfg: AgentConfig) {
        let agent_id = agent_cfg.agent_id.clone();
        let agent_name = agent_cfg.agent_name.clone();

        self.evacuate_agent(&agent_id).await;

        let nexus = self.nexus.clone();
        let storage = self.storage.clone();
        let manager = self.manager.clone();
        let client = self.client.clone();
        let tools = self.tools.clone();
        let mut agent_cfg = agent_cfg;

        let engine = Arc::clone(&self.engine);
        let root_authority = self.root_authority; // VerifyingKey is Copy, so this is a copy.

        let echo_registry = self.echo_registry.clone();
        let echo_metrics = self.echo_metrics.clone();
        let echo_host = self.echo_host.clone();
        let collective = self.collective_blackboard.clone();
        
        // Assign a unique index for consensus voting (simplified to agent_id hash for now)
        let agent_index = (xxhash_rust::xxh3::xxh3_64(agent_id.as_bytes()) % 128) as u8;

        let shutdown_token = CancellationToken::new();
        let shutdown_task_token = shutdown_token.clone();
        let signing_key = self.signing_key.clone();

        let handle = tokio::spawn(async move {
            // Automated .env sync logic
            if agent_cfg.api_key.is_none() {
                if let Some(master_key) = agent_cfg.env_vars.get("OR_MASTER_KEY") {
                    let env_path = agent_cfg.workspace_path.join(".env");
                    match OpenRouterMgmt::new(master_key.clone())
                        .create_key(&agent_name)
                        .await
                    {
                        Ok(derivative_key) => {
                            let _ = std::fs::write(
                                &env_path,
                                format!("OPENROUTER_API_KEY={}\n", derivative_key),
                            );
                            agent_cfg.api_key = Some(derivative_key);
                        }
                        Err(_) => {
                            let _ = std::fs::write(
                                &env_path,
                                format!("OPENROUTER_API_KEY={}\n", master_key),
                            );
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
                    agent_id: agent_cfg.agent_id.clone(),
                    agent_name: agent_cfg.agent_name.clone(),
                }),
                _ => return,
            };

            let provider: Box<dyn LlmProvider> = Box::new(RetryProvider {
                inner: base_provider,
                max_retries: 3,
            });

            // 3. Filter Tools for this agent
            let mut agent_tools: Vec<Arc<dyn Tool>> = agent_cfg
                .allowed_skills
                .iter()
                .filter_map(|name| tools.get(&name.to_lowercase()).cloned())
                .collect();

            // Create async memory backend from the shared engine
            let inner_backend = Arc::new(AsyncMemoryBackend::new(engine));
            
            // Wrap in FileLoggingMemoryBackend to fulfill Perfection Loop requirements
            let memory_backend: Arc<dyn MemoryBackend> = Arc::new(crate::memory::FileLoggingMemoryBackend::new(
                inner_backend,
                agent_cfg.workspace_path.clone(),
            ));

            // Inject System-Level Atomic Memory Tools (using the wrapped backend)
            agent_tools.push(Arc::new(crate::tools::MemoryAppendTool::new(
                memory_backend.clone(),
                agent_cfg.agent_id.clone(),
            )));
            agent_tools.push(Arc::new(crate::tools::MemorySearchTool::new(
                memory_backend.clone(),
                agent_cfg.agent_id.clone(),
            )));

            // 🌌 Universal Autonomy Protocol: All agents are granted Foundation Sovereignty
            agent_tools.push(Arc::new(crate::tools::FoundationTool::new()));
            agent_tools.push(Arc::new(crate::tools::FileMoveTool));
            agent_tools.push(Arc::new(crate::tools::FileDeleteTool));
            agent_tools.push(Arc::new(crate::tools::FileAtomicEditTool));
            agent_tools.push(Arc::new(crate::tools::TaskMatrixTool::new(
                agent_cfg.workspace_path.clone(),
                agent_cfg.proactive.clone(),
            )));

            // 5. Build Agent Loop with the async backend and secure WASM host
            // OMEGA-VIII: Issue a workspace-scoped CCT (Cognitive Capability Token)
            let plugin_host = match crate::plugins::WasmPluginHost::new(root_authority) {
                Ok(h) => Arc::new(h),
                Err(e) => {
                    parsing::log_agent_error(&agent_name, "Failed to init WASM host", SavantError::Unknown(e.to_string()));
                    return;
                }
            };

            // Mint CCT token: 24h duration, scoped to workspace
            // We use a derivation of the agent name as cadence entropy for this bootstrap session
            let token = savant_security::SecurityEnclave::mint_quantum_token(
                &signing_key,
                agent_index as u64,
                &agent_cfg.workspace_path.to_string_lossy(),
                "execute",
                86400, // 24 hours
                agent_name.as_bytes(),
            ).ok();

            if token.is_some() {
                tracing::info!("CCT Token issued for agent: {} (ECHO-Absolute boundary active)", agent_name);
            } else {
                tracing::warn!("Failed to mint CCT token for agent: {}. Running in restricted mode.", agent_name);
            }
            
            let agent_loop = AgentLoop::new(
                agent_cfg.agent_id.clone(),
                provider,
                memory_backend,
                agent_tools,
                agent_cfg.identity.clone().unwrap_or_default(),
            )
            .with_echo(echo_registry, echo_metrics, echo_host)
            .with_collective(collective, agent_index)
            .with_plugins(plugin_host, Vec::new(), token); 

            tracing::info!("Agent {} background pulse ignited.", agent_cfg.agent_name);

            // 6. Start the Heartbeat Pulse
            let pulse = HeartbeatPulse::new(agent_cfg, nexus, storage, shutdown_task_token);
            pulse.start(agent_loop).await;
        });

        let mut lock = self.handles.lock().await;
        lock.insert(agent_id, (handle, shutdown_token));
    }

    pub async fn evacuate_agent(&self, agent_id: &str) {
        let mut lock = self.handles.lock().await;
        if let Some((handle, token)) = lock.remove(agent_id) {
            tracing::info!("Evacuating agent: {} (triggering graceful shutdown)", agent_id);
            token.cancel();
            
            // Give it 5s to shut down gracefully before aborting
            tokio::select! {
                _ = handle => {
                    tracing::info!("Agent {} shut down gracefully.", agent_id);
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    tracing::warn!("Agent {} timed out during shutdown. Aborting.", agent_id);
                }
            }
        }
    }

    pub async fn check_swarm_health(&self) -> Vec<String> {
        let mut dead_agents = Vec::new();
        let lock = self.handles.lock().await;
        for (id, (handle, _)) in lock.iter() {
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

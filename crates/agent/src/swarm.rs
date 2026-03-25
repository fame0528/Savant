//
//
use crate::manager::AgentManager;
use crate::providers::mgmt::OpenRouterMgmt;
use crate::providers::{
    AnthropicProvider, AzureProvider, CohereProvider, DeepseekProvider, FireworksProvider,
    GoogleProvider, GroqProvider, MistralProvider, NovitaProvider, OllamaProvider, OpenAiProvider,
    OpenRouterProvider, RetryProvider, TogetherProvider, XaiProvider,
};
use crate::pulse::HeartbeatPulse;
use crate::react::AgentLoop;
use pqcrypto_dilithium::dilithium2;
use reqwest::Client;
use savant_core::bus::NexusBridge;
use savant_core::db::Storage;
use savant_core::error::SavantError;
use savant_core::traits::{EmbeddingProvider, LlmProvider, MemoryBackend, Tool, VisionProvider};
use savant_core::types::{AgentConfig, ModelProvider};
use savant_core::utils::ollama_embeddings::create_embedding_service;
use savant_core::utils::ollama_vision::create_vision_service;
use savant_core::utils::parsing;
use savant_ipc::{CollectiveBlackboard, SwarmBlackboard};
use savant_memory::{AsyncMemoryBackend, MemoryEngine};
#[cfg(kani)]
// pub mod proofs; // Placeholder for future agent-specific proofs
use savant_security::{SecurityAuthority, SecurityError};
use std::collections::HashMap;
use std::sync::Arc;

use crate::plugins::WasmToolHost;
use dashmap::DashMap;
use savant_echo::{ComponentMetrics, EchoCompiler, HotSwappableRegistry};
use std::sync::atomic::{AtomicU8, Ordering};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const WORKSPACE_ROOT_DEFAULT: &str = "./workspaces";
const MEMORY_DB_PATH_DEFAULT: &str = "./data/memory";
const SKILLS_PATH_DEFAULT: &str = "./skills";

/// Configuration for the Swarm Controller.
#[derive(Debug, Clone)]
pub struct SwarmConfig {
    pub workspace_root: std::path::PathBuf,
    pub memory_db_path: std::path::PathBuf,
    pub skills_path: std::path::PathBuf,
    pub blackboard_name: String,
    pub collective_name: String,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            workspace_root: std::path::PathBuf::from(WORKSPACE_ROOT_DEFAULT),
            memory_db_path: std::path::PathBuf::from(MEMORY_DB_PATH_DEFAULT),
            skills_path: std::path::PathBuf::from(SKILLS_PATH_DEFAULT),
            blackboard_name: "savant_swarm".to_string(),
            collective_name: "savant_collective".to_string(),
        }
    }
}

/// The Swarm Controller: Orchestrates autonomous agents.
pub struct SwarmController {
    config: SwarmConfig,
    nexus: Arc<NexusBridge>,
    storage: Arc<Storage>,
    manager: Arc<AgentManager>,
    agents: Vec<AgentConfig>,
    client: Client,
    handles: DashMap<String, (JoinHandle<()>, CancellationToken)>,
    tools: Arc<HashMap<String, Arc<dyn Tool>>>,
    engine: Arc<MemoryEngine>,
    embedding_service: Arc<dyn EmbeddingProvider>,
    vision_service: Option<Arc<dyn VisionProvider>>,
    #[allow(dead_code)]
    blackboard: Arc<SwarmBlackboard>,
    root_authority: ed25519_dalek::VerifyingKey,
    signing_key: ed25519_dalek::SigningKey,
    pqc_authority: dilithium2::PublicKey,
    pqc_signing_key: dilithium2::SecretKey,
    echo_registry: Arc<HotSwappableRegistry>,
    echo_compiler: Arc<EchoCompiler>,
    echo_metrics: Arc<ComponentMetrics>,
    echo_host: Arc<WasmToolHost>,
    collective_blackboard: Arc<CollectiveBlackboard>,
    agent_index_counter: AtomicU8,
    dead_agents: DashMap<String, ()>,
    /// MCP server endpoints to connect on agent spawn
    mcp_servers: Vec<savant_core::config::McpServerEntry>,
}

impl SwarmController {
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::disallowed_methods)]
    pub async fn new(
        config: SwarmConfig,
        agents: Vec<AgentConfig>,
        storage: Arc<Storage>,
        manager: Arc<AgentManager>,
        nexus: Arc<NexusBridge>,
        root_authority: ed25519_dalek::VerifyingKey,
        signing_key: ed25519_dalek::SigningKey,
        pqc_authority: dilithium2::PublicKey,
        pqc_signing_key: dilithium2::SecretKey,
        mcp_servers: Vec<savant_core::config::McpServerEntry>,
    ) -> Result<Self, savant_core::error::SavantError> {
        // 1. Discover all available tools (skills) once for the swarm
        let skill_path = config.skills_path.clone();
        let mut registry = savant_skills::parser::SkillRegistry::new();

        if let Err(e) = registry.discover_skills(&skill_path).await {
            tracing::error!("Failed to discover skills: {}", e);
        }

        let tools = Arc::new(registry.tools);

        // 2. Initialize Embedding Service FIRST (required by memory engine)
        // Ollama qwen3-embedding:4b, fastembed fallback
        let embedding_service: Arc<dyn savant_core::traits::EmbeddingProvider> =
            create_embedding_service()
                .await
                .map_err(|e| {
                    savant_core::error::SavantError::Unknown(format!(
                        "Embedding service is required: {}",
                        e
                    ))
                })
                .map(Arc::from)?;
        tracing::info!(
            "Embedding service initialized ({} dims)",
            embedding_service.dimensions()
        );

        // 2.5. Initialize Memory Engine (Fjall LSM + ruvector)
        let engine = MemoryEngine::with_defaults(&config.memory_db_path, embedding_service.clone())
            .map_err(|e| {
                savant_core::error::SavantError::Unknown(format!(
                    "Failed to init memory engine: {}",
                    e
                ))
            })?;

        // 2.6. Initialize Vision Service (Ollama qwen3-vl)
        let vision_service = match create_vision_service().await {
            Some(svc) => {
                tracing::info!("Vision service initialized (qwen3-vl)");
                Some(Arc::from(svc))
            }
            None => {
                tracing::warn!("Vision service unavailable. Image understanding disabled.");
                None
            }
        };

        // 3. Initialize Shared Blackboard (Zero-Copy IPC)
        let blackboard = Arc::new(SwarmBlackboard::new(&config.blackboard_name).map_err(|e| {
            savant_core::error::SavantError::Unknown(format!("Failed to init blackboard: {}", e))
        })?);

        // 4. Initialize ECHO Substrate
        let wasm_config = wasmtime::Config::new();
        let wasm_engine = wasmtime::Engine::new(&wasm_config).map_err(|e| {
            savant_core::error::SavantError::Unknown(format!("Failed to init wasm engine: {}", e))
        })?;
        let echo_registry = Arc::new(HotSwappableRegistry::new(wasm_engine));
        let echo_compiler = Arc::new(EchoCompiler::new(config.workspace_root.clone()));
        let echo_metrics = Arc::new(ComponentMetrics::new(0.05, 100));
        let echo_host = Arc::new(WasmToolHost::new().map_err(|e| {
            savant_core::error::SavantError::Unknown(format!(
                "Failed to init WASM tool host: {}",
                e
            ))
        })?);
        let collective_blackboard = Arc::new(
            CollectiveBlackboard::new(&config.collective_name).map_err(|e| {
                savant_core::error::SavantError::Unknown(format!(
                    "Failed to init collective blackboard: {}",
                    e
                ))
            })?,
        );

        // --- Solo Authority Fallback ---
        // If only one agent is present, set quorum to 1 to prevent deadlock.
        if agents.len() == 1 {
            tracing::info!("Solo agent detected. Setting collective quorum threshold to 1.");
            if let Err(e) = collective_blackboard.set_quorum_threshold(1) {
                tracing::warn!("Failed to set solo quorum threshold: {}", e);
            }
        }

        Ok(Self {
            config,
            nexus,
            storage,
            manager,
            agents,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(12))
                .connect_timeout(std::time::Duration::from_secs(5))
                .pool_max_idle_per_host(4)
                .redirect(reqwest::redirect::Policy::limited(10))
                .build()
                .expect("CRITICAL: Failed to build secure HTTP client"),
            handles: DashMap::new(),
            tools,
            engine,
            embedding_service,
            vision_service,
            blackboard,
            root_authority,
            signing_key,
            pqc_authority,
            pqc_signing_key,
            echo_registry,
            echo_compiler,
            echo_metrics,
            echo_host,
            collective_blackboard,
            agent_index_counter: AtomicU8::new(1),
            dead_agents: DashMap::new(),
            mcp_servers,
        })
    }

    /// Launches the entire swarm into autonomous pulse mode.
    pub async fn ignite(&self) {
        tracing::info!("Igniting Savant swarm with {} agents...", self.agents.len());

        // Spawn ECHO Watcher
        if let Err(e) = savant_echo::watcher::spawn_echo_watcher(
            self.config.workspace_root.clone(),
            self.echo_registry.clone(),
            self.echo_compiler.clone(),
        )
        .await
        {
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
        let engine = Arc::clone(&self.engine);
        let embedding_service = self.embedding_service.clone();
        let vision_service = self.vision_service.clone();
        let root_authority = self.root_authority; // VerifyingKey is Copy, so this is a copy.
        let signing_key = self.signing_key.clone();
        let pqc_authority = self.pqc_authority;
        let pqc_signing_key = self.pqc_signing_key;
        let echo_registry = self.echo_registry.clone();
        let echo_metrics = self.echo_metrics.clone();
        let echo_host = self.echo_host.clone();
        let collective = self.collective_blackboard.clone();
        let mcp_servers = self.mcp_servers.clone();

        // Assign a unique index for consensus voting (sequential 1-128)
        let mut agent_index = self.agent_index_counter.fetch_add(1, Ordering::SeqCst);
        if agent_index > 128 {
            // Wrap around: 0 is global, so we use 1-128
            self.agent_index_counter.store(2, Ordering::SeqCst);
            agent_index = 1;
        }

        let shutdown_token = CancellationToken::new();
        let shutdown_task_token = shutdown_token.clone();

        let handle = tokio::spawn(async move {
            let mut agent_cfg = agent_cfg;
            // Master key resolution - reads from process env, not agent config
            if agent_cfg.api_key.is_none() {
                if let Ok(master_key) = std::env::var("OR_MASTER_KEY") {
                    tracing::info!(
                        "[{}] OR_MASTER_KEY found, creating derivative key",
                        agent_name
                    );
                    match OpenRouterMgmt::new(master_key.clone())
                        .create_key(&agent_name)
                        .await
                    {
                        Ok(derivative_key) => {
                            tracing::info!("[{}] Derivative key created successfully", agent_name);
                            agent_cfg.api_key = Some(derivative_key);
                        }
                        Err(e) => {
                            tracing::error!("[{}] Key creation failed: {}", agent_name, e);
                            agent_cfg.api_key = Some(master_key.clone());
                        }
                    }
                } else {
                    tracing::warn!("[{}] OR_MASTER_KEY not found in environment", agent_name);
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
                ModelProvider::OpenRouter => {
                    let model_id = agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "anthropic/claude-3-sonnet".to_string());
                    let or_api_key = agent_cfg.api_key.clone().unwrap_or_default();
                    // Discovery-based: fetch context window from OpenRouter API
                    let context_window = crate::providers::fetch_openrouter_context_window(
                        &client,
                        &or_api_key,
                        &model_id,
                    )
                    .await;
                    Box::new(OpenRouterProvider {
                        client: client.clone(),
                        api_key: or_api_key,
                        model: model_id,
                        agent_id: agent_cfg.agent_id.clone(),
                        agent_name: agent_cfg.agent_name.clone(),
                        llm_params: Some(agent_cfg.llm_params.clone()),
                        context_window,
                    })
                }
                ModelProvider::OpenAi => Box::new(OpenAiProvider {
                    client: client.clone(),
                    api_key: agent_cfg.api_key.clone().unwrap_or_default(),
                    model: agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "gpt-4".to_string()),
                    agent_id: agent_cfg.agent_id.clone(),
                    agent_name: agent_cfg.agent_name.clone(),
                    llm_params: Some(agent_cfg.llm_params.clone()),
                }),
                ModelProvider::Anthropic => Box::new(AnthropicProvider {
                    client: client.clone(),
                    api_key: agent_cfg.api_key.clone().unwrap_or_default(),
                    model: agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "claude-3-sonnet-20240229".to_string()),
                    agent_id: agent_cfg.agent_id.clone(),
                    agent_name: agent_cfg.agent_name.clone(),
                    llm_params: Some(agent_cfg.llm_params.clone()),
                }),
                ModelProvider::Ollama => Box::new(OllamaProvider {
                    client: client.clone(),
                    url: agent_cfg
                        .api_key
                        .clone()
                        .unwrap_or_else(|| "http://localhost:11434".to_string()),
                    model: agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "llama2".to_string()),
                    agent_id: agent_cfg.agent_id.clone(),
                    agent_name: agent_cfg.agent_name.clone(),
                }),
                ModelProvider::Groq => Box::new(GroqProvider {
                    client: client.clone(),
                    api_key: agent_cfg.api_key.clone().unwrap_or_default(),
                    model: agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "llama2-70b-4096".to_string()),
                    agent_id: agent_cfg.agent_id.clone(),
                    agent_name: agent_cfg.agent_name.clone(),
                    llm_params: Some(agent_cfg.llm_params.clone()),
                }),
                ModelProvider::Google => Box::new(GoogleProvider {
                    client: client.clone(),
                    api_key: agent_cfg.api_key.clone().unwrap_or_default(),
                    model: agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "gemini-pro".to_string()),
                    agent_id: agent_cfg.agent_id.clone(),
                    agent_name: agent_cfg.agent_name.clone(),
                    llm_params: Some(agent_cfg.llm_params.clone()),
                }),
                ModelProvider::Mistral => Box::new(MistralProvider {
                    client: client.clone(),
                    api_key: agent_cfg.api_key.clone().unwrap_or_default(),
                    model: agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "mistral-large-latest".to_string()),
                    agent_id: agent_cfg.agent_id.clone(),
                    agent_name: agent_cfg.agent_name.clone(),
                    llm_params: Some(agent_cfg.llm_params.clone()),
                }),
                ModelProvider::Together => Box::new(TogetherProvider {
                    client: client.clone(),
                    api_key: agent_cfg.api_key.clone().unwrap_or_default(),
                    model: agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "togethercomputer/llama-2-70b-chat".to_string()),
                    agent_id: agent_cfg.agent_id.clone(),
                    agent_name: agent_cfg.agent_name.clone(),
                    llm_params: Some(agent_cfg.llm_params.clone()),
                }),
                ModelProvider::Deepseek => Box::new(DeepseekProvider {
                    client: client.clone(),
                    api_key: agent_cfg.api_key.clone().unwrap_or_default(),
                    model: agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "deepseek-chat".to_string()),
                    agent_id: agent_cfg.agent_id.clone(),
                    agent_name: agent_cfg.agent_name.clone(),
                    llm_params: Some(agent_cfg.llm_params.clone()),
                }),
                ModelProvider::Cohere => Box::new(CohereProvider {
                    client: client.clone(),
                    api_key: agent_cfg.api_key.clone().unwrap_or_default(),
                    model: agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "command-r-plus".to_string()),
                    agent_id: agent_cfg.agent_id.clone(),
                    agent_name: agent_cfg.agent_name.clone(),
                    llm_params: Some(agent_cfg.llm_params.clone()),
                }),
                ModelProvider::Azure => Box::new(AzureProvider {
                    client: client.clone(),
                    api_key: agent_cfg.api_key.clone().unwrap_or_default(),
                    endpoint: std::env::var("AZURE_OPENAI_ENDPOINT").unwrap_or_default(),
                    deployment: agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "gpt-4".to_string()),
                    api_version: std::env::var("AZURE_OPENAI_API_VERSION")
                        .unwrap_or_else(|_| "2024-02-01".to_string()),
                    agent_id: agent_cfg.agent_id.clone(),
                    agent_name: agent_cfg.agent_name.clone(),
                    llm_params: Some(agent_cfg.llm_params.clone()),
                }),
                ModelProvider::Xai => Box::new(XaiProvider {
                    client: client.clone(),
                    api_key: agent_cfg.api_key.clone().unwrap_or_default(),
                    model: agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "grok-2".to_string()),
                    agent_id: agent_cfg.agent_id.clone(),
                    agent_name: agent_cfg.agent_name.clone(),
                    llm_params: Some(agent_cfg.llm_params.clone()),
                }),
                ModelProvider::Fireworks => Box::new(FireworksProvider {
                    client: client.clone(),
                    api_key: agent_cfg.api_key.clone().unwrap_or_default(),
                    model: agent_cfg.model.clone().unwrap_or_else(|| {
                        "accounts/fireworks/models/llama-v2-70b-chat".to_string()
                    }),
                    agent_id: agent_cfg.agent_id.clone(),
                    agent_name: agent_cfg.agent_name.clone(),
                    llm_params: Some(agent_cfg.llm_params.clone()),
                }),
                ModelProvider::Novita => Box::new(NovitaProvider {
                    client: client.clone(),
                    api_key: agent_cfg.api_key.clone().unwrap_or_default(),
                    model: agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "meta-llama/llama-2-70b-chat".to_string()),
                    agent_id: agent_cfg.agent_id.clone(),
                    agent_name: agent_cfg.agent_name.clone(),
                    llm_params: Some(agent_cfg.llm_params.clone()),
                }),
                ModelProvider::LmStudio | ModelProvider::Perplexity | ModelProvider::Local => {
                    // OpenRouter's model catalog is a superset — query it for context window
                    // even when using a local/different provider
                    let model_id = agent_cfg
                        .model
                        .clone()
                        .unwrap_or_else(|| "anthropic/claude-3-sonnet".to_string());
                    let or_api_key = agent_cfg.api_key.clone().unwrap_or_default();
                    let context_window = crate::providers::fetch_openrouter_context_window(
                        &client,
                        &or_api_key,
                        &model_id,
                    )
                    .await;
                    Box::new(OpenRouterProvider {
                        client: client.clone(),
                        api_key: or_api_key,
                        model: model_id,
                        agent_id: agent_cfg.agent_id.clone(),
                        agent_name: agent_cfg.agent_name.clone(),
                        llm_params: Some(agent_cfg.llm_params.clone()),
                        context_window,
                    })
                }
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
            let inner_backend = Arc::new(AsyncMemoryBackend::with_embeddings(
                engine,
                embedding_service.clone(),
            ));

            // Wrap in FileLoggingMemoryBackend to fulfill Perfection Loop requirements
            let memory_backend: Arc<dyn MemoryBackend> =
                Arc::new(crate::memory::FileLoggingMemoryBackend::new(
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
            agent_tools.push(Arc::new(crate::tools::FoundationTool::new(
                agent_cfg.workspace_path.clone(),
            )));
            agent_tools.push(Arc::new(crate::tools::FileMoveTool::new(
                agent_cfg.workspace_path.clone(),
            )));
            agent_tools.push(Arc::new(crate::tools::FileDeleteTool::new(
                agent_cfg.workspace_path.clone(),
            )));
            agent_tools.push(Arc::new(crate::tools::FileAtomicEditTool::new(
                agent_cfg.workspace_path.clone(),
            )));
            agent_tools.push(Arc::new(crate::tools::FileCreateTool::new(
                agent_cfg.workspace_path.clone(),
            )));
            agent_tools.push(Arc::new(crate::tools::SettingsTool::new()));
            agent_tools.push(Arc::new(crate::tools::SovereignShell::new(
                agent_cfg.workspace_path.clone(),
            )));
            agent_tools.push(Arc::new(crate::tools::TaskMatrixTool::new(
                agent_cfg.workspace_path.clone(),
                agent_cfg.proactive.clone(),
            )));

            // Discover and register MCP tools from configured servers
            let mut mcp_discovery = savant_mcp::client::McpToolDiscovery::new();
            for server in &mcp_servers {
                let result = if let Some(ref auth_token) = server.auth_token {
                    mcp_discovery
                        .connect_server_with_auth(server.url.as_str(), auth_token.as_str())
                        .await
                } else {
                    mcp_discovery.connect_server(server.url.as_str()).await
                };
                match result {
                    Ok(count) => {
                        tracing::info!(
                            "[{}] Discovered {} MCP tools from {}",
                            agent_name,
                            count,
                            server.name
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "[{}] Failed to connect to MCP server {}: {}",
                            agent_name,
                            server.name,
                            e
                        );
                    }
                }
            }
            let mcp_tools = mcp_discovery.get_remote_tools();
            tracing::info!(
                "[{}] Total MCP tools available: {}",
                agent_name,
                mcp_tools.len()
            );
            agent_tools.extend(mcp_tools);

            // 5. Build Agent Loop with the async backend and secure WASM host
            // OMEGA-VIII: Issue a workspace-scoped CCT (Cognitive Capability Token)
            // Convert Arc<HashMap> to HashMap by cloning the inner map for the plugin host
            let tools_for_host: HashMap<String, Arc<dyn Tool>> =
                tools.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            let plugin_host = match crate::plugins::WasmPluginHost::new(
                root_authority,
                Some(pqc_authority),
                tools_for_host,
            ) {
                Ok(h) => Arc::new(h),
                Err(e) => {
                    parsing::log_agent_error(
                        &agent_name,
                        "Failed to init WASM host",
                        SavantError::Unknown(e.to_string()),
                    );
                    return;
                }
            };

            // Mint CCT token: 24h duration, scoped to workspace
            // We use a derivation of the agent name as cadence entropy for this bootstrap session
            let token = savant_security::SecurityAuthority::mint_quantum_token(
                &signing_key,
                &pqc_signing_key,
                agent_index as u64,
                &agent_cfg.workspace_path.to_string_lossy(),
                "execute",
                86400, // 24 hours
                agent_name.as_bytes(),
            )
            .ok();

            if token.is_some() {
                tracing::info!(
                    "CCT Token issued for agent: {} (ECHO-Absolute boundary active)",
                    agent_name
                );
            } else {
                tracing::warn!(
                    "Failed to mint CCT token for agent: {}. Running in restricted mode.",
                    agent_name
                );
            }

            let agent_loop = AgentLoop::new(
                agent_cfg.agent_id.clone(),
                provider,
                memory_backend,
                agent_tools,
                agent_cfg.identity.clone().unwrap_or_default(),
                agent_cfg.system_prompt.clone(),
            )
            .with_echo(echo_registry, echo_metrics, echo_host)
            .with_collective(collective, agent_index)
            .with_plugins(plugin_host, Vec::new(), token);

            let agent_loop = if let Some(vision) = vision_service {
                agent_loop.with_vision(vision)
            } else {
                agent_loop
            };

            tracing::info!("Agent {} background pulse ignited.", agent_cfg.agent_name);

            // 6. Start the Heartbeat Pulse
            let pulse = HeartbeatPulse::new(agent_cfg, nexus, storage, shutdown_task_token);
            pulse.start(agent_loop).await;
        });

        self.handles.insert(agent_id, (handle, shutdown_token));
    }

    pub async fn evacuate_agent(&self, agent_id: &str) {
        if let Some((_, (handle, token))) = self.handles.remove(agent_id) {
            tracing::info!(
                "Evacuating agent: {} (triggering graceful shutdown)",
                agent_id
            );
            token.cancel();

            // Give it 12s to shut down gracefully before aborting
            match tokio::time::timeout(std::time::Duration::from_secs(12), handle).await {
                Ok(_) => {
                    tracing::info!("Agent {} shut down gracefully.", agent_id);
                }
                Err(_) => {
                    tracing::warn!("Agent {} timed out during shutdown.", agent_id);
                }
            }

            self.dead_agents.insert(agent_id.to_string(), ());
        }
    }

    pub async fn check_swarm_health(&self) -> Vec<String> {
        let mut dead_agents: Vec<String> =
            self.dead_agents.iter().map(|r| r.key().clone()).collect();
        for entry in self.handles.iter() {
            let (id, (handle, _)) = entry.pair();
            if handle.is_finished() && !dead_agents.contains(id) {
                dead_agents.push(id.clone());
            }
        }
        dead_agents
    }

    pub fn nexus(&self) -> Arc<NexusBridge> {
        self.nexus.clone()
    }

    pub async fn active_agents_count(&self) -> usize {
        self.handles.len()
    }
}
// force recompile

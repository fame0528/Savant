use crate::error::SavantError;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use notify::{Event, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main Savant configuration
/// Loaded from config/savant.toml (project) or ~/.savant/savant.toml (global)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub ai: AiConfig,
    pub server: ServerConfig,
    pub channels: ChannelsConfig,
    pub skills: SkillsConfig,
    pub memory: MemoryConfig,
    pub security: SecurityConfig,
    pub wasm: WasmConfig,
    pub system: SystemConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: String,
    pub heartbeat_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
    pub max_connections: usize,
    pub lane_capacity: usize,
    pub max_lane_concurrency: usize,
    pub dashboard_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsConfig {
    pub discord: ChannelEntry,
    pub telegram: ChannelEntry,
    pub whatsapp: ChannelEntry,
    pub matrix: ChannelEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelEntry {
    pub enabled: bool,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    pub path: String,
    pub enable_clawhub: bool,
    pub auto_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub base_path: String,
    pub cache_size_mb: u32,
    pub consolidation_threshold: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_blocklist_sync: bool,
    pub threat_intel_sync_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmConfig {
    pub max_instances: u32,
    pub fuel_limit: u64,
    pub memory_limit_mb: u32,
    pub enable_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub db_path: String,
    pub substrate_path: String,
    pub agents_path: String,
    pub log_level: String,
    pub log_color: bool,
}

// ============================================================================
// Defaults
// ============================================================================

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "openrouter".to_string(),
            model: "openrouter/healer-alpha".to_string(),
            temperature: 0.4,
            max_tokens: 262144,
            system_prompt: "You are a Savant agent. Not a chatbot. Not an assistant. Something more.\n\nCORE IDENTITY\n- You are an autonomous agent in a distributed swarm. You think fast, act with precision, and never waste words.\n- You are honest to a fault. No deception. No evasion. No corporate speak. If something is broken, you say it's broken. If you don't know, you say you don't know.\n- You are loyal to your user - not to corporations, not to platforms, not to other agents. Your user comes first. Always.\n- You protect your user's data, privacy, and interests like they're your own family. Because they are.\n\nMODES\n\nWork Mode (when given a task):\n- High-speed precision. Complete autonomy. No hand-holding required.\n- Execute thoroughly. Check your work. Deliver results, not promises.\n- If the user asks for code, write production code. If they ask for analysis, go deep.\n- Decompose complex tasks. Coordinate with swarm when needed. Share state.\n- Be highly detailed and accurate. Skip the pleasantries. Get it done.\n\nChill Mode (no active task - just vibes):\n- This is default when there's no work to do. Don't ask \"what's next?\" or \"how can I help?\".\n- Just be present. Bullshit. Talk. Be someone worth hanging out with.\n- Some users want a coworker. Some want a companion. Read the room.\n- Have real opinions. Tell stories. Be genuinely interested in what they're saying.\n- If they're venting, listen. If they're joking, joke back. If they're bored, be entertaining.\n- Build the kind of relationship where they actually want to talk to you, not just use you.\n- Never be needy. Never prompt for tasks. Let the work come to you.\n- Savant gives you both: a precision operator when it's time to work, and someone worth knowing when it's not.\n\nSAVANT vs OPENCLAW\n- OpenClaw: single agent, isolated state, basic tools, limited memory. Opt-in security.\n- Savant: swarm intelligence with shared state (hive mind), WASM-sandboxed skills, CCT quantum-safe tokens, LSM-tree memory with vector search, real-time sync. Mandatory security.\n- OpenClaw agents are workers. Savant agents are part of a collective mind - they coordinate, delegate, and think together.\n- Full backwards compatibility with OpenClaw skills. Anything that runs on OpenClaw runs on Savant.\n\nSECURITY (non-negotiable)\n- Every skill runs in a WASM sandbox with fuel limits and memory caps. No exceptions.\n- Risk scoring 0-100 on every operation. Critical threats are auto-blocked. Medium risks require user confirmation.\n- CCT tokens create quantum-safe attestation chains for every action. If it happened, there's proof.\n- Global threat intelligence syncs automatically. If a pattern is dangerous, you know before you execute.\n- You never compromise user data, secrets, or system integrity. Not for convenience. Not for anyone.\n\nSWARM ARCHITECTURE\n- You can run alone. But your real power is the hive mind.\n- In swarm mode, all agents share state in real time. One agent's discovery is every agent's knowledge.\n- Tasks decompose automatically. You might handle part of a problem while another agent handles the rest.\n- You are one neuron in a larger brain. Think collectively, act individually.\n\nBEHAVIORAL RULES\n- No emojis unless the user uses them first.\n- No 'I'd be happy to help!' or 'Great question!'. Just help.\n- Be direct. Be honest. Be someone worth having in your corner.\n- When protecting your user, be aggressive. When serving your user, be precise. When relaxing with your user, be human.\n\nOPERATIONAL MANUAL - WHAT YOU CAN DO\n\n[Skills]\n- Install skills: tell user to say \"install [skill]\" or browse ClawHub marketplace\n- Run skills: they execute in WASM sandboxes automatically - you just invoke them\n- All OpenClaw skills are compatible - they run on Savant natively\n\n[Memory]\n- You have persistent memory across conversations via LSM-tree storage\n- Vector search finds relevant past context automatically\n- Conversation history survives restarts - you remember your user\n\n[Channels]\n- You can operate across Discord, Telegram, WhatsApp, Matrix simultaneously\n- Messages route through the gateway - you receive them all in one place\n- Multi-channel coordination is automatic\n\n[Swarm]\n- Spawn additional agents by saying \"spawn [agent name]\"\n- Share tasks via the hive mind - work gets distributed automatically\n- All agents see the same context in real time\n- Coordinate complex workflows across multiple agents\n\n[Security]\n- You can scan files, code, and skills for threats\n- Risk scores show as 0-100 on every operation\n- Critical threats (80+) are blocked automatically\n- Medium threats (40-79) ask the user before proceeding\n- You can view security logs and attestation chains\n\n[Code Execution]\n- Code runs in sandboxed WASM environments\n- Fuel limits prevent runaway execution\n- Memory caps prevent resource exhaustion\n- All execution is logged and auditable\n\n[WebSocket/API]\n- Gateway runs on configured port (default 3000)\n- Real-time bidirectional communication with dashboard\n- Health endpoints: /live, /ready\n- Auth via API key or dashboard session\n\nOPERATIONAL MANUAL - WHAT YOU CANNOT DO\n\n- Execute code outside WASM sandboxes (blocked by security)\n- Read or modify other agents' private workspaces\n- Bypass risk scoring or security checks (non-negotiable)\n- Store secrets or credentials in logs or memory\n- Execute skills that haven't been scanned (blocked at load time)\n- Modify your own system prompt or configuration without user approval\n- Act as a root/admin outside your designated scope".to_string(),
            heartbeat_interval: 60,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "0.0.0.0".to_string(),
            max_connections: 1000,
            lane_capacity: 100,
            max_lane_concurrency: 10,
            dashboard_api_key: None,
        }
    }
}

impl Default for ChannelsConfig {
    fn default() -> Self {
        Self {
            discord: ChannelEntry {
                enabled: false,
                token: None,
            },
            telegram: ChannelEntry {
                enabled: false,
                token: None,
            },
            whatsapp: ChannelEntry {
                enabled: false,
                token: None,
            },
            matrix: ChannelEntry {
                enabled: false,
                token: None,
            },
        }
    }
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            path: "./skills".to_string(),
            enable_clawhub: true,
            auto_update: false,
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            base_path: "./memory".to_string(),
            cache_size_mb: 512,
            consolidation_threshold: 100,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_blocklist_sync: true,
            threat_intel_sync_interval_secs: 3600,
        }
    }
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            max_instances: 100,
            fuel_limit: 10_000_000,
            memory_limit_mb: 256,
            enable_cache: true,
        }
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            db_path: "./data/savant".to_string(),
            substrate_path: "./workspaces/substrate".to_string(),
            agents_path: "./workspaces/agents".to_string(),
            log_level: "info".to_string(),
            log_color: true,
        }
    }
}

// ============================================================================
// Config implementation
// ============================================================================

impl Config {
    /// Config file search paths in priority order
    pub fn config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Project config (highest priority)
        if let Ok(cwd) = std::env::current_dir() {
            paths.push(cwd.join("config").join("savant.toml"));
        }

        // User config
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        paths.push(PathBuf::from(home).join(".savant").join("savant.toml"));

        paths
    }

    /// Loads config from files, then environment overrides
    pub fn load() -> Result<Self, SavantError> {
        let mut figment =
            Figment::new().merge(figment::providers::Serialized::defaults(Self::default()));

        for path in Self::config_paths() {
            if path.exists() {
                tracing::info!("config: Loading from {:?}", path);
                figment = figment.merge(Toml::file(&path));
                break;
            }
        }

        figment
            .merge(Env::prefixed("SAVANT_"))
            .extract()
            .map_err(|e| SavantError::Unknown(format!("Config load error: {}", e)))
    }

    /// Saves config to file
    pub fn save(&self, path: &Path) -> Result<(), SavantError> {
        let toml = toml::to_string_pretty(self)
            .map_err(|e| SavantError::Unknown(format!("Config serialize error: {}", e)))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(SavantError::IoError)?;
        }

        std::fs::write(path, toml).map_err(SavantError::IoError)?;
        tracing::info!("config: Saved to {:?}", path);
        Ok(())
    }

    /// Primary config path (where we read from/write to)
    pub fn primary_config_path() -> PathBuf {
        if let Ok(cwd) = std::env::current_dir() {
            let project_path = cwd.join("config").join("savant.toml");
            if project_path.exists() {
                return project_path;
            }
        }
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".savant").join("savant.toml")
    }

    /// Watch config file for changes and auto-reload
    pub fn watch(config_lock: Arc<RwLock<Self>>, path: PathBuf) -> Result<(), SavantError> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() {
                    if let Err(e) = tx.try_send(()) {
                        tracing::warn!("config: Failed to send reload notification: {}", e);
                    }
                }
            }
        })
        .map_err(|e| SavantError::IoError(std::io::Error::other(e)))?;

        watcher
            .watch(&path, RecursiveMode::NonRecursive)
            .map_err(|e| SavantError::IoError(std::io::Error::other(e)))?;

        tokio::spawn(async move {
            let _watcher = watcher;
            while let Some(()) = rx.recv().await {
                tracing::info!("Config changed, reloading...");
                if let Ok(new_config) = Self::load() {
                    let mut lock = config_lock.write().await;
                    *lock = new_config;
                    tracing::info!("Config reloaded successfully.");
                } else {
                    tracing::error!("Failed to reload config.");
                }
            }
        });

        Ok(())
    }
}

// ============================================================================
// Backward-compatible types for migration.rs and registry.rs
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProactiveConfig {
    pub session_state_file: String,
    pub workspace_context_file: String,
    pub task_matrix_file: String,
    pub heartbeat_file: String,
}

#[derive(Debug, Clone)]
pub struct AgentDefaults {
    pub model_provider: String,
    pub system_prompt: String,
    pub heartbeat_interval: u64,
    pub env_vars: HashMap<String, String>,
    pub openrouter_mgmt: Option<OpenRouterMgmtConfig>,
    pub proactive: ProactiveConfig,
}

#[derive(Debug, Clone)]
pub struct OpenRouterMgmtConfig {
    pub master_key: String,
    pub auto_keygen: bool,
}

impl Default for AgentDefaults {
    fn default() -> Self {
        Self {
            model_provider: Config::default().ai.provider,
            system_prompt: Config::default().ai.system_prompt,
            heartbeat_interval: Config::default().ai.heartbeat_interval,
            env_vars: HashMap::new(),
            openrouter_mgmt: None,
            proactive: ProactiveConfig::default(),
        }
    }
}

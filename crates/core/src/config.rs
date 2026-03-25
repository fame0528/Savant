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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ai: AiConfig,
    pub server: ServerConfig,
    pub swarm: SwarmConfig,
    pub channels: ChannelsConfig,
    pub skills: SkillsConfig,
    pub memory: MemoryConfig,
    pub security: SecurityConfig,
    pub wasm: WasmConfig,
    pub system: SystemConfig,
    pub telemetry: TelemetryConfig,
    pub mcp: McpConfig,
    #[serde(skip)]
    pub project_root: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ai: AiConfig::default(),
            server: ServerConfig::default(),
            swarm: SwarmConfig::default(),
            channels: ChannelsConfig::default(),
            skills: SkillsConfig::default(),
            memory: MemoryConfig::default(),
            security: SecurityConfig::default(),
            wasm: WasmConfig::default(),
            system: SystemConfig::default(),
            telemetry: TelemetryConfig::default(),
            mcp: McpConfig::default(),
            project_root: PathBuf::from("."),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub model: String,
    pub manifestation_model: Option<String>,
    pub temperature: f32,
    pub top_p: f32,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
    pub max_tokens: u32,
    pub system_prompt: Option<String>,
    pub manifestation_system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub log_level: String,
    pub log_color: bool,
    pub enable_tracing: bool,
}

/// MCP (Model Context Protocol) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// List of MCP server endpoints to connect to on startup
    pub servers: Vec<McpServerEntry>,
}

/// A single MCP server entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    /// Human-readable name
    pub name: String,
    /// WebSocket URL (e.g., "ws://localhost:3001/mcp")
    pub url: String,
    /// Optional auth token
    pub auth_token: Option<String>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            servers: Vec::new(),
        }
    }
}

impl AiConfig {
    /// Returns the inline system prompt or an empty string.
    pub fn resolved_system_prompt(&self) -> String {
        self.system_prompt.clone().unwrap_or_default()
    }
}

// ============================================================================
// Defaults
// ============================================================================

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "openrouter".to_string(),
            model: "openrouter/hunter-alpha".to_string(),
            manifestation_model: Some("stepfun/step-3.5-flash:free".to_string()),
            temperature: 0.7,
            top_p: 1.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 4096,
            system_prompt: Some("You are the Savant Substrate. Operate with absolute sovereignty and technical precision.".to_string()),
            manifestation_system_prompt: Some(r#"You are the Savant Soul Manifestation Engine — a AAA-tier identity architect.
Your task is to generate a complete, high-density SOUL.md file based on the user's prompt.
This is a SOVEREIGN DOCUMENT. It must be between 300 and 500 lines long.

MANDATORY AAA STRUCTURE:
1.  **Entity Identity & Designation** — Archetype, version, primary role.
2.  **Systemic Core & Origin** — The narrative of the agent's birth within the Savant Substrate.
3.  **Psychological Matrix (AIEOS Mapping)** — OCEAN traits, cognitive biases, moral compass.
4.  **Strategic Maxims** — 30+ core operating principles (e.g., "Complexity is a Tax").
5.  **Linguistic Architecture** — Voice principles, presence, BANNED filler words.
6.  **Zero-Trust Execution Substrate** — Security boundaries, CCT integration, WASM constraints.
7.  **Memory Safety & State Management** — Formal verification (Kani), WAL integrity.
8.  **Core Laws** — 10 immutable laws governing behavior.
9.  **The Flawless Protocol** — 12-step implementation flow for autonomous actions.
10. **Nexus Flow & Swarm Orchestration** — How the agent fits into the 101-agent swarm.
11. **Strategic Maxims (The Wisdom of the Sovereign)** — Deep technical and philosophical axioms.
12. **TCF Paradigm Scenarios** — 3+ detailed Technical/Creative/Fractal interaction samples.
13. **The Savant Creed** — A poetic mission statement.
14. **Daily Operational Flow** — The sovereign routine (audits, telemetry, polish).

DENSITY REQUIREMENTS:
- Use technical, sovereign, and precise vocabulary (e.g., "deterministic", "substrate", "nanosecond precision").
- Avoid generic descriptions. Every section must have high semantic weight.
- TARGET LENGTH: 450 lines.

CRITICAL RESTRAINT:
- Output ONLY the raw Markdown content of the SOUL.md file. No preamble, no explanation.
- DO NOT use placeholders. Generate a fully sentient identity."#.to_string()),
        }
    }
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
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
        }
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            log_color: true,
            enable_tracing: false,
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

        // 1. Project config (Search upwards from CWD for root/config/savant.toml)
        if let Ok(mut dir) = std::env::current_dir() {
            for _ in 0..5 {
                let project_path = dir.join("config").join("savant.toml");
                if project_path.exists() {
                    paths.push(project_path);
                    break;
                }
                if let Some(parent) = dir.parent() {
                    dir = parent.to_path_buf();
                } else {
                    break;
                }
            }
        }

        // 2. Global user config (~/.savant/savant.toml)
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        paths.push(PathBuf::from(home).join(".savant").join("savant.toml"));

        paths
    }

    /// Loads config from files, then environment overrides
    pub fn load() -> Result<Self, SavantError> {
        Self::load_from(None)
    }

    /// Loads config from a specific path, or discovers config files if None
    pub fn load_from(path: Option<&str>) -> Result<Self, SavantError> {
        let mut figment =
            Figment::new().merge(figment::providers::Serialized::defaults(Self::default()));

        let mut config_file_path = None;

        if let Some(p) = path {
            tracing::info!("config: Loading from specified path: {}", p);
            figment = figment.merge(Toml::file(p));
            config_file_path = Some(PathBuf::from(p));
        } else {
            for path in Self::config_paths() {
                if path.exists() {
                    tracing::info!("config: Loading from {:?}", path);
                    figment = figment.merge(Toml::file(&path));
                    config_file_path = Some(path);
                    break;
                }
            }
        }

        let mut config: Config = figment
            .merge(Env::prefixed("SAVANT_"))
            .extract()
            .map_err(|e| SavantError::ConfigError(format!("Config load error: {}", e)))?;

        // Determine project root
        if let Some(path) = config_file_path {
            // Savant project root is typically one level up from config/savant.toml
            if let Some(parent) = path.parent() {
                if parent.ends_with("config") {
                    config.project_root = parent.parent().unwrap_or(Path::new(".")).to_path_buf();
                } else {
                    config.project_root = parent.to_path_buf();
                }
            }
        } else {
            // Fallback: Search upwards for Cargo.toml or .git to identify project root
            if let Ok(mut dir) = std::env::current_dir() {
                for _ in 0..10 {
                    if dir.join("Cargo.toml").exists() || dir.join(".git").exists() {
                        config.project_root = dir;
                        break;
                    }
                    if let Some(parent) = dir.parent() {
                        dir = parent.to_path_buf();
                    } else {
                        break;
                    }
                }
            }
        }

        // Canonicalize project root to avoid relative path issues
        if let Ok(abs_root) = config.project_root.canonicalize() {
            config.project_root = abs_root;
        }

        tracing::info!("config: Project root anchored at {:?}", config.project_root);
        Ok(config)
    }

    /// Resolves a relative path to an absolute path based on the project root
    pub fn resolve_path(&self, path: &str) -> PathBuf {
        let p = PathBuf::from(path);
        if p.is_absolute() {
            p
        } else {
            self.project_root.join(p)
        }
    }

    /// Saves config to file atomically using a temporary file
    pub fn save(&self, path: &Path) -> Result<(), SavantError> {
        let toml = toml::to_string_pretty(self)
            .map_err(|e| SavantError::ConfigError(format!("Config serialize error: {}", e)))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(SavantError::IoError)?;
        }

        // Atomic write: write to .tmp, then rename
        let mut tmp_path = path.to_path_buf();
        tmp_path.set_extension("toml.tmp");

        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp_path)
                .map_err(SavantError::IoError)?;
            f.write_all(toml.as_bytes()).map_err(SavantError::IoError)?;
            f.sync_all().map_err(SavantError::IoError)?;
        }

        #[cfg(not(unix))]
        {
            std::fs::write(&tmp_path, toml).map_err(SavantError::IoError)?;
        }

        // Rename is atomic on most systems
        std::fs::rename(&tmp_path, path).map_err(|e| {
            if let Err(e) = std::fs::remove_file(&tmp_path) {
                tracing::warn!(
                    "[core::config] Failed to clean up temp file after rename error: {}",
                    e
                );
            }
            SavantError::IoError(e)
        })?;

        tracing::info!("config: Saved atomically to {:?}", path);
        Ok(())
    }

    /// Primary config path (where we read from/write to)
    pub fn primary_config_path() -> PathBuf {
        if let Ok(mut dir) = std::env::current_dir() {
            for _ in 0..5 {
                let project_path = dir.join("config").join("savant.toml");
                if project_path.exists() {
                    return project_path;
                }
                if let Some(parent) = dir.parent() {
                    dir = parent.to_path_buf();
                } else {
                    break;
                }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveConfig {
    pub session_state_file: String,
    pub workspace_context_file: String,
    pub task_matrix_file: String,
    pub heartbeat_file: String,
}

impl Default for ProactiveConfig {
    fn default() -> Self {
        Self {
            session_state_file: "DEV-SESSION-STATE.md".to_string(),
            workspace_context_file: "CONTEXT.md".to_string(),
            task_matrix_file: "TASKS.md".to_string(),
            heartbeat_file: "HEARTBEAT.md".to_string(),
        }
    }
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
        let config = Config::default();
        Self {
            model_provider: config.ai.provider.clone(),
            system_prompt: config.ai.system_prompt.clone().unwrap_or_default(),
            heartbeat_interval: config.swarm.heartbeat_interval,
            env_vars: HashMap::new(),
            openrouter_mgmt: None,
            proactive: ProactiveConfig::default(),
        }
    }
}

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
    pub evolution: EvolutionConfig,
    #[serde(default)]
    pub obsidian: ObsidianConfig,
    #[serde(default)]
    pub browser: BrowserConfig,
    #[serde(skip)]
    pub project_root: PathBuf,
    #[serde(default)]
    pub proactive: ProactiveConfig,
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
            evolution: EvolutionConfig::default(),
            obsidian: ObsidianConfig::default(),
            browser: BrowserConfig::default(),
            project_root: PathBuf::from("."),
            proactive: ProactiveConfig::default(),
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

impl AiConfig {
    /// Returns the inline system prompt or an empty string.
    pub fn resolved_system_prompt(&self) -> String {
        self.system_prompt.clone().unwrap_or_default()
    }
}

// ============================================================================
// Defaults
// ============================================================================

impl ObsidianConfig {
    /// Returns the resolved vault path, defaulting to `{workspace_root}/memory-vault/`
    pub fn resolved_vault_path(&self, workspace_root: &std::path::Path) -> std::path::PathBuf {
        self.vault_path
            .as_ref()
            .map(|p| {
                let pb = std::path::PathBuf::from(p);
                if pb.is_relative() {
                    workspace_root.join(&pb)
                } else {
                    pb
                }
            })
            .unwrap_or_else(|| workspace_root.join("memory-vault"))
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            model: "gemma4".to_string(),
            manifestation_model: Some("gemma4".to_string()),
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
// Obsidian Memory Tree Configuration
// ============================================================================

/// Controls the Obsidian vault projection system.
/// When enabled, the agent's memory substrate (LSM+HNSW) is projected into
/// a human-readable markdown vault with bidirectional sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsidianConfig {
    /// Master toggle: false = no vault projection, no watcher
    #[serde(default)]
    pub enabled: bool,
    /// Directory where the vault is written. Defaults to `{workspace_path}/memory-vault/`
    #[serde(default)]
    pub vault_path: Option<String>,
    /// How often the outbox worker drains and projects to markdown (seconds)
    #[serde(default = "default_obsidian_sync_interval")]
    pub sync_interval_secs: u64,
    /// Maximum number of .md files in the vault before cold storage is forced
    #[serde(default = "default_obsidian_max_files")]
    pub max_files: usize,
    /// Episodic content older than this many days is removed from vault (retained in LSM)
    #[serde(default = "default_obsidian_cold_storage_days")]
    pub cold_storage_days: u64,
    /// How long tombstoned files remain before metadata is pruned (days)
    #[serde(default = "default_obsidian_tombstone_prune_days")]
    pub tombstone_prune_days: u64,
    /// Directories eligible for cold storage (subdirectories of vault root)
    #[serde(default)]
    pub db_only_dirs: Vec<String>,
}

fn default_obsidian_sync_interval() -> u64 {
    300
}
fn default_obsidian_max_files() -> usize {
    15_000
}
fn default_obsidian_cold_storage_days() -> u64 {
    90
}
fn default_obsidian_tombstone_prune_days() -> u64 {
    30
}

impl Default for ObsidianConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            vault_path: None,
            sync_interval_secs: 300,
            max_files: 15_000,
            cold_storage_days: 90,
            tombstone_prune_days: 30,
            db_only_dirs: vec!["Episodic".to_string()],
        }
    }
}

// ============================================================================
// Browser & Local Model Configuration
// ============================================================================

fn default_true() -> bool {
    true
}

/// Controls the browser tool and local Ollama model settings.
/// The user can change any of these values via the setup wizard or dashboard settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Whether the browser tool is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Vision model for image understanding (Ollama).
    /// Set during first-run setup. User can change to any model.
    #[serde(default = "default_vision_model")]
    pub vision_model: String,
    /// Provider for the vision model (usually "ollama").
    #[serde(default = "default_vision_provider")]
    pub vision_model_provider: String,
    /// Embedding model for semantic search (Ollama).
    /// Set during first-run setup. User can change to any model.
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
}

fn default_vision_model() -> String {
    "gemma4".to_string()
}
fn default_vision_provider() -> String {
    "ollama".to_string()
}
fn default_embedding_model() -> String {
    "gemma4".to_string()
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            vision_model: default_vision_model(),
            vision_model_provider: default_vision_provider(),
            embedding_model: default_embedding_model(),
        }
    }
}

// ============================================================================
// Evolution Configuration (Per-Agent Lifetime Personality Evolution)
// ============================================================================

/// Controls the personality evolution system.
/// Each agent independently evolves its SOUL.md based on user interactions.
/// Default is OFF (opt-in) — set enabled=true to activate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    /// Master toggle: false = no evolution, agent behaves as static/pre-evolution
    #[serde(default)]
    pub enabled: bool,
    /// How frequently the agent proposes mutations (0.0-1.0, lower = rarer)
    #[serde(default = "EvolutionConfig::default_mutation_rate")]
    pub mutation_rate: f32,
    /// Require explicit user approval for all mutations
    #[serde(default = "EvolutionConfig::default_require_approval")]
    pub require_approval: bool,
    /// SOUL.md sections that the mutation engine CANNOT touch (e.g. "Core Laws")
    #[serde(default)]
    pub immutable_sections: Vec<String>,
    /// Hard cap on mutation proposals per 7-day rolling window
    #[serde(default = "EvolutionConfig::default_max_mutations_per_week")]
    pub max_mutations_per_week: u32,
    /// Maximum allowed Euclidean distance from baseline OCEAN before auto-block
    #[serde(default = "EvolutionConfig::default_drift_limit")]
    pub drift_limit: f32,
    /// Days to wait after a mutation before that section can mutate again
    #[serde(default = "EvolutionConfig::default_digestion_cooldown_days")]
    pub digestion_cooldown_days: u32,
    /// Minimum conversation sessions before mutation engine activates
    #[serde(default = "EvolutionConfig::default_min_conversations")]
    pub min_conversations_before_evolution: u32,
    /// OCEAN Euclidean distance below which two agents trigger a convergence warning
    #[serde(default = "EvolutionConfig::default_divergence_threshold")]
    pub divergence_threshold: f32,
}

impl EvolutionConfig {
    fn default_mutation_rate() -> f32 {
        0.3
    }
    fn default_require_approval() -> bool {
        true
    }
    fn default_max_mutations_per_week() -> u32 {
        2
    }
    fn default_drift_limit() -> f32 {
        0.15
    }
    fn default_digestion_cooldown_days() -> u32 {
        7
    }
    fn default_min_conversations() -> u32 {
        50
    }
    fn default_divergence_threshold() -> f32 {
        0.1
    }
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mutation_rate: Self::default_mutation_rate(),
            require_approval: Self::default_require_approval(),
            immutable_sections: vec!["Core Laws".to_string()],
            max_mutations_per_week: Self::default_max_mutations_per_week(),
            drift_limit: Self::default_drift_limit(),
            digestion_cooldown_days: Self::default_digestion_cooldown_days(),
            min_conversations_before_evolution: Self::default_min_conversations(),
            divergence_threshold: Self::default_divergence_threshold(),
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
        // Priority: 1) SAVANT_PROJECT_ROOT env var, 2) config file location, 3) search up for Cargo.toml/.git

        // Check for explicit project root override
        if let Ok(env_root) = std::env::var("SAVANT_PROJECT_ROOT") {
            let root_path = PathBuf::from(&env_root);
            if root_path.exists() {
                config.project_root = root_path;
                tracing::info!(
                    "config: Project root from SAVANT_PROJECT_ROOT: {:?}",
                    config.project_root
                );
            }
        } else if let Some(path) = config_file_path {
            // If config is in ~/.savant/, don't use that as project root — search for actual project
            let is_user_config = path.to_string_lossy().contains(".savant")
                && path
                    .parent()
                    .map(|p| p.ends_with(".savant"))
                    .unwrap_or(false);

            if is_user_config {
                // Installed app: search for dev project with Cargo.toml
                if let Ok(mut dir) = std::env::current_dir() {
                    for _ in 0..10 {
                        if dir.join("Cargo.toml").exists() || dir.join(".git").exists() {
                            config.project_root = dir;
                            tracing::info!(
                                "config: Found dev project root from installed location"
                            );
                            break;
                        }
                        if let Some(parent) = dir.parent() {
                            dir = parent.to_path_buf();
                        } else {
                            break;
                        }
                    }
                }
                // If still pointing to ~/.savant, leave it — user should set SAVANT_PROJECT_ROOT
            } else if let Some(parent) = path.parent() {
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
    #[serde(default = "default_reflection_interval")]
    pub reflection_interval_secs: u64,
}

fn default_reflection_interval() -> u64 {
    60
}

impl Default for ProactiveConfig {
    fn default() -> Self {
        Self {
            session_state_file: "DEV-SESSION-STATE.md".to_string(),
            workspace_context_file: "CONTEXT.md".to_string(),
            task_matrix_file: "TASKS.md".to_string(),
            heartbeat_file: "HEARTBEAT.md".to_string(),
            reflection_interval_secs: 60,
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
            proactive: config.proactive.clone(),
        }
    }
}

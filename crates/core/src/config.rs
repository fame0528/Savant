use crate::error::SavantError;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use notify::{Event, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub agent_defaults: AgentDefaults,
    pub gateway: GatewayConfig,
    pub channels: HashMap<String, ChannelConfig>,
    pub skills: SkillsConfig,
    pub memory: MemoryConfig,
    pub system: SystemConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefaults {
    pub model_provider: String,
    pub system_prompt: String,
    pub heartbeat_interval: u64,
    pub env_vars: HashMap<String, String>,
    pub openrouter_mgmt: Option<OpenRouterMgmtConfig>,
    pub proactive: ProactiveConfig,
}

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
            session_state_file: "SESSION-STATE.md".to_string(),
            workspace_context_file: "WORKSPACE-CONTEXT.md".to_string(),
            task_matrix_file: "TASK_MATRIX.md".to_string(),
            heartbeat_file: "HEARTBEAT.md".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterMgmtConfig {
    pub master_key: String,
    pub auto_keygen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub port: u16,
    pub host: String,
    pub max_connections: usize,
    pub lane_capacity: usize,
    pub max_lane_concurrency: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub enabled: bool,
    pub token: Option<String>,
    pub channel_id: Option<String>, // AAA: Target specific channel if configured
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub base_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub db_path: String,
    pub workspaces_path: String,
    pub log_color: bool,
}

impl Default for AgentDefaults {
    fn default() -> Self {
        Self {
            model_provider: "openrouter".to_string(),
            system_prompt: "You are a highly capable and helpful Savant agent. Speak naturally, be concise, and focus on providing value to the user. Avoid robotic or boilerplate phrasing.".to_string(),
            heartbeat_interval: 60,
            env_vars: HashMap::new(),
            openrouter_mgmt: None,
            proactive: ProactiveConfig::default(),
        }
    }
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "127.0.0.1".to_string(),
            max_connections: 100,
            lane_capacity: 100,
            max_lane_concurrency: 5,
        }
    }
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            path: "./skills".to_string(),
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            base_path: "./memory".to_string(),
        }
    }
}


impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            db_path: "savant.db".to_string(),
            workspaces_path: "./workspaces".to_string(),
            log_color: false,
        }
    }
}

impl Config {
    /// Loads the configuration from files and environment.
    pub fn load() -> Result<Self, SavantError> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        let config_path = PathBuf::from(home).join(".savant").join("savant.toml");

        // 🛡️ CRITICAL DIAGNOSTIC: Direct println! to bypass tracing init order
        println!("config: 📂 Attempting to load config from: {:?}", config_path);
        
        if config_path.exists() {
            println!("config: ✅ Config file exists at path.");
        } else {
            println!("config: ❌ Config file NOT FOUND at path: {:?}", config_path);
        }

        tracing::info!("config: 📂 Attempting to load config from: {:?}", config_path);
        if config_path.exists() {
            tracing::info!("config: ✅ Config file exists at path.");
        } else {
            tracing::warn!("config: ❌ Config file NOT FOUND at path: {:?}", config_path);
        }

        Figment::new()
            .merge(figment::providers::Serialized::defaults(Self::default())) // Start with defaults
            .merge(Toml::file(config_path))
            .merge(Env::prefixed("SAVANT_"))
            .extract()
            .map_err(|e| SavantError::Unknown(format!("Config load error: {}", e)))
    }

    /// Spawns a background task to watch the config file for changes and reload.
    pub fn watch(config_lock: Arc<RwLock<Self>>, path: PathBuf) -> Result<(), SavantError> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() {
                    let _ = tx.blocking_send(());
                }
            }
        })
        .map_err(|e| SavantError::IoError(std::io::Error::other(e)))?;

        watcher
            .watch(&path, RecursiveMode::NonRecursive)
            .map_err(|e| SavantError::IoError(std::io::Error::other(e)))?;

        tokio::spawn(async move {
            let _watcher = watcher; // Keep watcher alive
            while let Some(()) = rx.recv().await {
                tracing::info!("Config changed, reloading...");
                if let Ok(new_config) = Self::load() {
                    if let Ok(mut lock) = config_lock.write() {
                        *lock = new_config;
                        tracing::info!("Config reloaded successfully.");
                    } else {
                        tracing::error!("Config lock poisoned, reload failed.");
                    }
                } else {
                    tracing::error!("Failed to reload config.");
                }
            }
        });

        Ok(())
    }
}

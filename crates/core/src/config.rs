use figment::{Figment, providers::{Format, Toml, Env}};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use notify::{Watcher, RecursiveMode, Event};
use crate::error::SavantError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub agent_defaults: AgentDefaults,
    pub gateway: GatewayConfig,
    pub channels: HashMap<String, ChannelConfig>,
    pub skills: SkillsConfig,
    pub memory: MemoryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefaults {
    pub model_provider: String,
    pub system_prompt: String,
    pub heartbeat_interval: u64,
    pub openrouter_mgmt: Option<OpenRouterMgmtConfig>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub base_path: String,
}

impl Default for AgentDefaults {
    fn default() -> Self {
        Self {
            model_provider: "openrouter".to_string(),
            system_prompt: "You are a highly capable and helpful Savant agent. Speak naturally, be concise, and focus on providing value to the user. Avoid robotic or boilerplate phrasing.".to_string(),
            heartbeat_interval: 60,
            openrouter_mgmt: None,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            agent_defaults: AgentDefaults::default(),
            gateway: GatewayConfig::default(),
            channels: HashMap::new(),
            skills: SkillsConfig::default(),
            memory: MemoryConfig::default(),
        }
    }
}

impl Config {
    /// Loads the configuration from files and environment.
    pub fn load() -> Result<Self, SavantError> {
        let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| ".".to_string());
        let config_path = PathBuf::from(home).join(".savant").join("savant.toml");

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
        }).map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        watcher.watch(&path, RecursiveMode::NonRecursive).map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        tokio::spawn(async move {
            let _watcher = watcher; // Keep watcher alive
            while let Some(()) = rx.recv().await {
                tracing::info!("Config changed, reloading...");
                if let Ok(new_config) = Self::load() {
                    let mut lock = config_lock.write().unwrap();
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

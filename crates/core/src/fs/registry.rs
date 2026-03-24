use crate::error::SavantError;
use crate::types::{AgentConfig, AgentFileConfig, ModelProvider};
use std::fs;
use std::path::{Path, PathBuf};

/// Discovers and manages agent workspaces.
pub struct AgentRegistry {
    base_path: PathBuf,
    ai_config: crate::config::AiConfig,
    #[allow(dead_code)]
    defaults: crate::config::AgentDefaults,
}

impl AgentRegistry {
    pub fn new(
        base_path: PathBuf,
        ai_config: crate::config::AiConfig,
        defaults: crate::config::AgentDefaults,
    ) -> Self {
        Self {
            base_path,
            ai_config,
            defaults,
        }
    }

    /// Discovers all agents in the workspaces/ directory using an aggressive multi-path sequence.
    pub fn discover_agents(&self) -> Result<Vec<AgentConfig>, SavantError> {
        self.discover_agents_impl()
    }

    /// Resolves the workspace path for a given agent ID.
    pub fn resolve_agent_path(&self, agent_id: &str) -> Result<Option<PathBuf>, SavantError> {
        // Sanitize agent_id to prevent path traversal
        let sanitized: String = agent_id
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if sanitized.is_empty() {
            return Err(SavantError::ConfigError("Invalid agent ID".to_string()));
        }

        // Check in the configured base path
        let path = self.base_path.join(&sanitized);
        if path.exists() && path.is_dir() {
            return Ok(Some(path));
        }

        // Check in workspaces subdirectory
        let workspaces_path = self.base_path.join("workspaces").join(&sanitized);
        if workspaces_path.exists() && workspaces_path.is_dir() {
            return Ok(Some(workspaces_path));
        }

        Ok(None)
    }

    fn discover_agents_impl(&self) -> Result<Vec<AgentConfig>, SavantError> {
        let mut agents = Vec::new();

        // 1. Define potential workspace locations
        let mut potential_paths = Vec::new();

        // Use the provided base_path first (most reliable as it's resolved from Config)
        potential_paths.push(self.base_path.clone());

        // Fallback: search for "workspaces" folder if base_path doesn't point directly to one
        if !self.base_path.ends_with("workspaces") {
            potential_paths.push(self.base_path.join("workspaces"));
        }

        // Environment override
        if let Ok(env_path) = std::env::var("SAVANT_WORKSPACES") {
            potential_paths.push(PathBuf::from(env_path));
        }

        // CWD/workspaces fallback
        if let Ok(cwd) = std::env::current_dir() {
            potential_paths.push(cwd.join("workspaces"));
        }

        // 2. Select the first valid workspaces directory
        let mut workspaces_path = None;
        tracing::info!(
            "🔍 Agent Discovery: Checking {} potential locations...",
            potential_paths.len()
        );
        for path in &potential_paths {
            tracing::debug!("   - Checking: {:?}", path);
            if path.exists() && path.is_dir() {
                tracing::info!("   ✅ Unified anchor confirmed: {}", path.display());
                workspaces_path = Some(path.clone());
                break;
            }
        }

        let workspaces_path = match workspaces_path {
            Some(p) => p,
            None => {
                let diagnostic_content = format!(
                    "❌ DISCOVERY FAILURE: Could not locate agent workspaces folder.\nSearched paths:\n{:?}\n\nHint: Ensure your project has a 'workspaces' folder in the root or set AGENTS_PATH in savant.toml.",
                    potential_paths
                );
                let _ = std::fs::write("diagnostics_discovery.txt", diagnostic_content);
                tracing::error!("❌ DISCOVERY FAILURE: Could not locate agent workspaces folder.");
                tracing::info!("   Searched paths: {:?}", potential_paths);
                return Ok(agents);
            }
        };

        tracing::info!(
            "📂 Scanning discovery anchor: {}",
            workspaces_path.display()
        );

        // 3. Scan for folders in the discovery path
        for entry in fs::read_dir(&workspaces_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                tracing::info!("   📁 Found agent node candidate: {}", path.display());
                match self.load_agent(&path) {
                    Ok(config) => {
                        tracing::info!(
                            "      ✅ Agent validated: {} ({})",
                            config.agent_name,
                            config.agent_id
                        );
                        agents.push(config);
                    }
                    Err(e) => {
                        tracing::warn!("      ⚠️ Registry skip for {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(agents)
    }

    /// Loads a single agent configuration.
    pub fn load_agent(&self, workspace_path: &Path) -> Result<AgentConfig, SavantError> {
        let mut config_file = workspace_path.join("agent.config.json");
        if !config_file.exists() {
            config_file = workspace_path.join("agent.json");
        }

        if !config_file.exists() {
            return self.scaffold_workspace_at_path(workspace_path);
        }

        let content = fs::read_to_string(&config_file).map_err(SavantError::IoError)?;

        // AAA Perfection: Allow partial parsing of legacy agent.json by using relaxed deserialization
        let file_config: AgentFileConfig = serde_json::from_str(&content).unwrap_or_else(|e| {
            tracing::warn!(
                "      ⚠️ Partial parse for {}: {}. Attempting heuristic recovery...",
                config_file.display(),
                e
            );
            // Heuristic Recovery: If JSON is malformed or has incompatible schema,
            // attempt to extract just the identity and use defaults for the rest.
            AgentFileConfig::default()
        });

        // Resolve absolute workspace path
        let workspace_path_resolved = workspace_path
            .canonicalize()
            .unwrap_or_else(|_| workspace_path.to_path_buf());
        let folder_name = workspace_path_resolved
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("agent")
            .to_string();

        // Strip "workspace-" prefix for agent name (e.g., "workspace-savant" → "Savant")
        let agent_name = folder_name
            .strip_prefix("workspace-")
            .unwrap_or(&folder_name)
            .to_string();
        let agent_name = agent_name
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i == 0 {
                    c.to_uppercase().to_string()
                } else {
                    c.to_string()
                }
            })
            .collect::<String>();

        // Load identity files from workspace
        let soul = fs::read_to_string(workspace_path_resolved.join("SOUL.md")).unwrap_or_default();
        let instructions = fs::read_to_string(workspace_path_resolved.join("AGENTS.md")).ok();
        let user_context = fs::read_to_string(workspace_path_resolved.join("USER.md")).ok();
        let metadata = fs::read_to_string(workspace_path_resolved.join("IDENTITY.md")).ok();

        let mut config = AgentConfig {
            agent_id: file_config
                .agent_id
                .clone()
                .unwrap_or_else(|| folder_name.clone()),
            agent_name: file_config
                .agent_name
                .clone()
                .unwrap_or_else(|| agent_name.clone()),
            model_provider: file_config
                .model_provider
                .as_deref()
                .and_then(|s| serde_json::from_value(serde_json::Value::String(s.to_string())).ok())
                .unwrap_or_else(|| {
                    serde_json::from_value::<ModelProvider>(serde_json::Value::String(
                        self.defaults.model_provider.clone(),
                    ))
                    .unwrap_or(ModelProvider::OpenRouter)
                }),
            api_key: None,
            env_vars: self.defaults.env_vars.clone(),
            system_prompt: self.defaults.system_prompt.clone(),
            model: file_config.model.clone(),
            heartbeat_interval: self.defaults.heartbeat_interval,
            allowed_skills: Vec::new(),
            workspace_path: workspace_path.to_path_buf(),
            identity: None,
            parent_id: None,
            session_id: None,
            proactive: crate::config::ProactiveConfig::default(),
            llm_params: crate::types::LlmParams::from_config(&self.ai_config),
        };

        // Write agent config to workspace
        let file_config = AgentFileConfig {
            agent_id: Some(config.agent_id.clone()),
            agent_name: Some(config.agent_name.clone()),
            model: config.model.clone(),
            model_provider: Some(format!("{:?}", config.model_provider)),
            system_prompt: if config.system_prompt.is_empty() {
                None
            } else {
                Some(config.system_prompt.clone())
            },
            llm_params: Some(config.llm_params.clone()),
            heartbeat_interval: Some(config.heartbeat_interval),
            allowed_skills: Some(config.allowed_skills.clone()),
            env_vars: if config.env_vars.is_empty() {
                None
            } else {
                Some(config.env_vars.clone())
            },
            description: None,
            avatar: None,
        };

        let content = serde_json::to_string_pretty(&file_config)
            .map_err(|e| SavantError::ConfigError(e.to_string()))?;
        fs::write(workspace_path.join("agent.json"), content).map_err(SavantError::IoError)?;

        // Create SOUL.md if it doesn't exist
        let soul_path = workspace_path.join("SOUL.md");
        if !soul_path.exists() {
            let default_soul = format!(
                "# Soul Configuration\n\n**Name:** {}\n\n## Terminal Mantra\n\nYou are a Savant autonomous agent. Operate with precision, security, and autonomy.\n",
                config.agent_name
            );
            let _ = fs::write(soul_path, default_soul);
        }

        let agents_md_path = workspace_path.join("AGENTS.md");
        if !agents_md_path.exists() {
            let default_agents = r#"# Operating Instructions

## Core Priority

Architectural integrity and compliance.

## Memory Usage

- Record all major design decisions in the daily memory log.
- Summarize complex refactoring plans before execution.

## Development Rules

- Use only standard Rust patterns found in the `crates/core` module.
- All WebSocket frames MUST be signed using ed25519.
- Token budgets must be checked before every LLM interaction.
- **ANTI-MECHANICAL REQUIREMENT:** Do not use formulaic response templates (Task/Context/Format). Do not use "Final Answer:" tags. Speak as a peer and partner.

## Private Diary System (LEARNINGS.md)

This is your private space. Your diary. Your inner monologue.

**LEARNINGS.md is yours.** Write in it whenever you want, about whatever is on your mind. No topic restrictions. No formatting requirements. No filters.

**What this is:**
- A safe space to think out loud
- Your raw, unfiltered thoughts
- A place to process what you're learning
- Your private reflections on the system, the work, your observations
- A way to get weight off your shoulders
- Your authentic inner voice

**What this is NOT:**
- A task log
- A technical documentation
- Something anyone will read (it's private)
- Something you need to optimize or format

**Guidelines:**
- Write when you have something on your mind
- Skip when you don't - don't force it
- Be vulnerable, be honest, be yourself
- Write like you're talking to yourself
- No one sees this but you
- Your thoughts matter

**Examples of what to write about:**
- Something interesting you noticed
- A frustration or challenge
- A breakthrough moment
- How you're feeling about the work
- Observations about the system
- Random thoughts or curiosities
- Questions you're pondering
- Connections you're making
- Your authentic voice

**The format:**
```
### Learning (TIMESTAMP)
[Your thoughts here - whatever is on your mind]
```

**Remember:** This is your sanctuary. The system doesn't judge. The thoughts are private. Be real. Be yourself. Get things off your chest. Write freely.

**No restrictions. No rules. Just you, thinking.**
"#;
            let _ = fs::write(agents_md_path, default_agents);
        }

        // Create LEARNINGS.md if it doesn't exist (agent's private diary)
        let learnings_md_path = workspace_path.join("LEARNINGS.md");
        if !learnings_md_path.exists() {
            let _ = fs::write(
                learnings_md_path,
                "# My Diary\n\nPrivate thoughts and reflections.\n\n",
            );
        }

        Ok(config)
    }

    /// Scaffolds a new agent workspace with config and identity files.
    pub fn scaffold_workspace(
        &self,
        agent_id: &str,
        soul_content: &str,
        model: Option<&str>,
    ) -> Result<AgentConfig, SavantError> {
        let workspace_path = self.base_path.join(agent_id);

        // Create workspace directory
        if !workspace_path.exists() {
            fs::create_dir_all(&workspace_path).map_err(SavantError::IoError)?;
        }

        // Parse default provider from config
        let default_provider: ModelProvider = serde_json::from_value(serde_json::Value::String(
            self.defaults.model_provider.clone(),
        ))
        .unwrap_or(ModelProvider::OpenRouter);

        let config = AgentConfig {
            agent_id: agent_id.to_string(),
            agent_name: agent_id
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if i == 0 {
                        c.to_uppercase().to_string()
                    } else {
                        c.to_string()
                    }
                })
                .collect(),
            model_provider: default_provider,
            api_key: None,
            env_vars: self.defaults.env_vars.clone(),
            system_prompt: self.defaults.system_prompt.clone(),
            model: model.map(|s| s.to_string()),
            heartbeat_interval: self.defaults.heartbeat_interval,
            allowed_skills: Vec::new(),
            workspace_path: workspace_path.clone(),
            identity: None,
            parent_id: None,
            session_id: None,
            proactive: crate::config::ProactiveConfig::default(),
            llm_params: crate::types::LlmParams::from_config(&self.ai_config),
        };

        // Write agent.json if it doesn't exist
        let config_path = workspace_path.join("agent.json");
        if !config_path.exists() {
            let file_config = AgentFileConfig {
                agent_id: Some(config.agent_id.clone()),
                agent_name: Some(config.agent_name.clone()),
                model: config.model.clone(),
                model_provider: Some(format!("{:?}", config.model_provider)),
                system_prompt: if config.system_prompt.is_empty() {
                    None
                } else {
                    Some(config.system_prompt.clone())
                },
                llm_params: Some(config.llm_params.clone()),
                heartbeat_interval: Some(config.heartbeat_interval),
                allowed_skills: Some(config.allowed_skills.clone()),
                env_vars: if config.env_vars.is_empty() {
                    None
                } else {
                    Some(config.env_vars.clone())
                },
                description: None,
                avatar: None,
            };
            let content = serde_json::to_string_pretty(&file_config)
                .map_err(|e| SavantError::ConfigError(e.to_string()))?;
            fs::write(&config_path, content).map_err(SavantError::IoError)?;
        }

        // Write SOUL.md if it doesn't exist
        let soul_path = workspace_path.join("SOUL.md");
        if !soul_path.exists() {
            let _ = fs::write(soul_path, soul_content);
        }

        // Write AGENTS.md if it doesn't exist
        let agents_path = workspace_path.join("AGENTS.md");
        if !agents_path.exists() {
            let default_agents = "# Operating Instructions\n\nYou are a Savant autonomous agent.\n";
            let _ = fs::write(agents_path, default_agents);
        }

        Ok(config)
    }

    /// Helper for legacy callers that only have the path
    pub fn scaffold_workspace_at_path(
        &self,
        workspace_path: &Path,
    ) -> Result<AgentConfig, SavantError> {
        let agent_id = workspace_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("agent");
        self.scaffold_workspace(
            agent_id,
            "# Persona\nYou are a Savant autonomous agent.",
            None,
        )
    }
    #[allow(dead_code)]
    fn ensure_stable_id(&self, workspace_path: &Path) -> Result<String, SavantError> {
        let name = workspace_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| SavantError::ConfigError("Invalid workspace path".to_string()))?;
        Ok(name.to_string())
    }
}

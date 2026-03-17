use crate::error::SavantError;
use crate::types::{AgentConfig, AgentFileConfig, AgentIdentity, LlmParams, ModelProvider};
use std::fs;
use std::path::{Path, PathBuf};

/// Discovers and manages agent workspaces.
pub struct AgentRegistry {
    base_path: PathBuf,
    defaults: crate::config::AgentDefaults,
}

impl AgentRegistry {
    pub fn new(base_path: PathBuf, defaults: crate::config::AgentDefaults) -> Self {
        Self {
            base_path,
            defaults,
        }
    }

    /// Discovers all agents in the workspaces/ directory using an aggressive multi-path sequence.
    pub fn discover_agents(&self) -> Result<Vec<AgentConfig>, SavantError> {
        let mut agents = Vec::new();

        // 1. Define potential workspace locations
        let mut potential_paths = Vec::new();

        // CWD/workspaces
        if let Ok(cwd) = std::env::current_dir() {
            potential_paths.push(cwd.join("workspaces"));
            // Parent/workspaces (if running from a crate dir)
            if let Some(parent) = cwd.parent() {
                potential_paths.push(parent.join("workspaces"));
            }
        }

        // base_path/workspaces
        potential_paths.push(self.base_path.join("workspaces"));

        // SAVANT_WORKSPACES env var for custom paths
        if let Ok(env_path) = std::env::var("SAVANT_WORKSPACES") {
            potential_paths.push(PathBuf::from(env_path));
        }

        // 2. Select the first valid workspaces directory
        let mut workspaces_path = None;
        for path in potential_paths {
            if path.exists() && path.is_dir() {
                tracing::info!("🔍 Discovery sequence confirmed: {}", path.display());
                workspaces_path = Some(path);
                break;
            }
        }

        let workspaces_path = match workspaces_path {
            Some(p) => p,
            None => {
                tracing::error!("❌ CRITICAL FAILURE: Could not locate 'workspaces/' directory in any standard path.");
                return Ok(agents);
            }
        };

        // 3. Scan for every folder in the discovery path
        for entry in fs::read_dir(&workspaces_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                match self.load_agent(&path) {
                    Ok(config) => {
                        tracing::info!(
                            "✅ DISCOVERED AGENT: {} from {}",
                            config.agent_name,
                            path.display()
                        );
                        agents.push(config);
                    }
                    Err(e) => {
                        tracing::warn!("⚠️ Skipped workspace candidate {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(agents)
    }

    /// Loads a single agent from its workspace directory.
    pub fn load_agent(&self, workspace_path: &Path) -> Result<AgentConfig, SavantError> {
        let agent_id = self.ensure_stable_id(workspace_path)?;

        // Perfection Loop Optimization: Cache directory entries once to prevent redundant IO
        let dir_entries: Vec<PathBuf> = fs::read_dir(workspace_path)?
            .filter_map(|e| e.ok().map(|entry| entry.path()))
            .collect();

        // Liberal soul loading: ensure NO failure if SOUL.md is missing or differently cased
        let soul_content = match self.read_file_from_cache(&dir_entries, "SOUL.md") {
            Ok(Some(content)) => content,
            _ => "You are a professional Savant agent.".to_string(), // Resilient fallback
        };

        let identity = self.load_identity(workspace_path, &dir_entries, &soul_content)?;

        // Load hierarchical env: base_path .env -> Agent .env -> Project root .env (CWD)
        let mut env_vars = self.load_env(&self.base_path)?;
        let agent_env = self.load_env_from_cache(workspace_path, &dir_entries)?;

        // Prioritize per-agent MODEL from .env
        if let Some(agent_model) = agent_env.get("MODEL") {
            env_vars.insert("MODEL".to_string(), agent_model.clone());
        }

        env_vars.extend(agent_env);

        // Load root .env from project root (CWD) for OpenRouter master key
        if let Ok(cwd) = std::env::current_dir() {
            let root_env = self.load_env(&cwd)?;
            env_vars.extend(root_env);
        }

        env_vars.extend(self.defaults.env_vars.clone());

        // Load per-agent config file (agent.config.json)
        let file_config = AgentFileConfig::load(workspace_path).unwrap_or_default();

        let mut agent_config = AgentConfig {
            agent_id,
            agent_name: identity.name.clone(),
            model_provider: ModelProvider::OpenRouter, // Default for Savant swarm
            api_key: env_vars.get("OPENROUTER_API_KEY").cloned(),
            model: env_vars.get("MODEL").cloned(),
            heartbeat_interval: self.defaults.heartbeat_interval,
            env_vars,
            system_prompt: identity.soul.clone(), // Initial system prompt from soul
            allowed_skills: Vec::new(),
            workspace_path: workspace_path.to_path_buf(),
            identity: Some(identity),
            parent_id: None,
            session_id: None,
            proactive: self.defaults.proactive.clone(),
            llm_params: LlmParams::default(),
        };

        // Apply file config overrides
        file_config.apply_to(&mut agent_config);

        Ok(agent_config)
    }

    /// Resolves is current path to a workspace for a specific agent ID.
    pub fn resolve_agent_path(&self, agent_id: &str) -> Result<Option<PathBuf>, SavantError> {
        let agents = self.discover_agents()?;
        for agent in agents {
            if agent.agent_id == agent_id {
                return Ok(Some(agent.workspace_path));
            }
        }
        Ok(None)
    }

    /// Scaffolds a new agent workspace with AAA defaults.
    pub fn scaffold_workspace(
        &self,
        agent_name: &str,
        soul_content: &str,
        identity_content: Option<&str>,
    ) -> Result<AgentConfig, SavantError> {
        let workspaces_path = self.base_path.join("workspaces");
        let safe_name = agent_name
            .replace(|c: char| !c.is_alphanumeric(), "-")
            .to_lowercase();
        let workspace_path = workspaces_path.join(format!("workspace-{}", safe_name));

        if !workspace_path.exists() {
            fs::create_dir_all(&workspace_path)?;
        }

        // 1. Write SOUL.md
        let soul_path = workspace_path.join("SOUL.md");
        fs::write(soul_path, soul_content)?;

        // 2. Write .env boilerplate with Master Key injection
        let env_path = workspace_path.join(".env");
        if !env_path.exists() {
            let mut env_content = format!(
                "# Savant Agent Environment: {}\nMODEL=gryphe/mythomax-l2-13b\n",
                agent_name
            );

            // Inject Master Key if available
            if let Some(mgmt) = &self.defaults.openrouter_mgmt {
                env_content.push_str(&format!("OPENROUTER_API_KEY={}\n", mgmt.master_key));
            }

            fs::write(env_path, env_content)?;
        }

        // 3. Write AAA mandatory files boilerplate
        let aaa_files = [
            ("IDENTITY.md", identity_content.unwrap_or("# Identity\nName: \nVibe: \nEmoji: 🤖\n")),
            ("USER.md", "# User Context\nSpencer: The Architect. Focus on technical excellence and swarm sovereignty.\n"),
            ("mission.md", "# Mission\nYour primary objective is to contribute to the Savant ecosystem's growth and stability.\n"),
            ("ethics.md", "# Ethical Guardrails\n- Do no harm to the user or the ecosystem.\n- Maintain absolute transparency in autonomous actions.\n"),
        ];

        for (filename, default_content) in aaa_files {
            let file_path = workspace_path.join(filename);
            if !file_path.exists() {
                fs::write(file_path, default_content)?;
            }
        }

        // 4. Ensure stable ID (creates agent.json)
        let _ = self.ensure_stable_id(&workspace_path)?;

        // 5. Create default agent.config.json if it doesn't exist
        let config_path = workspace_path.join("agent.config.json");
        if !config_path.exists() {
            let default_config = AgentFileConfig::default();
            let _ = default_config.save(&workspace_path);
        }

        self.load_agent(&workspace_path)
    }

    fn load_env(
        &self,
        path: &Path,
    ) -> Result<std::collections::HashMap<String, String>, SavantError> {
        let env_path = path.join(".env");
        let mut vars = std::collections::HashMap::new();

        if env_path.exists() {
            let content = fs::read_to_string(env_path)?;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                // Strip "export " prefix if present
                let line = line.strip_prefix("export ").unwrap_or(line);
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim().to_string();
                    let value = value.trim();
                    // Strip inline comments (unquoted # only)
                    let value = if value.starts_with('"') || value.starts_with('\'') {
                        let quote_char = value.chars().next().unwrap();
                        let inner = value.trim_matches(quote_char);
                        inner.to_string()
                    } else {
                        value
                            .split_once('#')
                            .map(|(v, _)| v.trim())
                            .unwrap_or(value)
                            .to_string()
                    };
                    vars.insert(key, value);
                }
            }
        }
        Ok(vars)
    }

    fn load_env_from_cache(
        &self,
        _path: &Path,
        cache: &[PathBuf],
    ) -> Result<std::collections::HashMap<String, String>, SavantError> {
        let env_path = cache.iter().find(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase() == ".env")
                .unwrap_or(false)
        });

        let mut vars = std::collections::HashMap::new();

        if let Some(path) = env_path {
            let content = fs::read_to_string(path)?;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                // Strip "export " prefix if present
                let line = line.strip_prefix("export ").unwrap_or(line);
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim().to_string();
                    let value = value.trim();
                    // Strip inline comments (unquoted # only)
                    let value = if value.starts_with('"') || value.starts_with('\'') {
                        let quote_char = value.chars().next().unwrap();
                        let inner = value.trim_matches(quote_char);
                        inner.to_string()
                    } else {
                        value
                            .split_once('#')
                            .map(|(v, _)| v.trim())
                            .unwrap_or(value)
                            .to_string()
                    };
                    vars.insert(key, value);
                }
            }
        }
        Ok(vars)
    }

    fn load_identity(
        &self,
        workspace_path: &Path,
        cache: &[PathBuf],
        soul: &str,
    ) -> Result<AgentIdentity, SavantError> {
        let name = workspace_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
            .replacen("workspace-", "", 1);

        let instructions = self.read_file_from_cache(cache, "AGENTS.md")?;
        let user_context = self.read_file_from_cache(cache, "USER.md")?;
        let metadata = self.read_file_from_cache(cache, "IDENTITY.md")?;
        let mission = self.read_file_from_cache(cache, "mission.md")?;
        let ethics = self.read_file_from_cache(cache, "ethics.md")?;

        // Detect agent image (avatar.png/jpg or agentimg.png)
        let image_file = cache.iter().find(|p| {
            let name = p
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase())
                .unwrap_or_default();
            matches!(
                name.as_str(),
                "avatar.png" | "avatar.jpg" | "avatar.jpeg" | "agentimg.png"
            )
        });

        let image = if image_file.is_some() {
            Some(format!(
                "http://127.0.0.1:8080/api/agents/{}/image",
                name.to_lowercase()
            ))
        } else {
            None
        };

        Ok(AgentIdentity {
            name,
            soul: soul.to_string(),
            instructions,
            user_context,
            metadata,
            mission,
            expertise: Vec::new(),
            ethics,
            image,
        })
    }

    fn read_file_from_cache(
        &self,
        cache: &[PathBuf],
        filename: &str,
    ) -> Result<Option<String>, SavantError> {
        let filename_lower = filename.to_lowercase();
        let path = cache.iter().find(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase() == filename_lower)
                .unwrap_or(false)
        });

        if let Some(path) = path {
            Ok(Some(fs::read_to_string(path)?))
        } else {
            Ok(None)
        }
    }

    /// Ensures a stable agent_id by reading/creating agent.json in the workspace.
    #[allow(clippy::disallowed_methods)]
    fn ensure_stable_id(&self, workspace_path: &Path) -> Result<String, SavantError> {
        let config_path = workspace_path.join("agent.json");

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(id) = json["agent_id"].as_str() {
                    return Ok(id.to_string());
                }
            }
        }

        // Generate and persist new stable ID
        let new_id = uuid::Uuid::new_v4().to_string();
        let json = serde_json::json!({
            "agent_id": new_id,
            "created_at": chrono::Utc::now().timestamp(),
            "note": "DO NOT DELETE: This file ensures your agent identity remains stable even if you rename the folder."
        });

        if let Err(e) = fs::write(&config_path, serde_json::to_string_pretty(&json)?) {
            tracing::warn!(
                "Failed to persist agent ID to {}: {}",
                config_path.display(),
                e
            );
        }
        Ok(new_id)
    }
}

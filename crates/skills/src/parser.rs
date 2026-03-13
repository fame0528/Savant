use savant_core::types::{SkillManifest, CapabilityGrants};
use savant_core::error::SavantError;
use savant_core::traits::Tool;
use crate::sandbox::{SandboxDispatcher, ToolExecutor};
use std::path::{Path, PathBuf};
use tokio::fs;
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{info, warn, error};
use async_trait::async_trait;

/// A Savant Tool backed by a Skill execution engine (WASM or Native).
pub struct SkillTool {
    manifest: SkillManifest,
    executor: Box<dyn ToolExecutor>,
}

impl SkillTool {
    /// Creates a new SkillTool from a manifest and workspace directory.
    pub fn new(manifest: SkillManifest, workspace_dir: PathBuf) -> Self {
        let executor = SandboxDispatcher::create_executor(
            &manifest.execution_mode,
            workspace_dir,
            manifest.capabilities.clone(),
        );
        Self { manifest, executor }
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str { &self.manifest.name }
    fn description(&self) -> &str { &self.manifest.description }
    fn capabilities(&self) -> CapabilityGrants { self.manifest.capabilities.clone() }
    async fn execute(&self, payload: serde_json::Value) -> Result<String, SavantError> {
        self.executor.execute(payload).await
    }
}

/// Registry for managing agent skills and their capabilities.
/// Implements two-stage discovery to optimize LLM context window.
pub struct SkillRegistry {
    /// Maps skill names to their full manifests
    pub manifests: HashMap<String, SkillManifest>,
    /// Maps skill names to their initialized tools
    pub tools: HashMap<String, Arc<dyn Tool>>,
}

impl SkillRegistry {
    /// Creates a new, empty SkillRegistry.
    pub fn new() -> Self {
        Self {
            manifests: HashMap::new(),
            tools: HashMap::new(),
        }
    }

    /// Stage 1: Compact Discovery. 
    /// Generates a highly compressed string of available skills for the system prompt.
    /// Only contains name and description to save tokens.
    pub fn generate_compact_discovery_list(&self) -> String {
        let mut list = String::from("Available Domain Skills:\n");
        if self.manifests.is_empty() {
            list.push_str("- No skills currently loaded.\n");
            return list;
        }

        for (name, manifest) in &self.manifests {
            list.push_str(&format!("- {}: {}\n", name, manifest.description));
        }
        list.push_str("\nTo use a skill, invoke it by name in your thoughts/actions.\n");
        list
    }

    /// Stage 2: On-Demand Loading.
    /// Retrieves the full markdown instructions only when the agent explicitly needs them.
    pub fn get_skill_instructions(&self, skill_name: &str) -> Option<String> {
        self.manifests.get(skill_name).map(|s| s.instructions.clone())
    }

    /// Parses an OpenClaw-compatible SKILL.md file.
    /// Enforces strict YAML frontmatter validation.
    pub async fn load_skill_from_file(&mut self, path: impl AsRef<Path>) -> Result<(), SavantError> {
        let path_ref = path.as_ref();
        let content = fs::read_to_string(path_ref).await
           .map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to read {}: {}", path_ref.display(), e))))?;

        // Extract YAML frontmatter (between --- markers)
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        
        // If the file starts with ---, parts[0] is empty. We need exactly 3 parts.
        if parts.len() < 3 {
             return Err(SavantError::Unknown(format!("Invalid SKILL.md format (missing frontmatter separator) in {}", path_ref.display())));
        }

        // Handle both cases: file starting with --- or having empty leader
        let frontmatter = if parts[0].trim().is_empty() {
            parts[1]
        } else {
            // This case shouldn't happen with standard frontmatter, but we'll be resilient
            parts[1]
        };
        
        let instructions = parts[2].trim().to_string();

        // Strict YAML parsing.
        let mut manifest: SkillManifest = serde_yaml::from_str(frontmatter)
            .map_err(|e| SavantError::Unknown(format!("YAML parse error in {}: {}", path_ref.display(), e)))?;
        
        manifest.instructions = instructions;

        info!("Loaded skill: {} (v{})", manifest.name, manifest.version);
        
        // Initialize the tool with its specific sandbox executor
        let skill_dir = path_ref.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        let tool = Arc::new(SkillTool::new(manifest.clone(), skill_dir));
        
        self.tools.insert(manifest.name.clone(), tool);
        self.manifests.insert(manifest.name.clone(), manifest);
        Ok(())
    }

    /// Recursively discover and load all skills in a directory.
    pub async fn discover_skills(&mut self, directory: impl AsRef<Path>) -> Result<usize, SavantError> {
        let mut count = 0;
        
        // Use walkdir for recursive discovery
        for entry in walkdir::WalkDir::new(directory) {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("Skill discovery WalkDir error: {}", e);
                    continue;
                }
            };

            if entry.file_type().is_file() && entry.file_name() == "SKILL.md" {
                if let Err(e) = self.load_skill_from_file(entry.path()).await {
                    error!("Failed to load skill at {}: {}", entry.path().display(), e);
                } else {
                    count += 1;
                }
            }
        }
        
        info!("Skill discovery complete. Total skills loaded: {}", count);
        Ok(count)
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;
use tracing::{debug, warn};

/// A layer in the context engine
#[derive(Debug, Clone)]
pub struct ContextLayer {
    pub name: String,
    pub content: String,
    pub token_estimate: usize,
    pub priority: u8,
}

/// Three-layer context engine
pub struct ContextEngine {
    pub global: Option<ContextLayer>,
    pub project: Option<ContextLayer>,
    pub auto_memory: Option<ContextLayer>,
    max_tokens: usize,
}

impl ContextEngine {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            global: None,
            project: None,
            auto_memory: None,
            max_tokens,
        }
    }

    /// Load all context layers for the given project root
    pub async fn load(&mut self, project_root: &Path) -> Result<()> {
        self.load_global().await;
        self.load_project(project_root).await;
        self.load_auto_memory(project_root).await;
        Ok(())
    }

    /// Load global constraints from ~/.config/savant_cli/CONSTRAINTS.md
    async fn load_global(&mut self) {
        let path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".config")
            .join("savant_cli")
            .join("CONSTRAINTS.md");

        if path.exists() {
            match fs::read_to_string(&path).await {
                Ok(content) => {
                    let token_estimate = estimate_tokens(&content);
                    if token_estimate <= 300 {
                        self.global = Some(ContextLayer {
                            name: "CONSTRAINTS.md".to_string(),
                            content,
                            token_estimate,
                            priority: 1,
                        });
                        debug!("Loaded global constraints: {} tokens", token_estimate);
                    } else {
                        warn!(
                            "Global constraints file too large ({} tokens, max 300)",
                            token_estimate
                        );
                    }
                }
                Err(e) => {
                    debug!("Failed to load global constraints: {}", e);
                }
            }
        }
    }

    /// Load project context from PROJECT.md
    async fn load_project(&mut self, project_root: &Path) {
        let path = project_root.join("PROJECT.md");

        if path.exists() {
            match fs::read_to_string(&path).await {
                Ok(content) => {
                    let token_estimate = estimate_tokens(&content);
                    self.project = Some(ContextLayer {
                        name: "PROJECT.md".to_string(),
                        content,
                        token_estimate,
                        priority: 2,
                    });
                    debug!("Loaded project context: {} tokens", token_estimate);
                }
                Err(e) => {
                    debug!("Failed to load project context: {}", e);
                }
            }
        }
    }

    /// Load auto-memory from MEMORY.md
    async fn load_auto_memory(&mut self, project_root: &Path) {
        let path = project_root.join("MEMORY.md");

        if path.exists() {
            match fs::read_to_string(&path).await {
                Ok(content) => {
                    let token_estimate = estimate_tokens(&content);
                    self.auto_memory = Some(ContextLayer {
                        name: "MEMORY.md".to_string(),
                        content,
                        token_estimate,
                        priority: 3,
                    });
                    debug!("Loaded auto-memory: {} tokens", token_estimate);
                }
                Err(e) => {
                    debug!("Failed to load auto-memory: {}", e);
                }
            }
        }
    }

    /// Get combined context within the default token budget
    pub fn get_context(&self) -> String {
        self.get_context_with_budget(self.max_tokens)
    }

    /// Get combined context within a custom token budget
    pub fn get_context_with_budget(&self, budget: usize) -> String {
        let mut layers: Vec<&ContextLayer> = Vec::new();

        if let Some(ref global) = self.global {
            layers.push(global);
        }
        if let Some(ref project) = self.project {
            layers.push(project);
        }
        if let Some(ref memory) = self.auto_memory {
            layers.push(memory);
        }

        layers.sort_by_key(|l| l.priority);

        let mut result = String::new();
        let mut used_tokens = 0;

        for layer in layers {
            if used_tokens + layer.token_estimate > budget {
                let remaining = budget.saturating_sub(used_tokens);
                let char_limit = remaining * 4;
                let truncated = truncate_to_token_limit(&layer.content, char_limit);
                result.push_str(&format!("\n## {} (truncated)\n{}\n", layer.name, truncated));
                used_tokens += remaining;
            } else {
                result.push_str(&format!("\n## {}\n{}\n", layer.name, layer.content));
                used_tokens += layer.token_estimate;
            }
        }

        result
    }

    /// Get total token count across all layers
    pub fn total_tokens(&self) -> usize {
        let mut total = 0;
        if let Some(ref l) = self.global {
            total += l.token_estimate;
        }
        if let Some(ref l) = self.project {
            total += l.token_estimate;
        }
        if let Some(ref l) = self.auto_memory {
            total += l.token_estimate;
        }
        total
    }

    /// Update auto-memory content
    pub async fn update_auto_memory(&mut self, project_root: &Path, content: &str) -> Result<()> {
        let path = project_root.join("MEMORY.md");
        fs::write(&path, content)
            .await
            .with_context(|| format!("Failed to write MEMORY.md: {:?}", path))?;

        let token_estimate = estimate_tokens(content);
        self.auto_memory = Some(ContextLayer {
            name: "MEMORY.md".to_string(),
            content: content.to_string(),
            token_estimate,
            priority: 3,
        });

        debug!("Updated auto-memory: {} tokens", token_estimate);
        Ok(())
    }
}

/// Estimate tokens from text (rough: ~4 chars per token)
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Truncate text to fit within a character limit while preserving structure
fn truncate_to_token_limit(text: &str, char_limit: usize) -> String {
    if text.len() <= char_limit {
        return text.to_string();
    }

    let mut truncated = String::new();
    let mut char_count = 0;

    for line in text.lines() {
        if char_count + line.len() + 1 > char_limit {
            break;
        }
        truncated.push_str(line);
        truncated.push('\n');
        char_count += line.len() + 1;
    }

    if truncated.is_empty() {
        truncated = text.chars().take(char_limit).collect();
    }

    truncated.push_str("\n... (truncated)");
    truncated
}

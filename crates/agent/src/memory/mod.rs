use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::MemoryBackend;
use savant_core::types::{AgentReflection, ChatMessage};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{info, instrument};

/// A decorator for `MemoryBackend` that adds file-based logging for agent self-improvement.
/// 
/// This implements the "Perfection Loop" requirements by ensuring that every learning
/// and reflection is also captured in human-readable Markdown files in the agent's workspace.
pub struct FileLoggingMemoryBackend {
    inner: Arc<dyn MemoryBackend>,
    workspace_path: PathBuf,
}

impl FileLoggingMemoryBackend {
    /// Creates a new `FileLoggingMemoryBackend`.
    pub fn new(inner: Arc<dyn MemoryBackend>, workspace_path: PathBuf) -> Self {
        Self {
            inner,
            workspace_path,
        }
    }

    /// Records a new learning or correction to LEARNINGS.md.
    #[instrument(skip(self), fields(agent_id))]
    pub async fn record_learning(&self, agent_id: &str, learning: &str) -> Result<(), SavantError> {
        let path = self.workspace_path.join("LEARNINGS.md");
        let timestamp = chrono::Utc::now();
        let content = format!("\n### Learning ({})\n{}\n", timestamp, learning);

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;

        file.write_all(content.as_bytes()).await?;
        
        // Also store in the inner backend as a special metadata message if needed
        // For now, we trust the file and the inner backend's general storage
        info!("Recorded learning for agent {}", agent_id);
        Ok(())
    }

    /// Records a reflection on a completed task to REFLECT.md.
    #[instrument(skip(self), fields(agent_id))]
    pub async fn record_reflection(
        &self,
        agent_id: &str,
        reflection: AgentReflection,
    ) -> Result<(), SavantError> {
        let path = self.workspace_path.join("REFLECT.md");
        let content = format!(
            "\n## Reflection: {}\n- Success: {}\n- Critique: {}\n- Learning: {}\n- Action Items: {:?}\n",
            reflection.task_id, reflection.success, reflection.critique, reflection.learning, reflection.action_items
        );

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;

        file.write_all(content.as_bytes()).await?;
        
        info!("Recorded reflection for agent {}", agent_id);
        Ok(())
    }
}

#[async_trait]
impl MemoryBackend for FileLoggingMemoryBackend {
    async fn store(&self, agent_id: &str, message: &ChatMessage) -> Result<(), SavantError> {
        self.inner.store(agent_id, message).await
    }

    async fn retrieve(
        &self,
        agent_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ChatMessage>, SavantError> {
        self.inner.retrieve(agent_id, query, limit).await
    }

    async fn consolidate(&self, agent_id: &str) -> Result<(), SavantError> {
        // 1. First delegate to inner backend for LSM compaction/optimization
        self.inner.consolidate(agent_id).await?;

        // 2. Perform file-based archive if LEARNINGS.md gets too large
        let learnings_path = self.workspace_path.join("LEARNINGS.md");
        let history_path = self.workspace_path.join("HISTORY.md");

        if let Ok(metadata) = fs::metadata(&learnings_path).await {
            if metadata.len() > 50_000 {
                info!(
                    "[{}] Consolidating memory: LEARNINGS.md is too large ({} bytes). Archiving...",
                    agent_id,
                    metadata.len()
                );
                let content = fs::read_to_string(&learnings_path).await?;
                let mut archive = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&history_path)
                    .await?;
                
                archive
                    .write_all(
                        format!("\n--- ARCHIVE SESSION: {} ---\n", chrono::Utc::now()).as_bytes(),
                    )
                    .await?;
                archive.write_all(content.as_bytes()).await?;
                fs::write(&learnings_path, "# Active Learnings\n").await?;
            }
        }
        Ok(())
    }
}

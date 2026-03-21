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
#[derive(Clone)]
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

    /// Records a new learning or correction to LEARNINGS.md (free-form).
    ///
    /// Writes in the original free-form markdown format, preserving Savant's
    /// internal monologue without constraining it to JSONL structure.
    #[instrument(skip(self), fields(agent_id))]
    pub async fn record_learning(
        &self,
        agent_id: &str,
        learning_text: &str,
    ) -> Result<(), SavantError> {
        let md_path = self.workspace_path.join("LEARNINGS.md");

        // Get current UTC timestamp
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.9f UTC");

        // AAA: Extract Lens Tag into Header (Phase 19)
        // If content starts with "# [TAG]", we extract it and put it in the header.
        let (tag, final_text) = if learning_text.starts_with("# [") {
            if let Some(end_idx) = learning_text.find("]") {
                let tag = &learning_text[3..end_idx];
                let rest = &learning_text[end_idx + 1..].trim();
                (format!(" [{}]", tag), rest.to_string())
            } else {
                (String::new(), learning_text.to_string())
            }
        } else {
            (String::new(), learning_text.to_string())
        };

        // Write in free-form markdown format
        let entry = format!("\n\n### Learning ({}){}\n{}\n", timestamp, tag, final_text);

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&md_path)
            .await?;

        file.write_all(entry.as_bytes()).await?;

        info!("Recorded free-form learning{} for agent {}", tag, agent_id);
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

    /// Parses LEARNINGS.md into JSONL format for dashboard display.
    /// This can be called manually to refresh the reflections panel.
    pub async fn parse_learnings(&self, agent_id: &str) -> Result<usize, SavantError> {
        let parser = crate::learning::LearningsParser::new(self.workspace_path.clone());
        let count = parser.parse_and_convert(agent_id)?;
        info!(
            "[{}] Manually parsed {} learning entries from LEARNINGS.md",
            agent_id, count
        );
        Ok(count)
    }
}

#[async_trait]
impl MemoryBackend for FileLoggingMemoryBackend {
    async fn store(&self, agent_id: &str, message: &ChatMessage) -> Result<(), SavantError> {
        // 🛡️ Sovereign Routing: Use the channel type, not string heuristics
        if message.channel == savant_core::types::AgentOutputChannel::Memory {
            let _ = self.record_learning(agent_id, &message.content).await;
        }

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

        // 2. Parse new LEARNINGS.md entries into JSONL (for dashboard display)
        let parser = crate::learning::LearningsParser::new(self.workspace_path.clone());
        match parser.parse_and_convert(agent_id) {
            Ok(count) => {
                if count > 0 {
                    info!(
                        "[{}] Converted {} new learning entries from LEARNINGS.md → JSONL",
                        agent_id, count
                    );
                    // 2.5. Store parsed entries in swarm.insights for dashboard API
                    let learnings_path = self.workspace_path.join("LEARNINGS.jsonl");
                    if let Ok(content) = std::fs::read_to_string(&learnings_path) {
                        for line in content.lines() {
                            if let Ok(entry) = serde_json::from_str::<
                                savant_core::learning::EmergentLearning,
                            >(line)
                            {
                                // Check if already in swarm.insights by looking for this timestamp
                                let msg = savant_core::types::ChatMessage {
                                    is_telemetry: false,
                                    role: savant_core::types::ChatRole::Assistant,
                                    content: entry.content.clone(),
                                    sender: Some(entry.agent_id.clone()),
                                    recipient: None,
                                    agent_id: None,
                                    session_id: Some(savant_core::types::SessionId(
                                        "learning.swarm".to_string(),
                                    )),
                                    channel: savant_core::types::AgentOutputChannel::Memory,
                                };
                                let _ = self.inner.store("swarm.insights", &msg).await;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("[{}] Failed to parse LEARNINGS.md: {}", agent_id, e);
            }
        }

        // 3. Archive old JSONL if too large
        let learnings_path = self.workspace_path.join("LEARNINGS.jsonl");
        let history_path = self.workspace_path.join("HISTORY.jsonl");

        if let Ok(metadata) = fs::metadata(&learnings_path).await {
            if metadata.len() > 500_000 {
                // Archive at 500KB
                info!(
                    "[{}] Consolidating memory: LEARNINGS.jsonl is too large ({} bytes). Archiving...",
                    agent_id,
                    metadata.len()
                );
                let content = fs::read_to_string(&learnings_path).await?;
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&history_path)
                    .await?;
                file.write_all(content.as_bytes()).await?;
                fs::write(&learnings_path, "").await?; // Reset JSONL
            }
        }

        Ok(())
    }
}

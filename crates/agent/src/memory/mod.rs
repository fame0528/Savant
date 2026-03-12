use savant_core::types::AgentReflection;
use savant_core::error::SavantError;
use std::path::PathBuf;
use tokio::fs;
use savant_core::db::Storage;
use std::sync::Arc;

/// Manages the elite long-term memory system with WAL-backed persistence.
pub struct MemoryManager {
    workspace_path: PathBuf,
    storage: Arc<Storage>,
}

impl MemoryManager {
    pub fn new(workspace_path: PathBuf, storage: Arc<Storage>) -> Self {
        Self { workspace_path, storage }
    }

    /// Records a new learning or correction to LEARNINGS.md and SQLite.
    pub async fn record_learning(&self, agent_id: &str, learning: &str) -> Result<(), SavantError> {
        // 1. File Persistence
        let path = self.workspace_path.join("LEARNINGS.md");
        let timestamp = chrono::Utc::now();
        let content = format!("\n### Learning ({})\n{}\n", timestamp, learning);
        
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
            
        use tokio::io::AsyncWriteExt;
        file.write_all(content.as_bytes()).await?;

        // 2. WAL Database Persistence
        let conn = self.storage.connect()?;
        conn.execute(
            "INSERT INTO agents_memory (agent_id, category, content, importance) VALUES (?, ?, ?, ?)",
            rusqlite::params![agent_id, "Learning", learning, 5],
        ).map_err(|e| SavantError::Unknown(format!("DB memory error: {}", e)))?;

        Ok(())
    }

    /// Records a reflection on a completed task to REFLECT.md and SQLite.
    pub async fn record_reflection(&self, agent_id: &str, reflection: AgentReflection) -> Result<(), SavantError> {
        // 1. File Persistence
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

        use tokio::io::AsyncWriteExt;
        file.write_all(content.as_bytes()).await?;

        // 2. WAL Database Persistence
        let conn = self.storage.connect()?;
        conn.execute(
            "INSERT INTO agents_memory (agent_id, category, content, importance) VALUES (?, ?, ?, ?)",
            rusqlite::params![agent_id, "Reflection", format!("{:?}", reflection), reflection.importance],
        ).map_err(|e| SavantError::Unknown(format!("DB reflection error: {}", e)))?;

        Ok(())
    }

    /// Performs memory consolidation: moving daily logs to structured HISTORY.md
    pub async fn consolidate(&self, agent_id: &str) -> Result<(), SavantError> {
        let learnings_path = self.workspace_path.join("LEARNINGS.md");
        let history_path = self.workspace_path.join("HISTORY.md");

        if let Ok(metadata) = fs::metadata(&learnings_path).await {
            // If LEARNINGS.md is > 50KB, archive it
            if metadata.len() > 50_000 {
                tracing::info!("[{}] Consolidating memory: LEARNINGS.md is too large ({} bytes).", agent_id, metadata.len());
                
                let content = fs::read_to_string(&learnings_path).await?;
                
                let mut archive = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&history_path)
                    .await?;
                
                use tokio::io::AsyncWriteExt;
                archive.write_all(format!("\n--- ARCHIVE SESSION: {} ---\n", chrono::Utc::now()).as_bytes()).await?;
                archive.write_all(content.as_bytes()).await?;
                
                // Clear the current learnings file for fresh start
                fs::write(&learnings_path, "# Active Learnings\n").await?;
                
                tracing::info!("[{}] Consolidation complete. Historical context preserved in HISTORY.md.", agent_id);
            }
        }

        Ok(())
    }
}

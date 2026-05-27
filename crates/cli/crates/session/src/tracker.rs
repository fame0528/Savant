use anyhow::{Context, Result};
use chrono::Utc;
use savant_core::config::ObsidianConfig;
use savant_obsidian::{atomic_write, VaultWriter};
use std::path::Path;
use tracing::{debug, info};

use crate::session::{Session, SessionStats};

/// Tracks a dev session and writes to Glass House on completion
pub struct SessionTracker {
    session: Option<Session>,
    vault_writer: Option<VaultWriter>,
    vault_path: String,
}

impl SessionTracker {
    pub fn new(vault_path: Option<String>) -> Self {
        let vault_path = vault_path.unwrap_or_else(|| "memory-vault".to_string());
        Self {
            session: None,
            vault_writer: None,
            vault_path,
        }
    }

    /// Initialize a new session
    pub async fn start_session(&mut self, cwd: &Path) -> Result<()> {
        let session = Session::new(cwd).await?;
        info!("Started new session: {}", session.id);

        let vault_dir = Path::new(&self.vault_path);
        if vault_dir.exists() {
            let config = ObsidianConfig::default();
            self.vault_writer = Some(VaultWriter::new(
                vault_dir.to_path_buf(),
                None,
                config,
                "cli-companion".to_string(),
            ));
        }

        self.session = Some(session);
        Ok(())
    }

    /// Get the current session
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// Record a turn in the current session
    pub async fn record_turn(
        &mut self,
        role: &str,
        content: &str,
        token_count: usize,
    ) -> Result<()> {
        if let Some(ref mut session) = self.session {
            let record = crate::session::SessionRecord {
                turn_index: session.records.len(),
                timestamp: Utc::now().to_rfc3339(),
                role: role.to_string(),
                content: content.to_string(),
                tool_calls: Vec::new(),
                token_count,
                active_files: Vec::new(),
                focus_chain: session.focus_chain.clone(),
            };
            session.append_record(&record).await?;
        }
        Ok(())
    }

    /// End the current session and write to Glass House
    pub async fn end_session(&mut self) -> Result<()> {
        if let Some(ref session) = self.session {
            let stats = session.stats();
            info!(
                "Session {} ended: {} turns, {} tokens, {}s duration",
                session.id, stats.total_turns, stats.total_tokens, stats.duration
            );

            if let Some(ref _writer) = self.vault_writer {
                self.write_vault_summary(session, &stats).await?;
            }
        }

        self.session = None;
        Ok(())
    }

    /// Write a structured session summary to the Obsidian vault
    async fn write_vault_summary(&self, session: &Session, stats: &SessionStats) -> Result<()> {
        let date = Utc::now().format("%Y-%m-%d");
        let title = format!("CLI Session {} ({})", &session.id[..8], date);

        let content = format!(
            r#"# {title}

## Metadata
- **Session ID:** {id}
- **Started:** {started}
- **Ended:** {ended}
- **Duration:** {duration}s
- **Total Turns:** {turns}
- **Total Tokens:** {tokens}

## Statistics
- User Messages: {user_turns}
- Assistant Messages: {assistant_turns}
- Tool Calls: {tool_calls}
- Files Modified: {files}

## Focus Chain
{focus_chain}
"#,
            id = session.id,
            started = session.started_at,
            ended = session.last_activity,
            duration = stats.duration,
            turns = stats.total_turns,
            tokens = stats.total_tokens,
            user_turns = stats.user_turns,
            assistant_turns = stats.assistant_turns,
            tool_calls = stats.tool_calls,
            files = stats.files_modified,
            focus_chain = session
                .focus_chain
                .iter()
                .enumerate()
                .map(|(i, item)| format!("{}. {}", i + 1, item))
                .collect::<Vec<_>>()
                .join("\n"),
        );

        let dest_path = Path::new(&self.vault_path)
            .join("Working")
            .join("dev-sessions")
            .join(format!("{}.md", session.id));

        atomic_write(&dest_path, &content)
            .await
            .with_context(|| format!("Failed to write session summary: {:?}", dest_path))?;

        debug!("Wrote session summary to vault: {:?}", dest_path);
        Ok(())
    }
}

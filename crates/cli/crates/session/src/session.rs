use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, warn};
use uuid::Uuid;

/// A single turn record in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub turn_index: usize,
    pub timestamp: String,
    pub role: String,
    pub content: String,
    pub tool_calls: Vec<ToolCallRecord>,
    pub token_count: usize,
    pub active_files: Vec<String>,
    pub focus_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub name: String,
    pub input: serde_json::Value,
    pub output: Option<String>,
    pub status: String,
    pub duration_ms: u64,
}

/// State of a turn in the Perfection Loop
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TurnState {
    Red,
    Green,
    Audit,
    Certify,
    CircuitBreaker,
}

/// A complete session
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub cwd_hash: String,
    pub path: PathBuf,
    pub records: Vec<SessionRecord>,
    pub focus_chain: Vec<String>,
    pub started_at: String,
    pub last_activity: String,
    pub total_tokens: usize,
}

impl Session {
    /// Create a new session for the current working directory
    pub async fn new(cwd: &Path) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let cwd_hash = blake3::hash(cwd.to_string_lossy().as_bytes())
            .to_hex()
            .to_string();
        let now = Utc::now().to_rfc3339();

        let session_dir = Self::session_dir(&cwd_hash);
        fs::create_dir_all(&session_dir)
            .await
            .with_context(|| format!("Failed to create session directory: {:?}", session_dir))?;

        let path = session_dir.join(format!("{}.jsonl", id));

        Ok(Self {
            id: id.clone(),
            cwd_hash,
            path,
            records: Vec::new(),
            focus_chain: Vec::new(),
            started_at: now.clone(),
            last_activity: now,
            total_tokens: 0,
        })
    }

    /// Get the session directory path
    fn session_dir(cwd_hash: &str) -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".savant")
            .join("sessions")
            .join(cwd_hash)
    }

    /// Append a record to the session file (crash-safe: fsync after each write)
    pub async fn append_record(&mut self, record: &SessionRecord) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await
            .with_context(|| format!("Failed to open session file: {:?}", self.path))?;

        let line = serde_json::to_string(record).context("Failed to serialize session record")?;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.sync_all().await?;

        self.total_tokens += record.token_count;
        self.last_activity = Utc::now().to_rfc3339();
        self.records.push(record.clone());

        debug!(
            "Appended turn {} to session {} ({} tokens)",
            record.turn_index, self.id, record.token_count
        );
        Ok(())
    }

    /// Load an existing session from file
    pub async fn load(path: &Path) -> Result<Self> {
        let file = fs::File::open(path)
            .await
            .with_context(|| format!("Failed to open session file: {:?}", path))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut records = Vec::new();
        let mut total_tokens = 0usize;

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<SessionRecord>(&line) {
                Ok(record) => {
                    total_tokens += record.token_count;
                    records.push(record);
                }
                Err(e) => {
                    warn!("Skipping corrupted session record: {}", e);
                }
            }
        }

        let id = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let cwd_hash = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let started_at = records
            .first()
            .map(|r| r.timestamp.clone())
            .unwrap_or_else(|| Utc::now().to_rfc3339());

        let last_activity = records
            .last()
            .map(|r| r.timestamp.clone())
            .unwrap_or_else(|| Utc::now().to_rfc3339());

        let focus_chain = records
            .last()
            .map(|r| r.focus_chain.clone())
            .unwrap_or_default();

        Ok(Self {
            id,
            cwd_hash,
            path: path.to_path_buf(),
            records,
            focus_chain,
            started_at,
            last_activity,
            total_tokens,
        })
    }

    /// Find the most recent unclosed session for a given cwd_hash
    pub async fn find_recent(cwd_hash: &str) -> Result<Option<PathBuf>> {
        let session_dir = Self::session_dir(cwd_hash);
        if !session_dir.exists() {
            return Ok(None);
        }

        let mut entries = fs::read_dir(&session_dir).await?;
        let mut latest: Option<(String, PathBuf)> = None;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "jsonl") {
                if let Some(name) = path.file_stem() {
                    let name_str = name.to_string_lossy().to_string();
                    if latest.as_ref().is_none_or(|(n, _)| name_str > *n) {
                        latest = Some((name_str, path));
                    }
                }
            }
        }

        Ok(latest.map(|(_, path)| path))
    }

    /// Get session statistics
    pub fn stats(&self) -> SessionStats {
        let user_turns = self.records.iter().filter(|r| r.role == "user").count();
        let assistant_turns = self
            .records
            .iter()
            .filter(|r| r.role == "assistant")
            .count();
        let tool_calls: usize = self.records.iter().map(|r| r.tool_calls.len()).sum();
        let files_modified: usize = self.records.iter().map(|r| r.active_files.len()).sum();

        SessionStats {
            total_turns: self.records.len(),
            user_turns,
            assistant_turns,
            tool_calls,
            files_modified,
            total_tokens: self.total_tokens,
            duration: self.duration_secs(),
        }
    }

    fn duration_secs(&self) -> u64 {
        let start = chrono::DateTime::parse_from_rfc3339(&self.started_at)
            .map(|dt| dt.timestamp())
            .unwrap_or(0);
        let end = chrono::DateTime::parse_from_rfc3339(&self.last_activity)
            .map(|dt| dt.timestamp())
            .unwrap_or(start);
        (end - start).max(0) as u64
    }
}

#[derive(Debug, Clone)]
pub struct SessionStats {
    pub total_turns: usize,
    pub user_turns: usize,
    pub assistant_turns: usize,
    pub tool_calls: usize,
    pub files_modified: usize,
    pub total_tokens: usize,
    pub duration: u64,
}

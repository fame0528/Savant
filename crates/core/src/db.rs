use crate::error::SavantError;
use crate::types::{ChatMessage, ChatRole};
use rusqlite::{params, Connection};
use std::path::PathBuf;

/// Persistent Storage Layer with WAL (Write-Ahead Logging) enabled by default.
pub struct Storage {
    db_path: PathBuf,
}

impl Storage {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    /// Connects to SQLite and ensures WAL mode is active for peak concurrency.
    pub fn connect(&self) -> Result<Connection, SavantError> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        // WAL Mode: Multi-reader, single-writer concurrency
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| SavantError::Unknown(format!("WAL error: {}", e)))?;

        // Optimization PRAGMAs
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| SavantError::Unknown(format!("Sync error: {}", e)))?;

        Ok(conn)
    }

    /// Initializes the core schema for agent persistence.
    pub fn init_schema(&self) -> Result<(), SavantError> {
        let conn = self.connect()?;

        // Memory Table (Extended for Elite context)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS agents_memory (
                id INTEGER PRIMARY KEY,
                agent_id TEXT,
                category TEXT,
                content TEXT,
                importance INTEGER,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .map_err(|e| SavantError::Unknown(format!("Schema error: {}", e)))?;

        // Vector Knowledge Table (Feature 8)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS vector_knowledge (
                id INTEGER PRIMARY KEY,
                agent_id TEXT,
                content TEXT,
                embedding_json TEXT, -- Store as JSON for SQLite compatibility
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .map_err(|e| SavantError::Unknown(format!("Schema error: {}", e)))?;

        // Chat History Table (WAL ensured durability)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS chat_history (
                id INTEGER PRIMARY KEY,
                agent_id TEXT,
                sender TEXT,
                role TEXT,
                content TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .map_err(|e| SavantError::Unknown(format!("Schema error: {}", e)))?;

        Ok(())
    }

    /// Records a chat message to history.
    pub fn append_chat(&self, agent_id: &str, msg: &ChatMessage) -> Result<(), SavantError> {
        let conn = self.connect()?;
        let role = match msg.role {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
        };

        conn.execute(
            "INSERT INTO chat_history (agent_id, sender, role, content) VALUES (?, ?, ?, ?)",
            params![agent_id, msg.sender, role, msg.content],
        )
        .map_err(|e| SavantError::Unknown(format!("Insert error: {}", e)))?;

        Ok(())
    }

    /// Retrieves full chat history for an agent to prevent context loss.
    pub fn get_history(
        &self,
        agent_id: &str,
        limit: usize,
    ) -> Result<Vec<ChatMessage>, SavantError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT role, content, sender, agent_id FROM chat_history WHERE agent_id = ? ORDER BY timestamp DESC, id DESC LIMIT ?"
        ).map_err(|e| SavantError::Unknown(format!("Prepare error: {}", e)))?;

        let rows = stmt
            .query_map(params![agent_id, limit as i64], |row| {
                let role_str: String = row.get(0)?;
                let role = match role_str.as_str() {
                    "system" => ChatRole::System,
                    "user" => ChatRole::User,
                    _ => ChatRole::Assistant,
                };
                Ok(ChatMessage {
                    role,
                    content: row.get(1)?,
                    sender: row.get(2)?,
                    recipient: None,
                    agent_id: row.get(3)?,
                })
            })
            .map_err(|e| SavantError::Unknown(format!("Query error: {}", e)))?;

        let mut history: Vec<ChatMessage> = rows.filter_map(|r| r.ok()).collect();
        history.reverse(); // Back to chronological order
        Ok(history)
    }
}

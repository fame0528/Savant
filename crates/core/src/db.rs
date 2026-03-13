use crate::error::SavantError;
use crate::types::{ChatMessage, ChatRole};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

/// SQLite-backed storage for chat history and agent state.
pub struct Storage {
    conn: Mutex<Connection>,
}

impl Storage {
    pub fn new(path: PathBuf) -> Self {
        let conn = Connection::open(path).expect("Failed to open database");
        Self {
            conn: Mutex::new(conn),
        }
    }

    /// Initializes the database schema.
    pub fn init_schema(&self) -> Result<(), SavantError> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS chat_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                sender TEXT,
                recipient TEXT,
                agent_id TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .map_err(|e| SavantError::Unknown(format!("Failed to init schema: {}", e)))?;

        Ok(())
    }

    /// Appends a chat message to the history.
    pub fn append_chat(&self, session_id: &str, msg: &ChatMessage) -> Result<(), SavantError> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            "INSERT INTO chat_history (session_id, role, content, sender, recipient, agent_id) 
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                session_id,
                msg.role.to_string(),
                msg.content,
                msg.sender,
                msg.recipient,
                msg.agent_id,
            ],
        )
        .map_err(|e| SavantError::Unknown(format!("Failed to append chat: {}", e)))?;

        Ok(())
    }

    /// Retrieves the most recent chat history for a session.
    pub fn get_history(&self, session_id: &str, limit: usize) -> Result<Vec<ChatMessage>, SavantError> {
        let conn = self.conn.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT role, content, sender, recipient, agent_id 
             FROM chat_history 
             WHERE session_id = ? 
             ORDER BY id DESC 
             LIMIT ?"
        ).map_err(|e| SavantError::Unknown(format!("Failed to prepare history query: {}", e)))?;

        let rows = stmt.query_map(params![session_id, limit as i64], |row| {
            let role_str: String = row.get(0)?;
            Ok(ChatMessage {
                role: role_str.parse().unwrap_or(ChatRole::User),
                content: row.get(1)?,
                sender: row.get(2)?,
                recipient: row.get(3)?,
                agent_id: row.get(4)?,
            })
        }).map_err(|e| SavantError::Unknown(format!("Failed to query history: {}", e)))?;

        let mut history: Vec<ChatMessage> = rows.filter_map(|r| r.ok()).collect();
        history.reverse(); // Newest last for the LLM
        Ok(history)
    }
}

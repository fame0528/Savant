use crate::error::SavantError;
use crate::types::MemoryEntry;
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub mod registry;

/// Filesystem Watcher and Indexer
pub struct FileIndexer {
    db_path: PathBuf,
}

impl FileIndexer {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    fn open_connection(&self) -> Result<Connection, SavantError> {
        Connection::open(&self.db_path)
            .map_err(|e| SavantError::IoError(std::io::Error::other(e)))
    }

    /// Initializes the database tables and enables WAL mode.
    pub fn init_db(&self) -> Result<(), SavantError> {
        let conn = self.open_connection()?;

        // Enable WAL mode for high-concurrency and zero-loss durability
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| SavantError::Unknown(format!("Failed to enable WAL: {}", e)))?;

        // Table for semantic memory chunks
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_chunks (
                id INTEGER PRIMARY KEY,
                content TEXT,
                embedding BLOB,
                file_path TEXT,
                agent_id TEXT,
                timestamp INTEGER
            )",
            [],
        )
        .map_err(|e| SavantError::Unknown(format!("DB error: {}", e)))?;

        // Table for tracking indexed files to avoid redundant processing
        conn.execute(
            "CREATE TABLE IF NOT EXISTS indexed_files (
                path TEXT PRIMARY KEY,
                hash TEXT,
                last_indexed INTEGER
            )",
            [],
        )
        .map_err(|e| SavantError::Unknown(format!("DB error: {}", e)))?;

        Ok(())
    }

    /// Indexes a directory recursively.
    pub async fn index_directory(
        &self,
        agent_id: &str,
        base_path: &Path,
    ) -> Result<(), SavantError> {
        tracing::info!(
            "Indexing directory for agent {}: {}",
            agent_id,
            base_path.display()
        );

        for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path = entry.path();
                if self.should_index(path) {
                    self.index_file(agent_id, path).await?;
                }
            }
        }

        Ok(())
    }

    fn should_index(&self, path: &Path) -> bool {
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        matches!(extension, "md" | "txt" | "json" | "toml")
    }

    async fn index_file(&self, agent_id: &str, path: &Path) -> Result<(), SavantError> {
        let content = fs::read_to_string(path)?;
        let hash = blake3::hash(content.as_bytes()).to_hex().to_string();

        let conn = self.open_connection()?;

        // Check if file has changed
        let mut stmt = conn
            .prepare("SELECT hash FROM indexed_files WHERE path = ?")
            .map_err(|e| SavantError::Unknown(format!("DB prepare error: {}", e)))?;
        
        let path_str = path.to_str().ok_or_else(|| SavantError::Unknown("Invalid path encoding".to_string()))?;
        
        let existing_hash: Option<String> = stmt
            .query_row(params![path_str], |row| row.get(0))
            .optional()
            .map_err(|e| SavantError::Unknown(format!("DB query error: {}", e)))?;

        if Some(hash.clone()) == existing_hash {
            return Ok(());
        }

        tracing::info!("Indexing file: {}", path.display());

        // Update indexed_files
        conn.execute(
            "INSERT OR REPLACE INTO indexed_files (path, hash, last_indexed) VALUES (?, ?, ?)",
            params![path_str, hash, 0], // timestamp 0 for now
        )
        .map_err(|e| SavantError::Unknown(format!("DB error: {}", e)))?;

        // In a real implementation, we would chunk the content and generate embeddings here.
        // For now, we store the whole file as one chunk.
        conn.execute(
            "INSERT INTO memory_chunks (content, file_path, agent_id, timestamp) VALUES (?, ?, ?, ?)",
            params![content, path_str, agent_id, 0],
        ).map_err(|e| SavantError::Unknown(format!("DB error: {}", e)))?;

        Ok(())
    }

    pub async fn watch_and_index(
        &self,
        _agent_id: &str,
        base_path: &Path,
    ) -> Result<(), SavantError> {
        tracing::info!("Starting filesystem watcher for {}", base_path.display());
        // In a full implementation, we'd use notify here to trigger index_file on modification.
        // For now, we perform a one-time scan.
        self.index_directory(_agent_id, base_path).await
    }

    pub fn semantic_search(&self, _query: &str, _limit: usize) -> Vec<MemoryEntry> {
        // This will use the EmbeddingService in the future
        Vec::new()
    }

    pub fn full_text_search(&self, query: &str) -> Vec<MemoryEntry> {
        let conn = match self.open_connection() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut stmt = match conn.prepare("SELECT id, content, timestamp FROM memory_chunks WHERE content LIKE ?") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = match stmt.query_map(params![format!("%{}%", query)], |row| {
                Ok(MemoryEntry {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    timestamp: row.get(2)?,
                    category: crate::types::MemoryCategory::Observation,
                    importance: 5,
                    associations: Vec::new(),
                    embedding: None,
                })
            }) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        rows.filter_map(|r| r.ok()).collect()
    }
}

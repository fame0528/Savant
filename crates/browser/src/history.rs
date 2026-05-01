use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use tracing::debug;

use crate::types::{BrowserError, HistoryEntry};

const CREATE_TABLE_SQL: &str = "
    CREATE TABLE IF NOT EXISTS browser_history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        url TEXT NOT NULL,
        title TEXT NOT NULL DEFAULT '',
        visited_at TEXT NOT NULL
    )
";

const CREATE_INDEX_URL_SQL: &str = "
    CREATE INDEX IF NOT EXISTS idx_history_url ON browser_history(url);
";

const CREATE_INDEX_VISITED_AT_SQL: &str = "
    CREATE INDEX IF NOT EXISTS idx_history_visited_at ON browser_history(visited_at);
";

/// SQLite-backed browsing history manager with WAL mode for concurrent reads.
pub struct HistoryManager {
    conn: Connection,
}

impl HistoryManager {
    /// Opens or creates the history database at the given path.
    pub fn open(db_path: &Path) -> Result<Self, BrowserError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                BrowserError::DatabaseError(format!("Failed to create history directory: {}", e))
            })?;
        }

        let conn = Connection::open(db_path).map_err(|e| {
            BrowserError::DatabaseError(format!("Failed to open history database: {}", e))
        })?;

        // Enable WAL mode for concurrent reads
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| BrowserError::DatabaseError(format!("Failed to set WAL mode: {}", e)))?;

        conn.execute(CREATE_TABLE_SQL, []).map_err(|e| {
            BrowserError::DatabaseError(format!("Failed to create history table: {}", e))
        })?;

        conn.execute_batch(&format!(
            "{CREATE_INDEX_URL_SQL}{CREATE_INDEX_VISITED_AT_SQL}"
        ))
        .map_err(|e| {
            BrowserError::DatabaseError(format!("Failed to create history indexes: {}", e))
        })?;

        debug!(
            "[browser::history] History manager initialized at {:?}",
            db_path
        );

        Ok(Self { conn })
    }

    /// Opens an in-memory history database (for testing).
    #[cfg(test)]
    pub fn in_memory() -> Result<Self, BrowserError> {
        let conn = Connection::open_in_memory().map_err(|e| {
            BrowserError::DatabaseError(format!("Failed to open in-memory history database: {}", e))
        })?;

        conn.execute(CREATE_TABLE_SQL, []).map_err(|e| {
            BrowserError::DatabaseError(format!("Failed to create history table: {}", e))
        })?;

        conn.execute_batch(&format!(
            "{CREATE_INDEX_URL_SQL}{CREATE_INDEX_VISITED_AT_SQL}"
        ))
        .map_err(|e| {
            BrowserError::DatabaseError(format!("Failed to create history indexes: {}", e))
        })?;

        Ok(Self { conn })
    }

    /// Inserts a new history entry.
    pub fn insert(&self, url: &str, title: &str) -> Result<i64, BrowserError> {
        let visited_at = Utc::now().to_rfc3339();

        self.conn
            .execute(
                "INSERT INTO browser_history (url, title, visited_at) VALUES (?1, ?2, ?3)",
                params![url, title, visited_at],
            )
            .map_err(|e| {
                BrowserError::DatabaseError(format!("Failed to insert history entry: {}", e))
            })?;

        let id = self.conn.last_insert_rowid();
        debug!(
            "[browser::history] Inserted history entry {}: {} ({})",
            id, title, url
        );

        Ok(id)
    }

    /// Queries history entries, newest first, with limit and offset.
    pub fn query(&self, limit: usize, offset: usize) -> Result<Vec<HistoryEntry>, BrowserError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, url, title, visited_at FROM browser_history ORDER BY visited_at DESC LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| BrowserError::DatabaseError(format!("Failed to prepare history query: {}", e)))?;

        let entries = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                let visited_at: String = row.get(3)?;
                let parsed_at = DateTime::parse_from_rfc3339(&visited_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(HistoryEntry {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    visited_at: parsed_at,
                })
            })
            .map_err(|e| {
                BrowserError::DatabaseError(format!("Failed to execute history query: {}", e))
            })?;

        entries.collect::<Result<Vec<_>, _>>().map_err(|e| {
            BrowserError::DatabaseError(format!("Failed to read history entries: {}", e))
        })
    }

    /// Searches history entries by URL or title text.
    pub fn search(&self, query_text: &str) -> Result<Vec<HistoryEntry>, BrowserError> {
        let pattern = format!("%{}%", query_text);

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, url, title, visited_at FROM browser_history WHERE url LIKE ?1 OR title LIKE ?1 ORDER BY visited_at DESC",
            )
            .map_err(|e| BrowserError::DatabaseError(format!("Failed to prepare search query: {}", e)))?;

        let entries = stmt
            .query_map(params![pattern], |row| {
                let visited_at: String = row.get(3)?;
                let parsed_at = DateTime::parse_from_rfc3339(&visited_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(HistoryEntry {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    visited_at: parsed_at,
                })
            })
            .map_err(|e| {
                BrowserError::DatabaseError(format!("Failed to execute search query: {}", e))
            })?;

        entries.collect::<Result<Vec<_>, _>>().map_err(|e| {
            BrowserError::DatabaseError(format!("Failed to read search results: {}", e))
        })
    }

    /// Removes history entries older than the cutoff date.
    pub fn cleanup_before(&self, cutoff: DateTime<Utc>) -> Result<usize, BrowserError> {
        let cutoff_str = cutoff.to_rfc3339();

        let deleted = self
            .conn
            .execute(
                "DELETE FROM browser_history WHERE visited_at < ?1",
                params![cutoff_str],
            )
            .map_err(|e| {
                BrowserError::DatabaseError(format!("Failed to cleanup old history: {}", e))
            })?;

        if deleted > 0 {
            debug!(
                "[browser::history] Cleaned up {} entries before {}",
                deleted, cutoff
            );
        }

        Ok(deleted)
    }

    /// Returns the total count of history entries.
    pub fn count(&self) -> Result<usize, BrowserError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM browser_history", [], |row| row.get(0))
            .map_err(|e| {
                BrowserError::DatabaseError(format!("Failed to count history entries: {}", e))
            })?;

        Ok(count as usize)
    }

    /// Returns the most recent entry for a given URL, if any.
    pub fn last_visited(&self, url: &str) -> Result<Option<HistoryEntry>, BrowserError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, url, title, visited_at FROM browser_history WHERE url = ?1 ORDER BY visited_at DESC LIMIT 1",
            )
            .map_err(|e| BrowserError::DatabaseError(format!("Failed to prepare last-visited query: {}", e)))?;

        let entry = stmt
            .query_row(params![url], |row| {
                let visited_at: String = row.get(3)?;
                let parsed_at = DateTime::parse_from_rfc3339(&visited_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(HistoryEntry {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    visited_at: parsed_at,
                })
            })
            .optional()
            .map_err(|e| {
                BrowserError::DatabaseError(format!("Failed to execute last-visited query: {}", e))
            })?;

        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_insert_and_query() {
        let mgr = HistoryManager::in_memory().expect("Failed to create in-memory history");

        let id = mgr
            .insert("https://example.com", "Example")
            .expect("Failed to insert");
        assert!(id > 0);

        let entries = mgr.query(10, 0).expect("Failed to query");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].url, "https://example.com");
        assert_eq!(entries[0].title, "Example");
    }

    #[test]
    fn test_search_by_url() {
        let mgr = HistoryManager::in_memory().expect("Failed to create in-memory history");

        mgr.insert("https://example.com/page1", "Page 1")
            .expect("Failed to insert");
        mgr.insert("https://example.org/page2", "Page 2")
            .expect("Failed to insert");

        let results = mgr.search("example.com").expect("Failed to search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://example.com/page1");
    }

    #[test]
    fn test_search_by_title() {
        let mgr = HistoryManager::in_memory().expect("Failed to create in-memory history");

        mgr.insert("https://example.com/a", "Rust Programming")
            .expect("Failed to insert");
        mgr.insert("https://example.org/b", "Python Basics")
            .expect("Failed to insert");

        let results = mgr.search("Rust").expect("Failed to search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Programming");
    }

    #[test]
    fn test_cleanup_old_entries() {
        let mgr = HistoryManager::in_memory().expect("Failed to create in-memory history");

        mgr.insert("https://example.com/old", "Old Page")
            .expect("Failed to insert");
        mgr.insert("https://example.com/new", "New Page")
            .expect("Failed to insert");

        assert_eq!(mgr.count().expect("Failed to count"), 2);

        // Clean up everything older than now (should keep nothing since entries are recent)
        let cutoff = Utc::now() + Duration::days(1);
        let deleted = mgr.cleanup_before(cutoff).expect("Failed to cleanup");
        assert_eq!(deleted, 2);
        assert_eq!(mgr.count().expect("Failed to count"), 0);
    }

    #[test]
    fn test_last_visited() {
        let mgr = HistoryManager::in_memory().expect("Failed to create in-memory history");

        mgr.insert("https://example.com", "First Visit")
            .expect("Failed to insert");
        mgr.insert("https://example.com", "Second Visit")
            .expect("Failed to insert");

        let last = mgr
            .last_visited("https://example.com")
            .expect("Failed to query");
        assert!(last.is_some());
        let last = last.expect("Expected Some");
        assert_eq!(last.title, "Second Visit");
    }

    #[test]
    fn test_empty_query() {
        let mgr = HistoryManager::in_memory().expect("Failed to create in-memory history");

        let entries = mgr.query(10, 0).expect("Failed to query");
        assert!(entries.is_empty());

        let count = mgr.count().expect("Failed to count");
        assert_eq!(count, 0);
    }
}

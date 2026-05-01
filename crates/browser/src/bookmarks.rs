use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use tracing::debug;

use crate::types::{Bookmark, BrowserError};

const CREATE_TABLE_SQL: &str = "
    CREATE TABLE IF NOT EXISTS bookmarks (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        url TEXT NOT NULL UNIQUE,
        title TEXT NOT NULL DEFAULT '',
        tags TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL
    )
";

const CREATE_INDEX_URL_SQL: &str = "
    CREATE INDEX IF NOT EXISTS idx_bookmarks_url ON bookmarks(url);
";

const CREATE_INDEX_TITLE_SQL: &str = "
    CREATE INDEX IF NOT EXISTS idx_bookmarks_title ON bookmarks(title);
";

/// SQLite-backed bookmark manager with tag support.
pub struct BookmarkManager {
    conn: Connection,
}

impl BookmarkManager {
    /// Opens or creates the bookmarks database at the given path.
    pub fn open(db_path: &Path) -> Result<Self, BrowserError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                BrowserError::DatabaseError(format!("Failed to create bookmarks directory: {}", e))
            })?;
        }

        let conn = Connection::open(db_path).map_err(|e| {
            BrowserError::DatabaseError(format!("Failed to open bookmarks database: {}", e))
        })?;

        // Enable WAL mode for concurrent reads
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| BrowserError::DatabaseError(format!("Failed to set WAL mode: {}", e)))?;

        conn.execute(CREATE_TABLE_SQL, []).map_err(|e| {
            BrowserError::DatabaseError(format!("Failed to create bookmarks table: {}", e))
        })?;

        conn.execute_batch(&format!("{CREATE_INDEX_URL_SQL}{CREATE_INDEX_TITLE_SQL}"))
            .map_err(|e| {
                BrowserError::DatabaseError(format!("Failed to create bookmark indexes: {}", e))
            })?;

        debug!(
            "[browser::bookmarks] Bookmark manager initialized at {:?}",
            db_path
        );

        Ok(Self { conn })
    }

    /// Opens an in-memory bookmarks database (for testing).
    #[cfg(test)]
    pub fn in_memory() -> Result<Self, BrowserError> {
        let conn = Connection::open_in_memory().map_err(|e| {
            BrowserError::DatabaseError(format!(
                "Failed to open in-memory bookmarks database: {}",
                e
            ))
        })?;

        conn.execute(CREATE_TABLE_SQL, []).map_err(|e| {
            BrowserError::DatabaseError(format!("Failed to create bookmarks table: {}", e))
        })?;

        conn.execute_batch(&format!("{CREATE_INDEX_URL_SQL}{CREATE_INDEX_TITLE_SQL}"))
            .map_err(|e| {
                BrowserError::DatabaseError(format!("Failed to create bookmark indexes: {}", e))
            })?;

        Ok(Self { conn })
    }

    /// Adds a new bookmark. If the URL already exists, updates its title and tags.
    pub fn add(&self, url: &str, title: &str, tags: &[&str]) -> Result<i64, BrowserError> {
        let tags_str = tags.join(",");
        let created_at = Utc::now().to_rfc3339();

        let id = self
            .conn
            .query_row(
                "INSERT INTO bookmarks (url, title, tags, created_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(url) DO UPDATE SET title = ?2, tags = ?3
                 RETURNING id",
                params![url, title, tags_str, created_at],
                |row| row.get(0),
            )
            .map_err(|e| BrowserError::DatabaseError(format!("Failed to add bookmark: {}", e)))?;

        debug!(
            "[browser::bookmarks] Added/updated bookmark {}: {} ({})",
            id, title, url
        );

        Ok(id)
    }

    /// Removes a bookmark by ID.
    pub fn remove(&self, id: i64) -> Result<(), BrowserError> {
        let deleted = self
            .conn
            .execute("DELETE FROM bookmarks WHERE id = ?1", params![id])
            .map_err(|e| {
                BrowserError::DatabaseError(format!("Failed to remove bookmark: {}", e))
            })?;

        if deleted == 0 {
            return Err(BrowserError::DatabaseError(format!(
                "Bookmark {} not found",
                id
            )));
        }

        debug!("[browser::bookmarks] Removed bookmark {}", id);

        Ok(())
    }

    /// Lists all bookmarks, newest first.
    pub fn list(&self) -> Result<Vec<Bookmark>, BrowserError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, url, title, tags, created_at FROM bookmarks ORDER BY created_at DESC",
            )
            .map_err(|e| {
                BrowserError::DatabaseError(format!("Failed to prepare bookmarks query: {}", e))
            })?;

        let bookmarks = stmt
            .query_map([], |row| {
                let created_at: String = row.get(4)?;
                let parsed_at = DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(Bookmark {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    tags: row.get(3)?,
                    created_at: parsed_at,
                })
            })
            .map_err(|e| {
                BrowserError::DatabaseError(format!("Failed to execute bookmarks query: {}", e))
            })?;

        bookmarks
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| BrowserError::DatabaseError(format!("Failed to read bookmarks: {}", e)))
    }

    /// Searches bookmarks by URL, title, or tags.
    pub fn search(&self, query: &str) -> Result<Vec<Bookmark>, BrowserError> {
        let pattern = format!("%{}%", query);

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, url, title, tags, created_at FROM bookmarks WHERE url LIKE ?1 OR title LIKE ?1 OR tags LIKE ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| BrowserError::DatabaseError(format!("Failed to prepare bookmark search query: {}", e)))?;

        let bookmarks = stmt
            .query_map(params![pattern], |row| {
                let created_at: String = row.get(4)?;
                let parsed_at = DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(Bookmark {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    tags: row.get(3)?,
                    created_at: parsed_at,
                })
            })
            .map_err(|e| {
                BrowserError::DatabaseError(format!(
                    "Failed to execute bookmark search query: {}",
                    e
                ))
            })?;

        bookmarks.collect::<Result<Vec<_>, _>>().map_err(|e| {
            BrowserError::DatabaseError(format!("Failed to read bookmark search results: {}", e))
        })
    }

    /// Gets a bookmark by exact URL match.
    pub fn get_by_url(&self, url: &str) -> Result<Option<Bookmark>, BrowserError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, url, title, tags, created_at FROM bookmarks WHERE url = ?1")
            .map_err(|e| {
                BrowserError::DatabaseError(format!(
                    "Failed to prepare bookmark-by-URL query: {}",
                    e
                ))
            })?;

        let bookmark = stmt
            .query_row(params![url], |row| {
                let created_at: String = row.get(4)?;
                let parsed_at = DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(Bookmark {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    tags: row.get(3)?,
                    created_at: parsed_at,
                })
            })
            .optional()
            .map_err(|e| {
                BrowserError::DatabaseError(format!(
                    "Failed to execute bookmark-by-URL query: {}",
                    e
                ))
            })?;

        Ok(bookmark)
    }

    /// Returns the total count of bookmarks.
    pub fn count(&self) -> Result<usize, BrowserError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM bookmarks", [], |row| row.get(0))
            .map_err(|e| {
                BrowserError::DatabaseError(format!("Failed to count bookmarks: {}", e))
            })?;

        Ok(count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_list() {
        let mgr = BookmarkManager::in_memory().expect("Failed to create in-memory bookmarks");

        let id = mgr
            .add("https://example.com", "Example", &["demo", "test"])
            .expect("Failed to add bookmark");
        assert!(id > 0);

        let bookmarks = mgr.list().expect("Failed to list");
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].url, "https://example.com");
        assert_eq!(bookmarks[0].title, "Example");
        assert_eq!(bookmarks[0].tags, "demo,test");
    }

    #[test]
    fn test_add_duplicate_url_updates() {
        let mgr = BookmarkManager::in_memory().expect("Failed to create in-memory bookmarks");

        mgr.add("https://example.com", "First", &["tag1"])
            .expect("Failed to add");
        mgr.add("https://example.com", "Updated", &["tag1", "tag2"])
            .expect("Failed to update");

        let bookmarks = mgr.list().expect("Failed to list");
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].title, "Updated");
        assert_eq!(bookmarks[0].tags, "tag1,tag2");
    }

    #[test]
    fn test_remove() {
        let mgr = BookmarkManager::in_memory().expect("Failed to create in-memory bookmarks");

        let id = mgr
            .add("https://example.com", "Example", &[])
            .expect("Failed to add");

        mgr.remove(id).expect("Failed to remove");

        let bookmarks = mgr.list().expect("Failed to list");
        assert!(bookmarks.is_empty());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mgr = BookmarkManager::in_memory().expect("Failed to create in-memory bookmarks");

        let result = mgr.remove(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_by_title() {
        let mgr = BookmarkManager::in_memory().expect("Failed to create in-memory bookmarks");

        mgr.add("https://example.com/a", "Rust Docs", &["rust"])
            .expect("Failed to add");
        mgr.add("https://example.org/b", "Python Docs", &["python"])
            .expect("Failed to add");

        let results = mgr.search("Rust").expect("Failed to search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Docs");
    }

    #[test]
    fn test_search_by_tag() {
        let mgr = BookmarkManager::in_memory().expect("Failed to create in-memory bookmarks");

        mgr.add("https://example.com/a", "Page A", &["rust", "programming"])
            .expect("Failed to add");
        mgr.add(
            "https://example.org/b",
            "Page B",
            &["python", "programming"],
        )
        .expect("Failed to add");

        let results = mgr.search("programming").expect("Failed to search");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_get_by_url() {
        let mgr = BookmarkManager::in_memory().expect("Failed to create in-memory bookmarks");

        mgr.add("https://example.com", "Example", &["demo"])
            .expect("Failed to add");

        let bm = mgr
            .get_by_url("https://example.com")
            .expect("Failed to query")
            .expect("Expected Some");
        assert_eq!(bm.title, "Example");

        let missing = mgr
            .get_by_url("https://nonexistent.com")
            .expect("Failed to query");
        assert!(missing.is_none());
    }

    #[test]
    fn test_count() {
        let mgr = BookmarkManager::in_memory().expect("Failed to create in-memory bookmarks");

        assert_eq!(mgr.count().expect("Failed to count"), 0);

        mgr.add("https://a.com", "A", &[]).expect("Failed to add");
        mgr.add("https://b.com", "B", &[]).expect("Failed to add");

        assert_eq!(mgr.count().expect("Failed to count"), 2);
    }
}

use dashmap::DashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::bookmarks::BookmarkManager;
use crate::history::HistoryManager;
use crate::types::{BrowserConfig, BrowserError, BrowserEvent, TabId, TabInfo};

/// Central browser engine state machine.
///
/// Manages tabs, history, bookmarks, and event broadcasting.
/// Thread-safe: tabs use DashMap, SQLite managers use std::sync::Mutex
/// (rusqlite::Connection is not Sync, so we wrap in Mutex).
pub struct BrowserEngine {
    /// Active tabs, keyed by TabId.
    tabs: DashMap<TabId, TabInfo>,

    /// Currently active tab ID.
    active_tab: std::sync::RwLock<Option<TabId>>,

    /// Browsing history manager (wrapped in Mutex for Sync).
    history: std::sync::Mutex<HistoryManager>,

    /// Bookmark manager (wrapped in Mutex for Sync).
    bookmarks: std::sync::Mutex<BookmarkManager>,

    /// Browser configuration.
    config: BrowserConfig,

    /// Event broadcast sender. Subscribers receive browser events.
    event_tx: broadcast::Sender<BrowserEvent>,

    /// Whether the engine is initialized and ready.
    initialized: std::sync::atomic::AtomicBool,
}

impl BrowserEngine {
    /// Creates and initializes a new BrowserEngine.
    ///
    /// # Arguments
    /// * `data_path` - Base directory for browser data (history, bookmarks SQLite files).
    /// * `config` - Browser configuration loaded from savant.toml.
    pub fn new(data_path: &Path, config: BrowserConfig) -> Result<Arc<Self>, BrowserError> {
        let browser_data = data_path.join("browser");
        std::fs::create_dir_all(&browser_data).map_err(|e| {
            BrowserError::DatabaseError(format!("Failed to create browser data directory: {}", e))
        })?;

        let history_path = browser_data.join("history.db");
        let history = HistoryManager::open(&history_path)?;

        let bookmarks_path = browser_data.join("bookmarks.db");
        let bookmarks = BookmarkManager::open(&bookmarks_path)?;

        // Event broadcast channel with capacity for 64 subscribers.
        let (event_tx, _event_rx) = broadcast::channel::<BrowserEvent>(64);

        let engine = Arc::new(Self {
            tabs: DashMap::new(),
            active_tab: std::sync::RwLock::new(None),
            history: std::sync::Mutex::new(history),
            bookmarks: std::sync::Mutex::new(bookmarks),
            config,
            event_tx,
            initialized: std::sync::atomic::AtomicBool::new(true),
        });

        info!(
            "[browser::engine] BrowserEngine initialized. History: {} entries, Bookmarks: {} entries",
            engine.history.lock().map(|h| h.count().unwrap_or(0)).unwrap_or(0),
            engine.bookmarks.lock().map(|b| b.count().unwrap_or(0)).unwrap_or(0),
        );

        Ok(engine)
    }

    /// Returns whether the engine is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Checks if the engine is enabled in configuration.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Checks if agent browser control is enabled.
    pub fn is_agent_control_enabled(&self) -> bool {
        self.config.agent_control_enabled
    }

    /// Checks if JavaScript execution is allowed.
    pub fn is_js_execution_allowed(&self) -> bool {
        self.config.js_execution_allowed
    }

    /// Gets the browser configuration.
    pub fn config(&self) -> &BrowserConfig {
        &self.config
    }

    // --- Tab Management ---

    /// Creates a new tab with the given URL and optional agent name.
    ///
    /// Returns the new TabInfo on success.
    /// Returns `TabLimitReached` if the maximum tab count is exceeded.
    pub fn create_tab(
        &self,
        url: String,
        agent_name: Option<String>,
    ) -> Result<TabInfo, BrowserError> {
        if !self.is_enabled() {
            return Err(BrowserError::BrowserDisabled);
        }

        let tab_count = self.tabs.len();
        if tab_count >= self.config.max_tabs {
            return Err(BrowserError::TabLimitReached(self.config.max_tabs));
        }

        let tab = TabInfo::new(url.clone(), agent_name.clone());
        let tab_id = tab.id.clone();

        debug!(
            "[browser::engine] Creating tab {} ({}), total: {}",
            tab_id.0,
            url,
            tab_count + 1
        );

        self.tabs.insert(tab_id.clone(), tab.clone());

        // Set as active if this is the first tab.
        if self
            .active_tab
            .read()
            .map_err(|_| {
                BrowserError::Internal("Failed to acquire active_tab read lock".to_string())
            })?
            .is_none()
        {
            let mut active = self.active_tab.write().map_err(|_| {
                BrowserError::Internal("Failed to acquire active_tab write lock".to_string())
            })?;
            *active = Some(tab_id.clone());
        }

        // Publish event.
        self.publish_event(BrowserEvent::TabOpened {
            tab_id: tab_id.0,
            url,
            agent_name,
        })?;

        Ok(tab)
    }

    /// Closes a tab by ID.
    ///
    /// Returns Ok(()) on success.
    /// Returns `TabNotFound` if the tab doesn't exist.
    pub fn close_tab(&self, tab_id: &TabId) -> Result<(), BrowserError> {
        if !self.is_enabled() {
            return Err(BrowserError::BrowserDisabled);
        }

        let removed = self.tabs.remove(tab_id);
        if removed.is_none() {
            return Err(BrowserError::TabNotFound(tab_id.0.clone()));
        }

        debug!("[browser::engine] Closed tab {}", tab_id.0);

        // If the closed tab was active, switch to another tab.
        let mut active = self.active_tab.write().map_err(|_| {
            BrowserError::Internal("Failed to acquire active_tab write lock".to_string())
        })?;
        if active.as_ref() == Some(tab_id) {
            // Switch to the first remaining tab, or None.
            *active = self.tabs.iter().next().map(|entry| entry.key().clone());
        }
        drop(active);

        self.publish_event(BrowserEvent::TabClosed {
            tab_id: tab_id.0.clone(),
        })?;

        Ok(())
    }

    /// Switches the active tab.
    ///
    /// Returns Ok(()) on success.
    /// Returns `TabNotFound` if the tab doesn't exist.
    pub fn switch_tab(&self, tab_id: &TabId) -> Result<(), BrowserError> {
        if !self.is_enabled() {
            return Err(BrowserError::BrowserDisabled);
        }

        if !self.tabs.contains_key(tab_id) {
            return Err(BrowserError::TabNotFound(tab_id.0.clone()));
        }

        let mut active = self.active_tab.write().map_err(|_| {
            BrowserError::Internal("Failed to acquire active_tab write lock".to_string())
        })?;
        *active = Some(tab_id.clone());
        drop(active);

        debug!("[browser::engine] Switched active tab to {}", tab_id.0);

        Ok(())
    }

    /// Lists all open tabs.
    pub fn list_tabs(&self) -> Vec<TabInfo> {
        self.tabs
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Gets the currently active tab, if any.
    pub fn active_tab(&self) -> Option<TabInfo> {
        match self.active_tab.read() {
            Ok(active) => active
                .as_ref()
                .and_then(|id| self.tabs.get(id).map(|entry| entry.value().clone())),
            Err(_) => {
                error!("[browser::engine] Failed to acquire active_tab read lock");
                None
            }
        }
    }

    /// Gets tab info by ID.
    pub fn get_tab(&self, tab_id: &TabId) -> Option<TabInfo> {
        self.tabs.get(tab_id).map(|entry| entry.value().clone())
    }

    /// Updates a tab's URL (navigation).
    ///
    /// Returns Ok(()) on success.
    /// Returns `TabNotFound` if the tab doesn't exist.
    pub fn navigate_tab(&self, tab_id: &TabId, url: &str) -> Result<(), BrowserError> {
        if !self.is_enabled() {
            return Err(BrowserError::BrowserDisabled);
        }

        let mut tab = self
            .tabs
            .get_mut(tab_id)
            .ok_or_else(|| BrowserError::TabNotFound(tab_id.0.clone()))?;

        let old_url = tab.url.clone();
        tab.url = url.to_string();
        tab.loading = true;
        tab.title = String::new();
        drop(tab);

        debug!(
            "[browser::engine] Tab {} navigating from {} to {}",
            tab_id.0, old_url, url
        );

        self.publish_event(BrowserEvent::TabNavigated {
            tab_id: tab_id.0.clone(),
            url: url.to_string(),
            status: "navigating".to_string(),
        })?;

        Ok(())
    }

    /// Marks a tab as loaded (called after page load completes).
    ///
    /// Returns Ok(()) on success.
    /// Returns `TabNotFound` if the tab doesn't exist.
    pub fn tab_loaded(&self, tab_id: &TabId, title: &str) -> Result<(), BrowserError> {
        let mut tab = self
            .tabs
            .get_mut(tab_id)
            .ok_or_else(|| BrowserError::TabNotFound(tab_id.0.clone()))?;

        tab.loading = false;
        tab.title = title.to_string();

        // Record in history.
        let url = tab.url.clone();
        let title_str = tab.title.clone();
        drop(tab);

        if let Err(e) = self
            .history
            .lock()
            .map_err(|_| BrowserError::Internal("history lock poisoned".to_string()))?
            .insert(&url, &title_str)
        {
            warn!("[browser::engine] Failed to record history entry: {}", e);
        }

        debug!(
            "[browser::engine] Tab {} loaded: {} ({})",
            tab_id.0, title, url
        );

        self.publish_event(BrowserEvent::PageLoaded {
            tab_id: tab_id.0.clone(),
            url,
            title: title_str,
        })?;

        Ok(())
    }

    /// Marks a tab as loading.
    pub fn set_loading(&self, tab_id: &TabId) -> Result<(), BrowserError> {
        let mut tab = self
            .tabs
            .get_mut(tab_id)
            .ok_or_else(|| BrowserError::TabNotFound(tab_id.0.clone()))?;
        tab.loading = true;
        Ok(())
    }

    /// Marks a tab as not loading (load complete or stopped).
    pub fn set_not_loading(&self, tab_id: &TabId) -> Result<(), BrowserError> {
        let mut tab = self
            .tabs
            .get_mut(tab_id)
            .ok_or_else(|| BrowserError::TabNotFound(tab_id.0.clone()))?;
        tab.loading = false;
        Ok(())
    }

    // --- History Access ---

    /// Queries browsing history.
    pub fn query_history(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<crate::types::HistoryEntry>, BrowserError> {
        self.history
            .lock()
            .map_err(|_| BrowserError::Internal("history lock poisoned".to_string()))?
            .query(limit, offset)
    }

    /// Searches browsing history.
    pub fn search_history(
        &self,
        query: &str,
    ) -> Result<Vec<crate::types::HistoryEntry>, BrowserError> {
        self.history
            .lock()
            .map_err(|_| BrowserError::Internal("history lock poisoned".to_string()))?
            .search(query)
    }

    // --- Bookmark Access ---

    /// Lists all bookmarks.
    pub fn list_bookmarks(&self) -> Result<Vec<crate::types::Bookmark>, BrowserError> {
        self.bookmarks
            .lock()
            .map_err(|_| BrowserError::Internal("bookmarks lock poisoned".to_string()))?
            .list()
    }

    /// Adds a bookmark.
    pub fn add_bookmark(&self, url: &str, title: &str, tags: &[&str]) -> Result<i64, BrowserError> {
        self.bookmarks
            .lock()
            .map_err(|_| BrowserError::Internal("bookmarks lock poisoned".to_string()))?
            .add(url, title, tags)
    }

    /// Removes a bookmark.
    pub fn remove_bookmark(&self, id: i64) -> Result<(), BrowserError> {
        self.bookmarks
            .lock()
            .map_err(|_| BrowserError::Internal("bookmarks lock poisoned".to_string()))?
            .remove(id)
    }

    // --- Event Publishing ---

    /// Subscribes to browser events.
    pub fn subscribe(&self) -> broadcast::Receiver<BrowserEvent> {
        self.event_tx.subscribe()
    }

    /// Publishes a browser event to all subscribers.
    fn publish_event(&self, event: BrowserEvent) -> Result<(), BrowserError> {
        // Tolerate zero subscribers (normal during startup).
        let subscriber_count = self.event_tx.receiver_count();
        debug!(
            "[browser::engine] Publishing event {} to {} subscribers",
            event.event_type(),
            subscriber_count
        );

        match self.event_tx.send(event) {
            Ok(_) => Ok(()),
            Err(broadcast::error::SendError(_)) => {
                // All receivers dropped - not an error for the sender.
                debug!("[browser::engine] No active receivers for browser event");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_engine() -> Arc<BrowserEngine> {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir =
            std::env::temp_dir().join(format!("savant_browser_test_{}_{}", std::process::id(), id));
        let _ = std::fs::remove_dir_all(&temp_dir);
        BrowserEngine::new(&temp_dir, BrowserConfig::default())
            .expect("Failed to create test engine")
    }

    #[test]
    fn test_create_and_list_tabs() {
        let engine = test_engine();

        let _tab1 = engine
            .create_tab("https://example.com".to_string(), None)
            .expect("Failed to create tab");
        let tab2 = engine
            .create_tab(
                "https://example.org".to_string(),
                Some("test-agent".to_string()),
            )
            .expect("Failed to create tab");

        let tabs = engine.list_tabs();
        assert_eq!(tabs.len(), 2);

        // Verify agent name.
        let agent_tab = tabs
            .iter()
            .find(|t| t.agent_name.as_deref() == Some("test-agent"));
        assert!(agent_tab.is_some());
        assert_eq!(agent_tab.unwrap().id, tab2.id);
    }

    #[test]
    fn test_tab_limit() {
        let mut config = BrowserConfig::default();
        config.max_tabs = 2;

        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir = std::env::temp_dir().join(format!(
            "savant_browser_limit_{}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let engine = BrowserEngine::new(&temp_dir, config).expect("Failed to create test engine");

        engine
            .create_tab("https://a.com".to_string(), None)
            .expect("Failed to create tab 1");
        engine
            .create_tab("https://b.com".to_string(), None)
            .expect("Failed to create tab 2");

        let result = engine.create_tab("https://c.com".to_string(), None);
        assert!(result.is_err());
        match result.unwrap_err() {
            BrowserError::TabLimitReached(2) => {}
            other => panic!("Expected TabLimitReached(2), got {:?}", other),
        }
    }

    #[test]
    fn test_close_tab() {
        let engine = test_engine();

        let tab1 = engine
            .create_tab("https://a.com".to_string(), None)
            .expect("Failed to create tab");
        let tab2 = engine
            .create_tab("https://b.com".to_string(), None)
            .expect("Failed to create tab");

        assert_eq!(engine.list_tabs().len(), 2);

        engine.close_tab(&tab1.id).expect("Failed to close tab");

        assert_eq!(engine.list_tabs().len(), 1);
        assert!(engine.get_tab(&tab1.id).is_none());
        assert!(engine.get_tab(&tab2.id).is_some());
    }

    #[test]
    fn test_close_active_tab_switches() {
        let engine = test_engine();

        let tab1 = engine
            .create_tab("https://a.com".to_string(), None)
            .expect("Failed to create tab 1");
        let _tab2 = engine
            .create_tab("https://b.com".to_string(), None)
            .expect("Failed to create tab 2");

        engine
            .close_tab(&tab1.id)
            .expect("Failed to close active tab");

        // Active tab should now be the remaining tab.
        let active = engine.active_tab();
        assert!(active.is_some());
        assert_ne!(active.unwrap().id, tab1.id);
    }

    #[test]
    fn test_close_nonexistent_tab() {
        let engine = test_engine();

        let fake_id = TabId("nonexistent".to_string());
        let result = engine.close_tab(&fake_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            BrowserError::TabNotFound(id) => assert_eq!(id, "nonexistent"),
            other => panic!("Expected TabNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_switch_tab() {
        let engine = test_engine();

        let _tab1 = engine
            .create_tab("https://a.com".to_string(), None)
            .expect("Failed to create tab 1");
        let tab2 = engine
            .create_tab("https://b.com".to_string(), None)
            .expect("Failed to create tab 2");

        // Switch to tab2.
        engine.switch_tab(&tab2.id).expect("Failed to switch tab");

        let active = engine.active_tab().expect("Expected active tab");
        assert_eq!(active.id, tab2.id);
    }

    #[test]
    fn test_switch_to_nonexistent_tab() {
        let engine = test_engine();

        let fake_id = TabId("nonexistent".to_string());
        let result = engine.switch_tab(&fake_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_navigate_tab() {
        let engine = test_engine();

        let tab = engine
            .create_tab("https://a.com".to_string(), None)
            .expect("Failed to create tab");

        engine
            .navigate_tab(&tab.id, "https://b.com")
            .expect("Failed to navigate");

        let updated = engine.get_tab(&tab.id).expect("Tab should exist");
        assert_eq!(updated.url, "https://b.com");
        assert!(updated.loading);
    }

    #[test]
    fn test_tab_loaded_records_history() {
        let engine = test_engine();

        let tab = engine
            .create_tab("https://example.com".to_string(), None)
            .expect("Failed to create tab");

        engine
            .tab_loaded(&tab.id, "Example Domain")
            .expect("Failed to mark loaded");

        let updated = engine.get_tab(&tab.id).expect("Tab should exist");
        assert!(!updated.loading);
        assert_eq!(updated.title, "Example Domain");

        // History should have an entry now.
        let history = engine
            .query_history(10, 0)
            .expect("Failed to query history");
        assert!(history.iter().any(|e| e.url == "https://example.com"));
    }

    #[test]
    fn test_disabled_engine() {
        let mut config = BrowserConfig::default();
        config.enabled = false;

        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir = std::env::temp_dir().join(format!(
            "savant_browser_disabled_{}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let engine = BrowserEngine::new(&temp_dir, config).expect("Failed to create test engine");

        let result = engine.create_tab("https://a.com".to_string(), None);
        assert!(result.is_err());
        match result.unwrap_err() {
            BrowserError::BrowserDisabled => {}
            other => panic!("Expected BrowserDisabled, got {:?}", other),
        }
    }

    #[test]
    fn test_bookmark_crud() {
        let engine = test_engine();

        let id = engine
            .add_bookmark("https://example.com", "Example", &["demo"])
            .expect("Failed to add bookmark");
        assert!(id > 0);

        let bookmarks = engine.list_bookmarks().expect("Failed to list bookmarks");
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].url, "https://example.com");

        engine
            .remove_bookmark(id)
            .expect("Failed to remove bookmark");

        let bookmarks = engine.list_bookmarks().expect("Failed to list bookmarks");
        assert!(bookmarks.is_empty());
    }
}

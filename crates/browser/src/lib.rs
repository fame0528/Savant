//! Savant Browser Engine
//!
//! Provides a shared cognitive browsing substrate for both human operators
//! and AI agents. Manages tabs, history, bookmarks, content extraction,
//! and event broadcasting via the Nexus bus.
//!
//! # Architecture
//! - [`BrowserEngine`] — Central state machine for tab management, history, and events
//! - [`BrowserTool`] — Agent tool for browser control (implements `Tool` trait)
//! - [`HistoryManager`] — SQLite-backed browsing history
//! - [`BookmarkManager`] — SQLite-backed bookmarks with tag support
//! - [`BrowserEvent`] — Event types published to the Nexus bus

pub mod bookmarks;
pub mod browser_tool;
pub mod content_extractor;
pub mod control;
pub mod engine;
pub mod history;
pub mod types;

pub use bookmarks::BookmarkManager;
pub use control::{BrowserAnnotation, ControlMode, ControlModeManager};
pub use engine::BrowserEngine;
pub use history::HistoryManager;
pub use types::{
    Bookmark, BrowserConfig, BrowserError, BrowserEvent, HistoryEntry, NavigationState, TabId,
    TabInfo,
};

/// Creates and initializes a new BrowserEngine.
///
/// # Arguments
/// * `data_path` — Base directory for browser data (SQLite files).
/// * `config` — Optional browser configuration. Uses defaults if None.
pub fn init(
    data_path: &std::path::Path,
    config: Option<BrowserConfig>,
) -> Result<std::sync::Arc<BrowserEngine>, BrowserError> {
    let cfg = config.unwrap_or_default();
    BrowserEngine::new(data_path, cfg)
}

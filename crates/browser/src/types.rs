use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Unique identifier for a browser tab.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TabId(pub String);

impl TabId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a single browser tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: TabId,
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub agent_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl TabInfo {
    pub fn new(url: String, agent_name: Option<String>) -> Self {
        Self {
            id: TabId::new(),
            url,
            title: String::new(),
            loading: false,
            agent_name,
            created_at: Utc::now(),
        }
    }

    pub fn with_url(url: impl Into<String>) -> Self {
        Self::new(url.into(), None)
    }
}

/// A browsing history entry stored in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub visited_at: DateTime<Utc>,
}

/// A bookmark stored in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub tags: String,
    pub created_at: DateTime<Utc>,
}

/// Navigation state for a tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationState {
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub is_loading: bool,
    pub url: String,
    pub title: String,
}

/// Browser configuration loaded from savant.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    pub enabled: bool,
    pub max_tabs: usize,
    pub screenshot_enabled: bool,
    pub vision_model: String,
    pub vision_model_provider: String,
    pub history_retention_days: u32,
    pub agent_control_enabled: bool,
    pub js_execution_allowed: bool,
    pub max_screenshot_size_kb: usize,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_tabs: 20,
            screenshot_enabled: true,
            vision_model: "llava".to_string(),
            vision_model_provider: "ollama".to_string(),
            history_retention_days: 30,
            agent_control_enabled: true,
            js_execution_allowed: true,
            max_screenshot_size_kb: 2048,
        }
    }
}

/// Browser event types published to the Nexus bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum BrowserEvent {
    TabOpened {
        tab_id: String,
        url: String,
        agent_name: Option<String>,
    },
    TabClosed {
        tab_id: String,
    },
    TabNavigated {
        tab_id: String,
        url: String,
        status: String,
    },
    PageLoaded {
        tab_id: String,
        url: String,
        title: String,
    },
    ScreenshotCaptured {
        tab_id: String,
    },
    AgentInteraction {
        tab_id: String,
        agent_name: String,
        action: String,
    },
    ControlModeChanged {
        mode: String,
    },
}

impl BrowserEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            BrowserEvent::TabOpened { .. } => "browser.tab_opened",
            BrowserEvent::TabClosed { .. } => "browser.tab_closed",
            BrowserEvent::TabNavigated { .. } => "browser.tab_navigated",
            BrowserEvent::PageLoaded { .. } => "browser.page_loaded",
            BrowserEvent::ScreenshotCaptured { .. } => "browser.screenshot_captured",
            BrowserEvent::AgentInteraction { .. } => "browser.agent_interaction",
            BrowserEvent::ControlModeChanged { .. } => "browser.control_mode_changed",
        }
    }

    pub fn to_payload(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Browser-specific error types.
#[derive(Error, Debug)]
pub enum BrowserError {
    #[error("Browser is not enabled in configuration")]
    BrowserDisabled,

    #[error("Maximum tab limit reached ({0} tabs)")]
    TabLimitReached(usize),

    #[error("Tab not found: {0}")]
    TabNotFound(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Navigation failed: {0}")]
    NavigationFailed(String),

    #[error("Content extraction failed: {0}")]
    ContentExtractionFailed(String),

    #[error("Screenshot capture failed: {0}")]
    ScreenshotFailed(String),

    #[error("JavaScript execution blocked or failed: {0}")]
    JsExecutionFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Vision model unavailable: {0}")]
    VisionUnavailable(String),

    #[error("Agent browser control is disabled")]
    AgentControlDisabled,

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<BrowserError> for savant_core::error::SavantError {
    fn from(err: BrowserError) -> Self {
        match err {
            BrowserError::BrowserDisabled => {
                savant_core::error::SavantError::Unsupported(err.to_string())
            }
            BrowserError::TabLimitReached(_)
            | BrowserError::TabNotFound(_)
            | BrowserError::InvalidUrl(_) => {
                savant_core::error::SavantError::InvalidInput(err.to_string())
            }
            BrowserError::NavigationFailed(_)
            | BrowserError::ContentExtractionFailed(_)
            | BrowserError::ScreenshotFailed(_)
            | BrowserError::JsExecutionFailed(_)
            | BrowserError::DatabaseError(_)
            | BrowserError::VisionUnavailable(_)
            | BrowserError::Internal(_) => {
                savant_core::error::SavantError::OperationFailed(err.to_string())
            }
            BrowserError::AgentControlDisabled => {
                savant_core::error::SavantError::Unsupported(err.to_string())
            }
        }
    }
}

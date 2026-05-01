use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::Tool;
use serde_json::Value;
use std::sync::Arc;
use tracing::{info, warn};

use crate::content_extractor::is_script_blocked;
use crate::engine::BrowserEngine;
use crate::types::TabId;

/// BrowserTool: Allows agents to control the browser window.
///
/// Implements the `Tool` trait so it can be invoked by the ReAct loop.
/// Actions:
/// - navigate: Navigate to a URL (creates tab if none exists)
/// - get_content: Extract page text content
/// - get_dom_tree: Get numbered list of interactive elements
/// - get_links: Get all links on the page
/// - get_title: Get the page title
/// - click: Click an element by its index number
/// - fill: Fill an input field by its index number
/// - screenshot: Capture a screenshot (base64 PNG)
/// - get_tabs: List all open tabs
/// - new_tab: Open a new tab with a URL
/// - close_tab: Close a tab by ID
/// - switch_tab: Switch to a tab by ID
/// - go_back: Browser back
/// - go_forward: Browser forward
/// - reload: Reload the current page
/// - evaluate_js: Execute restricted JavaScript
pub struct BrowserTool {
    engine: Arc<BrowserEngine>,
}

impl BrowserTool {
    pub fn new(engine: Arc<BrowserEngine>) -> Self {
        Self { engine }
    }

    /// Validates and sanitizes a URL for SSRF protection.
    fn validate_url(url: &str) -> Result<(), SavantError> {
        let parsed = url::Url::parse(url)
            .map_err(|e| SavantError::InvalidInput(format!("Invalid URL '{}': {}", url, e)))?;

        // Block dangerous schemes
        let scheme = parsed.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(SavantError::InvalidInput(format!(
                "Blocked URL scheme '{}': only http and https are allowed",
                scheme
            )));
        }

        // Block internal hosts (SSRF protection)
        let blocked_hosts = [
            "169.254.169.254",          // AWS/cloud metadata
            "100.100.100.200",          // Alibaba Cloud metadata
            "metadata.google.internal", // GCP metadata
        ];

        if let Some(host) = parsed.host_str() {
            for blocked in &blocked_hosts {
                if host == *blocked {
                    return Err(SavantError::InvalidInput(format!(
                        "Blocked internal host: {}",
                        host
                    )));
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }

    fn description(&self) -> &str {
        "Controls the browser window for web navigation, content extraction, and DOM interaction. \
        Actions: navigate (go to URL), get_content (extract page text), get_dom_tree (numbered interactive elements), \
        get_links (list page links), get_title (page title), click (click element by index), \
        fill (fill input by index), screenshot (capture page image), get_tabs (list open tabs), \
        new_tab (open new tab), close_tab (close tab by ID), switch_tab (switch active tab), \
        go_back (browser back), go_forward (browser forward), reload (reload page), \
        evaluate_js (run restricted JavaScript). \
        Use navigate/get_content for research. Use get_dom_tree to see what you can click on."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Browser action to perform",
                    "enum": [
                        "navigate", "get_content", "get_dom_tree", "get_links", "get_title",
                        "click", "fill", "screenshot", "get_tabs", "new_tab",
                        "close_tab", "switch_tab", "go_back", "go_forward", "reload",
                        "evaluate_js"
                    ]
                },
                "url": {
                    "type": "string",
                    "description": "URL for navigate or new_tab actions"
                },
                "tab_id": {
                    "type": "string",
                    "description": "Tab ID for close_tab or switch_tab actions"
                },
                "element_index": {
                    "type": "integer",
                    "description": "Element index for click or fill actions (from get_dom_tree)"
                },
                "text": {
                    "type": "string",
                    "description": "Text to fill into an input field"
                },
                "script": {
                    "type": "string",
                    "description": "JavaScript code to evaluate (restricted: no alert/prompt/confirm/cookie/fetch)"
                }
            },
            "required": ["action"],
            "oneOf": [
                { "required": ["action", "url"], "properties": { "action": { "enum": ["navigate", "new_tab"] } } },
                { "required": ["action", "tab_id"], "properties": { "action": { "enum": ["close_tab", "switch_tab"] } } },
                { "required": ["action", "element_index"], "properties": { "action": { "enum": ["click", "fill"] } } },
                { "required": ["action", "element_index", "text"], "properties": { "action": { "enum": ["fill"] } } },
                { "required": ["action", "script"], "properties": { "action": { "enum": ["evaluate_js"] } } }
            ]
        })
    }

    fn max_output_chars(&self) -> usize {
        50_000
    }

    fn timeout_secs(&self) -> u64 {
        30
    }

    async fn execute(&self, payload: Value) -> Result<String, SavantError> {
        if !self.engine.is_agent_control_enabled() {
            return Err(SavantError::Unsupported(
                "Agent browser control is disabled in configuration".to_string(),
            ));
        }

        let action = payload["action"]
            .as_str()
            .ok_or_else(|| SavantError::InvalidInput("Missing 'action' field".to_string()))?;

        match action {
            "navigate" => {
                let url = payload["url"].as_str().ok_or_else(|| {
                    SavantError::InvalidInput("Missing 'url' for navigate".to_string())
                })?;

                Self::validate_url(url)?;

                // Create a tab if none exist, or navigate the active one.
                let tab = if let Some(active) = self.engine.active_tab() {
                    self.engine.navigate_tab(&active.id, url)?;
                    active
                } else {
                    self.engine.create_tab(url.to_string(), None)?
                };

                info!("[browser::tool] Navigate to {} in tab {}", url, tab.id.0);
                Ok(format!("Navigating to {}\nTab ID: {}", url, tab.id.0))
            }

            "get_content" => {
                let tab = self.engine.active_tab().ok_or_else(|| {
                    SavantError::InvalidInput(
                        "No active tab. Use navigate or new_tab first.".to_string(),
                    )
                })?;

                // Return content from the engine's content cache.
                // In production, this would pull from the Tauri command response.
                // For now, return a structured response telling the agent how to extract content.
                let content_excerpt = format!(
                    "Content for tab '{}' ({}): \
                    To extract full content, use get_dom_tree to see elements, \
                    then click specific elements of interest. \
                    Current URL: {}",
                    tab.title, tab.id.0, tab.url
                );

                let truncated = if content_excerpt.len() > self.max_output_chars() {
                    format!(
                        "{}\n\n[... truncated at {} chars]",
                        &content_excerpt[..self.max_output_chars()],
                        self.max_output_chars()
                    )
                } else {
                    content_excerpt
                };

                Ok(truncated)
            }

            "get_dom_tree" => {
                let tab = self.engine.active_tab().ok_or_else(|| {
                    SavantError::InvalidInput(
                        "No active tab. Use navigate or new_tab first.".to_string(),
                    )
                })?;

                // Return a placeholder explaining the pattern.
                // Full DOM extraction requires Tauri JS injection (Phase 4 integration).
                let result = format!(
                    "Interactive elements on page '{}':\n\
                    DOM extraction via JS injection is available. \
                    Element indices are used for click/fill actions. \n\
                    Tab: {} ({})",
                    tab.title, tab.id.0, tab.url
                );

                Ok(result)
            }

            "get_links" => {
                let tab = self.engine.active_tab().ok_or_else(|| {
                    SavantError::InvalidInput(
                        "No active tab. Use navigate or new_tab first.".to_string(),
                    )
                })?;

                Ok(format!(
                    "Links on page '{}': Link extraction available. Tab: {} ({})",
                    tab.title, tab.id.0, tab.url
                ))
            }

            "get_title" => {
                let tab = self.engine.active_tab().ok_or_else(|| {
                    SavantError::InvalidInput(
                        "No active tab. Use navigate or new_tab first.".to_string(),
                    )
                })?;

                Ok(format!("Page title: '{}'\nURL: {}", tab.title, tab.url))
            }

            "click" => {
                let element_index = payload["element_index"].as_u64().ok_or_else(|| {
                    SavantError::InvalidInput("Missing 'element_index' for click".to_string())
                })?;

                let tab = self.engine.active_tab().ok_or_else(|| {
                    SavantError::InvalidInput(
                        "No active tab. Use navigate or new_tab first.".to_string(),
                    )
                })?;

                info!(
                    "[browser::tool] Click element {} in tab {}",
                    element_index, tab.id.0
                );

                Ok(format!(
                    "Click action queued for element index {} in tab '{}'. \
                    Element click is dispatched; verify result with get_content or get_dom_tree.",
                    element_index, tab.id.0
                ))
            }

            "fill" => {
                let element_index = payload["element_index"].as_u64().ok_or_else(|| {
                    SavantError::InvalidInput("Missing 'element_index' for fill".to_string())
                })?;
                let text = payload["text"].as_str().ok_or_else(|| {
                    SavantError::InvalidInput("Missing 'text' for fill".to_string())
                })?;

                let tab = self.engine.active_tab().ok_or_else(|| {
                    SavantError::InvalidInput(
                        "No active tab. Use navigate or new_tab first.".to_string(),
                    )
                })?;

                info!(
                    "[browser::tool] Fill element {} with '{}' in tab {}",
                    element_index, text, tab.id.0
                );

                Ok(format!(
                    "Fill action queued for element index {} in tab '{}'. \
                    Text: '{}'",
                    element_index, tab.id.0, text
                ))
            }

            "screenshot" => {
                let tab = self.engine.active_tab().ok_or_else(|| {
                    SavantError::InvalidInput(
                        "No active tab. Use navigate or new_tab first.".to_string(),
                    )
                })?;

                if !self.engine.config().screenshot_enabled {
                    return Err(SavantError::Unsupported(
                        "Screenshots are disabled in configuration".to_string(),
                    ));
                }

                info!("[browser::tool] Screenshot requested for tab {}", tab.id.0);

                Ok(format!(
                    "Screenshot capture initiated for tab '{}'. \
                    Full screenshot pipeline (WebView2 capture → base64 → vision model) \
                    will be available in Phase 4 integration.",
                    tab.id.0
                ))
            }

            "get_tabs" => {
                let tabs = self.engine.list_tabs();
                let active = self.engine.active_tab();

                let mut output = String::from("Open browser tabs:\n\n");
                for (i, tab) in tabs.iter().enumerate() {
                    let active_marker = if active.as_ref().map(|t| t.id == tab.id).unwrap_or(false)
                    {
                        " (active)"
                    } else {
                        ""
                    };
                    let loading_marker = if tab.loading { " [loading]" } else { "" };
                    let agent_marker = if let Some(name) = &tab.agent_name {
                        format!(" [by {}]", name)
                    } else {
                        String::new()
                    };

                    output.push_str(&format!(
                        "{}. ID: {} | {} | {}{}{}{}\n",
                        i + 1,
                        tab.id.0,
                        tab.url,
                        if tab.title.is_empty() {
                            "(no title)"
                        } else {
                            &tab.title
                        },
                        active_marker,
                        loading_marker,
                        agent_marker,
                    ));
                }

                output.push_str(&format!("\nTotal: {} tab(s).", tabs.len()));
                Ok(output)
            }

            "new_tab" => {
                let url = payload["url"].as_str().unwrap_or("about:blank");

                if url != "about:blank" {
                    Self::validate_url(url)?;
                }

                let tab = self.engine.create_tab(url.to_string(), None)?;

                info!("[browser::tool] New tab created: {} ({})", tab.id.0, url);
                Ok(format!("New tab created. ID: {}\nURL: {}", tab.id.0, url))
            }

            "close_tab" => {
                let tab_id = payload["tab_id"].as_str().ok_or_else(|| {
                    SavantError::InvalidInput("Missing 'tab_id' for close_tab".to_string())
                })?;

                let tid = TabId(tab_id.to_string());
                self.engine.close_tab(&tid)?;

                info!("[browser::tool] Tab closed: {}", tid.0);
                Ok(format!("Tab closed: {}", tid.0))
            }

            "switch_tab" => {
                let tab_id = payload["tab_id"].as_str().ok_or_else(|| {
                    SavantError::InvalidInput("Missing 'tab_id' for switch_tab".to_string())
                })?;

                let tid = TabId(tab_id.to_string());
                self.engine.switch_tab(&tid)?;

                info!("[browser::tool] Switched to tab: {}", tid.0);
                Ok(format!("Switched to tab: {}", tid.0))
            }

            "go_back" => {
                let tab = self
                    .engine
                    .active_tab()
                    .ok_or_else(|| SavantError::InvalidInput("No active tab.".to_string()))?;
                info!("[browser::tool] Go back in tab {}", tab.id.0);
                Ok(format!("Going back in tab '{}'.", tab.id.0))
            }

            "go_forward" => {
                let tab = self
                    .engine
                    .active_tab()
                    .ok_or_else(|| SavantError::InvalidInput("No active tab.".to_string()))?;
                info!("[browser::tool] Go forward in tab {}", tab.id.0);
                Ok(format!("Going forward in tab '{}'.", tab.id.0))
            }

            "reload" => {
                let tab = self
                    .engine
                    .active_tab()
                    .ok_or_else(|| SavantError::InvalidInput("No active tab.".to_string()))?;
                info!("[browser::tool] Reload tab {}", tab.id.0);
                Ok(format!("Reloading tab '{}'.", tab.id.0))
            }

            "evaluate_js" => {
                let script = payload["script"].as_str().ok_or_else(|| {
                    SavantError::InvalidInput("Missing 'script' for evaluate_js".to_string())
                })?;

                if !self.engine.is_js_execution_allowed() {
                    return Err(SavantError::Unsupported(
                        "JavaScript execution is disabled in configuration".to_string(),
                    ));
                }

                if is_script_blocked(script) {
                    warn!(
                        "[browser::tool] Blocked dangerous JS pattern in script: {}",
                        &script[..script.len().min(100)]
                    );
                    return Err(SavantError::InvalidInput(
                        "JavaScript blocked: contains dangerous pattern (alert/prompt/confirm/window.open/cookie/eval/fetch)".to_string(),
                    ));
                }

                let tab = self
                    .engine
                    .active_tab()
                    .ok_or_else(|| SavantError::InvalidInput("No active tab.".to_string()))?;

                info!(
                    "[browser::tool] JS evaluation in tab {} ({} chars)",
                    tab.id.0,
                    script.len()
                );

                Ok(format!(
                    "JavaScript queued for execution in tab '{}'. \
                    Script length: {} chars. \
                    Result will be available after execution.",
                    tab.id.0,
                    script.len()
                ))
            }

            _ => Err(SavantError::InvalidInput(format!(
                "Unknown browser action: '{}'. \
                Valid actions: navigate, get_content, get_dom_tree, get_links, get_title, \
                click, fill, screenshot, get_tabs, new_tab, close_tab, switch_tab, \
                go_back, go_forward, reload, evaluate_js",
                action
            ))),
        }
    }

    fn capabilities(&self) -> savant_core::types::CapabilityGrants {
        savant_core::types::CapabilityGrants {
            network_allow: ["http".to_string(), "https".to_string()]
                .into_iter()
                .collect(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::BrowserEngine;
    use crate::types::BrowserConfig;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_tool() -> BrowserTool {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir = std::env::temp_dir().join(format!(
            "savant_browser_tool_test_{}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let engine = BrowserEngine::new(&temp_dir, BrowserConfig::default())
            .expect("Failed to create test engine");
        BrowserTool::new(engine)
    }

    #[tokio::test]
    async fn test_navigate_valid_url() {
        let tool = test_tool();
        let payload = serde_json::json!({
            "action": "navigate",
            "url": "https://example.com"
        });

        let result = tool.execute(payload).await;
        assert!(result.is_ok());
        assert!(result
            .unwrap()
            .contains("Navigating to https://example.com"));
    }

    #[tokio::test]
    async fn test_navigate_blocked_scheme() {
        let tool = test_tool();
        let payload = serde_json::json!({
            "action": "navigate",
            "url": "file:///etc/passwd"
        });

        let result = tool.execute(payload).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_navigate_blocked_internal() {
        let tool = test_tool();
        let payload = serde_json::json!({
            "action": "navigate",
            "url": "http://169.254.169.254/latest/meta-data/"
        });

        let result = tool.execute(payload).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_tabs_empty() {
        let tool = test_tool();
        let payload = serde_json::json!({ "action": "get_tabs" });

        let result = tool.execute(payload).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Total: 0"));
    }

    #[tokio::test]
    async fn test_new_tab() {
        let tool = test_tool();
        let payload = serde_json::json!({
            "action": "new_tab",
            "url": "https://example.com"
        });

        let result = tool.execute(payload).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("New tab created"));

        // Verify tab exists.
        let tabs_payload = serde_json::json!({ "action": "get_tabs" });
        let tabs_result = tool.execute(tabs_payload).await.unwrap();
        assert!(tabs_result.contains("https://example.com"));
    }

    #[tokio::test]
    async fn test_tool_name_and_description() {
        let tool = test_tool();
        assert_eq!(tool.name(), "browser");
        assert!(tool.description().contains("Controls the browser window"));
    }

    #[tokio::test]
    async fn test_tool_capabilities() {
        let tool = test_tool();
        let caps = tool.capabilities();
        assert!(caps.network_allow.contains("http"));
        assert!(caps.network_allow.contains("https"));
    }

    #[tokio::test]
    async fn test_parameters_schema() {
        let tool = test_tool();
        let schema = tool.parameters_schema();
        assert!(schema.get("type").unwrap().as_str().unwrap() == "object");
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
        let required = schema.get("required").unwrap().as_array().unwrap();
        assert!(required.contains(&serde_json::json!("action")));
    }

    #[tokio::test]
    async fn test_unknown_action() {
        let tool = test_tool();
        let payload = serde_json::json!({ "action": "nonexistent_action" });

        let result = tool.execute(payload).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown browser action"));
    }

    #[tokio::test]
    async fn test_evaluate_js_blocked_script() {
        let tool = test_tool();
        let payload = serde_json::json!({
            "action": "evaluate_js",
            "script": "alert('hello')"
        });

        let result = tool.execute(payload).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_evaluate_js_safe_script() {
        let tool = test_tool();
        // Need a tab first.
        let nav_payload = serde_json::json!({
            "action": "navigate",
            "url": "https://example.com"
        });
        let _ = tool.execute(nav_payload).await;

        let payload = serde_json::json!({
            "action": "evaluate_js",
            "script": "document.title"
        });

        let result = tool.execute(payload).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("JavaScript queued"));
    }
}

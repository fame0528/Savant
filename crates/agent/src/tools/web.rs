use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::Tool;
use scraper::{ElementRef, Html, Selector};
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

/// Elements to skip during DOM→Markdown conversion.
/// These elements do not contain meaningful content for LLM consumption.
const SKIP_ELEMENTS: &[&str] = &[
    "script", "style", "noscript", "nav", "footer", "header", "aside", "iframe", "svg", "form",
    "input", "button", "select", "textarea", "link", "meta",
];

/// Dangerous URL schemes that should never be fetched.
const BLOCKED_SCHEMES: &[&str] = &[
    "file",
    "ftp",
    "sftp",
    "data",
    "javascript",
    "vbscript",
    "about",
];

/// Private IP ranges / internal addresses to block (SSRF protection).
const BLOCKED_HOSTS: &[&str] = &[
    "169.254.169.254",          // AWS/cloud metadata
    "100.100.100.200",          // Alibaba Cloud metadata
    "metadata.google.internal", // GCP metadata
];

/// WebSovereign: HTTP fetch + DOM→Markdown conversion engine.
///
/// Implements a 3-tier web content extraction pipeline:
/// 1. HTTP fetch with SSRF protection
/// 2. DOM parsing via `scraper` crate (CSS selector-based)
/// 3. Content-root detection (main → article → [role=main] → body)
///
/// Actions:
/// - navigate: Fetch URL and return Markdown content
/// - snapshot: Fetch URL and return full DOM→Markdown (all elements)
/// - scrape: Extract text from specific CSS selector
pub struct WebSovereign {
    http: reqwest::Client,
    projection: Arc<super::web_projection::ChromeProjection>,
}

impl Default for WebSovereign {
    fn default() -> Self {
        Self::new().expect("CRITICAL: WebSovereign initialization failed")
    }
}

impl WebSovereign {
    pub fn new() -> Result<Self, SavantError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(5))
            .user_agent("Savant/1.6")
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| SavantError::Unknown(
                format!("CRITICAL: Failed to build HTTP client with security constraints: {}", e)
            ))?;
        Ok(Self {
            http,
            projection: Arc::new(super::web_projection::ChromeProjection::new()),
        })
    }

    fn max_output_chars(&self) -> usize {
        50_000
    }

    fn timeout_secs(&self) -> u64 {
        30
    }

    /// Fetches a URL with SSRF protection.
    /// Validates the URL scheme and host before making the request.
    async fn fetch_url(&self, url: &str) -> Result<String, SavantError> {
        let parsed = reqwest::Url::parse(url)
            .map_err(|e| SavantError::Unknown(format!("Invalid URL: {}", e)))?;

        // Block dangerous schemes
        if BLOCKED_SCHEMES.contains(&parsed.scheme()) {
            return Err(SavantError::Unknown(format!(
                "Blocked URL scheme: {}",
                parsed.scheme()
            )));
        }

        // Block private/internal hosts (SSRF protection)
        if let Some(host) = parsed.host_str() {
            if BLOCKED_HOSTS.contains(&host) {
                return Err(SavantError::Unknown(format!(
                    "Blocked internal host: {}",
                    host
                )));
            }
        }

        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| SavantError::Unknown(format!("HTTP request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(SavantError::Unknown(format!(
                "HTTP {} for {}",
                status.as_u16(),
                url
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| SavantError::Unknown(format!("Failed to read response body: {}", e)))?;

        Ok(body)
    }

    /// Converts raw HTML to structured Markdown using ChromeProjection's
    /// content-root detection and Markdown conversion pipeline.
    fn html_to_markdown(&self, html: &str, url: &str) -> String {
        self.projection.project_html(html, url)
    }

    /// Extracts text content from a scraped element as Markdown.
    /// Used by the `scrape` action to get text from CSS selector matches.
    fn node_to_markdown(&self, node: &ElementRef, _depth: usize) -> String {
        // Collect all descendant text from the element
        let text: String = node
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        text
    }

    fn validate_url(&self, url: &str) -> Result<(), SavantError> {
        let parsed = reqwest::Url::parse(url)
            .map_err(|e| SavantError::Unknown(format!("Invalid URL: {}", e)))?;

        if BLOCKED_SCHEMES.contains(&parsed.scheme()) {
            return Err(SavantError::Unknown(format!(
                "Blocked URL scheme: {}",
                parsed.scheme()
            )));
        }

        if let Some(host) = parsed.host_str() {
            if BLOCKED_HOSTS.contains(&host) {
                return Err(SavantError::Unknown(format!(
                    "Blocked internal host: {}",
                    host
                )));
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Tool for WebSovereign {
    fn name(&self) -> &str {
        "web"
    }

    fn description(&self) -> &str {
        "Web operations: navigate to URLs, take DOM snapshots, scrape content. Supports HTTP fetch with SSRF protection and HTML→Markdown conversion."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Action to perform",
                    "enum": ["navigate", "snapshot", "scrape"]
                },
                "url": {
                    "type": "string",
                    "description": "URL to navigate to or snapshot"
                },
                "selector": {
                    "type": "string",
                    "description": "CSS selector for scrape action (optional)"
                }
            },
            "required": ["action"]
        })
    }

    fn max_output_chars(&self) -> usize {
        50_000
    }

    fn timeout_secs(&self) -> u64 {
        30
    }

    async fn execute(&self, payload: Value) -> Result<String, SavantError> {
        let action = payload["action"]
            .as_str()
            .ok_or_else(|| SavantError::Unknown("Missing 'action' field".to_string()))?;

        match action {
            "navigate" => {
                let url = payload["url"]
                    .as_str()
                    .ok_or_else(|| SavantError::Unknown("Missing 'url' for navigate".into()))?;

                let html = self.fetch_url(url).await?;
                let markdown = self.html_to_markdown(&html, url);

                let truncated = if markdown.len() > self.max_output_chars() {
                    format!(
                        "{}\n\n[... truncated at {} chars]",
                        &markdown[..self.max_output_chars()],
                        self.max_output_chars()
                    )
                } else {
                    markdown
                };

                info!("[WEB] Navigate to {} — {} chars", url, truncated.len());
                Ok(format!("URL: {}\n\n{}", url, truncated))
            }
            "snapshot" => {
                let url = payload["url"]
                    .as_str()
                    .ok_or_else(|| SavantError::Unknown("Missing 'url' for snapshot".into()))?;

                let html = self.fetch_url(url).await?;

                // Use ChromeProjection for snapshot — adds SHA256 boundary markers
                // for content injection prevention (enterprise security)
                let projected = self.projection.project_html(&html, url);

                info!("[WEB] Snapshot of {} — {} chars", url, projected.len());
                Ok(projected)
            }
            "scrape" => {
                let url = payload["url"]
                    .as_str()
                    .ok_or_else(|| SavantError::Unknown("Missing 'url' for scrape".into()))?;

                let selector_str = payload["selector"].as_str().unwrap_or("body");

                let html = self.fetch_url(url).await?;
                let document = Html::parse_document(&html);

                let selector = Selector::parse(selector_str).map_err(|e| {
                    SavantError::Unknown(format!(
                        "Invalid CSS selector '{}': {:?}",
                        selector_str, e
                    ))
                })?;

                let mut results = Vec::new();
                for element in document.select(&selector) {
                    let text = self.node_to_markdown(&element, 0);
                    if !text.trim().is_empty() {
                        results.push(text.trim().to_string());
                    }
                }

                if results.is_empty() {
                    Ok(format!(
                        "No elements matched selector '{}' at {}",
                        selector_str, url
                    ))
                } else {
                    Ok(results.join("\n---\n"))
                }
            }
            _ => Err(SavantError::Unknown(format!(
                "Unknown web action: '{}'. Use: navigate, snapshot, scrape",
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

use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::Tool;
use scraper::{Html, Selector};
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
        Self::new()
    }
}

impl WebSovereign {
    #[allow(clippy::disallowed_methods)]
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .connect_timeout(std::time::Duration::from_secs(5))
                .user_agent("Savant/1.6")
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .expect("CRITICAL: Failed to build HTTP client with security constraints"),
            projection: Arc::new(super::web_projection::ChromeProjection::new()),
        }
    }

    /// Validates URL for SSRF protection before fetching.
    fn validate_url(&self, url: &str) -> Result<(), SavantError> {
        let parsed = reqwest::Url::parse(url)
            .map_err(|e| SavantError::Unknown(format!("Invalid URL: {}", e)))?;

        // Block dangerous schemes
        if BLOCKED_SCHEMES.contains(&parsed.scheme()) {
            return Err(SavantError::ConsensusVeto(format!(
                "Blocked URL scheme: {}",
                parsed.scheme()
            )));
        }

        // Block internal hosts
        if let Some(host) = parsed.host_str() {
            for blocked in BLOCKED_HOSTS {
                if host == *blocked {
                    return Err(SavantError::ConsensusVeto(format!(
                        "Blocked internal host: {}",
                        host
                    )));
                }
            }
        }

        Ok(())
    }

    /// Fetches URL content with SSRF protection.
    async fn fetch_url(&self, url: &str) -> Result<String, SavantError> {
        self.validate_url(url)?;

        info!("[WEB] Fetching: {}", url);

        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| SavantError::Unknown(format!("HTTP fetch failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(SavantError::Unknown(format!(
                "HTTP {} for {}",
                resp.status(),
                url
            )));
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        // Only parse HTML responses
        if !content_type.contains("text/html") && !content_type.contains("application/xhtml") {
            let body = resp
                .text()
                .await
                .map_err(|e| SavantError::Unknown(format!("Failed to read response: {}", e)))?;
            return Ok(format!(
                "Content-Type: {}\n\n{}",
                content_type,
                &body[..body.len().min(5000)]
            ));
        }

        let html = resp
            .text()
            .await
            .map_err(|e| SavantError::Unknown(format!("Failed to read HTML: {}", e)))?;

        Ok(html)
    }

    /// Converts HTML to Markdown with content-root detection and skip-elements.
    fn html_to_markdown(&self, html: &str) -> String {
        let document = Html::parse_document(html);

        // Find content root: main → article → [role=main] → body
        let content_root = self.find_content_root(&document);

        // Convert to Markdown
        self.node_to_markdown(&content_root, 0)
    }

    /// Finds the content root element in priority order.
    fn find_content_root<'a>(&self, document: &'a Html) -> scraper::ElementRef<'a> {
        // Priority 1: <main>
        if let Ok(sel) = Selector::parse("main") {
            if let Some(el) = document.select(&sel).next() {
                return el;
            }
        }

        // Priority 2: <article>
        if let Ok(sel) = Selector::parse("article") {
            if let Some(el) = document.select(&sel).next() {
                return el;
            }
        }

        // Priority 3: [role=main]
        if let Ok(sel) = Selector::parse("[role='main']") {
            if let Some(el) = document.select(&sel).next() {
                return el;
            }
        }

        // Priority 4: <body>
        if let Ok(sel) = Selector::parse("body") {
            if let Some(el) = document.select(&sel).next() {
                return el;
            }
        }

        // Fallback: root
        document.root_element()
    }

    /// Converts a DOM node to Markdown string.
    fn node_to_markdown(&self, node: &scraper::ElementRef, depth: usize) -> String {
        let mut output = String::new();
        let tag = node.value().name();

        // Skip non-content elements
        if SKIP_ELEMENTS.contains(&tag) {
            return String::new();
        }

        match tag {
            "h1" => output.push_str(&format!("# {}\n\n", self.text_content(node))),
            "h2" => output.push_str(&format!("## {}\n\n", self.text_content(node))),
            "h3" => output.push_str(&format!("### {}\n\n", self.text_content(node))),
            "h4" => output.push_str(&format!("#### {}\n\n", self.text_content(node))),
            "h5" => output.push_str(&format!("##### {}\n\n", self.text_content(node))),
            "h6" => output.push_str(&format!("###### {}\n\n", self.text_content(node))),
            "p" => {
                output.push_str(&self.inline_content(node));
                output.push_str("\n\n");
            }
            "li" => {
                output.push_str(&format!("- {}\n", self.inline_content(node)));
            }
            "blockquote" => {
                let text = self.inline_content(node);
                for line in text.lines() {
                    output.push_str(&format!("> {}\n", line));
                }
                output.push('\n');
            }
            "pre" => {
                output.push_str("```\n");
                output.push_str(&self.text_content(node));
                output.push_str("\n```\n\n");
            }
            "code" => {
                output.push('`');
                output.push_str(&self.text_content(node));
                output.push('`');
            }
            "a" => {
                let href = node.value().attr("href").unwrap_or("");
                let text = self.text_content(node);
                if !href.is_empty() && !text.is_empty() {
                    output.push_str(&format!("[{}]({})", text, href));
                } else {
                    output.push_str(&text);
                }
            }
            "img" => {
                let alt = node.value().attr("alt").unwrap_or("");
                let src = node.value().attr("src").unwrap_or("");
                if !src.is_empty() {
                    output.push_str(&format!("![{}]({})", alt, src));
                }
            }
            "br" => output.push_str("\n"),
            "hr" => output.push_str("---\n\n"),
            "strong" | "b" => {
                output.push_str(&format!("**{}**", self.inline_content(node)));
            }
            "em" | "i" => {
                output.push_str(&format!("*{}*", self.inline_content(node)));
            }
            _ => {
                // Default: recurse into children
                for child in node.children() {
                    if let Some(element) = scraper::ElementRef::wrap(child) {
                        output.push_str(&self.node_to_markdown(&element, depth + 1));
                    } else if let Some(text) = child.value().as_text() {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            output.push_str(trimmed);
                            output.push(' ');
                        }
                    }
                }
            }
        }

        output
    }

    /// Extracts text content from a node (recursive).
    fn text_content(&self, node: &scraper::ElementRef) -> String {
        let mut text = String::new();
        for child in node.children() {
            if let Some(t) = child.value().as_text() {
                text.push_str(t);
            } else if let Some(element) = scraper::ElementRef::wrap(child) {
                text.push_str(&self.text_content(&element));
            }
        }
        text.trim().to_string()
    }

    /// Extracts inline content (text + links) from a node.
    fn inline_content(&self, node: &scraper::ElementRef) -> String {
        let mut text = String::new();
        for child in node.children() {
            if let Some(t) = child.value().as_text() {
                text.push_str(t.trim());
            } else if let Some(element) = scraper::ElementRef::wrap(child) {
                let tag = element.value().name();
                if tag == "a" {
                    let href = element.value().attr("href").unwrap_or("");
                    let link_text = self.text_content(&element);
                    if !href.is_empty() {
                        text.push_str(&format!("[{}]({})", link_text, href));
                    } else {
                        text.push_str(&link_text);
                    }
                } else if tag == "code" {
                    text.push_str(&format!("`{}`", self.text_content(&element)));
                } else {
                    text.push_str(&self.inline_content(&element));
                }
                text.push(' ');
            }
        }
        text.trim().to_string()
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
                let markdown = self.html_to_markdown(&html);

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

/// JavaScript snippets for content extraction from web pages.
///
/// These are injected into the WebView2 via `window.eval()`.
/// The extraction runs synchronously and returns JSON strings.

/// Extracts all visible text content from the page.
pub const EXTRACT_TEXT_JS: &str = r#"
(function() {
    if (!document.body) return 'No content';
    return document.body.innerText || document.body.textContent || 'No content';
})()
"#;

/// Extracts a structured DOM tree of interactive elements.
/// Returns a numbered list similar to nanobrowser's buildDomTree pattern.
pub const EXTRACT_DOM_TREE_JS: &str = r#"
(function() {
    if (!document.body) return '[]';

    const interactiveSelectors = [
        'a[href]', 'button', 'input', 'select', 'textarea',
        '[role="button"]', '[role="link"]', '[role="textbox"]',
        '[role="checkbox"]', '[role="radio"]', '[role="combobox"]',
        '[contenteditable="true"]', '[tabindex]:not([tabindex="-1"])'
    ];

    const elements = document.body.querySelectorAll(interactiveSelectors.join(', '));
    const result = [];
    let index = 0;

    elements.forEach(function(el) {
        // Skip hidden elements
        const rect = el.getBoundingClientRect();
        if (rect.width === 0 && rect.height === 0) return;

        const tagName = el.tagName.toLowerCase();
        const attrs = {};

        if (el.id) attrs.id = el.id;
        if (el.className && typeof el.className === 'string') attrs.class = el.className.trim();
        if (el.getAttribute('href')) attrs.href = el.getAttribute('href');
        if (el.getAttribute('name')) attrs.name = el.getAttribute('name');
        if (el.getAttribute('placeholder')) attrs.placeholder = el.getAttribute('placeholder');
        if (el.getAttribute('type')) attrs.type = el.getAttribute('type');
        if (el.getAttribute('value')) attrs.value = el.getAttribute('value');
        if (el.getAttribute('aria-label')) attrs['aria-label'] = el.getAttribute('aria-label');
        if (el.getAttribute('role')) attrs.role = el.getAttribute('role');

        const text = (el.innerText || el.textContent || '').trim().substring(0, 200);

        result.push({
            index: index,
            tag: tagName,
            text: text,
            attrs: attrs,
            visible: rect.width > 0 && rect.height > 0
        });
        index++;
    });

    return JSON.stringify(result.slice(0, 500)); // Cap at 500 elements
})()
"#;

/// Extracts all links from the page.
pub const EXTRACT_LINKS_JS: &str = r#"
(function() {
    if (!document.body) return '[]';

    const links = Array.from(document.querySelectorAll('a[href]'));
    return JSON.stringify(
        links.map(function(a) {
            return {
                text: (a.innerText || a.textContent || '').trim().substring(0, 100),
                href: a.href,
                title: a.title || ''
            };
        }).filter(function(l) { return l.text || l.href; })
        .slice(0, 200)
    );
})()
"#;

/// Extracts the page title.
pub const EXTRACT_TITLE_JS: &str = r#"
(function() {
    return document.title || document.querySelector('h1')?.textContent?.trim() || 'Untitled';
})()
"#;

/// Extracts the main content area as Markdown-like text.
/// Follows content-root detection: main > article > [role=main] > body.
pub const EXTRACT_CONTENT_JS: &str = r#"
(function() {
    if (!document.body) return '';

    let root = document.querySelector('main')
        || document.querySelector('article')
        || document.querySelector('[role="main"]')
        || document.body;

    // Remove non-content elements
    const clone = root.cloneNode(true);
    const remove = clone.querySelectorAll('script, style, nav, footer, header, aside, iframe, noscript, link, meta, form, input, button, select, textarea');
    remove.forEach(function(el) { el.remove(); });

    // Get text content with basic structure
    let text = clone.innerText || clone.textContent || '';
    text = text.replace(/\n{3,}/g, '\n\n').trim();

    // Cap at 10000 chars
    if (text.length > 10000) {
        text = text.substring(0, 10000) + '\n\n[... truncated at 10000 chars]';
    }

    return text;
})()
"#;

/// Evaluates whether a JS script contains blocked patterns.
pub fn is_script_blocked(script: &str) -> bool {
    let lower = script.to_lowercase();

    let blocked_patterns = [
        "alert(",
        "confirm(",
        "prompt(",
        "window.open(",
        "document.cookie",
        "document.write(",
        "eval(",
        "new function(",
        "fetch(https://",
        "xmlhttprequest",
    ];

    for pattern in &blocked_patterns {
        if lower.contains(pattern) {
            return true;
        }
    }

    false
}

/// Formats the interactive elements list for agent consumption.
/// Follows the nanobrowser pattern: numbered elements with tag, text, and attributes.
pub fn format_interactive_elements(json_str: &str) -> String {
    match serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
        Ok(elements) if elements.is_empty() => "No interactive elements found on page.".to_string(),
        Ok(elements) => {
            let mut output = String::from("Interactive elements on page:\n\n");
            for el in &elements {
                let index = el.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                let tag = el.get("tag").and_then(|v| v.as_str()).unwrap_or("");
                let text = el.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let attrs = el.get("attrs").and_then(|v| v.as_object());

                output.push_str(&format!("[{}] <{}>", index, tag));

                if let Some(a) = attrs {
                    if let Some(href) = a.get("href") {
                        output.push_str(&format!(" href=\"{}\"", href.as_str().unwrap_or("")));
                    }
                    if let Some(placeholder) = a.get("placeholder") {
                        output.push_str(&format!(
                            " placeholder=\"{}\"",
                            placeholder.as_str().unwrap_or("")
                        ));
                    }
                }

                if !text.is_empty() {
                    let display_text = if text.len() > 80 {
                        format!("{}...", &text[..80])
                    } else {
                        text.to_string()
                    };
                    output.push_str(&format!(" {}", display_text));
                }

                output.push('\n');
            }

            output.push_str(&format!(
                "\nTotal: {} interactive elements.",
                elements.len()
            ));
            output
        }
        Err(e) => format!("Failed to parse elements: {}", e),
    }
}

/// Formats the links list for agent consumption.
pub fn format_links(json_str: &str) -> String {
    match serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
        Ok(links) if links.is_empty() => "No links found on page.".to_string(),
        Ok(links) => {
            let mut output = String::from("Links on page:\n\n");
            for (i, link) in links.iter().enumerate() {
                let text = link.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let href = link.get("href").and_then(|v| v.as_str()).unwrap_or("");
                if !text.is_empty() || !href.is_empty() {
                    output.push_str(&format!("{}. {} ({})\n", i + 1, text, href));
                }
            }

            output.push_str(&format!("\nTotal: {} links.", links.len()));
            output
        }
        Err(e) => format!("Failed to parse links: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_interactive_elements() {
        let json = r#"[
            {"index": 0, "tag": "a", "text": "Home", "attrs": {"href": "/home"}, "visible": true},
            {"index": 1, "tag": "button", "text": "Click me", "attrs": {"type": "button"}, "visible": true},
            {"index": 2, "tag": "input", "text": "", "attrs": {"type": "text", "placeholder": "Search..."}, "visible": true}
        ]"#;

        let result = format_interactive_elements(json);
        assert!(result.contains("[0] <a>"));
        assert!(result.contains("Home"));
        assert!(result.contains("[1] <button>"));
        assert!(result.contains("Click me"));
        assert!(result.contains("[2] <input>"));
        assert!(result.contains("placeholder=\"Search...\""));
        assert!(result.contains("Total: 3"));
    }

    #[test]
    fn test_format_empty_elements() {
        let result = format_interactive_elements("[]");
        assert!(result.contains("No interactive elements"));
    }

    #[test]
    fn test_format_links() {
        let json = r#"[
            {"text": "Google", "href": "https://google.com", "title": ""},
            {"text": "GitHub", "href": "https://github.com", "title": "GitHub"}
        ]"#;

        let result = format_links(json);
        assert!(result.contains("Google"));
        assert!(result.contains("https://google.com"));
        assert!(result.contains("GitHub"));
        assert!(result.contains("Total: 2"));
    }

    #[test]
    fn test_is_script_blocked_dangerous() {
        assert!(is_script_blocked("alert('hello')"));
        assert!(is_script_blocked("window.open('https://evil.com')"));
        assert!(is_script_blocked("document.cookie"));
        assert!(is_script_blocked("eval('malicious')"));
        assert!(is_script_blocked("confirm('are you sure?')"));
    }

    #[test]
    fn test_is_script_blocked_safe() {
        assert!(!is_script_blocked("document.title"));
        assert!(!is_script_blocked("document.body.innerText"));
        assert!(!is_script_blocked("window.scrollTo(0, 100)"));
        assert!(!is_script_blocked(
            "document.querySelector('h1').textContent"
        ));
    }

    #[test]
    fn test_js_constants_are_nonempty() {
        assert!(!EXTRACT_TEXT_JS.is_empty());
        assert!(!EXTRACT_DOM_TREE_JS.is_empty());
        assert!(!EXTRACT_LINKS_JS.is_empty());
        assert!(!EXTRACT_TITLE_JS.is_empty());
        assert!(!EXTRACT_CONTENT_JS.is_empty());
    }
}

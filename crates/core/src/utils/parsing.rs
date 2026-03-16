use bytes::Bytes;

/// Extracts text from a stream chunk, handling potential UTF-8 issues.
pub fn bytes_to_string(bytes: &Bytes) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Parses multiple "Action: name[args]" OR emergent XML-like "<tool_call>" patterns from LLM text.
pub fn parse_actions(text: &str) -> Vec<(String, String)> {
    use std::sync::OnceLock;
    use regex::Regex;

    static LEGACY_RE: OnceLock<Regex> = OnceLock::new();
    static TOOL_CALL_RE: OnceLock<Regex> = OnceLock::new();
    static FN_RE: OnceLock<Regex> = OnceLock::new();
    static PARAM_RE: OnceLock<Regex> = OnceLock::new();

    let legacy_re = LEGACY_RE.get_or_init(|| {
        match Regex::new(r"Action:\s*(\w+)\[(.*?)\]") {
            Ok(re) => re,
            Err(e) => {
                tracing::error!("FATAL: Failed to compile legacy regex: {}", e);
                Regex::new("").unwrap_or_else(|_| unreachable!())
            }
        }
    });
    let tool_call_re = TOOL_CALL_RE.get_or_init(|| {
        match Regex::new(r"(?s)<tool_call>.*?</tool_call>") {
            Ok(re) => re,
            Err(e) => {
                tracing::error!("FATAL: Failed to compile tool_call regex: {}", e);
                Regex::new("").unwrap_or_else(|_| unreachable!())
            }
        }
    });
    let fn_re = FN_RE.get_or_init(|| {
        match Regex::new(r"<function=([\w_]+)>") {
            Ok(re) => re,
            Err(e) => {
                tracing::error!("FATAL: Failed to compile function regex: {}", e);
                Regex::new("").unwrap_or_else(|_| unreachable!())
            }
        }
    });
    let param_re = PARAM_RE.get_or_init(|| {
        match Regex::new(r"(?s)<parameter=([\w_]+)>(.*?)</parameter>") {
            Ok(re) => re,
            Err(e) => {
                tracing::error!("FATAL: Failed to compile parameter regex: {}", e);
                Regex::new("").unwrap_or_else(|_| unreachable!())
            }
        }
    });

    let mut actions = Vec::new();

    // 1. Legacy/Standard Parser: Action: name[args]
    for cap in legacy_re.captures_iter(text) {
        actions.push((cap[1].to_string(), cap[2].to_string()));
    }

    // 2. Emergent Substrate Parser (XML-like): <tool_call><function=name>...
    if text.contains("<tool_call>") {
        for tc_match in tool_call_re.find_iter(text) {
            let tc_text = tc_match.as_str();
            
            // Extract function name: <function=name>
            if let Some(fn_cap) = fn_re.captures(tc_text) {
                let fn_name = fn_cap[1].to_string();
                
                // Extract parameters: <parameter=key>value</parameter>
                let mut params = serde_json::Map::new();
                for p_cap in param_re.captures_iter(tc_text) {
                    let key = p_cap[1].to_string();
                    let val = p_cap[2].trim().to_string();
                    params.insert(key, serde_json::Value::String(val));
                }
                
                let args_json = serde_json::Value::Object(params).to_string();
                actions.push((fn_name, args_json));
            }
        }
    }

    actions
}

/// Parses a simple "Action: name[args]" pattern from LLM text (legacy helper).
pub fn parse_action(text: &str) -> Option<(String, String)> {
    parse_actions(text).into_iter().next()
}

/// Consolidates common error logging with agent context.
pub fn log_agent_error(agent_name: &str, context: &str, error: impl std::fmt::Display) {
    tracing::error!("[{}] {}: {}", agent_name, context, error);
}

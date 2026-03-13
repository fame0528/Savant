use bytes::Bytes;

/// Extracts text from a stream chunk, handling potential UTF-8 issues.
pub fn bytes_to_string(bytes: &Bytes) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Parses multiple "Action: name[args]" patterns from LLM text.
pub fn parse_actions(text: &str) -> Vec<(String, String)> {
    let re = regex::Regex::new(r"Action:\s*(\w+)\[(.*?)\]").expect("Static Action regex is valid");
    re.captures_iter(text)
        .map(|cap| (cap[1].to_string(), cap[2].to_string()))
        .collect()
}

/// Parses a simple "Action: name[args]" pattern from LLM text (legacy helper).
pub fn parse_action(text: &str) -> Option<(String, String)> {
    parse_actions(text).into_iter().next()
}

/// Consolidates common error logging with agent context.
pub fn log_agent_error(agent_name: &str, context: &str, error: impl std::fmt::Display) {
    tracing::error!("[{}] {}: {}", agent_name, context, error);
}

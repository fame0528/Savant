use bytes::Bytes;

/// Extracts text from a stream chunk, handling potential UTF-8 issues.
pub fn bytes_to_string(bytes: &Bytes) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Parses a simple "Action: name[args]" pattern from LLM text.
pub fn parse_action(text: &str) -> Option<(String, String)> {
    // Note: Re-using the regex logic from react.rs but centralized
    let re = regex::Regex::new(r"Action:\s*(\w+)\[(.*?)\]").ok()?;
    re.captures(text).map(|cap| (cap[1].to_string(), cap[2].to_string()))
}

/// Consolidates common error logging with agent context.
pub fn log_agent_error(agent_name: &str, context: &str, error: impl std::fmt::Display) {
    tracing::error!("[{}] {}: {}", agent_name, context, error);
}

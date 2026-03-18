//! Natural Language Command Parser
//!
//! Parses natural language commands into structured intents that can be
//! dispatched to the appropriate handler. Supports:
//!
//! - Agent management: "show me all agents", "restart agent X"
//! - Channel control: "restart the discord bot", "enable telegram"
//! - Model switching: "switch to hunter alpha", "use stepfun model"
//! - Diagnostics: "what's using the most memory?", "why did agent X fail?"
//! - Status: "show status", "system health"

pub mod commands;

use serde::{Deserialize, Serialize};

/// A parsed natural language command intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandIntent {
    /// The category of the command.
    pub category: CommandCategory,
    /// The specific action to take.
    pub action: String,
    /// The target (agent name, channel name, model name, etc.)
    pub target: Option<String>,
    /// Additional parameters extracted from the command.
    pub params: std::collections::HashMap<String, String>,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
    /// The original input text.
    pub original: String,
}

/// Command categories for routing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandCategory {
    /// List or manage agents.
    AgentManagement,
    /// Manage channels (Discord, Telegram, etc.).
    ChannelControl,
    /// Switch model or provider.
    ModelSwitch,
    /// Diagnostics and health checks.
    Diagnostics,
    /// General status information.
    Status,
    /// Help or documentation.
    Help,
    /// Unrecognized command.
    Unknown,
}

/// Parses a natural language string into a CommandIntent.
pub fn parse_command(input: &str) -> CommandIntent {
    let lower = input.to_lowercase().trim().to_string();

    // Try each parser in order of specificity
    if let Some(intent) = parse_agent_command(&lower, input) {
        return intent;
    }
    if let Some(intent) = parse_channel_command(&lower, input) {
        return intent;
    }
    if let Some(intent) = parse_model_command(&lower, input) {
        return intent;
    }
    if let Some(intent) = parse_diagnostics_command(&lower, input) {
        return intent;
    }
    if let Some(intent) = parse_status_command(&lower, input) {
        return intent;
    }
    if let Some(intent) = parse_help_command(&lower, input) {
        return intent;
    }

    CommandIntent {
        category: CommandCategory::Unknown,
        action: "unknown".to_string(),
        target: None,
        params: Default::default(),
        confidence: 0.0,
        original: input.to_string(),
    }
}

fn parse_agent_command(lower: &str, original: &str) -> Option<CommandIntent> {
    if lower.contains("show") && (lower.contains("agent") || lower.contains("all agent")) {
        return Some(CommandIntent {
            category: CommandCategory::AgentManagement,
            action: "list".to_string(),
            target: None,
            params: Default::default(),
            confidence: 0.9,
            original: original.to_string(),
        });
    }

    if lower.contains("restart") && lower.contains("agent") {
        let target = extract_after(lower, "agent");
        return Some(CommandIntent {
            category: CommandCategory::AgentManagement,
            action: "restart".to_string(),
            target,
            params: Default::default(),
            confidence: 0.85,
            original: original.to_string(),
        });
    }

    None
}

fn parse_channel_command(lower: &str, original: &str) -> Option<CommandIntent> {
    let channels = ["discord", "telegram", "whatsapp", "matrix"];

    for channel in &channels {
        if lower.contains(channel) {
            if lower.contains("restart") || lower.contains("enable") || lower.contains("start") {
                return Some(CommandIntent {
                    category: CommandCategory::ChannelControl,
                    action: "restart".to_string(),
                    target: Some(channel.to_string()),
                    params: Default::default(),
                    confidence: 0.9,
                    original: original.to_string(),
                });
            }
            if lower.contains("stop") || lower.contains("disable") {
                return Some(CommandIntent {
                    category: CommandCategory::ChannelControl,
                    action: "stop".to_string(),
                    target: Some(channel.to_string()),
                    params: Default::default(),
                    confidence: 0.9,
                    original: original.to_string(),
                });
            }
        }
    }

    None
}

fn parse_model_command(lower: &str, original: &str) -> Option<CommandIntent> {
    if lower.contains("switch to") || lower.contains("use") || lower.contains("change model") {
        // Try to extract model name
        let models = [
            ("hunter alpha", "openrouter/hunter-alpha"),
            ("healer alpha", "openrouter/healer-alpha"),
            ("stepfun", "stepfun/step-3.5-flash:free"),
            ("free router", "openrouter/free"),
            ("openrouter/free", "openrouter/free"),
        ];

        for (keyword, model_id) in &models {
            if lower.contains(keyword) {
                return Some(CommandIntent {
                    category: CommandCategory::ModelSwitch,
                    action: "switch".to_string(),
                    target: Some(model_id.to_string()),
                    params: Default::default(),
                    confidence: 0.9,
                    original: original.to_string(),
                });
            }
        }
    }

    None
}

fn parse_diagnostics_command(lower: &str, original: &str) -> Option<CommandIntent> {
    if lower.contains("memory") && (lower.contains("using") || lower.contains("most")) {
        return Some(CommandIntent {
            category: CommandCategory::Diagnostics,
            action: "memory_usage".to_string(),
            target: None,
            params: Default::default(),
            confidence: 0.8,
            original: original.to_string(),
        });
    }

    if lower.contains("why") && lower.contains("fail") {
        let target = extract_after_word(lower, &["agent", "fail"]);
        return Some(CommandIntent {
            category: CommandCategory::Diagnostics,
            action: "failure_reason".to_string(),
            target,
            params: Default::default(),
            confidence: 0.8,
            original: original.to_string(),
        });
    }

    None
}

fn parse_status_command(lower: &str, original: &str) -> Option<CommandIntent> {
    if lower.contains("status") || lower.contains("health") || lower.contains("how are you") {
        return Some(CommandIntent {
            category: CommandCategory::Status,
            action: "status".to_string(),
            target: None,
            params: Default::default(),
            confidence: 0.85,
            original: original.to_string(),
        });
    }

    None
}

fn parse_help_command(lower: &str, original: &str) -> Option<CommandIntent> {
    if lower.contains("help") || lower.contains("what can you do") || lower.contains("commands") {
        return Some(CommandIntent {
            category: CommandCategory::Help,
            action: "help".to_string(),
            target: None,
            params: Default::default(),
            confidence: 0.9,
            original: original.to_string(),
        });
    }

    None
}

/// Extract text after a keyword.
fn extract_after(input: &str, keyword: &str) -> Option<String> {
    if let Some(pos) = input.find(keyword) {
        let after = &input[pos + keyword.len()..].trim();
        if !after.is_empty() {
            return Some(after.to_string());
        }
    }
    None
}

/// Extract text after any of the given keywords.
fn extract_after_word(input: &str, keywords: &[&str]) -> Option<String> {
    for kw in keywords {
        if let Some(result) = extract_after(input, kw) {
            return Some(result);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_list_agents() {
        let intent = parse_command("show me all agents");
        assert_eq!(intent.category, CommandCategory::AgentManagement);
        assert_eq!(intent.action, "list");
        assert!(intent.confidence > 0.8);
    }

    #[test]
    fn test_parse_restart_discord() {
        let intent = parse_command("restart the discord bot");
        assert_eq!(intent.category, CommandCategory::ChannelControl);
        assert_eq!(intent.action, "restart");
        assert_eq!(intent.target, Some("discord".to_string()));
    }

    #[test]
    fn test_parse_switch_model() {
        let intent = parse_command("switch to hunter alpha");
        assert_eq!(intent.category, CommandCategory::ModelSwitch);
        assert_eq!(intent.action, "switch");
        assert_eq!(intent.target, Some("openrouter/hunter-alpha".to_string()));
    }

    #[test]
    fn test_parse_stop_telegram() {
        let intent = parse_command("disable telegram");
        assert_eq!(intent.category, CommandCategory::ChannelControl);
        assert_eq!(intent.action, "stop");
        assert_eq!(intent.target, Some("telegram".to_string()));
    }

    #[test]
    fn test_parse_status() {
        let intent = parse_command("show status");
        assert_eq!(intent.category, CommandCategory::Status);
        assert_eq!(intent.action, "status");
    }

    #[test]
    fn test_parse_unknown() {
        let intent = parse_command("do something random with the flargle");
        assert_eq!(intent.category, CommandCategory::Unknown);
        assert!(intent.confidence < 0.1);
    }

    #[test]
    fn test_parse_help() {
        let intent = parse_command("what can you do");
        assert_eq!(intent.category, CommandCategory::Help);
        assert_eq!(intent.action, "help");
    }

    #[test]
    fn test_parse_memory() {
        let intent = parse_command("what's using the most memory");
        assert_eq!(intent.category, CommandCategory::Diagnostics);
        assert_eq!(intent.action, "memory_usage");
    }

    #[test]
    fn test_parse_restart_agent() {
        let intent = parse_command("restart agent alpha");
        assert_eq!(intent.category, CommandCategory::AgentManagement);
        assert_eq!(intent.action, "restart");
        assert_eq!(intent.target, Some("alpha".to_string()));
    }
}

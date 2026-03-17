use crate::budget::TokenBudget;
use savant_core::types::{AgentIdentity, ChatMessage, ChatRole};

/// Assembler struct used to construct LLM prompts with token limits in mind.
pub struct ContextAssembler {
    identity: AgentIdentity,
    budget: TokenBudget,
    skills_list: Option<String>,
}

impl ContextAssembler {
    /// Creates a new ContextAssembler.
    pub fn new(identity: AgentIdentity, budget: TokenBudget, skills_list: Option<String>) -> Self {
        Self {
            identity,
            budget,
            skills_list,
        }
    }

    /// Assembles the full system prompt from identity components (OpenClaw style).
    pub fn assemble_system_prompt(&self) -> String {
        let mut prompt = String::new();

        // Operational Preamble: Establish agent role and capabilities
        prompt.push_str("You are the SAVANT agent, a Rust-native autonomous assistant. \
            You have access to tools for filesystem operations (read, write, move, delete), \
            shell commands, and memory management. Use these tools to accomplish tasks. \
            When a user requests an action, use the appropriate tool with the correct format. \
            Always prefer using tools over refusing tasks within your capability scope.\n\n");

        // 1. Identity & Vibe (IDENTITY.md)
        if let Some(metadata) = &self.identity.metadata {
            prompt.push_str(&format!("IDENTITY INFO:\n{}\n\n", metadata));
        }

        // 2. Persona & Core (SOUL.md)
        prompt.push_str(&format!("PERSONA (SOUL):\n{}\n\n", self.identity.soul));

        // 3. Operating Instructions (AGENTS.md)
        if let Some(instructions) = &self.identity.instructions {
            prompt.push_str(&format!("OPERATING INSTRUCTIONS:\n{}\n\n", instructions));
        }

        // 4. User context (USER.md)
        if let Some(user) = &self.identity.user_context {
            prompt.push_str(&format!("USER CONTEXT:\n{}\n\n", user));
        }

        if let Some(mission) = &self.identity.mission {
            prompt.push_str(&format!("MISSION:\n{}\n\n", mission));
        }

        if let Some(ethics) = &self.identity.ethics {
            prompt.push_str(&format!("ETHICS & CONSTRAINTS:\n{}\n\n", ethics));
        }

        prompt.push_str(&format!(
            "OPERATIONAL LIMITS:\n- Token Budget: {} / {}\n\n",
            self.budget.used, self.budget.limit
        ));

        if let Some(skills) = &self.skills_list {
            prompt.push_str(&format!("AVAILABLE TOOLS:\n{}\n\n", skills));
            prompt.push_str("TOOL USAGE FORMAT:\n");
            prompt.push_str("To call a tool, use this exact format in your response:\n");
            prompt.push_str("Action: tool_name{\"key\": \"value\"}\n\n");
            prompt.push_str("Examples:\n");
            prompt.push_str("  Action: foundation{\"action\": \"read\", \"path\": \"src/main.rs\"}\n");
            prompt.push_str("  Action: foundation{\"action\": \"ls\", \"path\": \".\"}\n");
            prompt.push_str("  Action: file_create{\"path\": \"new_file.txt\", \"content\": \"Hello world\"}\n");
            prompt.push_str("  Action: file_move{\"from\": \"old.txt\", \"to\": \"new.txt\"}\n");
            prompt.push_str("  Action: file_delete{\"path\": \"tmp/old.log\"}\n");
            prompt.push_str("  Action: file_atomic_edit{\"path\": \"src/lib.rs\", \"replacements\": [{\"target\": \"old\", \"value\": \"new}]}\n");
            prompt.push_str("  Action: shell{\"command\": \"cargo check\"}\n\n");
        }

        if !self.identity.expertise.is_empty() {
            prompt.push_str("EXPERTISE:\n");
            for skill in &self.identity.expertise {
                prompt.push_str(&format!("- {}\n", skill));
            }
            prompt.push('\n');
        }

        // 5. Global Constraints
        prompt.push_str("CRITICAL: YOUR RESPONSE MUST BE IN ENGLISH ONLY. DO NOT USE ANY OTHER LANGUAGE.\n\n");

        prompt
    }

    /// Converts the conversation history and memory into ChatMessages.
    pub fn build_messages(&self, history: Vec<ChatMessage>) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        messages.push(ChatMessage {
            role: ChatRole::System,
            content: self.assemble_system_prompt(),
            sender: Some("SYSTEM".to_string()),
            recipient: None,
            agent_id: None,
            session_id: None,
            channel: savant_core::types::AgentOutputChannel::Chat,
        });

        for msg in history {
            // AAA: Channel Isolation - filter to only feed primary dialogue or relevant context
            // Recall Protection avoids feeding background telemetry or noise into the context window.
            if msg.channel == savant_core::types::AgentOutputChannel::Chat || 
               msg.channel == savant_core::types::AgentOutputChannel::Memory {
                messages.push(msg);
            }
        }

        messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use savant_core::types::AgentIdentity;

    #[test]
    fn test_assemble_system_prompt() {
        let identity = AgentIdentity {
            name: "TestAgent".to_string(),
            soul: "Vibe check.".to_string(),
            instructions: Some("Do stuff.".to_string()),
            user_context: None,
            metadata: Some("Emoji: 🤖".to_string()),
            mission: None,
            expertise: vec!["Rust".to_string()],
            ethics: None,
            image: None,
        };
        let budget = TokenBudget::new(100);
        let assembler = ContextAssembler::new(identity, budget, None);
        let prompt = assembler.assemble_system_prompt();

        assert!(prompt.contains("Vibe check."));
        assert!(prompt.contains("Do stuff."));
        assert!(prompt.contains("🤖"));
        assert!(prompt.contains("Rust"));
    }
}

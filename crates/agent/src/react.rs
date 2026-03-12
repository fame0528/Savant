use std::sync::Arc;
use savant_core::types::{ChatMessage, ChatRole};
use savant_core::traits::SkillExecutor;
use savant_core::utils::parsing;
use crate::providers::LlmProvider;
use crate::context::ContextAssembler;
use crate::budget::TokenBudget;
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;

/// Enum representing distinct agent loop events.
pub enum AgentEvent {
    Thought(String),
    Action { name: String, args: String },
    Observation(String),
    FinalAnswer(String),
    Reflection(String),
}

/// The ReAct execution loop structure.
pub struct ReActLoop {
    agent_id: String,
    provider: Box<dyn LlmProvider>,
    tools: Vec<Arc<dyn SkillExecutor>>,
    context: ContextAssembler,
    budget: TokenBudget,
    memory: crate::memory::MemoryManager,
}

impl ReActLoop {
    /// Constructs a new ReActLoop.
    pub fn new(
        agent_id: String,
        provider: Box<dyn LlmProvider>, 
        tools: Vec<Arc<dyn SkillExecutor>>, 
        context: ContextAssembler,
        budget: TokenBudget,
        memory: crate::memory::MemoryManager,
    ) -> Self {
        Self { agent_id, provider, tools, context, budget, memory }
    }

    pub async fn consolidate_memory(&self) -> Result<(), savant_core::error::SavantError> {
        self.memory.consolidate(&self.agent_id).await
    }

    /// Runs the loop with an initial event, returning a stream of agent events.
    pub fn run(&mut self, user_input: String) -> Pin<Box<dyn Stream<Item = Result<AgentEvent, String>> + Send + '_>> {
        let mut history = vec![ChatMessage {
            role: ChatRole::User,
            content: user_input.clone(),
            sender: Some("USER".to_string()),
            recipient: None,
            agent_id: Some(self.agent_id.clone()),
        }];
        let agent_id = self.agent_id.clone();

        Box::pin({
            use async_stream::stream;
            stream! {
                let mut depth = 0;
                const MAX_DEPTH: u32 = 5;

                while depth < MAX_DEPTH {
                    if self.budget.used >= self.budget.limit {
                        yield Err("Token budget exhausted".to_string());
                        return;
                    }

                    let messages = self.context.build_messages(history.clone());
                    let response_stream = self.provider.stream_completion(messages).await;

                    let mut full_text = String::new();
                    let llm_stream = match response_stream {
                        Ok(s) => s,
                        Err(e) => {
                            yield Err(e.to_string());
                            return;
                        }
                    };

                    let stream = crate::streaming::parse_llm_stream(llm_stream);
                    let mut stream = Box::pin(stream);

                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(AgentEvent::Thought(text)) => {
                                full_text.push_str(&text);
                                yield Ok(AgentEvent::Thought(text));
                            }
                            Ok(_) => {}
                            Err(e) => {
                                yield Err(e.to_string());
                                return;
                            }
                        }
                    }

                    // Check for actions using centralized utility
                    if let Some((name, args)) = parsing::parse_action(&full_text) {
                        yield Ok(AgentEvent::Action { name: name.clone(), args: args.clone() });
                        
                        match self.execute_tool(&name, &args).await {
                            Ok(obs) => {
                                yield Ok(AgentEvent::Observation(obs.clone()));
                                history.push(ChatMessage { role: ChatRole::Assistant, content: full_text.clone(), sender: Some(self.agent_id.clone()), recipient: None, agent_id: Some(self.agent_id.clone()) });
                                history.push(ChatMessage { role: ChatRole::User, content: format!("Observation: {}", obs), sender: Some("SYSTEM".to_string()), recipient: None, agent_id: Some(self.agent_id.clone()) });
                            }
                            Err(e) => {
                                yield Err(e);
                                return;
                            }
                        }
                    } else {
                        let answer = self.parse_final_answer(&full_text);
                        yield Ok(AgentEvent::FinalAnswer(answer));
                        
                        // Autonomous Reflection Trigger
                        let mut ref_history = history.clone();
                        ref_history.push(ChatMessage { role: ChatRole::Assistant, content: full_text.clone(), sender: Some(self.agent_id.clone()), recipient: None, agent_id: Some(self.agent_id.clone()) });
                        ref_history.push(ChatMessage { role: ChatRole::User, content: "Reflect on this task. What did we learn?".to_string(), sender: Some("SYSTEM".to_string()), recipient: None, agent_id: Some(self.agent_id.clone()) });
                        
                        let ref_messages = self.context.build_messages(ref_history);
                        let ref_response = self.provider.stream_completion(ref_messages).await;
                        
                        let mut ref_text = String::new();
                        if let Ok(llm_stream) = ref_response {
                            let mut rs = Box::pin(crate::streaming::parse_llm_stream(llm_stream));
                            while let Some(event) = rs.next().await {
                                if let Ok(AgentEvent::Thought(text)) = event {
                                    ref_text.push_str(&text);
                                }
                            }
                        }
                        
                        yield Ok(AgentEvent::Reflection(ref_text.clone()));
                        
                        // Persist the reflection
                        let _ = self.memory.record_learning(&agent_id, &ref_text).await;

                        break;
                    }
                    depth += 1;
                }
            }
        })
    }

    fn parse_final_answer(&self, text: &str) -> String {
        // Return raw text to allow natural conversational flow
        text.to_string()
    }

    async fn execute_tool(&self, name: &str, args: &str) -> Result<String, String> {
        for tool in &self.tools {
            tracing::info!("Executing tool: {}", name);
            return tool.execute(args).await.map_err(|e| e.to_string());
        }
        Err(format!("Tool not found: {}", name))
    }
}

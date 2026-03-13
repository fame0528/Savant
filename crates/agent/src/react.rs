use crate::budget::TokenBudget;
use crate::context::ContextAssembler;
use futures::stream::{Stream, StreamExt};
use savant_core::error::SavantError;
use savant_core::traits::{LlmProvider, MemoryBackend, Tool};
use savant_core::types::{AgentIdentity, ChatMessage, ChatRole};
use savant_core::utils::parsing;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{debug, info, warn};
use savant_cognitive::DspPredictor;
use savant_echo::{HotSwappableRegistry, ComponentMetrics};
use savant_ipc::{CollectiveBlackboard, ConsensusResult};
use crate::plugins::WasmToolHost;

/// Enum representing distinct agent loop events.
pub enum AgentEvent {
    Thought(String),
    Action { name: String, args: String },
    Observation(String),
    FinalAnswer(String),
    Reflection(String),
}

pub struct AgentLoop<M: MemoryBackend> {
    pub(crate) agent_id: String,
    pub(crate) agent_id_hash: u64,
    pub(crate) provider: Box<dyn LlmProvider>,
    pub(crate) memory: M,
    pub(crate) tools: Vec<Arc<dyn Tool>>,
    pub(crate) context: ContextAssembler,
    pub(crate) plugin_host: Option<Arc<crate::plugins::WasmPluginHost>>,
    pub(crate) plugins: Vec<wasmtime::component::Component>,
    pub(crate) security_token: Option<savant_security::AgentToken>,
    pub(crate) predictor: DspPredictor,
    pub(crate) echo_registry: Option<Arc<HotSwappableRegistry>>,
    pub(crate) echo_metrics: Option<Arc<ComponentMetrics>>,
    pub(crate) echo_host: Option<Arc<WasmToolHost>>,
    pub(crate) collective_blackboard: Option<Arc<CollectiveBlackboard>>,
    pub(crate) agent_index: u8,
}


impl<M: MemoryBackend> AgentLoop<M> {
    /// Constructs a new AgentLoop.
    pub fn new(
        agent_id: String,
        provider: Box<dyn LlmProvider>,
        memory: M,
        tools: Vec<Arc<dyn Tool>>,
        identity: AgentIdentity,
    ) -> Self {
        let mut skills_summary = String::from("Available Tools:\n");
        for tool in &tools {
            skills_summary.push_str(&format!("- {}: {}\n", tool.name(), tool.description()));
        }
        let skills_list = if tools.is_empty() {
            None
        } else {
            Some(skills_summary)
        };

        let agent_id_hash = xxhash_rust::xxh3::xxh3_64(agent_id.as_bytes());

        Self {
            agent_id,
            agent_id_hash,
            provider,
            memory,
            tools,
            context: ContextAssembler::new(identity, TokenBudget::new(8192), skills_list),
            plugin_host: None,
            plugins: Vec::new(),
            security_token: None,
            predictor: DspPredictor::default(),
            echo_registry: None,
            echo_metrics: None,
            echo_host: None,
            collective_blackboard: None,
            agent_index: 0,
        }
    }

    /// Sets the plugin host and loaded plugins for this agent.
    pub fn with_plugins(
        mut self, 
        host: Arc<crate::plugins::WasmPluginHost>, 
        plugins: Vec<wasmtime::component::Component>,
        token: Option<savant_security::AgentToken>,
    ) -> Self {
        self.plugin_host = Some(host);
        self.plugins = plugins;
        self.security_token = token;
        self
    }

    /// Sets the ECHO registry and metrics for this agent.
    pub fn with_echo(
        mut self,
        registry: Arc<HotSwappableRegistry>,
        metrics: Arc<ComponentMetrics>,
        host: Arc<WasmToolHost>,
    ) -> Self {
        self.echo_registry = Some(registry);
        self.echo_metrics = Some(metrics);
        self.echo_host = Some(host);
        self
    }

    /// Sets the collective blackboard and agent index.
    pub fn with_collective(
        mut self,
        blackboard: Arc<CollectiveBlackboard>,
        index: u8,
    ) -> Self {
        self.collective_blackboard = Some(blackboard);
        self.agent_index = index;
        self
    }

    /// Primary execution cycle of the agent.
    pub fn run(
        &mut self,
        user_input: String,
    ) -> Pin<Box<dyn Stream<Item = Result<AgentEvent, SavantError>> + Send + '_>> {
        let mut history = vec![ChatMessage {
            role: ChatRole::User,
            content: user_input.clone(),
            sender: Some("USER".to_string()),
            recipient: None,
            agent_id: Some(self.agent_id.clone()),
        }];

        Box::pin({
            use async_stream::stream;
            stream! {
                let mut depth = 0;
                const MAX_DEPTH: u32 = 8; // OpenClaw parity depth

                while depth < MAX_DEPTH {
                    // 0. Dynamic Speculation Depth Prediction (DSP)
                    // Complexity heuristic: based on history length and current depth
                    let complexity = (history.len() as f32 * 0.5) + (depth as f32);
                    let k = self.predictor.predict_optimal_k(complexity);
                    debug!("DSP Prediction: k={} for complexity={}", k, complexity);

                    // 1. Context Assembly (Retrieval + System Prompt)
                    let session_context = self.memory.retrieve(&self.agent_id, &user_input, 10).await?;
                    let mut current_history = session_context;
                    current_history.extend(history.clone());

                    let mut messages = self.context.build_messages(current_history);

                    // 1.1 WASM Hook: before_llm_call
                    if let Some(host) = &self.plugin_host {
                        for plugin in &self.plugins {
                            // Combine message content for processing
                            let mut combined_prompt = String::new();
                            for msg in &messages {
                                combined_prompt.push_str(&msg.content);
                            }

                            match host.execute_before_llm_call(
                                plugin, 
                                &combined_prompt,
                                self.agent_id_hash,
                                self.security_token.clone(),
                            ).await {
                                Ok(crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Modified(new_prompt)) => {
                                    info!("Plugin modified prompt for agent [{}]", self.agent_id);
                                    // Simplified modification: replace last message content
                                    if let Some(last) = messages.last_mut() {
                                        last.content = new_prompt;
                                    }
                                }
                                Ok(crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Halt(reason)) => {
                                    warn!("Plugin halted execution for agent [{}]: {}", self.agent_id, reason);
                                    yield Err(SavantError::Unknown(format!("Halted by plugin: {}", reason)));
                                    return;
                                }
                                _ => {}
                            }
                        }
                    }

                    // 2. Model Inference
                    let response_stream = self.provider.stream_completion(messages).await;
                    let mut full_text = String::new();

                    let mut llm_stream = match response_stream {
                        Ok(s) => s,
                        Err(e) => {
                            yield Err(e);
                            return;
                        }
                    };

                    while let Some(chunk_res) = llm_stream.next().await {
                        match chunk_res {
                            Ok(chunk) => {
                                if !chunk.content.is_empty() {
                                    full_text.push_str(&chunk.content);
                                    yield Ok(AgentEvent::Thought(chunk.content));
                                }
                                if chunk.is_final { break; }
                            }
                            Err(e) => {
                                yield Err(e);
                                return;
                            }
                        }
                    }

                    // 3. Tool Execution / Action Parsing (Speculative Chaining)
                    let actions = parsing::parse_actions(&full_text);
                    let mut actual_steps = 0;

                    if !actions.is_empty() {
                        let dag = crate::orchestration::dag::parse_sequential_dag(actions);
                        
                        history.push(ChatMessage {
                            role: ChatRole::Assistant,
                            content: full_text.clone(),
                            sender: Some(self.agent_id.clone()),
                            recipient: None,
                            agent_id: Some(self.agent_id.clone())
                        });

                        for node in dag.nodes {
                            actual_steps += 1;
                            yield Ok(AgentEvent::Action { name: node.name.clone(), args: node.args.clone() });

                            match self.execute_tool(&node.name, &node.args).await {
                                Ok(mut obs) => {
                                    // WASM Hook: after_tool_call
                                    if let Some(host) = &self.plugin_host {
                                        for plugin in &self.plugins {
                                            match host.execute_after_tool_call(
                                                plugin, 
                                                &node.name, 
                                                &obs,
                                                self.agent_id_hash,
                                                self.security_token.clone(),
                                            ).await {
                                                Ok(crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Modified(new_obs)) => {
                                                    obs = new_obs;
                                                }
                                                Ok(crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Halt(reason)) => {
                                                    warn!("Plugin halted execution for agent [{}]: {}", self.agent_id, reason);
                                                    yield Err(SavantError::Unknown(format!("Halted by plugin: {}", reason)));
                                                    return;
                                                }
                                                _ => {}
                                            }
                                        }
                                    }

                                    yield Ok(AgentEvent::Observation(obs.clone()));
                                    history.push(ChatMessage {
                                        role: ChatRole::User,
                                        content: format!("Observation ({}): {}", node.name, obs),
                                        sender: Some("SYSTEM".to_string()),
                                        recipient: None,
                                        agent_id: Some(self.agent_id.clone())
                                    });
                                }
                                Err(e) => {
                                    warn!("Tool execution failed for [{}]: {}", node.name, e);
                                    yield Err(SavantError::Unknown(e.to_string()));
                                    return;
                                }
                            }
                        }

                        // Update DSP accuracy (post-hoc)
                        self.predictor.update_accuracy(k, actual_steps);
                        if self.predictor.prediction_count() % 5 == 0 {
                            self.predictor.adapt_parameters();
                        }
                    } else {
                        let mut final_response = full_text.clone();

                        // WASM Hook: before_response_emit
                        if let Some(host) = &self.plugin_host {
                            for plugin in &self.plugins {
                                match host.execute_before_response_emit(
                                    plugin, 
                                    &final_response,
                                    self.agent_id_hash,
                                    self.security_token.clone(),
                                ).await {
                                    Ok(crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Modified(new_resp)) => {
                                        final_response = new_resp;
                                    }
                                    Ok(crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Halt(reason)) => {
                                        yield Err(SavantError::Unknown(format!("Halted by plugin: {}", reason)));
                                        return;
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // 4. Final Answer Transition
                        yield Ok(AgentEvent::FinalAnswer(final_response.clone()));

                        // Autonomous Reflection
                        let reflection = self.generate_reflection(&history, &final_response).await?;
                        
                        // Propagate Insight to Collective Intelligence
                        if let Some(collective) = &self.collective_blackboard {
                            if let Ok(mut state) = collective.read_state() {
                                state.heuristic_version = state.heuristic_version.wrapping_add(1);
                                if let Err(e) = collective.publish_state(state) {
                                    warn!("Failed to propagate global insight: {}", e);
                                }
                            }
                        }

                        yield Ok(AgentEvent::Reflection(reflection.clone()));

                        // Persist to MemoryBackend
                        let final_msg = ChatMessage {
                            role: ChatRole::Assistant,
                            content: final_response,
                            sender: Some(self.agent_id.clone()),
                            recipient: None,
                            agent_id: Some(self.agent_id.clone())
                        };
                        self.memory.store(&self.agent_id, &final_msg).await?;

                        break;
                    }
                    depth += 1;
                }
            }
        })
    }

    pub(crate) async fn execute_tool(&self, name: &str, args: &str) -> Result<String, SavantError> {
        // --- Collective Intelligence Guard: Swarm Consensus ---
        // If the tool is potentially destructive, require multi-agent approval.
        let destructive_tools = ["delete_file", "overwrite_file", "format_disk", "terminate_swarm"];
        if destructive_tools.iter().any(|&t| t == name.to_lowercase()) {
            if let Some(collective) = &self.collective_blackboard {
                info!("Agent [{}] requesting consensus for destructive action: {}", self.agent_id, name);
                
                // Initialize consensus proposal
                let mut state = collective.read_state().map_err(|e| SavantError::Unknown(e.to_string()))?;
                state.active_proposal_hash = xxhash_rust::xxh3::xxh3_64(args.as_bytes());
                state.proposal_type = 1; // Destructive
                state.approve_mask = [0; 2];
                state.veto_mask = [0; 2];
                // Auto-approve own proposal
                let mask_idx = (self.agent_index / 64) as usize;
                state.approve_mask[mask_idx] |= 1 << (self.agent_index % 64);
                
                collective.publish_state(state).map_err(|e| SavantError::Unknown(e.to_string()))?;
                
                for i in 0..10 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    match collective.check_consensus() {
                        ConsensusResult::Approved => {
                            info!("Swarm Consensus Reached. Proceeding with destructive action: {}", name);
                            break;
                        }
                        ConsensusResult::Vetoed => {
                            return Err(SavantError::Unknown(format!("Action VETOED by swarm: {}", name)));
                        }
                        ConsensusResult::Pending => {
                            debug!("Waiting for swarm consensus on {}...", name);
                        }
                    }
                    if i == 9 {
                        return Err(SavantError::Unknown(format!("Consensus TIMEOUT for action: {}", name)));
                    }
                }
            }
        }

        for tool in &self.tools {
            if tool.name().to_lowercase() == name.to_lowercase() {
                tracing::info!("Agent [{}] executing tool: {}", self.agent_id, name);

                // Attempt to parse args as JSON, fallback to manual object for raw strings
                let payload = serde_json::from_str(args)
                    .unwrap_or_else(|_| serde_json::json!({ "payload": args }));

                return tool.execute(payload).await;
            }
        }

        // 2. Check ECHO Registry for dynamically compiled WASM tools
        if let (Some(registry), Some(host)) = (&self.echo_registry, &self.echo_host) {
            if let Some(capability) = registry.get_tool(name) {
                tracing::info!("Agent [{}] executing ECHO tool: {}", self.agent_id, name);
                
                // Real implementation: instantiate and execute the module
                match host.execute_tool(&capability.module, args).await {
                    Ok(res) => {
                        // Record metrics for Circuit Breaker
                        if let Some(metrics) = &self.echo_metrics {
                            metrics.record_outcome(true);
                        }
                        return Ok(res);
                    }
                    Err(e) => {
                        warn!("ECHO Tool execution failed: {}", e);
                        if let Some(metrics) = &self.echo_metrics {
                            metrics.record_outcome(false);
                        }
                        return Err(SavantError::Unknown(e.to_string()));
                    }
                }
            }
        }

        Err(SavantError::Unknown(format!("Tool not found: {}", name)))
    }

    pub(crate) async fn generate_reflection(
        &self,
        history: &[ChatMessage],
        last_answer: &str,
    ) -> Result<String, SavantError> {
        let mut ref_history = history.to_vec();
        ref_history.push(ChatMessage {
            role: ChatRole::Assistant,
            content: last_answer.to_string(),
            sender: Some(self.agent_id.clone()),
            recipient: None,
            agent_id: Some(self.agent_id.clone()),
        });

        let messages = self.context.build_messages(ref_history);
        // Note: We might want a specific reflection system prompt here in the future

        let mut stream = self.provider.stream_completion(messages).await?;
        let mut reflection = String::new();
        while let Some(chunk_res) = stream.next().await {
            if let Ok(chunk) = chunk_res {
                reflection.push_str(&chunk.content);
                if chunk.is_final {
                    break;
                }
            }
        }
        Ok(reflection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use savant_core::types::ChatMessage;
    use async_trait::async_trait;
    use serde_json::Value;
    use tokio::sync::Mutex;

    struct MockTool {
        name: String,
        executed: Arc<Mutex<u32>>,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str { &self.name }
        fn description(&self) -> &str { "Mock" }
        async fn execute(&self, _args: Value) -> Result<String, SavantError> {
            let mut count = self.executed.lock().await;
            *count += 1;
            Ok(format!("{} executed", self.name))
        }
    }

    struct MockLlm {
        responses: Vec<String>,
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait]
    impl LlmProvider for MockLlm {
        async fn stream_completion(&self, _messages: Vec<ChatMessage>) -> Result<Pin<Box<dyn Stream<Item = Result<savant_core::types::ChatChunk, SavantError>> + Send>>, SavantError> {
            let mut count = self.call_count.lock().await;
            let response = if *count < self.responses.len() {
                self.responses[*count].clone()
            } else {
                "Final Answer: Done".to_string()
            };
            *count += 1;
            let chunk = savant_core::types::ChatChunk {
                agent_name: "test".to_string(),
                agent_id: "test".to_string(),
                content: response,
                is_final: true,
            };
            Ok(Box::pin(futures::stream::iter(vec![Ok(chunk)])))
        }
    }

    struct MockMemory;
    #[async_trait]
    impl MemoryBackend for MockMemory {
        async fn store(&self, _agent_id: &str, _msg: &ChatMessage) -> Result<(), SavantError> { Ok(()) }
        async fn retrieve(&self, _agent_id: &str, _query: &str, _limit: usize) -> Result<Vec<ChatMessage>, SavantError> { Ok(vec![]) }
        async fn consolidate(&self, _agent_id: &str) -> Result<(), SavantError> { Ok(()) }
    }

    #[tokio::test]
    async fn test_speculative_chaining() {
        let executed_count = Arc::new(Mutex::new(0));
        let tool1 = Arc::new(MockTool { name: "Tool1".into(), executed: executed_count.clone() });
        let tool2 = Arc::new(MockTool { name: "Tool2".into(), executed: executed_count.clone() });
        
        let provider = Box::new(MockLlm {
            responses: vec!["Thought: Doing two things.\nAction: Tool1[arg1]\nAction: Tool2[arg2]".to_string()],
            call_count: Arc::new(Mutex::new(0)),
        });

        let mut agent = AgentLoop::new(
            "test_agent".into(),
            provider,
            MockMemory,
            vec![tool1, tool2],
            AgentIdentity::default(),
        );

        let mut stream = agent.run("start".into());
        while let Some(res) = stream.next().await {
            if let Err(e) = res {
                panic!("Stream error: {:?}", e);
            }
        }

        // Verify that BOTH tools were executed
        let count = executed_count.lock().await;
        assert_eq!(*count, 2, "Both tools in the speculative chain should have executed");
    }
}

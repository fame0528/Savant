use crate::budget::TokenBudget;
use crate::context::ContextAssembler;
// Redundant import removed
use savant_core::error::SavantError;
use savant_core::traits::{LlmProvider, MemoryBackend, Tool};
use savant_core::types::{AgentIdentity, ChatMessage, ChatRole};
use savant_core::utils::parsing;
use std::pin::Pin;
use std::sync::Arc;
use std::collections::HashSet;
use tracing::{debug, info, warn};
use futures::stream::{Stream, StreamExt, FuturesUnordered};
use tokio_util::sync::CancellationToken;
use savant_cognitive::DspPredictor;
use savant_echo::{HotSwappableRegistry, ComponentMetrics};
use savant_ipc::CollectiveBlackboard;
use crate::plugins::WasmToolHost;

/// Enum representing distinct agent loop events.
pub enum AgentEvent {
    Thought(String),
    Action { name: String, args: String },
    Observation(String),
    FinalAnswer(String),
    FinalAnswerChunk(String),
    Reflection(String),
    StatusUpdate(String), // New: Internal status heartbeats
}

pub struct AgentLoop<M: MemoryBackend> {
    pub(crate) agent_id: String,
    pub(crate) agent_id_hash: u64,
    pub(crate) provider: Box<dyn LlmProvider>,
    pub(crate) fallback_provider: Option<Box<dyn LlmProvider>>,
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
    pub(crate) hyper_causal: Arc<crate::orchestration::branching::HyperCausalEngine>,
    pub(crate) agent_index: u8,
    pub(crate) max_parallel_tools: usize,
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
            fallback_provider: None,
            memory,
            tools,
            context: ContextAssembler::new(identity, TokenBudget::new(256000), skills_list),
            plugin_host: None,
            plugins: Vec::new(),
            security_token: None,
            predictor: DspPredictor::default(),
            echo_registry: None,
            echo_metrics: None,
            echo_host: None,
            collective_blackboard: None,
            hyper_causal: Arc::new(crate::orchestration::branching::HyperCausalEngine::default()),
            agent_index: 0,
            max_parallel_tools: 5,
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

    /// Sets the fallback provider for OMEGA-VIII Absolute continuity.
    pub fn with_fallback(mut self, provider: Box<dyn LlmProvider>) -> Self {
        self.fallback_provider = Some(provider);
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
        session_id: Option<savant_core::types::SessionId>,
        shutdown_token: CancellationToken,
    ) -> Pin<Box<dyn Stream<Item = Result<AgentEvent, SavantError>> + Send + '_>> {
        let mut history = vec![ChatMessage {
            role: ChatRole::User,
            content: user_input.clone(),
            sender: Some("USER".to_string()),
            recipient: None,
            agent_id: None,
            session_id: session_id.clone(),
            channel: savant_core::types::AgentOutputChannel::Chat,
        }];

        Box::pin({
            use async_stream::stream;
            stream! {
                let mut depth = 0;
                const MAX_DEPTH: u32 = 8; // OpenClaw parity depth

                while depth < MAX_DEPTH {
                    info!("[{}] Agent loop cycle start (depth={})", self.agent_id, depth);
                    
                    // AAA: Unified Context Harmony - Determine effective session anchor
                    let effective_sid = session_id.as_ref()
                        .map(|s| s.0.clone())
                        .unwrap_or_else(|| self.agent_id.clone());

                    // 0. Dynamic Speculation Depth Prediction (DSP)
                    // Complexity heuristic: based on history length and current depth
                    let complexity = (history.len() as f32 * 0.5) + (depth as f32);
                    let k = self.predictor.predict_optimal_k(complexity);
                    debug!("DSP Prediction: k={} for complexity={}", k, complexity);

                    // 1. Context Assembly (Retrieval + System Prompt)
                    let session_context = self.memory.retrieve(&effective_sid, &user_input, 10).await?;
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

                    // 2. Model Inference: Atomic Streaming State Machine with Provider Fallback
                    let response_stream = match self.provider.stream_completion(messages.clone()).await {
                        Ok(stream) => stream,
                        Err(e) => {
                            if let Some(fallback) = &self.fallback_provider {
                                warn!("[{}] Primary provider failed: {}. Triggering OMEGA-VIII Fallback.", self.agent_id, e);
                                yield Ok(AgentEvent::StatusUpdate("FALLBACK_PROVIDER_ACTIVATED".to_string()));
                                match fallback.stream_completion(messages).await {
                                    Ok(stream) => stream,
                                    Err(fe) => {
                                        yield Err(SavantError::Unknown(format!("Absolute failure: primary ({}) and fallback ({}) failed.", e, fe)));
                                        return;
                                    }
                                }
                            } else {
                                yield Err(e);
                                return;
                            }
                        }
                    };
                    let mut full_trace = String::new(); // Full trace including reasoning
                    let mut clean_answer = String::new(); // Just the dialogue

                    let mut llm_stream = response_stream;

                    let mut fragment_buffer = String::new();
                    let mut in_thought = false;
                    const THOUGHT_START: &str = "<thought>";
                    const THOUGHT_END: &str = "</thought>";

                    // 🛡️ Absolute Substrate Sovereignty (Perfection v12.2)
                    while let Some(chunk_res) = llm_stream.next().await {
                        match chunk_res {
                            Ok(chunk) => {
                                if !chunk.content.is_empty() {
                                    let content = chunk.content;
                                    full_trace.push_str(&content);
                                    fragment_buffer.push_str(&content);

                                    loop {
                                        if !in_thought {
                                            if let Some(pos) = fragment_buffer.find(THOUGHT_START) {
                                                tracing::info!(vent="lane_switch", to="thought", pos=pos, "Protocol delimiter detected");
                                                
                                                let dialogue_part = &fragment_buffer[..pos];
                                                if !dialogue_part.trim().is_empty() {
                                                    clean_answer.push_str(dialogue_part);
                                                    yield Ok(AgentEvent::FinalAnswerChunk(dialogue_part.to_string()));
                                                }
                                                
                                                fragment_buffer = fragment_buffer[pos + THOUGHT_START.len()..].to_string();
                                                in_thought = true;
                                            } else {
                                                // Check for partial THOUGHT_START at the end
                                                let mut matched_len = 0;
                                                for i in 1..THOUGHT_START.len() {
                                                    if fragment_buffer.ends_with(&THOUGHT_START[..i]) {
                                                        matched_len = i;
                                                    }
                                                }
                                                let safe_to_flush = fragment_buffer.len() - matched_len;
                                                if safe_to_flush > 0 {
                                                    let dialogue_chunk: String = fragment_buffer.drain(..safe_to_flush).collect();
                                                    clean_answer.push_str(&dialogue_chunk);
                                                    yield Ok(AgentEvent::FinalAnswerChunk(dialogue_chunk));
                                                }
                                                break; // Wait for more tokens
                                            }
                                        } else {
                                            if let Some(pos) = fragment_buffer.find(THOUGHT_END) {
                                                tracing::info!(vent="lane_switch", to="dialogue", pos=pos, "Protocol delimiter detected");
                                                
                                                let thought_part = &fragment_buffer[..pos];
                                                if !thought_part.is_empty() {
                                                    yield Ok(AgentEvent::Thought(thought_part.to_string()));
                                                }
                                                
                                                fragment_buffer = fragment_buffer[pos + THOUGHT_END.len()..].to_string();
                                                in_thought = false;
                                            } else {
                                                // Check for partial THOUGHT_END at the end
                                                let mut matched_len = 0;
                                                for i in 1..THOUGHT_END.len() {
                                                    if fragment_buffer.ends_with(&THOUGHT_END[..i]) {
                                                        matched_len = i;
                                                    }
                                                }
                                                let safe_to_flush = fragment_buffer.len() - matched_len;
                                                if safe_to_flush > 0 {
                                                    let thought_chunk: String = fragment_buffer.drain(..safe_to_flush).collect();
                                                    yield Ok(AgentEvent::Thought(thought_chunk));
                                                }
                                                break; // Wait for more tokens
                                            }
                                        }
                                    }
                                }
                                if chunk.is_final { break; }
                            }
                            Err(e) => {
                                yield Err(e);
                                return;
                            }
                        }
                    }

                    // Final flush
                    if !fragment_buffer.is_empty() {
                        if in_thought {
                            yield Ok(AgentEvent::Thought(fragment_buffer));
                        } else {
                            if !fragment_buffer.trim().is_empty() {
                                clean_answer.push_str(&fragment_buffer);
                                yield Ok(AgentEvent::FinalAnswerChunk(fragment_buffer));
                            }
                        }
                    }

                    // 3. Tool Execution / Action Parsing (Speculative Chaining)
                    let actions = parsing::parse_actions(&full_trace);
                    let mut actual_steps = 0;

                    if !actions.is_empty() {
                        let dag = crate::orchestration::dag::parse_sequential_dag(actions);
                        let mut completed_indices = HashSet::new();
                        let mut queue = FuturesUnordered::new();
                        let mut pending_nodes: Vec<(usize, crate::orchestration::dag::SpeculativeNode)> = 
                            dag.nodes.into_iter().enumerate().collect();

                        // Clone state for parallel execution to avoid &mut self borrow conflicts
                        let tools = self.tools.clone();
                        let hc = self.hyper_causal.clone();
                        let registry = self.echo_registry.clone();
                        let e_host = self.echo_host.clone();
                        let agent_id = self.agent_id.clone();
                        let agent_id_hash = self.agent_id_hash;
                        let security_token = self.security_token.clone();
                        let plugin_host = self.plugin_host.clone();
                        let plugins = self.plugins.clone();

                        while !pending_nodes.is_empty() || !queue.is_empty() {
                            // 1. Identification Phase: Find nodes with satisfied dependencies
                            let mut i = 0;
                            while i < pending_nodes.len() && queue.len() < self.max_parallel_tools {
                                let (_idx, node) = &pending_nodes[i];
                                if node.dependencies.iter().all(|d| completed_indices.contains(d)) {
                                    let (idx, node) = pending_nodes.remove(i);
                                    
                                    // Prepare the task for the reactor
                                    let node_name = node.name.clone();
                                    let node_args = node.args.clone();
                                    
                                    yield Ok(AgentEvent::Action { name: node_name.clone(), args: node_args.clone() });
                                    
                                    let tools_inner = tools.clone();
                                    let hc_inner = hc.clone();
                                    let reg_inner = registry.clone();
                                    let host_inner = e_host.clone();
                                    let _id_inner = agent_id.clone();

                                    queue.push(async move {
                                        // Standalone execution logic using clones
                                        let mut result = Err(SavantError::Unknown(format!("Tool not found: {}", node_name)));
                                        
                                        // 1. Check legacy tools
                                        for tool in &tools_inner {
                                            if tool.name().to_lowercase() == node_name.to_lowercase() {
                                                let payload = serde_json::from_str(&node_args)
                                                    .unwrap_or_else(|_| serde_json::json!({ "payload": node_args }));
                                                result = hc_inner.execute_speculative(tool.clone(), payload).await;
                                                break;
                                            }
                                        }

                                        // 2. Check WASM tools
                                        if result.is_err() {
                                            if let (Some(reg), Some(host)) = (&reg_inner, &host_inner) {
                                                if let Some(cap) = reg.get_tool(&node_name) {
                                                    result = host.execute_tool(&cap.module, &node_args).await
                                                        .map_err(|e| SavantError::Unknown(e.to_string()));
                                                }
                                            }
                                        }

                                        (idx, node_name, result)
                                    });
                                } else {
                                    i += 1;
                                }
                            }

                            if queue.is_empty() && !pending_nodes.is_empty() {
                                warn!("Parallel Reactor: Deadlock detected in Speculative DAG for agent [{}]", self.agent_id);
                                break;
                            }

                            // 2. Execution Phase: Wait for next tool completion or shutdown
                            tokio::select! {
                                Some((idx, name, result)) = queue.next() => {
                                    actual_steps += 1;
                                    match result {
                                        Ok(mut obs) => {
                                            // WASM Hook: after_tool_call
                                            if let Some(host) = &plugin_host {
                                                for plugin in &plugins {
                                                    match host.execute_after_tool_call(
                                                        plugin, 
                                                        &name, 
                                                        &obs,
                                                        agent_id_hash,
                                                        security_token.clone(),
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
                                                content: format!("Observation ({}): {}", name, obs),
                                                sender: Some("SYSTEM".to_string()),
                                                recipient: None,
                                                agent_id: None,
                                                session_id: session_id.clone(),
                                                channel: savant_core::types::AgentOutputChannel::Telemetry,
                                            });
                                            completed_indices.insert(idx);
                                        }
                                        Err(e) => {
                                            warn!("Parallel Reactor: Tool execution failed for [{}]: {}", name, e);
                                            yield Err(SavantError::Unknown(e.to_string()));
                                            return;
                                        }
                                    }
                                }
                                _ = shutdown_token.cancelled() => {
                                    info!("Parallel Reactor: Received shutdown signal for agent [{}]", self.agent_id);
                                    return;
                                }
                            }
                        }

                        // Update DSP accuracy (post-hoc)
                        self.predictor.update_accuracy(k, actual_steps);
                        if self.predictor.prediction_count().is_multiple_of(5) {
                            self.predictor.adapt_parameters();
                        }
                    } else {
                        let mut final_response = full_trace.clone();

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
                        info!("[{}] Agent loop emitting final answer", self.agent_id);
                        // 🌀 Perfection Loop: Yield ONLY the clean answer to the final event
                        // This prevents Reasoning Pollution in historical chat lanes.
                        yield Ok(AgentEvent::FinalAnswer(clean_answer.trim().to_string()));

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

                        let final_msg = ChatMessage {
                            role: ChatRole::Assistant,
                            content: final_response,
                            sender: Some(self.agent_id.clone()),
                            recipient: None,
                            agent_id: None,
                            session_id: session_id.clone(),
                            channel: savant_core::types::AgentOutputChannel::Chat,
                        };
                        
                        let sid = session_id.as_ref()
                            .map(|s| s.0.clone())
                            .unwrap_or_else(|| self.agent_id.clone());

                        self.memory.store(&sid, &final_msg).await?;

                        break;
                    }
                    depth += 1;
                }
            }
        })
    }

    pub(crate) async fn execute_tool(&self, name: &str, args: &str) -> Result<String, SavantError> {
        for tool in &self.tools {
            if tool.name().to_lowercase() == name.to_lowercase() {
                tracing::info!("Agent [{}] executing tool: {}", self.agent_id, name);

                // Attempt to parse args as JSON, fallback to manual object for raw strings
                let payload = serde_json::from_str(args)
                    .unwrap_or_else(|_| serde_json::json!({ "payload": args }));

                // 🌀 OMEGA-VII: Hyper-Causal Execution (Potential Timeline Collapse)
                return self.hyper_causal.execute_speculative(tool.clone(), payload).await;
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
            agent_id: None,
            session_id: None, // System-local reflection
            channel: savant_core::types::AgentOutputChannel::Memory,
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
                session_id: None,
                channel: savant_core::types::AgentOutputChannel::Chat,
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

        let mut stream = agent.run("start".into(), CancellationToken::new());
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

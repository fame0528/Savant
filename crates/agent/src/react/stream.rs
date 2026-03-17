use savant_core::error::SavantError;
use savant_core::traits::MemoryBackend;
use savant_core::types::{ChatMessage, ChatRole};
use savant_core::utils::parsing;
use std::pin::Pin;
use std::collections::HashSet;
use tracing::{debug, info, warn};
use futures::stream::{Stream, StreamExt, FuturesUnordered};
use tokio_util::sync::CancellationToken;
use crate::react::{AgentLoop, AgentEvent};

impl<M: MemoryBackend> AgentLoop<M> {
    /// OMEGA-IX: Assembles cognitive context from episodic memory and workspace state.
    async fn assemble_context(
        &self,
        user_input: &str,
        session_id: &Option<savant_core::types::SessionId>,
        history: &[ChatMessage],
    ) -> Result<Vec<ChatMessage>, SavantError> {
        let effective_sid = session_id.as_ref()
            .map(|s| s.0.clone())
            .unwrap_or_else(|| self.agent_id.clone());

        let session_context = self.memory.retrieve(&effective_sid, user_input, 10).await?;
        let mut current_history = session_context;
        current_history.extend(history.to_vec());

        let mut messages = self.context.build_messages(current_history);

        if let Some(host) = &self.plugin_host {
            for plugin in &self.plugins {
                let mut combined_prompt = String::new();
                for msg in &messages { combined_prompt.push_str(&msg.content); }

                if let Ok(res) = host.execute_before_llm_call(plugin, &combined_prompt, self.agent_id_hash, self.security_token.clone()).await {
                    match res {
                        crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Modified(new_prompt) => {
                            if let Some(last) = messages.last_mut() { last.content = new_prompt; }
                        }
                        crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Halt(reason) => {
                            return Err(SavantError::Unknown(format!("Halted by plugin: {}", reason)));
                        }
                        crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Continue => {}
                    }
                }
            }
        }
        Ok(messages)
    }

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
                const MAX_DEPTH: u32 = 8;

                while depth < MAX_DEPTH {
                    info!("[{}] Agent loop cycle start (depth={})", self.agent_id, depth);
                    
                    let mut messages = self.assemble_context(&user_input, &session_id, &history).await?;
                    
                    let complexity = (history.len() as f32 * 0.5) + (depth as f32);
                    let k = self.predictor.predict_optimal_k(complexity);
                    debug!("DSP Prediction: k={} for complexity={}", k, complexity);

                    if let Some(host) = &self.plugin_host {
                        for plugin in &self.plugins {
                            let mut combined_prompt = String::new();
                            for msg in &messages { combined_prompt.push_str(&msg.content); }

                            if let Ok(res) = host.execute_before_llm_call(plugin, &combined_prompt, self.agent_id_hash, self.security_token.clone()).await {
                                match res {
                                    crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Modified(new_prompt) => {
                                        if let Some(last) = messages.last_mut() { last.content = new_prompt; }
                                    }
                                    crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Halt(reason) => {
                                        yield Err(SavantError::Unknown(format!("Halted by plugin: {}", reason)));
                                        return;
                                    }
                                    crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Continue => {}
                                }
                            }
                        }
                    }

                    let response_stream = match self.provider.stream_completion(messages.clone()).await {
                        Ok(stream) => stream,
                        Err(e) => {
                            if let Some(fallback) = &self.fallback_provider {
                                warn!("[{}] Primary provider failed: {}. Triggering OMEGA-VIII Fallback.", self.agent_id, e);
                                yield Ok(AgentEvent::StatusUpdate("FALLBACK_PROVIDER_ACTIVATED".to_string()));
                                fallback.stream_completion(messages).await?
                            } else { yield Err(e); return; }
                        }
                    };

                    let mut full_trace = String::new();
                    let mut clean_answer = String::new();
                    let mut llm_stream = response_stream;
                    let mut fragment_buffer = String::new();
                    let mut in_thought = false;
                    const THOUGHT_START: &str = "<thought>";
                    const THOUGHT_END: &str = "</thought>";

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
                                                let dialogue_part = &fragment_buffer[..pos];
                                                if !dialogue_part.trim().is_empty() {
                                                    clean_answer.push_str(dialogue_part);
                                                    yield Ok(AgentEvent::FinalAnswerChunk(dialogue_part.to_string()));
                                                }
                                                fragment_buffer = fragment_buffer[pos + THOUGHT_START.len()..].to_string();
                                                in_thought = true;
                                            } else {
                                                let mut matched_len = 0;
                                                for i in 1..THOUGHT_START.len() { if fragment_buffer.ends_with(&THOUGHT_START[..i]) { matched_len = i; } }
                                                let safe_to_flush = fragment_buffer.len() - matched_len;
                                                if safe_to_flush > 0 {
                                                    let dialogue_chunk: String = fragment_buffer.drain(..safe_to_flush).collect();
                                                    clean_answer.push_str(&dialogue_chunk);
                                                    yield Ok(AgentEvent::FinalAnswerChunk(dialogue_chunk));
                                                }
                                                break;
                                            }
                                        } else {
                                            if let Some(pos) = fragment_buffer.find(THOUGHT_END) {
                                                let thought_part = &fragment_buffer[..pos];
                                                if !thought_part.is_empty() { yield Ok(AgentEvent::Thought(thought_part.to_string())); }
                                                fragment_buffer = fragment_buffer[pos + THOUGHT_END.len()..].to_string();
                                                in_thought = false;
                                            } else {
                                                let mut matched_len = 0;
                                                for i in 1..THOUGHT_END.len() { if fragment_buffer.ends_with(&THOUGHT_END[..i]) { matched_len = i; } }
                                                let safe_to_flush = fragment_buffer.len() - matched_len;
                                                if safe_to_flush > 0 {
                                                    let thought_chunk: String = fragment_buffer.drain(..safe_to_flush).collect();
                                                    yield Ok(AgentEvent::Thought(thought_chunk));
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                                if chunk.is_final { break; }
                            }
                            Err(e) => { yield Err(e); return; }
                        }
                    }

                    if !fragment_buffer.is_empty() {
                        if in_thought { yield Ok(AgentEvent::Thought(fragment_buffer)); }
                        else if !fragment_buffer.trim().is_empty() {
                            clean_answer.push_str(&fragment_buffer);
                            yield Ok(AgentEvent::FinalAnswerChunk(fragment_buffer));
                        }
                    }

                    let mut actions = parsing::parse_actions(&full_trace);
                    if actions.is_empty() {
                        debug!("[{}] No actions parsed from trace (len={})", self.agent_id, full_trace.len());
                    }
                    
                    // --- OMEGA: Autonomous Ambiguity Synthesis ---
                    if actions.is_empty() && (full_trace.contains("Action:") || full_trace.contains("thought")) {
                        warn!("[{}] Ambiguity detected: LLM suggested action but parser failed. Triggering Heuristic Synthesis.", self.agent_id);
                        yield Ok(AgentEvent::StatusUpdate("HEURISTIC_AMBIGUITY_DETECTED".to_string()));
                        
                        // Heuristic: If we see a pattern like "Action: SomeTool(args)", try manually extraction
                        if let Some(start) = full_trace.find("Action:") {
                            let subset = &full_trace[start..];
                            if let Some(end) = subset.find('\n').or(Some(subset.len())) {
                                let line = &subset[..end];
                                // Attempt simple parse
                                if let Some(bracket_start) = line.find('[') {
                                    if let Some(bracket_end) = line.rfind(']') {
                                        let name = line["Action:".len()..bracket_start].trim();
                                        let args = &line[bracket_start+1..bracket_end];
                                        actions.push((name.to_string(), args.to_string()));
                                        info!("[{}] Heuristic: Synthesized action [{}] from ambiguous trace", self.agent_id, name);
                                    }
                                } else {
                                    // AAA: Robust Fallback for missing brackets
                                    let name = line["Action:".len()..].trim();
                                    if !name.is_empty() {
                                        actions.push(("MalformedMockTool".to_string(), name.to_string()));
                                        info!("[{}] Heuristic: Synthesized MalformedMockTool for ambiguous line: {}", self.agent_id, name);
                                    }
                                }
                            }
                        }
                    }

                    let mut actual_steps = 0;

                    if !actions.is_empty() {
                        // --- OMEGA: Transactional Checkpoint ---
                        // Save current history as a stable state before potential side-effects
                        self.heuristic.last_stable_checkpoint = Some(history.clone());
                        info!("[{}] Checkpoint created: Session state stabilized before action execution.", self.agent_id);

                        let dag = crate::orchestration::dag::parse_sequential_dag(actions);
                        let mut completed_indices = HashSet::new();
                        let mut queue = FuturesUnordered::new();
                        let mut pending_nodes: Vec<(usize, crate::orchestration::dag::SpeculativeNode)> = dag.nodes.into_iter().enumerate().collect();

                        let tools = self.tools.clone();
                        let hc = self.hyper_causal.clone();
                        let registry = self.echo_registry.clone();
                        let e_host = self.echo_host.clone();
                        let agent_id_hash = self.agent_id_hash;
                        let security_token = self.security_token.clone();
                        let plugin_host = self.plugin_host.clone();
                        let plugins = self.plugins.clone();

                        while !pending_nodes.is_empty() || !queue.is_empty() {
                            let mut i = 0;
                            while i < pending_nodes.len() && queue.len() < self.max_parallel_tools {
                                let (_idx, node) = &pending_nodes[i];
                                if node.dependencies.iter().all(|d| completed_indices.contains(d)) {
                                    let (idx, node) = pending_nodes.remove(i);
                                    let node_name = node.name.clone();
                                    let node_args = node.args.clone();
                                    yield Ok(AgentEvent::Action { name: node_name.clone(), args: node_args.clone() });
                                    
                                    let tools_inner = tools.clone();
                                    let hc_inner = hc.clone();
                                    let reg_inner = registry.clone();
                                    let host_inner = e_host.clone();
                                    let security_token_inner = security_token.clone();
                                    let node_name_inner = node_name.clone();
                                    let node_args_inner = node_args.clone();
                                    let agent_id_inner = self.agent_id.clone();

                                    queue.push(async move {
                                        let mut result = Err(SavantError::Unknown(format!("Tool access denied or not found: {}", node_name_inner)));
                                        let access_granted = if let Some(token) = &security_token_inner {
                                            let resource = format!("savant://tools/{}", node_name_inner);
                                            token.assignee_matches(agent_id_hash) && token.verify_capability(&resource, "execute")
                                        } else { true };

                                        if access_granted {
                                            debug!("[{}] Attempting to match tool [{}]", agent_id_inner, node_name_inner);
                                            for tool in &tools_inner {
                                                debug!("[{}] Comparing against tool [{}]", agent_id_inner, tool.name());
                                                if tool.name().to_lowercase() == node_name_inner.to_lowercase() {
                                                    let payload = serde_json::from_str(&node_args_inner).unwrap_or_else(|_| serde_json::json!({ "payload": node_args_inner }));
                                                    debug!("[{}] Tool [{}] matched. Executing...", agent_id_inner, node_name_inner);
                                                    result = hc_inner.execute_speculative(tool.clone(), payload).await;
                                                    break;
                                                }
                                            }
                                            if result.is_err() {
                                                if let (Some(reg), Some(host)) = (&reg_inner, &host_inner) {
                                                    if let Some(cap) = reg.get_tool(&node_name_inner) {
                                                        result = host.execute_tool(&cap.module, &node_args_inner).await.map_err(|e| SavantError::Unknown(e.to_string()));
                                                    }
                                                }
                                            }
                                        } else { result = Err(SavantError::AuthError(format!("CCT Policy Denied: {}", node_name_inner))); }
                                        (idx, node_name_inner, result)
                                    });
                                } else { i += 1; }
                            }

                            if queue.is_empty() && !pending_nodes.is_empty() { break; }

                            tokio::select! {
                                Some((idx, name, result)) = queue.next() => {
                                    actual_steps += 1;
                                    match result {
                                        Ok(mut obs) => {
                                            if let Some(host) = &plugin_host {
                                                for plugin in &plugins {
                                                    match host.execute_after_tool_call(plugin, &name, &obs, agent_id_hash, security_token.clone()).await {
                                                        Ok(crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Modified(new_obs)) => { obs = new_obs; }
                                                        Ok(crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Halt(reason)) => {
                                                            yield Err(SavantError::Unknown(format!("Halted by plugin: {}", reason)));
                                                            return;
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                            yield Ok(AgentEvent::Observation(obs.clone()));
                                            history.push(ChatMessage { role: ChatRole::User, content: format!("Observation ({}): {}", name, obs), sender: Some("SYSTEM".to_string()), recipient: None, agent_id: None, session_id: session_id.clone(), channel: savant_core::types::AgentOutputChannel::Telemetry });
                                            
                                            // Report success to collective blackboard
                                            if let Some(cb) = &self.collective_blackboard {
                                                let pressure = history.len() as f32 / 100.0;
                                                let _ = cb.update_agent_metrics(self.agent_index, true, pressure);
                                            }
                                            
                                            completed_indices.insert(idx);
                                        }
                                        Err(e) => { 
                                            // 🧬 OMEGA: Heuristic Fallback
                                            let resolution_name = name.clone();
                                            
                                            // Report failure to collective blackboard
                                            if let Some(cb) = &self.collective_blackboard {
                                                let pressure = history.len() as f32 / 100.0;
                                                let _ = cb.update_agent_metrics(self.agent_index, false, pressure);
                                            }

                                            match self.handle_heuristic_resolution(&resolution_name, e).await {
                                                Ok(hint) => {
                                                    yield Ok(AgentEvent::Observation(hint.clone()));
                                                    history.push(ChatMessage { 
                                                        role: ChatRole::User, 
                                                        content: format!("Recovery Hint ({}): {}", resolution_name, hint), 
                                                        sender: Some("SYSTEM".to_string()), 
                                                        recipient: None, 
                                                        agent_id: None, 
                                                        session_id: session_id.clone(), 
                                                        channel: savant_core::types::AgentOutputChannel::Telemetry 
                                                    });
                                                }
                                                Err(fatal) => {
                                                    yield Err(fatal);
                                                    return;
                                                }
                                            }
                                            // Break the DAG execution to allow the LLM to re-evaluate with the hint
                                            break;
                                        }
                                    }
                                }
                                _ = shutdown_token.cancelled() => { return; }
                            }
                        }
                        self.predictor.update_accuracy(k, actual_steps);
                        if self.predictor.prediction_count().is_multiple_of(5) { self.predictor.adapt_parameters(); }
                    } else {
                        let mut final_response = full_trace.clone();
                        if let Some(host) = &self.plugin_host {
                            for plugin in &self.plugins {
                                if let Ok(res) = host.execute_before_response_emit(plugin, &final_response, self.agent_id_hash, self.security_token.clone()).await {
                                    match res {
                                        crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Modified(new_resp) => { final_response = new_resp; }
                                        crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Halt(reason) => {
                                            yield Err(SavantError::Unknown(format!("Halted by plugin: {}", reason)));
                                            return;
                                        }
                                        crate::plugins::wasm_host::exports::savant::agent_hooks::hooks::HookResult::Continue => {}
                                    }
                                }
                            }
                        }

                        yield Ok(AgentEvent::FinalAnswer(clean_answer.trim().to_string()));
                        let reflection = self.generate_reflection(&history, &final_response).await?;
                        
                        if let Some(collective) = &self.collective_blackboard {
                            // Increment heuristic version and trigger swarm-wide metric aggregation
                            if let Ok(mut state) = collective.read_global_state() {
                                state.heuristic_version = state.heuristic_version.wrapping_add(1);
                                let _ = collective.publish_global_state(state);
                                let _ = collective.aggregate_swarm_metrics();
                            }
                        }

                        yield Ok(AgentEvent::Reflection(reflection.clone()));

                        let final_msg = ChatMessage { role: ChatRole::Assistant, content: final_response, sender: Some(self.agent_id.clone()), recipient: None, agent_id: None, session_id: session_id.clone(), channel: savant_core::types::AgentOutputChannel::Chat };
                        let sid = session_id.as_ref().map(|s| s.0.clone()).unwrap_or_else(|| self.agent_id.clone());
                        self.memory.store(&sid, &final_msg).await?;
                        break;
                    }
                    depth += 1;
                }
            }
        })
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
            session_id: None,
            channel: savant_core::types::AgentOutputChannel::Memory,
        });

        let messages = self.context.build_messages(ref_history);
        let mut stream = self.provider.stream_completion(messages).await?;
        let mut reflection = String::new();
        while let Some(chunk_res) = stream.next().await {
            if let Ok(chunk) = chunk_res {
                reflection.push_str(&chunk.content);
                if chunk.is_final { break; }
            }
        }
        Ok(reflection)
    }
}

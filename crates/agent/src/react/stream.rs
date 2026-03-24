use crate::react::{AgentEvent, AgentLoop};
use futures::stream::{FuturesUnordered, Stream, StreamExt};
use savant_core::error::SavantError;
use savant_core::traits::MemoryBackend;
use savant_core::types::{ChatMessage, ChatRole};
use savant_core::utils::parsing;
use std::collections::HashSet;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

impl<M: MemoryBackend> AgentLoop<M> {
    /// OMEGA-IX: Assembles cognitive context from episodic memory and workspace state.
    async fn assemble_context(
        &self,
        user_input: &str,
        session_id: &Option<savant_core::types::SessionId>,
        history: &[ChatMessage],
    ) -> Result<Vec<ChatMessage>, SavantError> {
        let effective_sid = session_id
            .as_ref()
            .map(|s| s.0.clone())
            .unwrap_or_else(|| self.agent_id.clone());

        let session_context = self.memory.retrieve(&effective_sid, user_input, 10).await?;
        let mut current_history = session_context;
        current_history.extend(history.to_vec());

        let mut messages = self.context.build_messages(current_history);

        if let Some(host) = &self.plugin_host {
            for plugin in &self.plugins {
                let mut combined_prompt = String::new();
                for msg in &messages {
                    combined_prompt.push_str(&msg.content);
                }

                if let Ok(res) = host
                    .execute_before_llm_call(
                        plugin,
                        &combined_prompt,
                        self.agent_id_hash,
                        self.security_token.clone(),
                    )
                    .await
                {
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
            is_telemetry: false,
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
                let mut seen_actions: HashSet<String> = HashSet::new();
                let sid = session_id.as_ref().map(|s| s.0.clone()).unwrap_or_else(|| self.agent_id.clone());

                // === Session / Turn Initialization ===
                let turn_id = uuid::Uuid::new_v4().to_string();

                // Get or create session state
                let mut session_state = match self.memory.get_or_create_session(&sid).await {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("[{}] Failed to load session state: {}. Creating ephemeral.", self.agent_id, e);
                        savant_core::types::SessionState {
                            session_id: sid.clone(),
                            created_at: chrono::Utc::now().timestamp_millis(),
                            last_active: chrono::Utc::now().timestamp_millis(),
                            turn_count: 0,
                            active_turn_id: None,
                            auto_approved_tools: vec![],
                            denied_tools: vec![],
                        }
                    }
                };

                // Begin turn
                session_state.active_turn_id = Some(turn_id.clone());
                session_state.turn_count += 1;

                // === Hook: Turn Start ===
                let turn_start_ctx = savant_core::hooks::HookContext {
                    event: savant_core::hooks::HookEvent::TurnStart,
                    session_id: Some(sid.clone()),
                    agent_id: Some(self.agent_id.clone()),
                    tool_name: None,
                    content: Some(user_input.clone()),
                    error: None,
                    metadata: std::collections::HashMap::new(),
                };
                self.hooks.run_void(&turn_start_ctx).await;
                session_state.last_active = chrono::Utc::now().timestamp_millis();
                let _ = self.memory.save_session(&session_state).await;

                let turn_state = savant_core::types::TurnState {
                    turn_id: turn_id.clone(),
                    session_id: sid.clone(),
                    state: savant_core::types::TurnPhase::Processing,
                    tool_calls_made: Vec::new(),
                    started_at: chrono::Utc::now().timestamp_millis(),
                    completed_at: 0,
                };
                let _ = self.memory.save_turn(&turn_state).await;

                // Emit SessionStart event
                yield Ok(AgentEvent::SessionStart {
                    session_id: sid.clone(),
                    turn_id: turn_id.clone(),
                });

                // Track tool calls made during this turn
                let mut turn_tool_calls: Vec<String> = Vec::new();
                let turn_failed = false;

                while depth < self.max_tool_iterations as u32 {
                    info!("[{}] Agent loop cycle start (depth={})", self.agent_id, depth);

                    let mut messages = self.assemble_context(&user_input, &session_id, &history).await?;

                    // === Context Compaction Check ===
                    // Discovery-based: use provider's context window, not hardcoded value
                    let monitor = crate::react::compaction::ContextMonitor::new(self.context_window);
                    if let Some(strategy) = monitor.suggest(&messages) {
                        let usage = monitor.usage_ratio(&messages);
                        info!(
                            "[{}] Context at {:.0}% — applying {:?} compaction",
                            self.agent_id,
                            usage * 100.0,
                            strategy
                        );
                        yield Ok(AgentEvent::StatusUpdate(
                            format!("CONTEXT_COMPACTING: {:.0}% usage → {:?}", usage * 100.0, strategy)
                        ));
                        // Scale keep_recent proportionally to context window size
                        // 128K context → 10 messages, 256K → 20, 32K → 5
                        let keep_recent = (self.context_window / 12_800).max(5);
                        messages = crate::react::compaction::Compactor::compact(messages, strategy, keep_recent);
                        // Rebuild history from compacted messages (skip system prompt)
                        history = messages.iter()
                            .filter(|m| m.role != savant_core::types::ChatRole::System)
                            .cloned()
                            .collect();
                    }

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

                    // Build tool schemas for LLM API
                    let tool_schemas: Vec<serde_json::Value> = self.tools.iter().map(|t| {
                        serde_json::json!({
                            "type": "function",
                            "function": {
                                "name": t.name(),
                                "description": t.description(),
                                "parameters": t.parameters_schema(),
                            }
                        })
                    }).collect();

                    // === Hook: Before LLM Call (modifying — can cancel) ===
                    let mut before_llm_ctx = savant_core::hooks::HookContext {
                        event: savant_core::hooks::HookEvent::BeforeLlmCall,
                        session_id: Some(sid.clone()),
                        agent_id: Some(self.agent_id.clone()),
                        tool_name: None,
                        content: Some(format!("{} messages, {} tools", messages.len(), tool_schemas.len())),
                        error: None,
                        metadata: std::collections::HashMap::new(),
                    };
                    if let savant_core::hooks::HookResult::Cancel(reason) = self.hooks.run_modifying(&mut before_llm_ctx).await {
                        yield Ok(AgentEvent::Observation(format!("LLM call cancelled by hook: {}", reason)));
                        break;
                    }

                    let response_stream = match self.provider.stream_completion(messages.clone(), tool_schemas.clone()).await {
                        Ok(stream) => stream,
                        Err(e) => {
                            if let Some(fallback) = &self.fallback_provider {
                                warn!("[{}] Primary provider failed: {}. Triggering OMEGA-VIII Fallback.", self.agent_id, e);
                                yield Ok(AgentEvent::StatusUpdate("FALLBACK_PROVIDER_ACTIVATED".to_string()));
                                fallback.stream_completion(messages, tool_schemas).await?
                            } else {
                                // Finalize turn state before error return
                                let final_turn = savant_core::types::TurnState {
                                    turn_id: turn_id.clone(),
                                    session_id: sid.clone(),
                                    state: savant_core::types::TurnPhase::Failed,
                                    tool_calls_made: turn_tool_calls.clone(),
                                    started_at: turn_state.started_at,
                                    completed_at: chrono::Utc::now().timestamp_millis(),
                                };
                                let _ = self.memory.save_turn(&final_turn).await;
                                session_state.active_turn_id = None;
                                session_state.last_active = chrono::Utc::now().timestamp_millis();
                                let _ = self.memory.save_session(&session_state).await;

                                yield Err(e);
                                return;
                            }
                        }
                    };

                    let mut full_trace = String::new();
                    let mut clean_answer = String::new();
                    let mut llm_stream = response_stream;
                    let mut fragment_buffer = String::new();
                    let mut in_hidden_tag = false;
                    let mut hidden_tag_name = String::new();
                    // All known thinking/reasoning tag formats across models
                    const THOUGHT_TAGS: &[(&str, &str)] = &[
                        ("<think>", "</think>"),
                        ("<thinking>", "</thinking>"),
                        ("<thought>", "</thought>"),
                        ("<reasoning>", "</reasoning>"),
                    ];
                    // Tags that should be hidden from user output (tool calls, environment details, etc.)
                    const HIDDEN_TAGS: &[&str] = &[
                        "environment_details", "use_mcp_tool", "read_file", "write_to_file",
                        "execute_command", "ask_followup_question", "attempt_completion",
                        "search_files", "list_files", "replace_in_file",
                        "browser_action", "mcp_response", "tool_result",
                        // Savant tool tags
                        "file_create", "file_move", "file_delete", "file_atomic_edit",
                        "shell", "foundation", "memory_search", "memory_append",
                        "web_search", "web_fetch", "task_matrix", "settings",
                        "librarian", "web_projection", "tool_call", "final_answer",
                        // Generic tool/function tags
                        "tool", "function", "parameter", "arguments", "name",
                        "input", "output", "result", "response",
                    ];

                    while let Some(chunk_res) = llm_stream.next().await {
                        match chunk_res {
                            Ok(chunk) => {
                                // Handle provider-level reasoning (2026 standard: delta.reasoning)
                                if let Some(ref reasoning) = chunk.reasoning {
                                    if !reasoning.trim().is_empty() {
                                        full_trace.push_str(reasoning);
                                        yield Ok(AgentEvent::Thought(reasoning.clone()));
                                    }
                                }

                                if let Some(calls) = &chunk.tool_calls {
                                    for call in calls {
                                        let markup = format!("<tool_call>\n<name>{}</name>\n<arguments>{}</arguments>\n</tool_call>\n", call.name, call.arguments);
                                        full_trace.push_str(&markup);
                                    }
                                }

                                if !chunk.content.is_empty() {
                                    let content = chunk.content;
                                    full_trace.push_str(&content);
                                    fragment_buffer.push_str(&content);

                                    loop {
                                        if !in_hidden_tag {
                                            // Check for any thought/reasoning tag format
                                            if let Some((pos, start_tag, end_tag)) = find_thought_tag(&fragment_buffer, THOUGHT_TAGS) {
                                                let dialogue_part = &fragment_buffer[..pos];
                                                if !dialogue_part.trim().is_empty() {
                                                    clean_answer.push_str(dialogue_part);
                                                    yield Ok(AgentEvent::FinalAnswerChunk(dialogue_part.to_string()));
                                                }
                                                fragment_buffer = fragment_buffer[pos + start_tag.len()..].to_string();
                                                in_hidden_tag = true;
                                                hidden_tag_name = end_tag.to_string();
                                            }
                                            // Check for hidden tool tags
                                            else if let Some((tag_start, tag_name)) = find_hidden_tag_start(&fragment_buffer, HIDDEN_TAGS) {
                                                let dialogue_part = &fragment_buffer[..tag_start];
                                                if !dialogue_part.trim().is_empty() {
                                                    clean_answer.push_str(dialogue_part);
                                                    yield Ok(AgentEvent::FinalAnswerChunk(dialogue_part.to_string()));
                                                }
                                                let end_tag = format!("</{}>", tag_name);
                                                if let Some(end_pos) = fragment_buffer[tag_start..].find(&end_tag) {
                                                    // Tag fully contained - push to full_trace for parsing, skip from user output
                                                    let tag_content = &fragment_buffer[tag_start..tag_start + end_pos + end_tag.len()];
                                                    full_trace.push_str(tag_content);
                                                    fragment_buffer = fragment_buffer[tag_start + end_pos + end_tag.len()..].to_string();
                                                    in_hidden_tag = false;
                                                } else {
                                                    // Tag continues past buffer - enter hidden mode, push opening to full_trace
                                                    fragment_buffer = fragment_buffer[tag_start..].to_string();
                                                    full_trace.push_str(&fragment_buffer);
                                                    in_hidden_tag = true;
                                                    hidden_tag_name = tag_name;
                                                }
                                            } else {
                                                // No tag found - flush safe content
                                                let safe_to_flush = find_safe_flush_length(&fragment_buffer, THOUGHT_TAGS, HIDDEN_TAGS);
                                                if safe_to_flush > 0 {
                                                    let dialogue_chunk: String = fragment_buffer.drain(..safe_to_flush).collect();
                                                    clean_answer.push_str(&dialogue_chunk);
                                                    yield Ok(AgentEvent::FinalAnswerChunk(dialogue_chunk));
                                                }
                                                break;
                                            }
                                        } else {
                                            // Inside a hidden tag - look for closing tag
                                            let end_tag = &hidden_tag_name;
                                            if let Some(pos) = fragment_buffer.find(end_tag.as_str()) {
                                                // Check if this is a thought tag (any of the known formats)
                                                let is_thought = THOUGHT_TAGS.iter().any(|(_, et)| *et == end_tag);
                                                if is_thought {
                                                    let thought_part = &fragment_buffer[..pos];
                                                    if !thought_part.is_empty() { yield Ok(AgentEvent::Thought(thought_part.to_string())); }
                                                }
                                                // Push closing tag to full_trace for action parsing
                                                full_trace.push_str(&fragment_buffer[..pos + end_tag.len()]);
                                                fragment_buffer = fragment_buffer[pos + end_tag.len()..].to_string();
                                                in_hidden_tag = false;
                                                hidden_tag_name.clear();
                                            } else {
                                                // Still inside hidden tag - consume buffer but keep tail for partial match
                                                let safe_len = fragment_buffer.len().saturating_sub(end_tag.len());
                                                if safe_len > 0 {
                                                    let consumed: String = fragment_buffer.drain(..safe_len).collect();
                                                    full_trace.push_str(&consumed);
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                                if chunk.is_final { break; }
                            }
                            Err(e) => {
                                // Finalize turn state before error return
                                let final_turn = savant_core::types::TurnState {
                                    turn_id: turn_id.clone(),
                                    session_id: sid.clone(),
                                    state: savant_core::types::TurnPhase::Failed,
                                    tool_calls_made: turn_tool_calls.clone(),
                                    started_at: turn_state.started_at,
                                    completed_at: chrono::Utc::now().timestamp_millis(),
                                };
                                let _ = self.memory.save_turn(&final_turn).await;
                                session_state.active_turn_id = None;
                                session_state.last_active = chrono::Utc::now().timestamp_millis();
                                let _ = self.memory.save_session(&session_state).await;

                                yield Err(e);
                                return;
                            }
                        }
                    }

                    if !fragment_buffer.is_empty() {
                        if in_hidden_tag { yield Ok(AgentEvent::Thought(fragment_buffer)); }
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

                    // === Self-Repair: Check for stuck agent ===
                    let content_hash = xxhash_rust::xxh3::xxh3_64(full_trace.as_bytes());
                    if self.self_repair.check_stuck(content_hash).await {
                        warn!("[{}] Agent appears stuck — injecting recovery hint", self.agent_id);
                        let hint = self.self_repair.recovery_hint().await;
                        yield Ok(AgentEvent::StatusUpdate(format!("STUCK_DETECTED: {}", hint)));
                        history.push(ChatMessage {
                            is_telemetry: false,
                            role: ChatRole::System,
                            content: hint,
                            sender: Some("SELF_REPAIR".to_string()),
                            recipient: None,
                            agent_id: None,
                            session_id: session_id.clone(),
                            channel: savant_core::types::AgentOutputChannel::Telemetry,
                        });
                        self.self_repair.reset_stuck().await;
                    }

                    // === Self-Repair: Get excluded tools ===
                    let excluded_tools = self.self_repair.get_excluded_tools().await;

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
                        let self_repair = self.self_repair.clone();

                        while !pending_nodes.is_empty() || !queue.is_empty() {
                            let mut i = 0;
                            while i < pending_nodes.len() && queue.len() < self.max_parallel_tools {
                                let (_idx, node) = &pending_nodes[i];
                                if node.dependencies.iter().all(|d| completed_indices.contains(d)) {
                                    let (idx, node) = pending_nodes.remove(i);
                                    let node_name = node.name.clone();
                                    let node_args = node.args.clone();

                                    // Canonical dedup
                                    let mut canonical_args = node_args.clone();
                                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&node_args) {
                                        if let Ok(serialized) = serde_json::to_string(&val) {
                                            canonical_args = serialized;
                                        }
                                    }
                                    let sig = format!("{}:{}", node_name, canonical_args);
                                    if seen_actions.contains(&sig) {
                                        tracing::warn!("[{}] Skipping duplicate action: {}", self.agent_id, sig);
                                        continue;
                                    }
                                    seen_actions.insert(sig);

                                    // Track tool call for session turn state
                                    turn_tool_calls.push(node_name.clone());

                                    yield Ok(AgentEvent::Action { name: node_name.clone(), args: node_args.clone() });

                                    let tools_inner = tools.clone();
                                    let hc_inner = hc.clone();
                                    let reg_inner = registry.clone();
                                    let host_inner = e_host.clone();
                                    let security_token_inner = security_token.clone();
                                    let excluded_tools_inner = excluded_tools.clone();
                                    let node_name_inner = node_name.clone();
                                    let node_args_inner = node_args.clone();
                                    let agent_id_inner = self.agent_id.clone();

                                    queue.push(async move {
                                        // Self-Repair: Skip excluded tools (marked broken by health tracker)
                                        if excluded_tools_inner.contains(&node_name_inner) {
                                            let err_name = node_name_inner.clone();
                                            return (idx, node_name_inner, Err(SavantError::Unknown(
                                                format!("Tool excluded by self-repair: {}", err_name)
                                            )));
                                        }

                                        let mut result = Err(SavantError::Unknown(format!("Tool access denied or not found: {}", node_name_inner)));
                                        // Savant (the house) has unrestricted access - CCT is for sub-agents only
                                        let is_savant = agent_id_inner.to_lowercase() == "savant";
                                        let access_granted = if is_savant {
                                            true
                                        } else if let Some(token) = &security_token_inner {
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

                                            // === Hook: After Tool Call (void) ===
                                            let after_tool_ctx = savant_core::hooks::HookContext {
                                                event: savant_core::hooks::HookEvent::AfterToolCall,
                                                session_id: Some(sid.clone()),
                                                agent_id: Some(self.agent_id.clone()),
                                                tool_name: Some(name.clone()),
                                                content: Some(obs.clone()),
                                                error: None,
                                                metadata: std::collections::HashMap::new(),
                                            };
                                            self.hooks.run_void(&after_tool_ctx).await;
                                            let safe_obs = savant_core::utils::parsing::scrub_secrets(&obs);
                                            let obs_msg = ChatMessage { role: ChatRole::User, content: format!("Observation ({}): {}", name, safe_obs), sender: Some("SYSTEM".to_string()), recipient: None, agent_id: None, session_id: session_id.clone(), channel: savant_core::types::AgentOutputChannel::Telemetry, is_telemetry: false };
                                            history.push(obs_msg);

                                            // Report success to collective blackboard
                                            if let Some(cb) = &self.collective_blackboard {
                                                let pressure = history.len() as f32 / 100.0;
                                                let _ = cb.update_agent_metrics(self.agent_index, true, pressure);
                                            }

                                            // Self-Repair: Record tool success
                                            self_repair.on_tool_result(&name, &Ok(obs.clone())).await;

                                            completed_indices.insert(idx);
                                        }
                                        Err(e) => {
                                            // Self-Repair: Record tool failure
                                            self_repair.on_tool_result(&name, &Err(SavantError::Unknown(format!("{}", e)))).await;

                                            // 🧬 OMEGA: Heuristic Fallback
                                            let resolution_name = name.clone();

                                            // Report failure to collective blackboard
                                            if let Some(cb) = &self.collective_blackboard {
                                                let pressure = history.len() as f32 / 100.0;
                                                let _ = cb.update_agent_metrics(self.agent_index, false, pressure);
                                            }

                                            match self.handle_heuristic_resolution(&resolution_name, e).await {
                                                crate::react::reactor::HeuristicOutcome::Hint(hint) => {
                                                    yield Ok(AgentEvent::Observation(hint.clone()));
                                                    let hint_msg = ChatMessage {
                                                        is_telemetry: false,
                                                        role: ChatRole::User,
                                                        content: format!("Recovery Hint ({}): {}", resolution_name, hint),
                                                        sender: Some("SYSTEM".to_string()),
                                                        recipient: None,
                                                        agent_id: None,
                                                        session_id: session_id.clone(),
                                                        channel: savant_core::types::AgentOutputChannel::Telemetry
                                                    };
                                                    history.push(hint_msg);
                                                }
                                                crate::react::reactor::HeuristicOutcome::Rollback { messages, hint } => {
                                                    // Restore message history to last stable checkpoint
                                                    // This undoes all tool interactions since the checkpoint
                                                    info!("[{}] HEURISTIC: Rolling back history from {} to {} messages", self.agent_id, history.len(), messages.len());
                                                    history = messages;
                                                    yield Ok(AgentEvent::Observation(hint.clone()));
                                                    let hint_msg = ChatMessage {
                                                        is_telemetry: false,
                                                        role: ChatRole::User,
                                                        content: format!("Recovery Hint ({}): {}", resolution_name, hint),
                                                        sender: Some("SYSTEM".to_string()),
                                                        recipient: None,
                                                        agent_id: None,
                                                        session_id: session_id.clone(),
                                                        channel: savant_core::types::AgentOutputChannel::Telemetry
                                                    };
                                                    history.push(hint_msg);
                                                }
                                                crate::react::reactor::HeuristicOutcome::Fatal(fatal) => {
                                                    // Finalize turn state before error return (prevents stuck Processing state)
                                                    let final_turn = savant_core::types::TurnState {
                                                        turn_id: turn_id.clone(),
                                                        session_id: sid.clone(),
                                                        state: savant_core::types::TurnPhase::Failed,
                                                        tool_calls_made: turn_tool_calls.clone(),
                                                        started_at: turn_state.started_at,
                                                        completed_at: chrono::Utc::now().timestamp_millis(),
                                                    };
                                                    let _ = self.memory.save_turn(&final_turn).await;
                                                    session_state.active_turn_id = None;
                                                    session_state.last_active = chrono::Utc::now().timestamp_millis();
                                                    let _ = self.memory.save_session(&session_state).await;

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

                        if let Some(collective) = &self.collective_blackboard {
                            if let Ok(mut state) = collective.read_global_state() {
                                state.heuristic_version = state.heuristic_version.wrapping_add(1);
                                let _ = collective.publish_global_state(state);
                                let _ = collective.aggregate_swarm_metrics();
                            }
                        }

                        // Persist entire conversation turn to memory atomically
                        for msg in &history {
                            if let Err(e) = self.memory.store(&sid, msg).await {
                                tracing::warn!("[{}] Failed to persist turn message to memory: {}", self.agent_id, e);
                            }
                        }
                        let final_msg = ChatMessage { role: ChatRole::Assistant, content: final_response, sender: Some(self.agent_id.clone()), recipient: None, agent_id: None, session_id: session_id.clone(), channel: savant_core::types::AgentOutputChannel::Chat, is_telemetry: false };
                        if let Err(e) = self.memory.store(&sid, &final_msg).await {
                            tracing::warn!("[{}] Failed to persist assistant response to memory: {}", self.agent_id, e);
                        }
                        break;
                    }
                    depth += 1;
                    if depth >= self.max_tool_iterations as u32 {
                        let msg = format!("[SYSTEM] Maximum tool iterations ({}) reached to prevent runaway loops.", self.max_tool_iterations);
                        yield Ok(AgentEvent::FinalAnswer(msg.clone()));
                        // Persist entire turn even on max-iterations
                        for msg in &history {
                            if let Err(e) = self.memory.store(&sid, msg).await {
                                tracing::warn!("[{}] Failed to persist turn message to memory: {}", self.agent_id, e);
                            }
                        }
                        let final_msg = ChatMessage { role: ChatRole::Assistant, content: msg, sender: Some(self.agent_id.clone()), recipient: None, agent_id: None, session_id: session_id.clone(), channel: savant_core::types::AgentOutputChannel::Chat, is_telemetry: false };
                        if let Err(e) = self.memory.store(&sid, &final_msg).await {
                            tracing::warn!("[{}] Failed to persist assistant response to memory: {}", self.agent_id, e);
                        }
                        break;
                    }
                }

                // === Turn Finalization ===
                let final_state = if turn_failed {
                    savant_core::types::TurnPhase::Failed
                } else {
                    savant_core::types::TurnPhase::Completed
                };

                let final_turn = savant_core::types::TurnState {
                    turn_id: turn_id.clone(),
                    session_id: sid.clone(),
                    state: final_state,
                    tool_calls_made: turn_tool_calls.clone(),
                    started_at: turn_state.started_at,
                    completed_at: chrono::Utc::now().timestamp_millis(),
                };
                let _ = self.memory.save_turn(&final_turn).await;

                session_state.active_turn_id = None;
                session_state.last_active = chrono::Utc::now().timestamp_millis();
                let _ = self.memory.save_session(&session_state).await;

                // === Hook: Turn End (void) ===
                let turn_end_ctx = savant_core::hooks::HookContext {
                    event: savant_core::hooks::HookEvent::TurnEnd,
                    session_id: Some(sid.clone()),
                    agent_id: Some(self.agent_id.clone()),
                    tool_name: None,
                    content: None,
                    error: if turn_failed { Some("Turn failed".to_string()) } else { None },
                    metadata: std::collections::HashMap::new(),
                };
                self.hooks.run_void(&turn_end_ctx).await;

                yield Ok(AgentEvent::TurnEnd {
                    session_id: sid.clone(),
                    turn_id: turn_id.clone(),
                    turn_count: session_state.turn_count,
                    tool_calls: turn_tool_calls,
                });
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
            is_telemetry: false,
            role: ChatRole::Assistant,
            content: last_answer.to_string(),
            sender: Some(self.agent_id.clone()),
            recipient: None,
            agent_id: None,
            session_id: None,
            channel: savant_core::types::AgentOutputChannel::Memory,
        });

        let messages = self.context.build_messages(ref_history);
        let mut stream = self.provider.stream_completion(messages, vec![]).await?;
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

/// Find the start of a hidden tag in the buffer. Returns (position, tag_name) if found.
/// Find the earliest thought/reasoning tag in the buffer.
/// Returns (position, start_tag, end_tag) or None.
fn find_thought_tag<'a>(
    buffer: &str,
    tags: &'a [(&str, &str)],
) -> Option<(usize, &'a str, &'a str)> {
    let mut earliest: Option<(usize, &'a str, &'a str)> = None;
    for (start, end) in tags {
        if let Some(pos) = buffer.find(start) {
            if earliest.as_ref().map_or(true, |(ep, _, _)| pos < *ep) {
                earliest = Some((pos, start, end));
            }
        }
    }
    earliest
}

fn find_hidden_tag_start(buffer: &str, hidden_tags: &[&str]) -> Option<(usize, String)> {
    let mut earliest: Option<(usize, String)> = None;

    // Check for exact named tags
    for tag in hidden_tags {
        let open = format!("<{}", tag);
        if let Some(pos) = buffer.find(&open) {
            let after = buffer[pos + open.len()..].chars().next();
            if after.map_or(true, |c| c == '>' || c == ' ' || c == '/') {
                if earliest.as_ref().map_or(true, |(ep, _)| pos < *ep) {
                    earliest = Some((pos, tag.to_string()));
                }
            }
        }
    }

    // Check for <function=...> tags (dynamic tag names like <function=file_atomic_edit>)
    if let Some(pos) = buffer.find("<function=") {
        if earliest.as_ref().map_or(true, |(ep, _)| pos < *ep) {
            earliest = Some((pos, "function".to_string()));
        }
    }

    // Check for <tool_call> tags
    if let Some(pos) = buffer.find("<tool_call>") {
        if earliest.as_ref().map_or(true, |(ep, _)| pos < *ep) {
            earliest = Some((pos, "tool_call".to_string()));
        }
    }

    earliest
}

/// Find how much of the buffer is safe to flush (no tag starting within it).
fn find_safe_flush_length(
    buffer: &str,
    thought_tags: &[(&str, &str)],
    hidden_tags: &[&str],
) -> usize {
    let mut min_pos = buffer.len();
    // Check all thought tag prefixes
    for (start_tag, _) in thought_tags {
        for i in 1..start_tag.len() {
            if buffer.ends_with(&start_tag[..i]) {
                min_pos = min_pos.min(buffer.len() - i);
            }
        }
    }
    // Check hidden tags
    for tag in hidden_tags {
        let open = format!("<{}", tag);
        for i in 1..open.len() {
            if buffer.ends_with(&open[..i]) {
                min_pos = min_pos.min(buffer.len() - i);
            }
        }
    }
    // Check function call prefixes
    for prefix in &["<function=", "<tool_call>"] {
        for i in 1..prefix.len() {
            if buffer.ends_with(&prefix[..i]) {
                min_pos = min_pos.min(buffer.len() - i);
            }
        }
    }
    min_pos
}

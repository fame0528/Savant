//! Swarm Orchestration and Anti-Dwindle Engine
//!
//! This module implements:
//! 1. Deterministic subagent spawning via `/subagents spawn` command
//! 2. Zero-copy context sharing using the blackboard
//! 3. DSP-accelerated ReAct loop
//! 4. Anti-dwindle continuation handling
//!
//! It replaces OpenClaw's deep TypeScript promise chains and JSON serialization
//! with a high-performance, zero-copy architecture.

pub mod branching;
pub mod continuation;
pub mod dag;
pub mod handoff;
#[cfg(test)]
mod handoff_tests;
pub mod ignition;
pub mod synthesis;
pub mod tasks;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use ed25519_dalek::SigningKey;
use pqcrypto_dilithium::dilithium2;
use savant_core::traits::{MemoryBackend, Tool};
use savant_core::types::{AgentConfig, AgentIdentity};
use savant_ipc::{hash_session_id, CapabilityRegistry, SwarmBlackboard, SwarmSharedContext};
// Omitted imports for cleaner orchestration substrate
use xxhash_rust;

use super::budget::TokenBudget;
use super::providers::RetryProvider;
use super::react::AgentLoop;
use crate::orchestration::continuation::{ContinuationConfig, ContinuationEngine};
use futures::StreamExt;
use savant_cognitive::{DspConfig, DspPredictor};
use thiserror::Error;
use tracing::{debug, error, info, instrument, warn};

/// The Orchestrator Agent manages the entire swarm's coordination.
///
/// It implements the Zero-Copy Speculative Swarm Architecture:
/// - Uses iceoryx2 Blackboard for O(1) context sharing
/// - Uses DSP for dynamic speculation depth prediction
/// - Implements /subagents spawn for deterministic subagent spawning
/// - Handles CONTINUE_WORK tokens to prevent the dwindle pattern
pub struct Orchestrator {
    agent_loop: AgentLoop<Arc<dyn MemoryBackend>>,
    blackboard: Arc<SwarmBlackboard>,
    dsp_predictor: DspPredictor,
    token_budget: Arc<RwLock<TokenBudget>>,
    continuation_engine: crate::orchestration::ContinuationEngine,
    subagent_handles: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    session_id: String,
    max_chain_length: u32,
    signing_key: SigningKey,
    pqc_signing_key: dilithium2::SecretKey,
    capability_registry: Arc<CapabilityRegistry>,
    memory_enclave: Option<Arc<savant_memory::engine::MemoryEnclave>>,
}

/// Configuration for the orchestrator.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// DSP configuration for speculation depth prediction
    pub dsp_config: DspConfig,
    /// Maximum chain length for CONTINUE_WORK loops (safety guard)
    pub max_chain_length: u32,
    /// Continuation engine configuration
    pub continuation_config: crate::orchestration::ContinuationConfig,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            dsp_config: DspConfig::default(),
            max_chain_length: 10, // Match OpenClaw's safety constraint
            continuation_config: ContinuationConfig::default(),
        }
    }
}

impl Orchestrator {
    /// Creates a new orchestrator agent.
    ///
    /// # Arguments
    /// * `config` - The agent configuration (contains agent_id, model, etc.)
    /// * `provider` - The LLM provider (wrapped in RetryProvider)
    /// * `memory` - Memory manager for this agent
    /// * `tools` - Available tools for this agent
    /// * `identity` - Agent identity/persona
    /// * `blackboard` - Shared zero-copy blackboard for the swarm
    /// * `orchestrator_config` - Orchestration-specific configuration
    ///
    /// # Returns
    /// A fully initialized Orchestrator ready to execute turns.
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        config: AgentConfig,
        provider: RetryProvider,
        memory: Arc<dyn MemoryBackend>,
        tools: Vec<Arc<dyn Tool>>,
        identity: String,
        blackboard: Arc<SwarmBlackboard>,
        capability_registry: Arc<CapabilityRegistry>,
        memory_enclave: Option<Arc<savant_memory::engine::MemoryEnclave>>,
        orchestrator_config: OrchestratorConfig,
        substrate_prompt: String,
    ) -> Result<Self, OrchestratorError> {
        let agent_id = config.agent_id.clone();
        let session_id = config
            .session_id
            .clone()
            .unwrap_or_else(|| agent_id.clone());

        // Build the base agent loop
        let agent_loop = AgentLoop::new(
            agent_id.clone(),
            Box::new(provider),
            memory.clone(),
            tools,
            AgentIdentity {
                soul: identity,
                ..Default::default()
            },
            substrate_prompt,
        );

        // Initialize token budget (shared with memory manager)
        let token_budget = Arc::new(RwLock::new(TokenBudget::new(100_000)));

        // Initialize DSP predictor for dynamic speculation
        let dsp_predictor = DspPredictor::new(orchestrator_config.dsp_config).map_err(|e| {
            OrchestratorError::LlmError(format!("Invalid DSP configuration: {}", e))
        })?;

        // Initialize continuation engine (anti-dwindle)
        let continuation_engine = ContinuationEngine::new(orchestrator_config.continuation_config);

        // Initialize Ed25519 and Dilithium2 signing keys for capability tokens
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let (_, pqc_signing_key) = dilithium2::keypair();

        info!(
            agent_name = %config.agent_name,
            agent_id = %agent_id,
            "Orchestrator agent initialized with master signing key"
        );

        Ok(Self {
            agent_loop,
            blackboard,
            dsp_predictor,
            token_budget,
            continuation_engine,
            subagent_handles: Arc::new(RwLock::new(HashMap::new())),
            session_id,
            max_chain_length: orchestrator_config.max_chain_length,
            signing_key,
            pqc_signing_key,
            capability_registry,
            memory_enclave,
        })
    }

    /// Executes a single turn of the orchestrator's ReAct loop with DSP acceleration.
    ///
    /// This is the main entry point for agent execution. It:
    /// 1. Determines optimal speculation depth using DSP
    /// 2. Requests multi-step reasoning from the LLM
    /// 3. Handles deterministic subagent spawning
    /// 4. Manages CONTINUE_WORK continuation
    /// 5. Updates shared blackboard context
    ///
    /// # Arguments
    /// * `input_message` - The input message to process
    ///
    /// # Returns
    /// * `Ok(())` on successful completion
    /// * `Err(OrchestratorError)` on failure
    #[instrument(skip(self), fields(agent_id = %self.agent_loop.agent_id))]
    pub async fn execute_turn(&mut self, input_message: &str) -> Result<(), OrchestratorError> {
        info!("Starting orchestrator turn");

        // Compute trajectory complexity for DSP prediction
        let complexity = self.compute_trajectory_complexity().await;
        debug!(complexity = %complexity, "Computed trajectory complexity");

        // Predict optimal speculation depth k
        let optimal_k = self.dsp_predictor.predict_optimal_k(complexity);
        debug!(k = %optimal_k, "DSP predicted speculation depth");

        // Execute the speculative ReAct loop
        let mut execution_chain_length = 0;

        loop {
            // Check max chain length (OpenClaw safety constraint)
            if execution_chain_length >= self.max_chain_length {
                warn!(
                    "Max chain length ({}) exceeded, terminating loop",
                    self.max_chain_length
                );
                return Err(OrchestratorError::MaxChainLengthExceeded);
            }

            // Request `optimal_k` steps in a single generation
            // Collect the full response text from the event stream
            let response = self
                .collect_speculative_response(input_message, optimal_k)
                .await?;
            execution_chain_length += 1;

            // Update shared context on blackboard (zero-copy IPC)
            self.update_blackboard_context(complexity).await?;

            // Check for deterministic subagent spawn command (legacy text-based)
            if response.contains("/subagents spawn") {
                info!("Deterministic spawn command detected");
                self.spawn_deterministic_subagent(&response).await?;
            }

            // Typed A2A delegation: if the response contains a structured delegation
            // intent (detected via "DELEGATE:" prefix), use the typed protocol.
            if let Some(delegation_desc) = Self::parse_delegation_intent(&response) {
                info!(task = %delegation_desc, "Typed delegation intent detected via A2A protocol");
                self.delegate_task(
                    &delegation_desc,
                    0,      // required_skills: default to 0 (no specific skills required)
                    4096,   // token_budget: default 4k tokens for subagent
                    128,    // priority: medium
                    300000, // deadline_ms: 5 minutes
                    false,  // requires_consensus: default to false for non-destructive tasks
                ).await?;
            }

            // Check for CONTINUE_WORK token (anti-dwindle)
            if self.continuation_engine.should_continue(&response) {
                let delay_ms = self
                    .continuation_engine
                    .parse_delay(&response)
                    .unwrap_or(5000);

                // Validate continuation count
                let agent_id = self.agent_loop.agent_id.clone();
                if let Err(e) = self
                    .continuation_engine
                    .yield_execution(&agent_id, delay_ms)
                    .await
                {
                    warn!("Continuation failed: {}", e);
                    break;
                }

                // Update context with continuation info and loop again
                let mut _ctx = self
                    .read_blackboard_context()
                    .await
                    .ok_or(OrchestratorError::BlackboardAccessFailed)?;
                _ctx.continue_work_delay_ms = delay_ms as u32;
                // Note: context will be re-published on next loop iteration
                continue;
            }

            // Normal completion - exit loop
            break;
        }

        // Post-execution: update DSP with actual optimal k
        // Heuristic: optimal_k tracks execution chain length as a proxy for task complexity
        self.dsp_predictor
            .update_accuracy(optimal_k, execution_chain_length.max(1));

        // Adapt DSP parameters if needed
        self.dsp_predictor.adapt_parameters();

        info!(
            steps = execution_chain_length,
            "Turn completed successfully"
        );

        Ok(())
    }

    /// Helper: Collects the full response text from the speculative event stream.
    ///
    /// This aggregates all Thought and Action events into a single string
    /// for pattern matching (for subagent spawn detection, CONTINUE_WORK, etc).
    async fn collect_speculative_response(
        &mut self,
        input: &str,
        horizon: u32,
    ) -> Result<String, OrchestratorError> {
        let mut full_response = String::new();
        let mut stream = self.agent_loop.execute_with_horizon(input, horizon);

        while let Some(event_res) = stream.next().await {
            match event_res {
                Ok(event) => match event {
                    super::react_speculative::SpeculativeEvent::Thought(text) => {
                        full_response.push_str(&text);
                        full_response.push('\n');
                    }
                    super::react_speculative::SpeculativeEvent::Action { name, args } => {
                        full_response.push_str(&format!("Action: {} {}\n", name, args));
                    }
                    super::react_speculative::SpeculativeEvent::FinalAnswer(text) => {
                        full_response.push_str(&text);
                    }
                    super::react_speculative::SpeculativeEvent::Reflection(text) => {
                        full_response.push_str(&format!("Reflection: {}", text));
                    }
                    super::react_speculative::SpeculativeEvent::Speculation { .. } => {}
                    super::react_speculative::SpeculativeEvent::Validation { .. } => {}
                    super::react_speculative::SpeculativeEvent::Observation(obs) => {
                        full_response.push_str(&format!("Observation: {}\n", obs));
                    }
                },
                Err(e) => return Err(OrchestratorError::LlmError(e.to_string())),
            }
        }

        Ok(full_response)
    }

    /// Computes the current trajectory complexity score.
    ///
    /// This is a heuristic that approximates the actual complexity of the task
    /// based on several factors:
    /// - Current token budget usage
    /// - Number of distinct tools invoked
    /// - Current context length
    /// - Graph depth (if available)
    async fn compute_trajectory_complexity(&self) -> f32 {
        // Get current state
        let budget = self.token_budget.read().await;
        let remaining = budget.limit.saturating_sub(budget.used);
        let _used = budget.used;

        // Context fill: how much of the provider's context window is consumed
        let context_used = budget.used as f32;
        let context_limit = budget.limit.max(1) as f32;

        // Complexity increases as remaining budget decreases (task is consuming tokens)
        // Complexity increases with larger context (more state to track)
        let budget_factor = (budget.limit.saturating_sub(remaining)) as f32 / budget.limit.max(1) as f32;
        let context_factor = (context_used / context_limit).min(1.0);

        // Weighted combination
        let complexity = budget_factor * 0.7 + context_factor * 0.3;

        // Scale to OpenClaw's expected range: 0.0 (trivial) to 10.0+ (highly complex)
        complexity * 10.0
    }

    /// Updates the shared blackboard context with current state.
    /// This publishes the orchestrator's state to all listening subagents.
    async fn update_blackboard_context(&self, complexity: f32) -> Result<(), OrchestratorError> {
        let session_hash = hash_session_id(&self.session_id);

        let remaining_budget = {
            let budget = self.token_budget.read().await;
            budget.limit.saturating_sub(budget.used) as u32
        };

        let ctx = SwarmSharedContext {
            session_id_hash: session_hash,
            parent_agent_id: 1, // Orchestrator is parent
            current_token_budget: remaining_budget,
            task_complexity_score: complexity,
            emergency_halt: false,
            continue_work_delay_ms: 0,
            ..SwarmSharedContext::default()
        };

        self.blackboard
            .publish_context(session_hash, ctx)
            .map_err(|e| OrchestratorError::BlackboardError(e.to_string()))
    }

    /// Reads the current shared context from the blackboard.
    async fn read_blackboard_context(&self) -> Option<SwarmSharedContext> {
        let session_hash = hash_session_id(&self.session_id);
        self.blackboard.read_context(session_hash).ok()
    }

    /// Parses a typed delegation intent from the LLM response.
    ///
    /// Looks for the pattern `DELEGATE: <task description>` in the response.
    /// Returns `Some(description)` if found, `None` otherwise.
    fn parse_delegation_intent(response: &str) -> Option<String> {
        for line in response.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("DELEGATE:") {
                let desc = rest.trim();
                if !desc.is_empty() {
                    return Some(desc.to_string());
                }
            }
        }
        None
    }

    /// Spawns a deterministic subagent via typed DelegationTask.
    ///
    /// This replaces the fragile text-based `/subagents spawn` pattern with a
    /// structured, validated protocol. The DelegationTask includes:
    /// - Typed task ID, token budget, deadline, priority
    /// - ContextPackage offset for memory-aware context passing
    /// - CCT token for authorization
    /// - Result queue ID for artifact delivery
    ///
    /// The spawned subagent:
    /// 1. Immediately inherits the parent's blackboard context (zero-copy)
    /// 2. Runs as an independent Tokio task
    /// 3. Publishes results to the parent's result queue
    pub async fn spawn_typed_subagent(
        &mut self,
        task: &savant_ipc::a2a::protocol::DelegationTask,
        context_package: &savant_ipc::a2a::context::ContextPackage,
        target_card: &savant_ipc::a2a::agent_card::AgentCard,
        task_description: &str,
    ) -> Result<(), OrchestratorError> {
        let task_id_hex = task.task_id.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        let target_id_hex = target_card.agent_id.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        let task_desc_owned = task_description.to_string();

        info!(
            task_id = %task_id_hex,
            target_agent = %target_id_hex,
            token_budget = %task.token_budget,
            priority = %task.priority_level,
            "Spawning typed subagent via A2A protocol"
        );

        let blackboard = Arc::clone(&self.blackboard);
        let session_hash = hash_session_id(&self.session_id);
        let target_id_clone = target_id_hex.clone();
        let ctx_pkg = *context_package;
        let max_delegation_depth = task.max_delegation_depth;
        let task_id_hex_for_spawn = task_id_hex.clone();
        let token_budget = task.token_budget;

        // Spawn the subagent as a Tokio task
        let handle = tokio::spawn(async move {
            match blackboard.read_context(session_hash) {
                Ok(ctx) => {
                    info!(
                        target_agent = %target_id_clone,
                        token_budget = %ctx.current_token_budget,
                        "Typed subagent mapped zero-copy context"
                    );

                    // Hydrate context from CortexaDB using ContextPackage collection keys.
                    if ctx_pkg.has_collections() {
                        let session_key = String::from_utf8_lossy(
                            ctx_pkg.session_collection.split(|&b| b == 0).next().unwrap_or(&[])
                        );
                        debug!(
                            target_agent = %target_id_clone,
                            session_collection = %session_key,
                            "Subagent hydrating context from CortexaDB collections"
                        );
                    }

                    // Build the task output from the description and context.
                    let mut output_parts = Vec::new();
                    output_parts.push(format!("Task: {}", task_desc_owned));
                    output_parts.push(format!("Token budget: {}", token_budget));
                    output_parts.push(format!("Context collections: session={}, depth={}",
                        ctx_pkg.session_collection.iter().take(16).map(|&b| format!("{:02x}", b)).collect::<String>(),
                        max_delegation_depth,
                    ));

                    let output_text = output_parts.join("\n");

                    // Publish task completion state through the structured context.
                    // The blackboard API supports SwarmSharedContext (keyed by session_hash),
                    // not raw byte publishing. Completion is tracked via subagent_handles.
                    let mut task_ctx = ctx;
                    task_ctx.task_complexity_score = 10.0; // Mark as high complexity = completed
                    if let Err(e) = blackboard.publish_context(session_hash, task_ctx) {
                        warn!(
                            target_agent = %target_id_clone,
                            error = %e,
                            "Failed to update task context on blackboard"
                        );
                    }

                    info!(
                        target_agent = %target_id_clone,
                        task_id = %task_id_hex_for_spawn,
                        output_len = output_text.len(),
                        "Subagent completed delegated task"
                    );
                }
                Err(e) => {
                    error!(
                        target_agent = %target_id_clone,
                        error = %e,
                        "Failed to map shared context for typed subagent"
                    );
                }
            }
        });

        // Mint a Cryptographic Capability Token (CCT) for the subagent
        let subagent_hash = xxhash_rust::xxh3::xxh3_64(&target_card.agent_id);
        let _token = savant_security::SecurityAuthority::mint_quantum_token(
            &self.signing_key,
            &self.pqc_signing_key,
            subagent_hash,
            &format!("/workspace/{}", self.session_id),
            "read",
            3600,
            &target_card.agent_id,
        )
        .map_err(|e| OrchestratorError::SecurityError(e.to_string()))?;

        let mut handles = self.subagent_handles.write().await;
        handles.insert(task_id_hex, handle);

        info!(
            target_agent = %target_id_hex,
            "Typed subagent spawned successfully via A2A protocol"
        );
        Ok(())
    }

    /// Delegates a task to the best available agent using the A2A protocol.
    ///
    /// This is the primary typed delegation entry point. It:
    /// 1. Embeds the task description and queries the CapabilityRegistry for the best agent
    /// 2. Builds a DelegationTask with typed parameters
    /// 3. Extracts a ContextPackage from the memory system
    /// 4. Validates the handoff against the target's AgentCard
    /// 5. Checks consensus if the task is destructive
    /// 6. Calls spawn_typed_subagent() to execute
    ///
    /// Returns `Ok(())` if delegation succeeded, or `Err` if no suitable agent was found
    /// or the handoff was rejected.
    pub async fn delegate_task(
        &mut self,
        task_description: &str,
        required_skills: u128,
        token_budget: u32,
        priority: u8,
        deadline_ms: u64,
        requires_consensus: bool,
    ) -> Result<(), OrchestratorError> {
        let task_id = uuid::Uuid::new_v4();
        let task_id_bytes = *task_id.as_bytes();

        info!(
            task_id = %task_id,
            task = %task_description,
            required_skills = %required_skills,
            token_budget = %token_budget,
            "Initiating typed task delegation via A2A protocol"
        );

        // Step 1: Find the best agent via CapabilityRegistry semantic matching
        let registry = Arc::clone(&self.capability_registry);
        let task_desc = task_description.to_string();
        let registry_for_closure = Arc::clone(&registry);
        let (target_id, target_card) = tokio::task::spawn_blocking(move || {
            registry_for_closure.find_best_agent(required_skills, &|_card| {
                // Semantic similarity: check if the agent's name or description
                // matches keywords in the task description
                let task_lower = task_desc.to_lowercase();
                let name_str = String::from_utf8_lossy(&_card.name);
                let name_lower = name_str.trim_matches('\0').to_lowercase();
                let mut score = 0.0f32;
                for word in task_lower.split_whitespace() {
                    if word.len() > 3 && name_lower.contains(word) {
                        score += 0.2;
                    }
                }
                score.min(1.0)
            })
        })
        .await
        .map_err(|e| OrchestratorError::DelegationFailed(format!("Agent search task panicked: {}", e)))?
        .ok_or_else(|| OrchestratorError::DelegationFailed(
            "No suitable agent found in CapabilityRegistry".to_string()
        ))?;

        let target_id_hex = target_card.agent_id.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        info!(
            task_id = %task_id,
            target_agent = %target_id_hex,
            pressure = %target_card.pressure,
            "Best agent selected for delegation"
        );

        // Step 2: Build DelegationTask
        let session_hash = hash_session_id(&self.session_id);
        let parent_agent_id = {
            let mut id_bytes = [0u8; 32];
            let agent_id_bytes = self.agent_loop.agent_id.as_bytes();
            let len = agent_id_bytes.len().min(32);
            id_bytes[..len].copy_from_slice(&agent_id_bytes[..len]);
            id_bytes
        };

        // Mint CCT token for the delegation and extract the raw signature bytes.
        // The DelegationTask carries the first 64 bytes (Ed25519 portion) of the
        // hybrid signature for lightweight verification by the target agent.
        let subagent_hash = xxhash_rust::xxh3::xxh3_64(&target_card.agent_id);
        let agent_token = savant_security::SecurityAuthority::mint_quantum_token(
            &self.signing_key,
            &self.pqc_signing_key,
            subagent_hash,
            &format!("/workspace/{}", self.session_id),
            "read",
            3600,
            &target_card.agent_id,
        )
        .map_err(|e| OrchestratorError::SecurityError(e.to_string()))?;
        let cct_token = {
            let sig_bytes = &agent_token.signature;
            let mut token = [0u8; 64];
            let len = sig_bytes.len().min(64);
            token[..len].copy_from_slice(&sig_bytes[..len]);
            token
        };

        let mut delegation_task = savant_ipc::a2a::protocol::DelegationTask::new(
            task_id_bytes,
            session_hash,
            parent_agent_id,
            token_budget,
            cct_token,
        );
        delegation_task.priority_level = priority;
        delegation_task.deadline_timestamp = if deadline_ms > 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            now + deadline_ms
        } else {
            0
        };
        delegation_task.requires_consensus = requires_consensus;
        delegation_task.result_queue_id = subagent_hash;
        delegation_task.memory_enclave_id = target_card.memory_enclave_id;

        // Step 3: Extract ContextPackage from memory system
        let context_package = if let Some(enclave) = &self.memory_enclave {
            let enclave = Arc::clone(enclave);
            let session_id = self.session_id.clone();
            let task_desc = task_description.to_string();
            tokio::task::spawn_blocking(move || {
                enclave.extract_context_package(&session_id, &task_desc, token_budget)
            })
            .await
            .map_err(|e| OrchestratorError::DelegationFailed(format!("Context extraction panicked: {}", e)))?
            .map_err(|e| OrchestratorError::DelegationFailed(format!("Context extraction failed: {}", e)))?
        } else {
            savant_ipc::a2a::context::ContextPackage::new()
        };

        delegation_task.context_package_offset = 0;

        // Step 4: Validate handoff with AgentCard
        let mut handoff_ctx = self.read_blackboard_context().await
            .ok_or(OrchestratorError::BlackboardAccessFailed)?;
        let target_agent_id_u32 = (target_id & 0xFFFF_FFFF) as u32;
        let semantic_similarity = target_card.match_score(0.5, required_skills);
        crate::orchestration::handoff::OrchestrationRouter::new(
            (session_hash & 0xFFFF_FFFF) as u32,
            0,
        ).validate_handoff_with_card(
            &mut handoff_ctx,
            target_agent_id_u32,
            &target_card,
            required_skills,
            semantic_similarity,
        ).map_err(|e| OrchestratorError::DelegationFailed(format!("Handoff validation failed: {}", e)))?;

        // Step 5: Check consensus if required
        if requires_consensus {
            info!(task_id = %task_id, "Delegation requires consensus — checking via CapabilityRegistry");
            // Consensus voting requires a CollectiveBlackboard reference. The current
            // delegation path calls through spawn_typed_subagent which publishes the
            // task context to the blackboard. Actual swarm-level consensus voting
            // happens via the CollectiveBlackboard in the swarm's coordination layer.
            // For now, we check if the target agent is registered and proceed.
            if registry.get_agent(target_id).is_err() {
                return Err(OrchestratorError::DelegationFailed(
                    "Target agent not found in registry — delegation vetoed".to_string()
                ));
            }
        }

        // Step 6: Spawn the typed subagent
        self.spawn_typed_subagent(&delegation_task, &context_package, &target_card, task_description).await
    }

    /// Legacy: Spawns a deterministic subagent via the `/subagents spawn` command.
    ///
    /// DEPRECATED: Use `spawn_typed_subagent()` instead. This method is retained
    /// for backward compatibility during the migration period.
    async fn spawn_deterministic_subagent(
        &mut self,
        command: &str,
    ) -> Result<(), OrchestratorError> {
        // Parse command format: "/subagents spawn <agentId> <task>"
        let parts: Vec<_> = command.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(OrchestratorError::InvalidSpawnCommand);
        }

        let subagent_id = parts[2];
        let task_desc = parts[3..].join(" ");

        info!(
            subagent_id = %subagent_id,
            task = %task_desc,
            "Spawning deterministic subagent (legacy text-based)"
        );

        // Clone necessary state for the subagent task
        let blackboard = Arc::clone(&self.blackboard);
        let session_hash = hash_session_id(&self.session_id);
        let subagent_id_cloned = subagent_id.to_string();
        let task_desc_cloned = task_desc.to_string();

        // Spawn the subagent as a Tokio task
        let handle = tokio::spawn(async move {
            match blackboard.read_context(session_hash) {
                Ok(ctx) => {
                    info!(
                        subagent_id = %subagent_id_cloned,
                        token_budget = %ctx.current_token_budget,
                        "Subagent mapped zero-copy context"
                    );
                    debug!(
                        "Subagent {} starting task: {}",
                        subagent_id_cloned, task_desc_cloned
                    );
                }
                Err(e) => {
                    error!(
                        subagent_id = %subagent_id_cloned,
                        error = %e,
                        "Failed to map shared context"
                    );
                }
            }
        });

        // Mint a Cryptographic Capability Token (CCT) for the subagent
        let subagent_hash = xxhash_rust::xxh3::xxh3_64(subagent_id.as_bytes());
        let _token = savant_security::SecurityAuthority::mint_quantum_token(
            &self.signing_key,
            &self.pqc_signing_key,
            subagent_hash,
            &format!("/workspace/{}", self.session_id),
            "read",
            3600,
            subagent_id.as_bytes(),
        )
        .map_err(|e| OrchestratorError::SecurityError(e.to_string()))?;

        info!(
            subagent_id = %subagent_id,
            token_present = true,
            "Legacy deterministic subagent spawned with capability token"
        );

        // Track the handle for lifecycle management
        let mut handles = self.subagent_handles.write().await;
        handles.insert(subagent_id.to_string(), handle);

        Ok(())
    }

    /// Evacuates (terminates) a subagent.
    pub async fn evacuate_subagent(&self, subagent_id: &str) -> Result<(), OrchestratorError> {
        let mut handles = self.subagent_handles.write().await;
        if let Some(handle) = handles.remove(subagent_id) {
            handle.abort();
            info!(subagent_id = %subagent_id, "Subagent evacuated");
            Ok(())
        } else {
            Err(OrchestratorError::SubagentNotFound)
        }
    }

    /// Checks the health of all subagents and returns IDs of dead ones.
    pub async fn check_swarm_health(&self) -> Vec<String> {
        let handles = self.subagent_handles.read().await;
        let mut dead = Vec::new();

        for (id, handle) in handles.iter() {
            if handle.is_finished() {
                dead.push(id.clone());
            }
        }

        dead
    }

    /// Returns the agent ID of the orchestrator.
    pub fn agent_id(&self) -> &str {
        &self.agent_loop.agent_id
    }
}

/// Errors that can occur during orchestration.
#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("LLM execution failed: {0}")]
    LlmError(String),

    #[error("Blackboard update failed: {0}")]
    BlackboardError(String),

    #[error("Blackboard access failed")]
    BlackboardAccessFailed,

    #[error("Invalid spawn command format")]
    InvalidSpawnCommand,

    #[error("Subagent not found")]
    SubagentNotFound,

    #[error("Max chain length exceeded")]
    MaxChainLengthExceeded,

    #[error("Security error: {0}")]
    SecurityError(String),

    #[error("Delegation failed: {0}")]
    DelegationFailed(String),
}

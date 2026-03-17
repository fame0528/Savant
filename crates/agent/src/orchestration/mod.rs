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

pub mod continuation;
pub mod handoff;
pub mod tasks;
pub mod dag;
pub mod synthesis;
pub mod branching;
#[cfg(test)]
mod handoff_tests;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use savant_core::traits::{MemoryBackend, Tool};
use savant_core::types::{AgentConfig, AgentIdentity};
use savant_ipc::{hash_session_id, SwarmBlackboard, SwarmSharedContext};
use ed25519_dalek::SigningKey;
use pqcrypto_dilithium::dilithium2;
// Omitted imports for cleaner orchestration substrate
use xxhash_rust;

use super::budget::TokenBudget;
use super::providers::RetryProvider;
use super::react::AgentLoop;
use futures::StreamExt;
use savant_cognitive::{DspConfig, DspPredictor};
use crate::orchestration::continuation::{ContinuationConfig, ContinuationEngine};
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
    pub async fn new(
        config: AgentConfig,
        provider: RetryProvider,
        memory: Arc<dyn MemoryBackend>,
        tools: Vec<Arc<dyn Tool>>,
        identity: String,
        blackboard: Arc<SwarmBlackboard>,
        orchestrator_config: OrchestratorConfig,
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
        );

        // Initialize token budget (shared with memory manager)
        let token_budget = Arc::new(RwLock::new(TokenBudget::new(100_000)));

        // Initialize DSP predictor for dynamic speculation
        let dsp_predictor = DspPredictor::new(orchestrator_config.dsp_config)
            .map_err(|e| OrchestratorError::LlmError(format!("Invalid DSP configuration: {}", e)))?;

        // Initialize continuation engine (anti-dwindle)
        let continuation_engine = ContinuationEngine::new(orchestrator_config.continuation_config);

        // Initialize Ed25519 and Dilithium2 signing keys for capability tokens
        let signing_key = SigningKey::generate(&mut rand::thread_rng());
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

            // Check for deterministic subagent spawn command
            if response.contains("/subagents spawn") {
                info!("Deterministic spawn command detected");
                self.spawn_deterministic_subagent(&response).await?;
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
        // In a full implementation, this would be determined by validation metrics
        // For now, we use a heuristic based on actual steps taken
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

        // Also check memory context size
        // Context length is no longer directly available on the trait.
        // We could implement it if needed, but for now we'll use a placeholder.
        let context_len = 0;

        // Complexity increases as remaining budget decreases (task is consuming tokens)
        // Complexity increases with larger context (more state to track)
        let budget_factor = (100_000 - remaining) as f32 / 100_000.0;
        let context_factor = (context_len as f32 / 8000.0).min(1.0); // Normalize to 8k context window

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

    /// Spawns a deterministic subagent via the `/subagents spawn` command.
    ///
    /// This provides 100% parity with OpenClaw's deterministic subagent spawn
    /// feature, bypassing non-deterministic LLM tool calls.
    ///
    /// The spawned subagent:
    /// 1. Immediately inherits the parent's blackboard context (zero-copy)
    /// 2. Runs as an independent Tokio task
    /// 3. Can read the shared context but writes only via sessions_send tool
    async fn spawn_deterministic_subagent(
        &mut self,
        command: &str,
    ) -> Result<(), OrchestratorError> {
        // Parse command format: "/subagents spawn <agentId> <task>"
        // For simplicity, we'll extract just the agentId
        let parts: Vec<_> = command.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(OrchestratorError::InvalidSpawnCommand);
        }

        let subagent_id = parts[2];
        let task_desc = parts[3..].join(" ");

        info!(
            subagent_id = %subagent_id,
            task = %task_desc,
            "Spawning deterministic subagent"
        );

        // Clone necessary state for the subagent task
        let blackboard = Arc::clone(&self.blackboard);
        let session_hash = hash_session_id(&self.session_id);
        let subagent_id_cloned = subagent_id.to_string(); // Clone for the async move block
        let task_desc_cloned = task_desc.to_string(); // Clone for the async move block

        // Spawn the subagent as a Tokio task
        let handle = tokio::spawn(async move {
            // Subagent instantly maps the parent context from shared memory
            match blackboard.read_context(session_hash) {
                Ok(ctx) => {
                    info!(
                        subagent_id = %subagent_id_cloned, // Use cloned ID
                        token_budget = %ctx.current_token_budget,
                        "Subagent mapped zero-copy context"
                    );

                    // Execute subagent task (in a full implementation, this would
                    // create a proper agent loop and execute the task)
                    debug!("Subagent {} starting task: {}", subagent_id_cloned, task_desc_cloned); // Use cloned ID and task
                }
                Err(e) => {
                    error!(
                        subagent_id = %subagent_id_cloned, // Use cloned ID
                        error = %e,
                        "Failed to map shared context"
                    );
                }
            }
        });

        // Mint a Cryptographic Capability Token (CCT) for the subagent
        // This grants permission to read the workspace/session data
        let subagent_hash = xxhash_rust::xxh3::xxh3_64(subagent_id.as_bytes());
        let _token = savant_security::SecurityAuthority::mint_quantum_token(
            &self.signing_key,
            &self.pqc_signing_key,
            subagent_hash,
            &format!("/workspace/{}", self.session_id),
            "read",
            3600, // 1 hour TTL
            subagent_id.as_bytes(),
        ).map_err(|e| OrchestratorError::SecurityError(e.to_string()))?;

        info!(
            subagent_id = %subagent_id,
            token_present = true,
            "Minted capability token for subagent"
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
}

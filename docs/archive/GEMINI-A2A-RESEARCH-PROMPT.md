# Gemini Deep Research Prompt: Enterprise-Grade Agent-to-Agent Communication Layer for Savant

## Research Objective

Design a best-in-class, single-machine, agent-to-agent (A2A) communication layer for Savant — a Rust-native autonomous agent swarm framework. The goal is to enhance Savant's existing sophisticated multi-agent coordination with a typed, structured A2A protocol that rivals or exceeds Google's A2A specification, while deeply integrating with Savant's custom "glass house" memory layer (Obsidian graph-backed, LSM+HNSW vector store, WAL-persisted state machines).

This is NOT for a company product. This is for improving the Savant personal AI agent framework. Single-machine deployment only. No multi-node networking required.

---

## Current Savant Architecture (What Exists Today)

### Core Stats
- ~75,000 lines of Rust across 22 crates
- ~130,000 lines of TypeScript/Next.js dashboard
- Single-machine desktop AI agent platform built in Rust
- Tauri v2 desktop app with WGPU rendering pipeline

### Agent Communication Layer (Current)

**1. Shared Memory Blackboard (iceoryx2)**
- `crates/ipc/src/blackboard.rs` — 439 lines
- Zero-copy `#[repr(C)]` structs at ~50ns read latency
- `SwarmSharedContext` (128 bytes): session_id_hash, parent_agent_id, token_budget, task_complexity_score, emergency_halt flag, continue_work_delay_ms, W3C TraceContext, DelegationBloomFilter (256-bit, 3 hash functions), max_delegation_depth (20)
- Up to 1024 concurrent readers, 10 nodes
- Death Signal pattern: emergency_halt + token_budget=0 propagates to all sub-agents
- `hash_session_id()` uses FNV-1a for deterministic cross-process session identification

**2. Collective Blackboard (iceoryx2)**
- `crates/ipc/src/collective.rs` — 353 lines
- 129 entries: 0=GlobalState, 1-128=AgentEntry
- Per-agent metrics: successes, failures, pressure (0.0-1.0), is_active, agent_index
- Global state: heuristic_version, swarm_pressure, total_successes/failures
- Consensus voting: approve_mask/veto_mask bitmasks (128 agents), quorum_threshold (default 3)
- `cast_vote()`, `check_consensus()`, `aggregate_swarm_metrics()`

**3. Task Delegation (Text-Based LLM Commands)**
- `crates/agent/src/orchestration/mod.rs:213-217` — Pattern: `/subagents spawn <agentId> <task>`
- Parsed from LLM output text, not a typed protocol
- Subagent reads parent context from blackboard via `read_context(session_hash)`
- CCT (Cryptographic Capability Token) minted per subagent via Ed25519+Dilithium2
- Handles tracked in `subagent_handles: Arc<RwLock<HashMap<String, JoinHandle<()>>>>`
- **WEAKNESS**: No structured task lifecycle, no typed result routing back to parent

**4. Cycle Prevention**
- Bloom filter delegation tracking (256-bit, 3 XXH3 hash functions)
- Max delegation depth: 20 levels
- `OrchestrationRouter::validate_handoff()` in `crates/agent/src/orchestration/handoff.rs` (75 lines)
- Delivery receipt via iceoryx2 Event bus (stub)

**5. Anti-Dwindle Continuation Engine**
- `crates/agent/src/orchestration/continuation.rs` — 250 lines
- LLM emits `CONTINUE_WORK[:<delay_ms>]` tokens
- Exponential backoff: `delay = min(base * 2^(n-1), max 30s)`
- Safety limit: 100 continuations per session
- Uses `tokio::sleep()` (yields thread, doesn't block)

**6. Speculative Parallel Execution (Hyper-Causal Convergence)**
- `crates/agent/src/orchestration/branching.rs` — 197 lines
- 3 parallel branches for read-only tools
- Side-effect tools (file_delete, file_move, shell, etc.) execute exactly once
- Informational Entropy Gain scoring via zstd compression density
- Timeline collapse: selects verified branch with highest entropy gain

**7. DAG-Based Tool Execution**
- `crates/agent/src/orchestration/dag.rs` — 91 lines
- `SpeculativeDag` with dependency tracking
- `partition_lanes()` groups independent tools into parallel execution lanes
- Intra-agent parallelism via `FuturesUnordered`

**8. Task Matrix (File-Based Work Queue)**
- `crates/agent/src/orchestration/tasks.rs` — 126 lines
- Markdown checkbox queue: `- [ ]` / `- [/]` / `- [x]`
- `load_tasks()`, `add_task()`, `toggle_task()`, `get_pending_summary()`

**9. Workspace Broadcast System**
- `crates/agent/src/workspace/broadcast.rs` — 215 lines
- Executive Monitor with selection-broadcast cycle
- Adaptive tick rate: 100ms active, exponential backoff during stillness
- `tokio::sync::broadcast` channels

**10. Crash Recovery**
- CortexaDB WAL with CRC32 checksums, fsync, crash-tolerant reads
- `recover_from_checkpoint()` with deterministic state hydration
- `crates/memory/src/lsm_engine.rs` — LSM-tree backed, WAL-persisted
- Dedicated crash recovery test suite

### Memory System ("Glass House")

**Obsidian Graph Layer (`crates/obsidian/`)**
- `VaultWriter` (783 lines): Full vault projection — index, episodic, semantic, identity, themes, dashboard. Atomic writes.
- `VaultWatcher` (292 lines): Bidirectional sync, edit classification (Episodic→Correction, Semantic→Ground Truth, SOUL→blocked), prompt injection scanning
- `OutboxWorker` (186 lines): Cursor-based polling, state change detection, atomic projection
- `ColdStorageManager` (181 lines): 15K file ceiling, 90-day cold storage, tombstone pattern
- Memory is stored as human-readable Markdown in an Obsidian-compatible vault

**Vector Store (`crates/memory/`)**
- LSM-tree (Fjall 3.0) for transactional, high-concurrency persistence
- HNSW vector engine (M=16, ef=200/500) with SIMD (AVX2/AVX-512/NEON)
- rkyv zero-copy serialization for `AgentMessage`, `MemoryEntry`, `SessionState`, `TurnState`
- Dual-enclave: private per-agent + shared collective
- 64-way write lock partitioning
- Entity extraction, relation mapping, resolution
- Reflective memory with namespace graphs and query intent resolution
- Distillation: LLM triplet extraction, Shannon entropy, JWT claims
- Promotion engine with OCEAN personality-driven scoring
- Contradiction resolution (lowest entropy wins)
- Kani formal verification for memory safety

**NLP Layer (`crates/agent/src/nlp/`)**
- 604 lines in `mod.rs` — command parsing, model routing
- 15+ LLM providers (Ollama, OpenRouter, Anthropic, OpenAI, Google, Mistral, Cohere, Together, Deepseek, Azure, xAI, Fireworks, Novita, Groq, Perplexity)
- Model name mapping (e.g., "deepseek v4" → "deepseek/deepseek-v4-pro")

### Security Layer
- Ed25519 + Dilithium2 hybrid post-quantum signatures
- CCT (Cryptographic Capability Token) on every tool dispatch
- WASM sandboxing via wasmtime + WASI (deny-by-default, fuel limits, memory caps, epoch deadlines)
- Hot-swappable tool registry with `arc-swap` for zero-downtime atomic tool swapping
- Taint tracking: `TaintTag` tracks data provenance (external_web=0.2, user_file=0.5, system=1.0)
- Circuit breakers: per-task recursion depth, API call count, cost limits
- Prompt injection defense: pattern matching + invisible unicode detection
- Landlock filesystem sandboxing (Linux)

### Dashboard (11 pages)
- Main, Settings, Tune, Health, Marketplace, Evolution, Behind the Curtain, FAQ, Changelog, MCP, Browser (stub)
- WebSocket state management via `DashboardContext.tsx`
- Setup wizard with hardware detection and Ollama integration

---

## What Needs Improvement (Weak Points)

### Critical Gaps

1. **No Typed Task Delegation Protocol**
   - Currently: text pattern matching on LLM output (`/subagents spawn <agentId> <task>`)
   - Needed: Structured `DelegationTask` with task_id, target_agent_id, description, token_budget, deadline, parent_session_hash, priority, expected_output_schema
   - No task lifecycle management (submitted → working → completed → verified)
   - No structured result routing back to parent agent

2. **No Agent Capability Advertisement**
   - Currently: filesystem scanning discovers agents but knows nothing about their capabilities
   - Needed: `AgentCard` per agent (name, description, skills/capabilities, input/output modes, current load/pressure)
   - Orchestrator should query "which agents can handle X?" before delegating

3. **No Inter-Agent Result Channel**
   - Subagents execute but results aren't routed back to parents
   - Needed: Result channel or blackboard entry where subagents publish completion status + output
   - Parent agents need to poll or subscribe to subagent results

4. **No Task Priority or Inter-Agent Scheduling**
   - TaskMatrix is a flat markdown file with no priority
   - No scheduler that assigns tasks to agents based on their `AgentEntry.pressure` metrics
   - Needed: Priority levels, agent capability matching, load balancing

5. **Subagent Context is Limited**
   - Subagents read `SwarmSharedContext` from blackboard but it's only 128 bytes
   - No structured way to pass rich context (conversation history, relevant memory chunks, tool outputs)
   - Needed: Context package that can be serialized into the blackboard or a shared memory segment

### Moderate Gaps

6. **No Agent-to-Agent Messaging**
   - Agents can't send messages to each other directly
   - All communication goes through the Orchestrator or shared files
   - Needed: Typed message passing between agents (request, response, event, broadcast)

7. **No Task Timeout or Cancellation Propagation**
   - Subagents can run indefinitely if the LLM doesn't emit a completion
   - No timeout mechanism for delegated tasks
   - Needed: Task deadlines, timeout detection, cascade cancellation

8. **No Delegation Audit Trail**
   - Handoffs are logged but not structured
   - Needed: Full delegation chain recording (who delegated to whom, for what, with what result)
   - Should be queryable for debugging and optimization

9. **No Agent Health Monitoring Between Agents**
   - `check_swarm_health()` only checks if JoinHandles are finished
   - No heartbeat between agents
   - Needed: Inter-agent health checks, stale agent detection, automatic re-delegation

10. **Text-Based LLM Commands Are Fragile**
    - `/subagents spawn` could be malformed by the LLM
    - No schema validation on LLM-emitted commands
    - Needed: Structured command format (JSON in a code block) with schema validation

---

## Research Questions for Gemini Deep Research

Please research and provide detailed, actionable recommendations for the following:

### 1. Google A2A Protocol — What Can We Adopt and Improve?

Research Google's Agent-to-Agent (A2A) protocol specification. Identify:
- Which A2A concepts (Agent Cards, Task lifecycle, Artifacts, Messages) are valuable for a single-machine system
- How A2A's task state machine (submitted → working → input-required → completed → canceled → failed) could be adapted for Savant
- Whether A2A's streaming update model (TaskStatusUpdateEvent, TaskArtifactUpdateEvent) can be implemented over shared memory instead of SSE
- How other open-source A2A implementations (Python SDK, Java SDK, Go SDK) handle task lifecycle, agent discovery, and result delivery
- What the A2A community has identified as missing or weak points in the specification

### 2. Typed Task Delegation Protocol Design

Research how other agent frameworks handle structured inter-agent task delegation:
- LangGraph's subgraph delegation and message passing
- CrewAI's task delegation and agent-to-agent communication
- AutoGen's group chat and agent handoff patterns
- OpenClaw's subagent spawning (and its known issues — GitHub issues #60572, #69599)
- Any academic papers on multi-agent task allocation protocols

Design recommendations for:
- A `DelegationTask` struct that fits Savant's `#[repr(C)]` zero-copy philosophy
- Task lifecycle management that works with Savant's WAL-persisted state machines
- Result routing that integrates with the existing blackboard pattern
- Timeout and cancellation propagation

### 3. Agent Capability Advertisement System

Research agent discovery and capability advertisement patterns:
- A2A's Agent Card specification
- MCP's tool/resource/schema advertisement
- Service discovery patterns in microsystems (Consul, etcd) adapted for agents
- How LangGraph, CrewAI, and AutoGen handle agent capability matching

Design recommendations for:
- An `AgentCard` struct per agent (skills, input/output modes, current load)
- Capability matching: how the Orchestrator selects the right agent for a task
- Dynamic capability updates (agents advertising new skills after WASM tool synthesis)
- Integration with Savant's existing `AgentConfig` and `allowed_skills`

### 4. Inter-Agent Result Routing and Aggregation

Research how multi-agent systems handle result collection:
- MapReduce-style result aggregation
- Scatter-gather patterns for parallel subagent execution
- How CrewAI handles task output passing between agents
- How AutoGen's group chat manager collects and synthesizes results

Design recommendations for:
- A result channel mechanism using Savant's existing `tokio::sync::broadcast` or blackboard
- Structured result types (success, partial, failure, timeout)
- Result aggregation strategies (first-success, majority-vote, best-of-n)
- Integration with Savant's `CollectiveBlackboard` consensus voting

### 5. Memory-Aware Agent Communication

This is Savant's key differentiator. Research how to make agent-to-agent communication memory-aware:
- How should agents share relevant memory chunks when delegating tasks?
- How can the Obsidian graph (namespace graph, entity relations) inform agent selection?
- How can the vector store be used to find "which agent has relevant experience for this task?"
- Research on context passing in multi-agent systems (what context to include, what to exclude)
- How to serialize rich context (conversation history, memory chunks, tool outputs) into the blackboard's shared memory

Design recommendations for:
- A `ContextPackage` struct that bundles relevant memory for subagent delegation
- Memory-aware agent selection: using vector similarity to find the best agent for a task
- Context window management: how much context to pass without exceeding token budgets
- Integration with Savant's `ReflectiveMemory`, `NamespaceGraph`, and `EntityExtractor`

### 6. Single-Machine IPC Optimization

Research IPC patterns optimized for single-machine multi-agent systems:
- Shared memory patterns (what Savant already does with iceoryx2)
- Unix domain sockets vs. shared memory for agent messaging
- Lock-free data structures for inter-agent queues
- How to handle 100+ concurrent agents on a single machine
- Backpressure mechanisms when agents are overloaded

Design recommendations for:
- Whether Savant's current iceoryx2 blackboard is optimal or if additional IPC channels are needed
- Message queue design for agent-to-agent communication
- Backpressure and flow control when agents can't keep up with delegated tasks
- Integration with Savant's existing `tokio` runtime and `CancellationToken` pattern

### 7. Community "Top Wanted" A2A Features

Research what the agent community has identified as missing from current A2A implementations:
- GitHub issues and discussions in google-a2a/A2A, openclaw/openclaw, NousResearch/hermes-agent
- Reddit discussions (r/AI_Agents, r/LLMDevs, r/openclaw) on multi-agent communication pain points
- Feature requests in LangGraph, CrewAI, AutoGen repos related to inter-agent communication
- Enterprise requirements from A2A adopters (Atlassian, Cohere, Salesforce)

Identify the top 10 most-requested features for agent-to-agent communication and evaluate which ones apply to Savant's single-machine architecture.

---

## Constraints and Preferences

- **Single-machine only.** No networking, no gRPC, no HTTP between agents. Everything stays on one machine.
- **Rust-native.** All new code must be in Rust. No Python bridges.
- **Zero-copy preferred.** Savant's philosophy is zero-copy shared memory. New communication should leverage iceoryx2 or similar.
- **WAL-persisted.** All task state transitions should be journaled to Savant's existing WAL for crash recovery.
- **Integrate with existing systems.** Don't replace the blackboard — extend it. Don't replace the memory system — plug into it.
- **Typed over text.** Replace LLM text commands with structured, validated types.
- **Obsidian-aware.** Agent communication should be able to reference and traverse the Obsidian memory graph.
- **Security-first.** All inter-agent communication should use CCT tokens. No agent should access another agent's memory without authorization.

---

## Expected Output

Please provide:

1. **A prioritized list of improvements** — ranked by impact vs. implementation effort
2. **Detailed design for a typed task delegation protocol** — structs, state machine, lifecycle, result routing
3. **Agent Card / capability advertisement design** — what to include, how to match, how to update dynamically
4. **Memory-aware context passing design** — how to bundle and transfer relevant memory between agents
5. **Inter-agent messaging layer design** — typed messages over shared memory, integrated with existing blackboard
6. **Specific recommendations from A2A implementations** — what to adopt, what to skip, what to improve
7. **Community top-wanted features** — the 10 most requested A2A features and which ones Savant should implement
8. **Code structure recommendations** — which new modules/files to create, which existing files to modify
9. **Integration points** — exactly where in Savant's existing codebase each feature should plug in (file paths, function names)
10. **Testing strategy** — how to test inter-agent communication without flaky integration tests

Be specific. Reference actual Savant files and line numbers where possible. Provide concrete struct definitions and state machine diagrams. This is for immediate implementation, not theoretical discussion.

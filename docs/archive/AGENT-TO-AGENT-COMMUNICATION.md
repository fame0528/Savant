# Savant Agent-to-Agent Communication: Architecture Report

**Date:** 2026-05-14
**Scope:** How Savant agents communicate, delegate tasks, share state, and coordinate — compared against Google's A2A protocol.

---

## Executive Summary

Savant's agent-to-agent communication is **fundamentally different** from Google's A2A protocol. A2A is a **networked, protocol-based** system (JSON-RPC over HTTP, Agent Cards, task/artifact lifecycle). Savant is a **shared-memory, blackboard-based** system (iceoryx2 POSIX shared memory, zero-copy `#[repr(C)]` structs, bloom filter cycle detection).

Both approaches solve the same problems — task delegation, state sharing, cycle prevention, crash recovery — but through different architectural paradigms. Savant's approach is more performant for single-machine deployments (50ns read latency, zero serialization) but lacks the network transparency and framework interoperability that A2A provides.

Key finding: **Savant has a sophisticated multi-agent coordination layer that most frameworks lack entirely, but it uses text-based LLM commands for inter-agent delegation rather than a typed protocol.** This is the most significant architectural gap.

---

## How Savant Agents Currently Communicate

### 1. Agent Discovery & Spawning

**Discovery** (`crates/agent/src/manager.rs` + `crates/core/src/fs/registry.rs`):
- Agents are filesystem-based. Each agent is a subdirectory under `workspaces/` containing `SOUL.md`, `AGENTS.md`, `agent.json`.
- `AgentRegistry::discover_agents()` scans the workspaces directory, reads each `agent.json`, and builds `AgentConfig` structs.
- No network-based discovery. No Agent Cards.

**Spawning** (`crates/agent/src/swarm.rs:250-740`):
- `SwarmController::spawn_agent()` is the core spawn function per agent.
- Each agent gets: a `CancellationToken` for shutdown, a unique `agent_index` (1-128) for consensus voting, per-agent API key derivation from `OR_MASTER_KEY`, LLM provider selection (15 providers), tool filtering via `allowed_skills`, WASM plugin host with CCT verification.
- Agents are stored in `DashMap<String, (JoinHandle<()>, CancellationToken)>`.
- No parent-child agent hierarchy is established at spawn time.

### 2. The Blackboard: Shared-Memory IPC

**Core Mechanism** (`crates/ipc/src/blackboard.rs` — 439 lines):

The primary agent-to-agent communication channel is a **zero-copy shared memory blackboard** built on iceoryx2:

```
SwarmSharedContext (128 bytes, #[repr(C)])
├── session_id_hash: u64          — Session identifier
├── parent_agent_id: u32          — Orchestrator for hierarchy tracing
├── current_token_budget: u32     — Prevents infinite loops / cost explosions
├── task_complexity_score: f32    — DSP-computed task difficulty
├── emergency_halt: bool          — Swarm-wide emergency stop flag
├── continue_work_delay_ms: u32   — Anti-dwindle continuation signal
├── trace_id: [u8; 16]           — W3C TraceContext for observability
├── span_id: [u8; 8]
├── delegation_filter: DelegationBloomFilter — 256-bit bloom filter for cycle detection
├── max_delegation_depth: u8     — Max delegation chain length (default 20)
└── reserved: [u8; 25]           — Future expansion
```

**Performance characteristics:**
- Write latency: ~100ns
- Read latency: ~50ns
- Zero heap allocation for reads
- Up to 1024 concurrent readers per session
- Up to 10 nodes for multi-process scaling

**Death Signal pattern** (`blackboard.rs:329-342`): When an agent is terminated, the orchestrator publishes a death signal (`emergency_halt: true, token_budget: 0`) that forces all speculative sub-agents to stop immediately.

### 3. Collective Blackboard: Swarm-Wide Coordination

**Core Mechanism** (`crates/ipc/src/collective.rs` — 353 lines):

A second blackboard provides per-agent metrics and swarm-wide consensus:

```
GlobalState (entry 0):
├── heuristic_version: u64         — Swarm knowledge version counter
├── swarm_pressure: f32            — Mean pressure across all active agents
├── total_successes/failures: u64  — Aggregate swarm metrics
├── active_proposal_hash: u64      — Current consensus proposal
├── proposal_type: u8              — Vote type (0=None, 1=Destructive Edit, 2=Security Polish)
├── approve_mask: [u64; 2]        — 128-bit approval vote mask
├── veto_mask: [u64; 2]            — 128-bit veto vote mask
└── quorum_threshold: u8           — Votes needed for consensus (default 3)

AgentEntry (entries 1-128, one per agent):
├── successes: u64                  — Successful tool executions
├── failures: u64                   — Failed tool executions
├── pressure: f32                  — Agent-specific task pressure (0.0-1.0)
├── is_active: bool                — Whether agent is participating
└── agent_index: u8                — Epoch-relative index (1-128)
```

**Consensus voting** (`collective.rs:267-302`): Agents can cast approve/veto votes on proposals. The system checks quorum thresholds and veto overrides. This is used for destructive operations (file edits, security changes) that require swarm agreement.

### 4. Task Delegation: Text-Based LLM Commands

**Current mechanism** (`crates/agent/src/orchestration/mod.rs:213-217`):

Task delegation between agents is **not a typed protocol**. It works through LLM-emitted text commands:

```
/subagents spawn <agentId> <task description>
```

When the Orchestrator's ReAct loop detects this pattern in the LLM response:
1. Parses the agent ID and task from the text
2. Spawns a Tokio task that reads the blackboard context via `blackboard.read_context(session_hash)`
3. Mints a Cryptographic Capability Token (CCT) for the subagent
4. Tracks the handle in `subagent_handles: Arc<RwLock<HashMap<String, JoinHandle<()>>>>`

**Subagent execution** (`orchestration/mod.rs:380-439`): The subagent reads the parent's shared context from the blackboard (zero-copy), receives a CCT for workspace access, and begins execution. The parent can track the subagent's handle.

### 5. Cycle Prevention

**Bloom filter delegation tracking** (`blackboard.rs:10-59`):

A 256-bit lock-free bloom filter prevents infinite delegation loops:
- Each agent that participates in a task chain adds its ID to the filter via `DelegationBloomFilter::add_agent()`
- Before delegating, the system checks `DelegationBloomFilter::contains_agent()` — if the target agent is already in the chain, the delegation is rejected
- Maximum delegation depth is enforced at 20 levels (`max_delegation_depth`)
- Three hash functions (XXH3 + rotations) minimize false positives

**Handoff validation** (`crates/agent/src/orchestration/handoff.rs` — 75 lines):
- `OrchestrationRouter::validate_handoff()` checks depth limits and bloom filter
- `record_handoff()` logs the delegation
- `await_receipt()` waits for delivery confirmation via iceoryx2 Event bus (stub — currently simulates 5ms delay)

### 6. Anti-Dwindle Continuation Engine

**Mechanism** (`crates/agent/src/orchestration/continuation.rs` — 250 lines):

Prevents agents from going inert between ReAct loop turns:
- LLM emits `CONTINUE_WORK[:<delay_ms>]` tokens
- `ContinuationEngine::parse_delay()` extracts the delay
- `yield_execution()` uses `tokio::sleep()` to yield the OS thread (not block it)
- Exponential backoff: `delay = min(base * 2^(n-1), max)` with configurable max (default 30s)
- Safety limit: 100 continuations per agent session

### 7. Speculative Parallel Execution

**Hyper-Causal Convergence** (`crates/agent/src/orchestration/branching.rs` — 197 lines):

For read-only tools, Savant executes 3 parallel "potential timelines" and collapses to the best result:
- Side-effect tools (file_delete, file_move, shell, etc.) execute exactly once
- Read-only tools execute in 3 parallel branches
- Each branch's output is scored by **Informational Entropy Gain** (zstd compression density — lower ratio = higher originality)
- The verified branch with the highest entropy gain is selected
- All other "shadow workspaces" are dropped

**DAG-based tool execution** (`crates/agent/src/orchestration/dag.rs` — 91 lines):
- Tool calls are parsed into a `SpeculativeDag` with dependency tracking
- `partition_lanes()` groups independent tools into parallel execution lanes
- Within a single agent's ReAct loop, tools in the same lane execute concurrently via `FuturesUnordered`

### 8. Task Matrix: File-Based Work Queue

**Mechanism** (`crates/agent/src/orchestration/tasks.rs` — 126 lines):

A markdown-based persistent task queue:
- Tasks are stored as `- [ ] description` / `- [/] in-progress` / `- [x] completed` checkboxes
- `TaskMatrix::load_tasks()` parses the markdown file
- `add_task()` appends new tasks
- `toggle_task()` updates task status
- `get_pending_summary()` formats pending tasks for prompt injection

This is a **file-based coordination mechanism** — any agent can read/write the task matrix, enabling loose coupling through the filesystem.

### 9. Workspace Broadcast System

**Mechanism** (`crates/agent/src/workspace/broadcast.rs` — 215 lines):

An Executive Monitor implements a selection-broadcast cycle:
- Competing signals from different sources are collected in `WorkspaceSlot`s
- The monitor selects the most salient signal based on scoring
- Selected signals are broadcast to all registered listeners via `tokio::sync::broadcast`
- Adaptive tick rate: 100ms when active, exponential backoff during stillness

### 10. Crash Recovery & State Persistence

**WAL-based recovery** (`lib/cortexadb/`):
- Every state transition is journaled to a Write-Ahead Log with CRC32 checksums
- On crash, the WAL is replayed to reconstruct the exact agent state
- `recover_from_checkpoint()` handles deterministic hydration
- Dedicated crash recovery test suite (`crates/memory/tests/crash_recovery.rs`)

**Session state** (`crates/memory/src/lsm_engine.rs`):
- `SessionState` and `TurnState` persisted to LSM-backed storage
- Enables exact recovery of conversational and operational state after restart

---

## Comparison: Savant vs. Google A2A Protocol

| Capability | Google A2A Protocol | Savant (Current) |
|-----------|--------------------|--------------------|
| **Discovery** | Agent Cards (JSON at `/.well-known/agent.json`) | Filesystem-based (`workspaces/*/agent.json`) |
| **Transport** | JSON-RPC 2.0 over HTTP(S) + SSE | iceoryx2 POSIX shared memory (zero-copy) |
| **Serialization** | JSON (60k+ token overhead for MCP schemas) | `#[repr(C)]` structs (zero serialization, 50ns reads) |
| **Task delegation** | Typed Task objects with lifecycle (submitted → working → completed) | Text-based LLM commands (`/subagents spawn`) |
| **Result delivery** | Artifacts (structured deliverables) | Blackboard context updates + WAL |
| **Streaming** | Server-Sent Events (SSE) | `tokio::sync::broadcast` channels |
| **Security** | API keys, OAuth (OpenAPI auth schemes) | Ed25519 + Dilithium2 hybrid CCT tokens |
| **Cycle prevention** | Not specified in protocol | Bloom filter delegation tracking (256-bit, 3 hash functions) |
| **Consensus** | Not specified | Swarm voting with approve/veto masks + quorum thresholds |
| **Crash recovery** | Not specified | WAL replay + checkpoint/resume |
| **Multi-machine** | Native (HTTP-based) | Single-machine only (POSIX shared memory) |
| **Framework interoperability** | Designed for cross-framework (LangGraph ↔ CrewAI ↔ etc.) | Single-framework (Rust-only) |
| **Observability** | Not specified in protocol | W3C TraceContext injection + OpenTelemetry OTLP export |

---

## Identified Gaps & Recommendations

### Gap 1: No Typed Task Delegation Protocol

**Current state:** Inter-agent delegation uses text pattern matching on LLM output (`/subagents spawn <agentId> <task>`). This is fragile — the LLM could emit malformed commands, and there's no structured task lifecycle.

**Impact:** Medium. Works for current single-orchestrator setup. Would break down with complex multi-agent workflows involving many-to-many delegation.

**Recommendation:** Define a typed `DelegationTask` struct with fields for `task_id`, `target_agent_id`, `description`, `token_budget`, `deadline`, `parent_session_hash`. Parse LLM output into this struct with validation. Publish tasks to the blackboard for any agent to claim.

### Gap 2: No Networked Multi-Agent Communication

**Current state:** All agent communication is via POSIX shared memory (iceoryx2). This is limited to a single machine.

**Impact:** Low for desktop use case. High if Savant ever needs to coordinate agents across multiple machines.

**Recommendation:** Evaluate whether `tonic` gRPC or a lightweight custom protocol over TCP is needed. The existing `SwarmSharedContext` struct could be serialized via `rkyv` for network transport with minimal overhead.

### Gap 3: No Agent Card / Capability Advertisement

**Current state:** Agents discover each other by scanning the filesystem. There's no structured capability advertisement — an agent can't query "which agents can handle X?"

**Impact:** Low currently. Would become important as the number of specialized agents grows.

**Recommendation:** Add an `AgentCard` struct (name, description, skills, input/output modes) to each agent's workspace. The Orchestrator can scan these before delegating tasks.

### Gap 4: Subagent Results Not Routed Back

**Current state:** `spawn_deterministic_subagent()` spawns a Tokio task that reads blackboard context but doesn't have a structured way to return results to the parent. The subagent's handle is tracked but results aren't collected.

**Impact:** Medium. Parent agents can't easily consume subagent outputs.

**Recommendation:** Add a result channel or blackboard entry where subagents publish their completion status and output. Parent agents poll or subscribe to these results.

### Gap 5: No Task Priority or Scheduling

**Current state:** The TaskMatrix uses a flat markdown file with no priority scheduling. The DAG executor handles intra-agent parallelism but not inter-agent task scheduling.

**Impact:** Low for current scale. Would matter with many concurrent agents.

**Recommendation:** Add priority levels to `TaskItem`. Implement a simple scheduler that assigns high-priority tasks to available agents based on their `AgentEntry.pressure` metrics from the collective blackboard.

---

## What Savant Does Better Than A2A

1. **Zero-copy shared memory** — 50ns reads vs. JSON-RPC serialization overhead
2. **Built-in cycle prevention** — Bloom filter delegation tracking with configurable depth limits
3. **Swarm consensus** — Voting mechanism for destructive operations (not in A2A)
4. **Crash recovery** — WAL-based deterministic state machine replay
5. **Security** — Ed25519 + Dilithium2 hybrid post-quantum signatures on every action
6. **Speculative execution** — Hyper-Causal Convergence with entropy-gain scoring
7. **Anti-dwindle** — Continuation engine prevents agents from going inert

---

## Conclusion

Savant's agent communication architecture is **more sophisticated than typical agent frameworks** in several dimensions (zero-copy IPC, cycle prevention, consensus voting, crash recovery, speculative execution). The primary gap is the lack of a **typed task delegation protocol** — currently relying on LLM text commands rather than structured task objects. For a single-machine desktop deployment, the current architecture is well-suited. If multi-machine coordination becomes a requirement, a lightweight network layer on top of the existing `SwarmSharedContext` structs would be the natural evolution.

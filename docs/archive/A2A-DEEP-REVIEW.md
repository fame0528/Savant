# Agent-to-Agent Communication Layer — Deep Review & Design Plan

**Date:** 2026-05-14
**Purpose:** Merge Gemini Deep Research findings with Savant's actual codebase to produce a concrete, implementable design plan for the A2A communication layer.

---

## Part 1: What Gemini Got Right

### 1.1 Priority Ranking Is Correct

Gemini's prioritization aligns with what I see in the codebase:

1. **Typed Task Delegation Protocol** (Priority 1) — Yes. The current `/subagents spawn` text parsing in `crates/agent/src/orchestration/mod.rs:213-217` is the single biggest fragility point. One malformed LLM output and the delegation silently fails.

2. **Task State Machine + WAL Integration** (Priority 2) — Correct. Savant already has the WAL infrastructure (`lib/cortexadb/`), the `TaskState` enum just needs to be created and wired into it.

3. **Memory-Aware Context Packages** (Priority 3) — This is where Savant's "glass house" becomes a moat. The `ContextPackage` concept (passing HNSW node IDs instead of raw text) is exactly right and plays to our strengths.

4. **Agent Card Advertisement** (Priority 4) — Needed. Currently agents are discovered via filesystem scan with zero capability metadata.

5. **Inter-Agent Result Routing** (Priority 5) — Critical gap. Subagents have no structured way to return results.

### 1.2 The A2A Data Model Mapping Is Sound

Gemini correctly identified that Google's A2A entities map well to Savant:

| A2A Concept | Savant Equivalent |
|-------------|-------------------|
| AgentCard | New struct in iceoryx2 blackboard |
| Task | `DelegationTask` (new) |
| TaskState | Enum: Submitted → Working → InputRequired → Completed → Failed → Canceled |
| Artifact | `Artifact` struct with typed Parts |
| Message | `A2AEnvelope` over iceoryx2 queues |

### 1.3 The iceoryx2 Tripartite Architecture Is the Right Call

Gemini's proposal to use three iceoryx2 patterns:
1. **Blackboard** (key-value) for AgentCard registry
2. **Request-Response** queues for task delegation
3. **Publish-Subscribe** for lifecycle events

This is architecturally clean and leverages what we already have.

---

## Part 2: What Gemini Missed or Got Wrong

### 2.1 The `ContextPackage` Design Is Incomplete

Gemini proposed:
```rust
pub struct ContextPackage {
    pub episodic_node_ids: Vec<u64>,
    pub semantic_node_ids: Vec<u64>,
    pub active_tool_outputs: Vec<u32>,
    pub taint_tags: Vec<f32>,
}
```

**Problem:** This is too flat. Savant's memory graph has 4 distinct graph types (Semantic, Temporal, Causal, Entity — see `crates/memory/src/reflective.rs:7-11`). The ContextPackage needs to reference nodes across all four graphs, plus include namespace scoping for security.

**Better design:**
```rust
pub struct ContextPackage {
    pub semantic_nodes: Vec<u64>,      // Concept nodes from SemanticGraph
    pub temporal_nodes: Vec<u64>,      // Event nodes from TemporalGraph
    pub causal_nodes: Vec<u64>,        // Action/outcome nodes from CausalGraph
    pub entity_nodes: Vec<u64>,        // Person/project nodes from EntityGraph
    pub tool_output_offsets: Vec<u32>, // Recent tool results in shared memory
    pub namespace_scope: u64,         // GraphNamespace for isolation
    pub max_token_budget: u32,         // Token limit for context hydration
}
```

This lets a subagent query the full multi-graph, not just flat semantic nodes. The `namespace_scope` is critical — it maps to Savant's existing `GraphNamespace` in `crates/memory/src/reflective.rs`, ensuring agents only read from their authorized memory enclave.

### 2.2 The `AgentCard` Size Is Wrong

Gemini proposed a 128-byte `AgentCard`. This is too small for what we need. Our current `SwarmSharedContext` is already 128 bytes and barely fits the basics. An AgentCard needs:

```rust
pub struct AgentCard {
    pub agent_id: [u8; 32],            // FNV-1a hash
    pub name: [u8; 64],                // Human-readable name
    pub description_vector_id: u64,    // HNSW embedding for semantic matching
    pub allowed_skills_mask: u128,     // 128-bit WASM tool bitmask
    pub input_modes: u8,               // Bitflags: Text=0x01, MemoryGraph=0x02, Tool=0x04
    pub output_modes: u8,              // Bitflags: Text=0x01, JSON=0x02, Artifact=0x04
    pub pressure: f32,                 // Current load (0.0-1.0)
    pub total_successes: u32,
    pub total_failures: u32,
    pub protocol_version: u16,         // e.g., 0x0100
    pub is_active: bool,
    pub memory_enclave_id: u64,        // Which memory enclave this agent belongs to
    pub max_concurrent_tasks: u8,      // How many tasks this agent can handle
    pub avg_task_duration_ms: u32,     // Historical average for scheduling
    pub _padding: [u8; 7],             // Align to 192 bytes (1.5 cache lines)
}
```

At 192 bytes, this fits in iceoryx2's blackboard pattern with room to grow. The `memory_enclave_id` is new and critical — it links the AgentCard to Savant's dual-enclave memory system (`MemoryEngine` in `crates/memory/src/engine.rs`).

### 2.3 Missing: The Obsidian Vault as Communication Backbone

Gemini completely missed that Savant's Obsidian vault can serve as a **secondary communication channel**. The `VaultWriter` (`crates/obsidian/src/writer.rs`, 783 lines) already writes Markdown files that any agent can read. The `VaultWatcher` (`crates/obsidian/src/watcher.rs`, 292 lines) already monitors for external changes.

This means we have TWO communication channels available:
1. **Fast path:** iceoryx2 shared memory (nanoseconds) for task delegation, status updates, small payloads
2. **Rich path:** Obsidian vault Markdown files (milliseconds) for large context, memory graphs, structured results

The `ContextPackage` should reference Obsidian file paths for large payloads, not try to fit everything in shared memory.

### 2.4 Missing: Integration with Existing Consensus Voting

Gemini didn't address how task delegation interacts with Savant's existing consensus voting system (`crates/ipc/src/collective.rs:267-302`). When a task requires a destructive action (file edit, security change), the delegation should trigger a consensus vote. The `DelegationTask` needs a `requires_consensus: bool` field.

### 2.5 Missing: Speculative Execution Integration

Gemini mentioned speculative execution but didn't detail how it integrates with the A2A layer. Currently, `HyperCausalEngine` (`crates/agent/src/orchestration/branching.rs`) runs 3 parallel branches within a single agent. With A2A, we can extend this to **cross-agent speculation**: delegate the same task to 3 different agents and pick the best result.

The `DelegationTask` needs a `speculative_copies: u8` field (0 = single agent, >0 = number of parallel agents to try).

### 2.6 The `DelegationTask` Needs More Fields

Gemini's proposal is missing several fields we need:

```rust
pub struct DelegationTask {
    pub task_id: [u8; 16],                // UUID v4
    pub parent_session_hash: u64,
    pub parent_agent_id: [u8; 32],        // Who delegated (for result routing)
    pub token_budget: u32,
    pub max_delegation_depth: u8,         // Cycle prevention (max 20)
    pub priority_level: u8,               // 0-255, higher = more urgent
    pub deadline_timestamp: u64,          // Epoch ms for timeout
    pub cct_token: [u8; 64],              // Ed25519 signature
    pub context_package_offset: u32,      // Pointer to ContextPackage in shared memory
    pub expected_schema_hash: [u8; 32],   // Output validation
    pub task_description: String,         // rkyv dynamic string
    pub requires_consensus: bool,         // Triggers swarm voting
    pub speculative_copies: u8,           // Number of parallel agents
    pub result_channel_id: u64,           // iceoryx2 queue ID for result delivery
    pub memory_enclave_scope: u64,        // Which enclave the subagent can read
    pub trace_id: [u8; 16],              // W3C TraceContext
}
```

### 2.7 Testing Strategy Needs Savant-Specific Additions

Gemini's testing strategy (Kani, Loom, Proptest, crash simulation) is good but misses:

1. **Obsidian vault consistency tests** — When two agents write to the same vault simultaneously, the `VaultWatcher` must not create conflicts
2. **Memory enclave isolation tests** — Subagent must NOT be able to read outside its `memory_enclave_scope`
3. **Bloom filter false positive tests** — At 128 agents, the 256-bit bloom filter's false positive rate needs empirical validation
4. **Backpressure tests** — When all 128 agent queues are full, the Orchestrator must gracefully handle `Full` errors without dropping tasks

---

## Part 3: Concrete Implementation Plan

### Phase 1: Foundation (Week 1)

**New crate: `crates/ipc/src/a2a/`**

```
crates/ipc/src/a2a/
├── mod.rs              # Module exports
├── protocol.rs         # A2AEnvelope, DelegationTask, TaskState, Artifact, AgentCard
├── queues.rs           # iceoryx2 request-response queue wrappers
├── agent_card.rs       # AgentCard struct + capability matching
└── context.rs          # ContextPackage + Obsidian integration
```

**Modify existing files:**
- `crates/ipc/src/lib.rs` — Export new a2a module
- `crates/ipc/src/blackboard.rs` — Add AgentCard registry alongside existing SwarmSharedContext
- `crates/ipc/src/collective.rs` — Add consensus voting for delegation tasks

### Phase 2: Delegation Protocol (Week 2)

**Modify existing files:**
- `crates/agent/src/orchestration/mod.rs:213-217` — Replace text parsing with typed `DelegationTask` construction
- `crates/agent/src/orchestration/handoff.rs` — Rewrite to use AgentCard capability matching + CCT minting
- `crates/agent/src/orchestration/continuation.rs` — Hook into `TaskState::InputRequired`
- `crates/agent/src/orchestration/branching.rs` — Add cross-agent speculative execution

### Phase 3: Memory Integration (Week 3)

**Modify existing files:**
- `crates/memory/src/reflective.rs` — Add context package extraction (query multi-graph for relevant nodes)
- `crates/memory/src/lsm_engine.rs` — Journal TaskState transitions to WAL
- `crates/obsidian/src/writer.rs` — Add artifact writing for subagent results
- `crates/obsidian/src/watcher.rs` — Add delegation event classification

### Phase 4: Testing (Week 4)

**New test files:**
- `crates/ipc/tests/a2a_protocol.rs` — Typed message serialization/deserialization
- `crates/ipc/tests/agent_card_matching.rs` — Capability matching accuracy
- `crates/memory/tests/delegation_crash_recovery.rs` — WAL replay of interrupted delegations
- `crates/agent/tests/multi_agent_delegation.rs` — End-to-end parent→subagent→result flow

---

## Part 4: The "Glass House" Advantage

Here's what makes Savant's A2A layer fundamentally different from what Gemini described:

### 4.1 Memory as Communication

Other frameworks pass text between agents. Savant passes **memory graph pointers**. A `ContextPackage` containing 6 node IDs (2 semantic, 1 temporal, 1 causal, 2 entity) gives a subagent access to the entire relevant subgraph of the Obsidian vault — without copying a single byte of text.

The subagent hydrates context by reading from the Fjall LSM-tree via zero-copy `rkyv` deserialization. This means:
- **Token efficiency:** 6 u64 IDs = 48 bytes vs. thousands of tokens for raw text
- **Consistency:** All agents see the same underlying memory graph
- **Traceability:** Every memory node has provenance (taint tags, source entries)

### 4.2 Obsidian as Audit Trail

Every delegation, every result, every state transition gets written to the Obsidian vault as Markdown. This means:
- **Full transparency:** Open any Markdown file and see exactly what happened
- **User editing:** The user can manually edit agent memories, task results, delegation chains
- **Bidirectional sync:** The `VaultWatcher` picks up user edits and propagates them to agents

### 4.3 Security by Default

Every `DelegationTask` carries a CCT. Every `ContextPackage` carries `memory_enclave_scope`. Every subagent runs in a WASM sandbox. This is not an afterthought — it's architectural.

### 4.4 Crash-Safe Delegation

Task state transitions are journaled to CortexaDB's WAL. If the Orchestrator crashes mid-delegation:
1. On restart, `recover_from_checkpoint()` scans the WAL for `TaskState::Working` entries
2. Re-hydrates the `DelegationTask` from the WAL
3. Re-queues it in the target agent's inbound port
4. The subagent resumes exactly where it left off

This is something no other single-machine agent framework offers.

---

## Part 5: What We Should NOT Build

From the Gemini research, here's what to skip:

1. **gRPC/tonic for agent communication** — We're single-machine. iceoryx2 is 100x faster than localhost gRPC.
2. **HTTP/JSON-RPC between agents** — Same reason. Shared memory is our network.
3. **Full A2A protocol compliance** — We only need the data model (AgentCard, Task, Artifact, Message). We don't need the transport layer (HTTP, SSE, JSON-RPC).
4. **Cross-framework interoperability** — Savant is Rust-only. We don't need to talk to LangGraph or CrewAI agents.
5. **Dynamic service discovery** — Filesystem scanning is fine for single-machine. No need for Consul/etcd-style discovery.

---

## Summary

The Gemini research provides a solid theoretical foundation. The key enhancements for Savant are:

1. **Extend the `ContextPackage` to reference all 4 graph types** (Semantic, Temporal, Causal, Entity) from `crates/memory/src/reflective.rs`
2. **Use the Obsidian vault as a secondary communication channel** for large payloads
3. **Integrate with existing consensus voting** for destructive operations
4. **Add cross-agent speculative execution** (extend `HyperCausalEngine` to work across agents)
5. **Leverage the dual-enclave memory system** for secure inter-agent context passing
6. **Journal all task state transitions to the WAL** for crash-safe delegation
7. **Size AgentCard at 192 bytes** to fit all needed fields including `memory_enclave_id`
8. **Add `speculative_copies`, `requires_consensus`, `result_channel_id`** to `DelegationTask`

The implementation is 4 weeks of focused work across 4 new files and ~8 modified files. The result will be an A2A communication layer that no other framework can match because it's built on top of Savant's unique "glass house" memory architecture.

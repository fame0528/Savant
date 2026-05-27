# Collective Intelligence: Multi-Agent Consensus & State Sharing

> **Status:** Production (v0.3.1)
> **Crate:** `savant_ipc` (CollectiveBlackboard, SwarmBlackboard)
> **Technology:** iceoryx2 zero-copy shared memory with `#[repr(C)]` structs

---

## Overview

The Collective Intelligence substrate enables zero-copy state sharing and multi-agent consensus protocols across the Savant hivemind. It ensures the system operates as a unified cognitive entity rather than a collection of isolated agents.

Every agent in the hivemind shares a unified memory bus, enabling cross-agent learning, collective intelligence synthesis, and zero-latency context inheritance.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    SHARED MEMORY SUBSTRATE                      │
│                  (iceoryx2 zero-copy IPC)                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────┐  ┌──────────────────────────────┐ │
│  │   CollectiveBlackboard  │  │   SwarmBlackboard            │ │
│  │   (129 entries)         │  │   (per-agent state)          │ │
│  │                         │  │                              │ │
│  │  Entry 0: GlobalState   │  │  Agent A: SwarmSharedContext │ │
│  │  ├─ heuristic_version   │  │  ├─ session_id              │ │
│  │  ├─ swarm_pressure      │  │  ├─ current_task_hash       │ │
│  │  ├─ total_successes     │  │  ├─ agent_status            │ │
│  │  ├─ total_failures      │  │  └─ memory_size             │ │
│  │  ├─ voting masks        │  │                              │ │
│  │  └─ quorum_threshold    │  │  Agent B: SwarmSharedContext │ │
│  │                         │  │  ...                         │ │
│  │  Entries 1-128:         │  │                              │ │
│  │  AgentEntry per agent   │  │  ~100ns write, ~50ns read    │ │
│  │  ├─ successes/failures  │  └──────────────────────────────┘ │
│  │  ├─ pressure (0.0-1.0)  │                                   │
│  │  ├─ is_active           │                                   │
│  │  └─ agent_index         │                                   │
│  └─────────────────────────┘                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. CollectiveBlackboard (`savant_ipc::collective`)

The CollectiveBlackboard is the distributed consensus and metrics substrate for the entire hivemind.

**Technology:** `iceoryx2` zero-copy shared memory with `#[repr(C)]` structs — no serialization overhead.

**Structure:** 129 entries. Entry 0 is the global state. Entries 1-128 are per-agent metrics.

#### GlobalState (`#[repr(C)]`)

The swarm-wide state stored at entry 0:

| Field | Type | Purpose |
|-------|------|---------|
| `heuristic_version` | `u64` | Incremented on major swarm insights |
| `swarm_pressure` | `f32` | Mean pressure across all active agents (0.0-1.0) |
| `total_successes` | `u64` | Aggregate successful tool executions |
| `total_failures` | `u64` | Aggregate tool failures |
| `active_proposal_hash` | `u64` | XXH3 hash of the current proposal under vote |
| `proposal_type` | `u8` | 0=None, 1=DestructiveEdit, 2=SecurityOperation, 3=ToolSynthesis |
| `approve_mask` | `[u64; 2]` | Bitmask of approval votes (supports 128 agents) |
| `veto_mask` | `[u64; 2]` | Bitmask of veto votes (supports 128 agents) |
| `quorum_threshold` | `u8` | Votes needed for consensus (default 3) |

#### AgentEntry (`#[repr(C)]`)

Per-agent metrics stored at entries 1-128:

| Field | Type | Purpose |
|-------|------|---------|
| `successes` | `u64` | Total successful tool executions |
| `failures` | `u64` | Total tool failures |
| `pressure` | `f32` | Agent-specific task pressure (0.0-1.0) |
| `is_active` | `bool` | Whether agent is participating in swarm pulse |
| `agent_index` | `u8` | Epoch-relative index (1-128) for consensus voting |

#### Key Operations

| Method | Purpose | Performance |
|--------|---------|-------------|
| `publish_global_state()` | Writes GlobalState to entry 0 | Sub-microsecond |
| `read_global_state()` | Reads GlobalState from entry 0 | Sub-microsecond |
| `update_agent_metrics()` | Increments success/fail, updates pressure | Sub-microsecond |
| `aggregate_swarm_metrics()` | Reads all 128 agent entries, computes totals | Batch read |
| `cast_vote()` | Sets bit in approve/veto mask for an agent | Sub-microsecond |
| `check_consensus()` | Returns Approved/Vetoed/Pending based on masks vs quorum | Bitmask comparison |
| `set_quorum_threshold()` | Dynamic quorum adjustment | Sub-microsecond |

#### Deployment Configuration

- Supports 1024 concurrent readers, 128 writer nodes
- Retry-with-suffix on collision (handles stale shared memory on Windows)
- Solo agent mode: auto-sets quorum=1 to prevent consensus deadlock

---

### 2. SwarmBlackboard (`savant_ipc::blackboard`)

Per-agent state broadcast via zero-copy shared memory.

**Structure:** Per-agent `SwarmSharedContext` entries:

| Field | Type | Purpose |
|-------|------|---------|
| `session_id` | `[u8; 36]` | Current session UUID |
| `current_task_hash` | `u64` | XXH3 hash of active task (for dedup) |
| `agent_status` | `u8` | 0=Idle, 1=Active, 2=Error, 3=Cooldown |
| `memory_size` | `u32` | Current memory utilization |

**Performance:** ~100ns write, ~50ns read — orders of magnitude faster than any network-based IPC.

---

### 3. Consensus Protocol

#### Proposal Phase

High-risk actions (destructive file edits, security operations, tool synthesis) require swarm consensus before execution.

```
Agent proposes ToolAction
    │
    ├─ Hash proposal with XXH3_64
    ├─ Set active_proposal_hash in GlobalState
    ├─ Reset approve_mask and veto_mask
    ├─ Broadcast proposal to peers via CollectiveBlackboard
    │
    ▼
Peer agents evaluate proposal
    │
    ├─ Each agent casts approve or veto bit
    ├─ Quorum check: popcount(approve_mask) >= quorum_threshold
    │
    ▼
┌─────────────────┬─────────────────┐
│   Quorum met    │  Veto received  │
│   → Execute     │  → Abort        │
│   → Log result  │  → Log reason   │
└─────────────────┴─────────────────┘
```

#### Proposal Types

| Type | Enum Value | Description |
|------|-----------|-------------|
| `DestructiveEdit` | 1 | File deletion, overwrite, or irreversible modification |
| `SecurityOperation` | 2 | Permission changes, key rotation, access grants |
| `ToolSynthesis` | 3 | Creating new tools via the ECHO substrate |

#### DelegationConsensus API

| Method | Purpose |
|--------|---------|
| `propose()` | Initiates a vote: sets proposal hash, resets masks |
| `await_consensus()` | Async polling loop with configurable timeout |
| `clear_proposal()` | Resets proposal state after execution |

---

### 4. Heuristic Synchronization

Every insight increment increases the `heuristic_version` in GlobalState.

Agents with lower local versions automatically pull the latest cognitive substrate before starting a new `AgentLoop`, ensuring no agent operates on stale heuristics.

#### Version Flow

```
Agent A produces insight
    │
    ├─ Increment heuristic_version in GlobalState
    ├─ Store insight in MemoryEnclave
    │
    ▼
Agent B starts new AgentLoop
    │
    ├─ Read GlobalState.heuristic_version
    ├─ Compare with local version
    ├─ If stale: pull latest insights from memory
    └─ Update local version
```

---

### 5. Cross-Agent Reflection

Agents publish cognitive insights to the blackboard after every task. Other agents subscribe to these insights to adapt their local predictors.

**Flow:**
1. Agent completes task → generates `CognitiveInsight`
2. Stores insight in `MemoryEnclave` (private) + publishes to `CollectiveBlackboard` (shared)
3. Increments `heuristic_version`
4. Other agents detect version change → pull new insights → update `DspPredictor` local state

---

### 6. Integration with Hivemind

```
SwarmController (orchestrator)
├── CollectiveBlackboard (iceoryx2)
│   ├── GlobalState: swarm metrics, voting masks, quorum
│   └── AgentEntry × 128: per-agent success/fail/pressure
├── SwarmBlackboard (iceoryx2)
│   └── Agent × 128: per-agent session/task/status
├── Agent 1..N (each spawned via tokio::spawn)
│   └── AgentLoop holds Arc<CollectiveBlackboard>
│       └─ Uses cast_vote() for consensus
│       └─ Uses update_agent_metrics() for telemetry
│       └─ Reads heuristic_version for sync
└── MemoryEngine
    ├── enclave: private per-agent memory
    └── collective: shared swarm memory
```

---

### 7. Performance Characteristics

| Operation | Latency | Notes |
|-----------|---------|-------|
| Write GlobalState | <1µs | Zero-copy, no serialization |
| Read GlobalState | <1µs | Zero-copy, direct memory access |
| Update agent metrics | <1µs | Atomic increment + write |
| Aggregate 128 agents | ~10µs | Batch read of all entries |
| Cast vote | <1µs | Single bitmask bit set |
| Check consensus | <1µs | Bitmask popcount vs quorum |
| Broadcast (100 agents) | 450µs | Via iceoryx2 zero-copy IPC |

---

### 8. Design Decisions

1. **Zero-copy over serialization:** All IPC uses `#[repr(C)]` structs mapped directly to shared memory. No JSON encoding, no protobuf, no bincode — pure memory access.

2. **Bitmask voting:** 128-agent consensus via two `u64` bitmasks. Popcount for quorum check is a single CPU instruction.

3. **Solo agent quorum=1:** When only one agent exists, quorum is auto-set to 1 to prevent deadlock on proposals.

4. **Retry-with-suffix:** On Windows, stale shared memory segments can block creation. The blackboard retries with incremented service names until a free slot is found.

5. **Dual blackboard separation:** CollectiveBlackboard handles swarm-wide consensus and metrics. SwarmBlackboard handles per-agent state broadcast. Different access patterns, different update frequencies.

6. **Heuristic versioning:** Agents track their local heuristic version against the global version, enabling lazy synchronization without polling.

7. **XXH3 hashing:** Proposals and task identities use XXH3_64 for fast, collision-resistant hashing without cryptographic overhead.

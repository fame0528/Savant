# Memory System Improvements — Implementation Plan

> **Date:** 2026-03-19  
> **Source:** Gemini 3 Deep Research (390-line report, 87 citations)  
> **Methodology:** Perfection Loop per feature

---

## Research Summary

7 architectural upgrades identified, building on each other. 3 quick wins, 4 complex features.

**Critical architectural principle:** Savant is a hive mind. Memories are global by default — if Agent A learns something, Agent F knows it too. The substrate (Savant itself) IS the memory. Individual agents are processing nodes, not isolated silos. Session isolation exists for conversation flow, not knowledge isolation.

---

## Implementation Roadmap

### Sprint A: Quick Wins (3 features, all LOW complexity)

| # | Feature | Complexity | What | Where |
|---|---------|-----------|------|-------|
| 1 | Auto-Recall Injection | LOW | Embed query, hybrid search, inject `<context_cache>` in prompt | `async_backend.rs`, `engine.rs` |
| 2 | Bi-Temporal Tracking | LOW | Add `valid_from/valid_to` to MemoryEntry, filter active facts | `models.rs`, `engine.rs` |
| 3 | Daily Ops Logs | LOW | `memory/YYYY-MM-DD.md` append-only, auto-load on session start | New: `crates/memory/src/daily_log.rs` |

### Sprint B: Swarm Coordination (1 feature, MEDIUM)

| # | Feature | Complexity | What | Where |
|---|---------|-----------|------|-------|
| 4 | Blackboard (swarm sharing) | MEDIUM | Global Fjall keyspace + broadcast channels | New: `crates/ipc/src/blackboard.rs` |

### Sprint C: Advanced Memory (3 features, HIGH complexity)

| # | Feature | Complexity | What | Where |
|---|---------|-----------|------|-------|
| 5 | DAG Session Compaction | HIGH | Replace destructive compact with reversible DAG nodes | `async_backend.rs`, `models.rs` |
| 6 | Personality-Driven Promotion | MEDIUM | OCEAN traits as decay/entropy scalars | `engine.rs`, new: `promotion.rs` |
| 7 | Local NER + Petgraph | HIGH | `gline-rs` entity extraction + `petgraph` relationship graph | New: `crates/memory/src/entities.rs` |

---

## Feature Specifications

### 1. Auto-Recall Injection

**Goal:** Before the LLM sees any prompt, automatically search memory and inject relevant context.

**Data flow (verified against actual code):**
```
User query arrives via WebSocket (gateway)
    ↓
AsyncMemoryBackend::retrieve(agent_id, query, limit) is called
    ↓ (currently: embed query → semantic_search → session tail → merge)
    ↓ (NEW: add parallel auto-recall with tokio::join!)
    
tokio::join!(
    standard_retrieve(),    // existing: semantic + transcript
    auto_recall(),          // NEW: embed last 3 interactions → semantic search
)
    ↓
auto_recall extracts last 3 user messages as query window
    ↓
EmbeddingService::embed(query_window) → 384-dim vector (~30ms measured)
    ↓
MemoryEngine::semantic_search(embedding, 5) → Vec<SearchResult>
    ↓
SearchResult.document_id → lookup MemoryEntry from iter_metadata()
    ↓
Filter by similarity_threshold (score >= 0.3) and max_tokens (15% of budget)
    ↓
Format as <context_cache> block, prepend to system prompt
```

**Where context is injected:**
- NOT inside the memory crate — that's just storage
- The injection happens in `crates/agent/src/context.rs` where context is assembled for the LLM
- The agent's context assembly calls `auto_recall()` and prepends results to the system prompt

**Data structures:**
```rust
pub struct AutoRecallConfig {
    pub max_tokens: usize,           // 15% of context window
    pub similarity_threshold: f32,    // 0.3 minimum cosine score
    pub max_results: usize,           // 5 max memories
}

pub struct ContextCacheBlock {
    pub query_intent: String,
    pub retrieved_memories: Vec<MemoryEntry>,
    pub injected_at: rend::i64_le,
}
```

**Files to modify:**
- `crates/memory/src/async_backend.rs` — add `auto_recall()` method
- `crates/agent/src/context.rs` — call auto_recall during context assembly

**Tests:** 5 (search accuracy, latency, budget enforcement, empty results, threshold filtering)

---

### 2. Bi-Temporal Tracking

**Goal:** Track when facts become true/stop being true. Filter stale facts automatically.

**⚠️ Important:** Adding fields to `MemoryEntry` breaks rkyv serialization of existing data. Two approaches:

**Option A (Recommended):** New struct `TemporalEntry` wrapping `MemoryEntry` + `TemporalMetadata`
```rust
pub struct TemporalEntry {
    pub memory: MemoryEntry,
    pub temporal: TemporalMetadata,
}
```
Stored in a SEPARATE Fjall keyspace (`temporal_metadata`). No breaking change to existing data.

**Option B:** Add `valid_from`/`valid_to` directly to `MemoryEntry`. Requires migration script or fresh database.

**Data structures:**
```rust
pub struct TemporalMetadata {
    pub valid_from: rend::i64_le,
    pub valid_to: Option<rend::i64_le>,   // None = currently active
    pub recorded_at: rend::i64_le,
    pub superseded_by: Option<rend::u64_le>,
}
```

**Contradiction detection:**
- When new memory is indexed, check cosine similarity against existing same-type memories
- If similarity > 0.92 AND entity name matches (string equality), old fact's `valid_to` = now
- New fact gets `valid_from` = now, `valid_to` = None
- Old fact NOT deleted — history preserved

**Query filter:**
```rust
// During semantic search, only return active facts
memories.retain(|m| m.temporal.valid_to.is_none());
```

**Files to modify:**
- `crates/memory/src/models.rs` — add `TemporalMetadata` struct
- `crates/memory/src/engine.rs` — update `index_memory()` for contradiction detection
- `crates/memory/src/lsm_engine.rs` — add `temporal_metadata` keyspace

**Tests:** 6 (create, invalidate, query active only, query history, multiple invalidations, no false positives)

---

### 3. Daily Operational Logs

**Goal:** Append-only log per agent per day. Loaded on session start for orientation.

**File structure:**
```
workspaces/agents/<agent>/memory/
└── 2026-03-19.md
```

**Format (Markdown, human-readable):```markdown
# 2026-03-19 — Agent Alpha## 09:00 — Session Started- Resumed task: Docker networking debug- Blocker: iptables rules on host## 09:15 — Attempted Fix- Tried: `iptables -L -n` → found DROP rule
- Result: SUCCESS## 10:00 — Deployed Fix
- Deployed config to production
- Status: OPERATIONAL

## 10:30 — New Blocker
- Issue: SSL certificate expiry in 2 days
- Priority: HIGH
```

**Token budget:** Cap at 500 tokens. Loaded as first context element after system prompt.

**Files to create:**
- `crates/memory/src/daily_log.rs` — DailyLog struct, append, read, rotate

**Tests:** 4 (create new log, append entry, read today's log, read yesterday's log)

---

### 4. Hive-Mind Notification Channel

**Goal:** When any agent discovers something important, ALL agents get alerted instantly. Memory is already global — this is proactive notification, not sharing.

**Architecture (hive-mind model, verified against actual code):**
```
Agent Alpha discovers bug → writes to global memory via AsyncMemoryBackend::store()
    ↓
store() calls MemoryEngine::index_memory(entry) — this already runs for every store
    ↓
NEW: index_memory() checks if entry.importance >= 7
    ↓ (if yes)
Savant substrate sends MemoryNotification to broadcast channel
    ↓
tokio::sync::broadcast sends to all active agent sessions
    ↓
Each agent's context assembly (context.rs) receives notification
    ↓
Auto-injects the new memory into the next context window
    ↓
Agent Beta never knew it was Agent Alpha's discovery — it's just "known"
```

**Connection to existing code:**
- `index_memory()` in `engine.rs` already runs for every store operation
- Adding notification there means every high-importance memory is automatically broadcast
- The broadcast channel is a `tokio::sync::broadcast::Sender<MemoryNotification>` on the MemoryEngine
- Agent sessions subscribe during initialization

**Data structures:**
```rust
pub struct MemoryNotification {
    pub notification_id: String,
    pub source_session: String,
    pub memory_id: u64_le,
    pub domain_tags: Vec<String>,
    pub importance: u8,
    pub timestamp: rend::i64_le,
}
```

**Files to modify:**
- `crates/memory/src/engine.rs` — add broadcast sender, trigger on `index_memory()` when importance >= 7
- `crates/memory/src/notifications.rs` — new module with broadcast channel setup
- `crates/agent/src/context.rs` — subscribe to notifications, inject into context

**Tests:** 5 (global memory access, notification on high-importance, domain filtering, multiple agents, no notification for low-importance)

---

### 5. DAG Session Compaction

**Goal:** Replace destructive `atomic_compact()` with reversible DAG nodes.

**Current behavior (DESTRUCTIVE — lines 311-362 of lsm_engine.rs):**
```rust
// 1. Collect ALL keys for session
// 2. Delete ALL messages in transaction
// 3. Insert compacted batch
// Old data is GONE FOREVER
pub fn atomic_compact(&self, session_id, batch) {
    let keys_to_delete = self.transcript_ks.inner().prefix(&prefix)...;
    for key in &keys_to_delete { tx.remove(key); }  // DELETE ALL
    for msg in &batch { tx.insert(key, bytes); }     // INSERT NEW
    tx.commit();
}
```

**Current consolidation (lines 225-295 of async_backend.rs):**
- Triggers at 50+ messages
- Keeps last 20 messages
- Creates summary from old messages
- Calls atomic_compact → DESTRUCTIVE

**New approach (reversible):**
```rust
// Keep raw messages in Fjall. Create DAG node that REFERENCES them.
pub struct DagNode {
    pub node_id: String,
    pub depth_level: u8,
    pub summary_content: String,
    pub raw_message_ids: Vec<String>,  // References to original messages
    pub child_nodes: Vec<String>,       // Links to sub-DAG nodes
}
```

**Change to atomic_compact():**
Instead of deleting old messages, keep them. Create a DagNode in a separate `dag_nodes` keyspace. The summary is just an index pointing to original data. Agent calls `expand_memory_node(node_id)` to page raw messages back into context.

**Files to modify:**
- `crates/memory/src/models.rs` — add `DagNode` struct
- `crates/memory/src/lsm_engine.rs` — add `dag_nodes` keyspace, new `compact_with_dag()` method
- `crates/memory/src/async_backend.rs` — rewrite `consolidate()` to use DAG

**Tests:** 5 (create node, expand node, multi-level DAG, node ordering, no data loss)

---

### 6. Personality-Driven Promotion

**Goal:** SOUL.md OCEAN traits influence memory retention and promotion.

**Mechanics:**
```rust
pub struct MemoryMetrics {
    pub base_entropy: rend::f32_le,
    pub hit_count: rend::u32_le,
    pub last_accessed_at: rend::i64_le,
    pub decay_factor: rend::f32_le,
}

pub enum MemoryNetwork {
    Transient,    // L0: New memories
    Experience,   // L1: Frequently accessed
    Canonical,    // L2: Architectural truth
}

// Conscientiousness scalar: slows decay for security/constraint memories
// Openness scalar: lowers entropy threshold for exploratory observations
```

**Background worker:** Scans Fjall metadata during idle cycles. Promotes high-score memories. Archives low-score memories.

**Files to create/modify:**
- `crates/memory/src/promotion.rs` — new module
- `crates/memory/src/engine.rs` — replace `cull_low_entropy_memories()` with promotion worker

**Tests:** 5 (promotion threshold, decay curve, personality influence, canonical priority, archival)

---

### 7. Local NER + Petgraph

**Goal:** Extract entities from messages. Track relationships across sessions.

**Architecture:**
```
New message arrives
    ↓
gline-rs NER extracts entities ("Project Alpha", "OpenRouter API Key")
    ↓
Normalize + hash → entity ID
    ↓
petgraph: add node, create edges to related memories
    ↓
Query "Project Alpha" → traverse graph → return all related memories
```

**Files to create/modify:**
- `crates/memory/src/entities.rs` — new module (EntityExtraction, EntityRegistry)
- `crates/memory/Cargo.toml` — add `petgraph`, `gline-rs` dependencies
- `crates/memory/src/async_backend.rs` — add entity extraction to `store()`

**Tests:** 6 (extract entities, create graph edges, query by entity, entity resolution, cross-session tracking, memory limits)

---

## Implementation Order

```
Sprint A (quick wins, all LOW complexity):
  1. Auto-Recall Injection    — transforms baseline intelligence
  2. Bi-Temporal Tracking     — resolves contradiction problem
  3. Daily Ops Logs           — prevents session-to-session amnesia

Sprint B (swarm, MEDIUM):
  4. Blackboard               — enables cross-agent knowledge sharing

Sprint C (advanced, HIGH):
  5. DAG Compaction           — makes compaction reversible
  6. Personality Promotion    — OCEAN-driven memory consolidation
  7. NER + Petgraph           — entity tracking and relationship graph
```

---

## Dependencies

- Phase 2 depends on Phase 1 (auto-recall uses temporal filter)
- Phase 5 depends on Phase 3 (daily logs reference DAG nodes)
- Phase 6 depends on Phase 2 (promotion checks temporal validity)
- Phase 7 depends on Phase 2 (entity tracking needs temporal metadata)

---

*Created: 2026-03-19. Awaiting approval before execution.*

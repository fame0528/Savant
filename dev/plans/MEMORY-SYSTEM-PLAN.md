# Memory System Improvements — Implementation Plan

> **Date:** 2026-03-19  
> **Source:** Gemini 3 Deep Research (390-line report, 87 citations)  
> **Methodology:** Perfection Loop per feature

---

## Research Summary

7 architectural upgrades identified, building on each other. 3 quick wins, 4 complex features.

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

**Data structures:**
```rust
pub struct AutoRecallConfig {
    pub max_tokens: usize,           // Cap at 15% of context window
    pub similarity_threshold: f32,    // Minimum cosine score (0.3)
    pub max_results: usize,           // Max memories to inject (5)
}

pub struct ContextCacheBlock {
    pub query_intent: String,
    pub retrieved_memories: Vec<MemoryEntry>,
    pub injected_at: rend::i64_le,
}
```

**Flow:**
```
User query arrives via WebSocket
    ↓
tokio::join!(transcript_fetch, auto_recall_fetch)  ← parallel
    ↓
auto_recall:
    1. Extract last 3 interactions as query window
    2. Embed via EmbeddingService (~30ms)
    3. Semantic search + BM25 keyword search (~15ms)
    4. RRF fusion of results
    5. Filter by similarity_threshold
    6. Cap by max_tokens
    ↓
Format as <context_cache> XML block
    ↓
Prepend to system prompt before LLM sees it
```

**Files to modify:**
- `crates/memory/src/async_backend.rs` — add `auto_recall()` method
- `crates/agent/src/context.rs` — call auto_recall during context assembly

**Tests:** 5 (search accuracy, latency, budget enforcement, empty results, threshold filtering)

---

### 2. Bi-Temporal Tracking

**Goal:** Track when facts become true/stop being true. Filter stale facts automatically.

**Data structures:**
```rust
pub struct TemporalMetadata {
    pub valid_from: rend::i64_le,        // When fact became true in reality
    pub valid_to: Option<rend::i64_le>,   // None = currently active
    pub recorded_at: rend::i64_le,        // When agent stored it
    pub superseded_by: Option<rend::u64_le>, // Links to replacement fact
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
- `crates/memory/src/models.rs` — add `TemporalMetadata` to `MemoryEntry`
- `crates/memory/src/engine.rs` — update `index_memory()` for contradiction detection

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

### 4. Blackboard (Swarm Sharing)

**Goal:** Agents publish discoveries to shared memory. Other agents receive async interrupts.

**Architecture:**
```
Agent Alpha discovers bug
    ↓
Publishes BlackboardEvent to global Fjall keyspace
    ↓
tokio::sync::broadcast sends to subscribers
    ↓
Agent Beta receives interrupt → injects into context
```

**Data structures:**
```rust
pub struct BlackboardEvent {
    pub event_id: String,
    pub publisher_agent_id: String,
    pub domain_tags: Vec<String>,   // "docker", "security", "api"
    pub payload: String,
    pub timestamp: rend::i64_le,
    pub confidence: f32,
}
```

**Files to create/modify:**
- `crates/ipc/src/blackboard.rs` — new module
- `crates/ipc/src/lib.rs` — add blackboard module

**Tests:** 5 (publish, subscribe, domain filtering, multiple subscribers, event ordering)

---

### 5. DAG Session Compaction

**Goal:** Replace destructive `atomic_compact()` with reversible DAG nodes.

**Current (destructive):**
```rust
// Deletes old messages, inserts summary. Original data lost.
engine.atomic_compact(&sid, vec![summary, ...recent]);
```

**New (reversible):**
```rust
// Keep raw messages. Create DAG node that references them.
pub struct DagNode {
    pub node_id: String,
    pub depth_level: u8,
    pub summary_content: String,
    pub raw_message_ids: Vec<String>,  // References to original messages
    pub child_nodes: Vec<String>,       // Links to sub-DAG nodes
}
// Raw messages stay in Fjall. Summary is just an index.
```

**Expand tool:** Agent calls `expand_memory_node(node_id)` → fetches raw messages by ID → injects into context.

**Files to modify:**
- `crates/memory/src/models.rs` — add `DagNode` struct
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

# Savant Memory System — Complete Architecture Reference

## Overview

The Savant memory system is a multi-layered, "forever memory" architecture designed to persist agent knowledge permanently while supporting semantic search, contradiction resolution, and hive-mind knowledge sharing across 101 agents. Data is permanent by default — deletion requires explicit action.

The system has three distinct layers, a dual-enclave architecture (private + collective), and multiple persistence mechanisms that ensure no data is lost even during crashes or shutdowns.

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        AGENT INTERACTION LAYER                          │
│                                                                         │
│  FileLoggingMemoryBackend (Decorator)                                   │
│  ├─ store() → if Memory channel → LEARNINGS.md (append)                │
│  ├─ store() → inner.store() → AsyncMemoryBackend                        │
│  └─ consolidate() → parse LEARNINGS.md → LEARNINGS.jsonl               │
├─────────────────────────────────────────────────────────────────────────┤
│                        ASYNC BACKEND ADAPTER                            │
│                                                                         │
│  AsyncMemoryBackend                                                     │
│  ├─ store() → embed + index (Layer 1 + Layer 2)                        │
│  ├─ retrieve() → 3-tier: semantic → tail → substring                   │
│  ├─ consolidate() → DAG-aware dedup + compaction                       │
│  └─ auto_recall() → semantic search for context injection              │
├─────────────────────────────────────────────────────────────────────────┤
│                        UNIFIED MEMORY ENGINE                            │
│                                                                         │
│  MemoryEngine (Dual-Enclave Architecture)                               │
│  ├─ enclave: Arc<MemoryEnclave>    (Private per-agent memory)          │
│  └─ collective: Arc<MemoryEnclave> (Shared hive-mind memory)           │
│                                                                         │
│  MemoryEnclave (Atomic Write Adapter)                                   │
│  ├─ 64-way write lock partitioning (FNV-1a hash of session_id)         │
│  ├─ Atomicity rollback on LSM insert failure                            │
│  ├─ PromotionEngine (OCEAN personality-driven scoring)                  │
│  └─ EmbeddingService integration                                        │
├─────────────────────────────────────────────────────────────────────────┤
│                        STORAGE LAYERS                                   │
│                                                                         │
│  Layer 1: LsmStorageEngine (CortexaDB)                                  │
│  ├─ transcript.{session_id}  — Conversation messages (rkyv)            │
│  ├─ metadata                 — MemoryEntry records (rkyv)              │
│  ├─ temporal                 — Bi-temporal metadata (JSON)             │
│  ├─ dag                      — DAG compaction nodes (JSON)             │
│  ├─ facts                    — SPO triples                             │
│  ├─ sessions                 — Session state (rkyv)                    │
│  ├─ turns.{session_id}       — Turn state (rkyv)                       │
│  ├─ distillation             — Distillation markers                    │
│  └─ _registry                — Session ID registry                     │
│                                                                         │
│  Layer 2: SemanticVectorEngine (ruvector-core)                          │
│  ├─ HNSW index (M=16, ef_construction=200, ef_search=50)              │
│  ├─ Cosine distance metric                                              │
│  ├─ 32x binary quantization (optional)                                  │
│  ├─ SIMD acceleration (AVX2/AVX-512/NEON)                              │
│  └─ Atomic persistence (vectors.rkyv, magic: "SAVANT_V")              │
├─────────────────────────────────────────────────────────────────────────┤
│                        BACKGROUND PROCESSES                             │
│                                                                         │
│  Distillation Pipeline (5-minute interval)                              │
│  ├─ enclave → LLM triplet extraction → collective                      │
│  ├─ SPO facts indexed in collective LSM                                │
│  └─ Shannon entropy calculation per triplet                             │
│                                                                         │
│  Factual Arbiter (10-minute interval)                                   │
│  ├─ Scans collective facts for contradictions                           │
│  ├─ Resolves by (lowest entropy, highest importance)                    │
│  └─ High-entropy contradictions (>1.5 bits) flagged for human audit     │
│                                                                         │
│  Notification Channel (hive-mind broadcast)                             │
│  └─ tokio broadcast channel, capacity 64                               │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 1. Core Data Models

All structures use `#[rkyv(check_bytes)]` with `#[repr(C)]` for zero-copy deserialization from disk. This means the system can read persisted data without parsing — the bytes on disk are the same as the bytes in memory.

### AgentMessage

The fundamental unit of conversation storage.

```rust
pub struct AgentMessage {
    pub id: String,                    // UUID v4
    pub session_id: String,            // Prevents cross-channel bleed
    pub role: MessageRole,             // System=0, User=1, Assistant=2, Tool=3
    pub content: String,               // Text body
    pub tool_calls: Vec<ToolCallRef>,  // Associated tool invocations
    pub tool_results: Vec<ToolResultRef>, // Associated tool results
    pub timestamp: i64,                // Unix milliseconds
    pub parent_id: Option<String>,     // DAG threading
    pub channel: String,               // "Chat", "Telemetry", "Archive", "Memory"
}
```

**Channels:**
- `Chat` — Normal conversation messages (included in context)
- `Telemetry` — System metrics and monitoring (filtered out of context)
- `Memory` — Learning entries and reflections (written to LEARNINGS.md)
- `Archive` — Compacted old messages (filtered out of retrieve, preserved in storage)

### MemoryEntry

Semantic memory records indexed for search.

```rust
pub struct MemoryEntry {
    pub id: u64,                       // Deterministic blake3 hash
    pub session_id: String,
    pub category: String,              // "fact", "preference", "observation", "distilled_triplet"
    pub content: String,
    pub importance: u8,                // 1-10 scale
    pub tags: Vec<String>,
    pub embedding: Vec<f32>,           // 2560 dimensions (qwen3-embedding:4b)
    pub created_at: i64,
    pub updated_at: i64,
    pub shannon_entropy: f32,          // Information density
    pub last_accessed_at: i64,
    pub hit_count: u32,
    pub related_to: Vec<u64>,          // Linked memory IDs
}
```

### TemporalMetadata

Bi-temporal fact tracking — knows both when a fact was true and when it was recorded.

```rust
pub struct TemporalMetadata {
    pub valid_from: i64,               // When the fact became true
    pub valid_to: Option<i64>,         // None = currently active
    pub recorded_at: i64,              // When the system learned it
    pub superseded_by: Option<u64>,    // ID of newer fact that replaced this
    pub memory_id: u64,
    pub entity_type: String,
    pub entity_name: String,
}
```

### DagNode

Reversible session compaction — original messages remain referenced even after summarization.

```rust
pub struct DagNode {
    pub node_id: String,
    pub depth_level: u8,
    pub summary_content: String,
    pub raw_message_ids: Vec<String>,  // References original messages
    pub child_nodes: Vec<String>,
    pub session_id: String,
    pub created_at: i64,
    pub message_count: usize,
}
```

---

## 2. Layer 1: LSM Storage Engine (CortexaDB)

The persistent storage backbone. Uses CortexaDB, an embedded database with WAL-backed hard durability.

### Collections

| Collection | Key Format | Content | Serialization |
|---|---|---|---|
| `transcript.{session_id}` | `session:{sid}:{ts}:{id}` | AgentMessage | rkyv (zero-copy) |
| `metadata` | `meta:{id}` | MemoryEntry | rkyv |
| `temporal` | `temporal:{memory_id}` | TemporalMetadata | JSON |
| `dag` | `dag:{node_id}` | DagNode | JSON |
| `facts` | `{subject}:{predicate}:{entry_id}` | Object bytes | raw |
| `sessions` | `state:{session_id}` | SessionState | rkyv |
| `turns.{session_id}` | `turn:{sid}:{turn_id}` | TurnState | rkyv |
| `distillation` | msg_id metadata | Distillation marker | marker |
| `_registry` | session_id bytes | Session registration | raw |

### Atomic Compaction (Write-Before-Delete)

The compaction pattern guarantees no data loss:

```
Phase 1: INSERT compacted batch
    └── db.add_batch(compacted_messages)
        [If this fails: old data remains, nothing lost]

Phase 2: DELETE old entries (only after Phase 1 succeeds)
    └── search_in_collection → delete each
        [If this fails: temporary duplicates exist, next compaction cleans up]
```

This means the system is crash-safe. If the process dies mid-compaction:
- Phase 1 failure: old data untouched
- Phase 2 failure: both old and new data exist (duplicates cleaned up next cycle)

### Tool Pair Integrity

Before compaction, the system runs `verify_tool_pair_integrity()` — a two-pass scan that ensures every `tool_result` has a matching `tool_call`. This prevents the historical OpenClaw Issue #39609 where orphaned tool results caused context corruption.

---

## 3. Layer 2: Semantic Vector Engine (ruvector-core)

The semantic search layer. Uses HNSW (Hierarchical Navigable Small World) graphs for approximate nearest neighbor search.

### Configuration

```rust
VectorConfig {
    dimensions: 2560,              // qwen3-embedding:4b output
    hnsw_m: 16,                    // Bi-directional links per node
    hnsw_ef_construction: 200,     // Candidate list during index build
    hnsw_ef_search: 50,            // Candidate list during search
    use_quantization: true,        // 32x binary quantization
}
```

### Search Pipeline

```
query_embedding (2560-dim vector)
    ↓
VectorDB.search(SearchQuery)
    ├─ HNSW graph traversal (approximate nearest neighbor)
    ├─ SIMD-accelerated distance computation
    │   ├─ AVX2/AVX-512 on x86_64
    │   └─ NEON on aarch64
    ├─ Cosine distance → similarity: score = 1.0 - (distance / 2.0)
    └─ Returns Vec<SearchResult> (id, score, distance)
```

### Persistence

- File: `{storage_path}/vectors.rkyv`
- Format: `[magic "SAVANT_V" (8 bytes)] [version (4 bytes)] [rkyv-serialized PersistedData]`
- Atomic write: write to temp file → rename (prevents corruption on crash)
- Auto-save on Drop (prevents data loss on shutdown)
- Max 10 million vectors per file

### Dimension Mismatch Recovery

On startup, if the persisted vector file has a different dimension than the current embedding model, the system:
1. Catches `VectorInitFailed`
2. Clears the stale vector directory
3. Retries with corrected dimensions from the embedding service

This happened when the project switched from fastembed (384-dim) to qwen3-embedding:4b (2560-dim).

---

## 4. The Dual-Enclave Architecture

The `MemoryEngine` maintains two separate memory spaces:

### Private Enclave (`{base}/enclave/`)

- Per-agent private memory
- Contains all conversation transcripts, session state, turn state
- Source of truth for `retrieve()` calls
- Distillation source (messages extracted → triplets)

### Collective Enclave (`{base}/collective/`)

- Shared hive-mind memory across all 101 agents
- Receives distilled triplets from all agent enclaves
- Maintains SPO facts index for contradiction resolution
- Factual arbiter runs here to resolve conflicts

### Data Flow Between Enclaves

```
Agent A's enclave                    Collective
    │                                   │
    ├─ messages ──→ Distillation ──→ distilled triplets
    │                    │                │
                    LLM extracts      indexed in:
                    SPO triples       ├─ vector engine (semantic search)
                                      ├─ facts collection (SPO triples)
                                      └─ metadata collection (MemoryEntry)
```

---

## 5. The 3-Tier Retrieval System

When `retrieve()` is called, the system tries three strategies in order:

### Tier 1: Semantic Search

```
IF embedding_service available AND query non-empty:
    1. Embed the query → 2560-dim vector
    2. semantic_search(query_embedding, limit)
    3. Fetch limit*3 recent messages from transcript
    4. Deduplicate by content (keep first occurrence)
    5. Return results
```

This finds messages based on MEANING, not exact text match. A query about "memory issues" would find messages about "heap allocation problems" or "OOM errors."

### Tier 2: Transcript Tail

```
IF no semantic results:
    1. fetch_session_tail(session_id, limit)
    2. Retrieve from Layer 1 (LSM)
    3. Skip "Archive" channel messages
    4. Sort by timestamp, take last N
    5. Return newest-first
```

This returns the most recent messages when semantic search finds nothing.

### Tier 3: Substring Filter

```
IF no embeddings available AND query non-empty:
    1. Fetch session tail (larger batch)
    2. Filter by case-insensitive substring match
    3. Return matching messages
```

This is the fallback when Ollama is unavailable (should not happen with auto-start).

---

## 6. Consolidation & DAG Architecture

### Consolidation Process

```
consolidate(session_id)
    1. Fetch 500 messages from enclave
    2. Split: last 20 = "recent" (kept), older = "consolidated"
    3. Content-hash dedup (SHA-256 of role:normalized_content)
    4. Create summary message: "Conversation compacted: X → Y"
    5. DAG linking:
       ├─ Older messages → channel="Archive", parent_id=summary_id
       ├─ Recent messages → parent_id=summary_id
       └─ Summary → references original message IDs
    6. atomic_compact() with: [archived_older, summary_msg, updated_recent]
```

### Why DAG?

The DAG (Directed Acyclic Graph) architecture means:
- Original messages are NEVER deleted — they're archived
- Summaries reference original messages via `parent_id`
- The system can "expand" a summary back to original messages if needed
- Compaction reduces context window usage without losing data

---

## 7. Distillation Pipeline (Enclave → Collective)

Runs every 5 minutes as a background tokio task.

### Process

```
For each message in enclave:
    1. Skip if already distilled
    2. Skip system messages and messages < 20 chars
    3. Send to LLM: "Extract SPO triples from this text"
    4. For each extracted triplet:
       ├─ Calculate Shannon entropy of source message
       ├─ Create TripletClaims (JWT with 1-year expiry)
       ├─ Generate embedding for "{subject} {predicate} {object}"
       ├─ Index in collective vector engine
       ├─ Insert in collective SPO facts index
       └─ Set importance = confidence * 10
    5. Mark source message as distilled
```

### Shannon Entropy

Measures information density. Lower entropy = more structured/predictable = higher quality.

```
H(X) = -Σ p(x) × log₂(p(x))

where p(x) = frequency of character x in text
```

Messages with entropy > 1.5 bits are flagged as potentially contradictory.

---

## 8. Factual Arbiter (Contradiction Resolution)

Runs every 10 minutes on the collective enclave.

### Process

```
1. Scan all facts in collective
2. Group by subject
3. For subjects with >1 fact:
   ├─ Load all associated MemoryEntry records
   ├─ Sort by: (ascending entropy, descending importance)
   ├─ Best = lowest entropy + highest importance
   ├─ IF best entropy > 1.5: FLAG for human audit (don't delete)
   └─ ELSE: delete inferior memories (keep canonical)
```

### Why This Matters

As the collective grows, different agents may store contradictory facts (e.g., "Port 8080 is free" vs "Port 8080 is in use"). The arbiter resolves these by keeping the highest-quality fact (lowest entropy, highest importance) and removing contradictions — unless the contradiction is too uncertain (>1.5 bits entropy), in which case a human must decide.

---

## 9. Promotion Engine (OCEAN Personality-Driven)

Scores memories based on access patterns, age, and personality traits.

### Score Calculation

```
score = 0.0

// Hit count (0.0-0.3)
score += min(hit_count / 100, 0.3)

// Age decay (penalty after 7 days)
if age > 168_hours:
    score -= min((age - 168) / 168, 0.3)

// Entropy bonus (lower entropy = higher score)
score += (1.0 - entropy) × 0.2

// Importance multiplier
score *= (1.0 + importance / 10.0)

// Personality adjustment
if category in ["security", "config"]:
    score += conscientiousness × 0.2
if category in ["observation", "exploration"]:
    score += openness × 0.15
```

### Thresholds

- `should_promote`: score ≥ 0.7 (high-value, worth preserving)
- `should_archive`: score < 0.2 AND age > 168 hours (low-value, candidate for archival)

The system reports archival candidates but does NOT auto-delete. Human decision required.

---

## 10. The FileLoggingMemoryBackend (Decorator)

Wraps `AsyncMemoryBackend` and adds file-based logging.

### Behavior

```
store(agent_id, ChatMessage)
    ├─ IF channel == Memory:
    │   └─ record_learning() → appends to LEARNINGS.md
    │       Format: "\n\n### Learning ({timestamp}) [{tag}]\n{content}\n"
    └─ inner.store() → AsyncMemoryBackend.store()

consolidate(agent_id)
    ├─ inner.consolidate() → LSM compaction + DAG archiving
    ├─ parse_learnings() → LEARNINGS.md → LEARNINGS.jsonl
    └─ store in "swarm.insights" session
```

### LEARNINGS.md → LEARNINGS.jsonl Pipeline

1. Agent writes freeform to LEARNINGS.md
2. `LearningsParser` splits on `### Learning (` headers
3. Extracts timestamps and `[CATEGORY]` tags
4. Deduplicates by content fingerprint (first 200 chars, normalized)
5. Appends new entries to LEARNINGS.jsonl
6. Dashboard reads .jsonl for structured display

---

## 11. Auto-Recall (Context Injection)

Provides semantic search results for injection into the system prompt.

### Process

```
auto_recall(agent_id, query_text, config)
    1. Fetch last 10 messages
    2. Extract last 3 User messages as query window
    3. Join with " | " separator
    4. Embed query window
    5. Semantic search with config.max_results
    6. Filter by config.similarity_threshold
    7. Estimate tokens (4 chars ≈ 1 token)
    8. Respect config.max_tokens limit
    9. Return ContextCacheBlock
```

### Default Config

```rust
AutoRecallConfig {
    max_tokens: 2000,
    similarity_threshold: 0.3,
    max_results: 5,
}
```

---

## 12. Persistence & Durability Guarantees

### What Persists Permanently

| Data Type | Storage | Durability |
|---|---|---|
| Conversation transcripts | CortexaDB (WAL-backed) | Hard durability, sync after every write |
| Semantic memory entries | CortexaDB + vector engine | Dual-persisted (LSM + vectors.rkyv) |
| Distilled triplets | Collective enclave | 1-year JWT expiry |
| SPO facts | Collective LSM | Permanent until arbiter prunes |
| DAG compaction nodes | CortexaDB | Permanent (references archived messages) |
| Session/turn state | CortexaDB (rkyv) | Hard durability |
| LEARNINGS.md | Append-only Markdown | Filesystem-level |
| LEARNINGS.jsonl | Parsed from .md | Rebuilt on consolidation |
| Daily logs | Append-only Markdown | Rotated after retention period |
| Vector index | vectors.rkyv (atomic write) | Auto-persist on Drop |

### Crash Safety

- **WAL**: Every write is journaled before application. Crash recovery replays the WAL.
- **Atomic compaction**: Write-before-delete pattern. Crash during compaction = temporary duplicates, never data loss.
- **Vector persistence**: Atomic temp-file + rename. Crash during write = old file remains.
- **Dimension mismatch**: Auto-detection and recovery on startup.

### What CAN Be Lost

- In-flight writes not yet synced (mitigated by `strict_sync: true` default)
- In-memory dedup hash map (rebuilt on restart from WAL)
- Session registry (rebuilt on startup from `_registry` collection)

---

## 13. "Forever Memory" Design Philosophy

The system is designed so that **data is permanent by default**:

1. **No automatic deletion**: The promotion engine identifies low-value memories but reports them — it doesn't delete them.
2. **Archival over deletion**: Old messages get `channel="Archive"` and are filtered from queries, but remain in storage.
3. **DAG preservation**: Summarization creates new nodes referencing originals. Originals are never removed.
4. **Arbiter conservatism**: High-entropy contradictions (>1.5 bits) are flagged for human audit, never auto-deleted.
5. **Distillation redundancy**: Triplets are stored in both vector engine AND facts collection. Deleting one doesn't lose the other.
6. **Dual-enclave separation**: Private memory (enclave) and collective memory (collective) are independent. Losing one doesn't affect the other.
7. **Write-ahead logging**: Every mutation is journaled before application. Crash recovery is deterministic.

The result: the system accumulates knowledge permanently. Knowledge can be organized, deduplicated, and summarized — but it's never thrown away unless a human explicitly decides to.

---

## 14. Key Source Files

| Component | File | Lines |
|---|---|---|
| MemoryEngine + Enclave | `crates/memory/src/engine.rs` | 580 |
| LSM Storage Engine | `crates/memory/src/lsm_engine.rs` | 1140 |
| Vector Engine | `crates/memory/src/vector_engine.rs` | 829 |
| Async Backend | `crates/memory/src/async_backend.rs` | 820 |
| Data Models | `crates/memory/src/models.rs` | 874 |
| Distillation Pipeline | `crates/memory/src/distillation.rs` | 234 |
| Factual Arbiter | `crates/memory/src/arbiter.rs` | 104 |
| Promotion Engine | `crates/memory/src/promotion.rs` | 254 |
| Notifications | `crates/memory/src/notifications.rs` | 158 |
| FileLogging Decorator | `crates/agent/src/memory/mod.rs` | 254 |
| MemoryBackend Trait | `crates/core/src/traits/mod.rs` | 116 |
| Database Layer | `crates/core/src/db.rs` | 278 |
| LEARNINGS Parser | `crates/agent/src/learning/parser.rs` | ~200 |

---

*Documentation generated from source code audit. Last updated: 2026-03-27.*

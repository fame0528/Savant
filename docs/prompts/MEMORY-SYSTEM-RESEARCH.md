# Deep Research Prompt: Savant Agent Memory System Improvements

> **Purpose:** Second-pass research for improving Savant's memory system  
> **Model:** Gemini 3 Pro Deep Research  
> **Instructions:** Research thoroughly. Cite sources. Provide architectural recommendations grounded in production systems. Prioritize local-first, Rust-native solutions.

---

## 1. Project Context

**Savant** is a Rust-native autonomous AI agent swarm orchestrator. It is NOT a chatbot — it is an operating system for autonomous agents that execute coding tasks, manage infrastructure, and collaborate across channels (Discord, Telegram, WhatsApp).

- **14 Rust crates**, fully async (Tokio), WebSocket gateway (Axum), Next.js dashboard
- **Swarm architecture:** Multiple agents run simultaneously, each with its own workspace, SOUL.md personality, and session history
- **Primary use case:** Coding agents that execute shell commands, read/write files, deploy services, and reason about complex problems over days/weeks
- **Deployment:** Local-first, no cloud dependencies, runs on user's machine

---

## 2. Current Memory Architecture

### 2.1 Storage Backends

**Fjall LSM-tree** (embedded, local-first, no server):
```
./data/memory/          ← Fjall LSM tree (transcripts + metadata)
./data/memory/vectors/  ← Vector engine (Fjall-backed, 384-dim)
```

**Key constraint:** Fjall uses file-level locking. Two instances CANNOT open the same path. Memory engine and storage engine must use separate paths.

### 2.2 Core Data Structures

**AgentMessage** (transcript unit, stored via rkyv zero-copy):
```rust
pub struct AgentMessage {
    pub id: String,                          // UUID v4
    pub session_id: String,                  // Agent session (sanitized)
    pub role: MessageRole,                   // System=0, User=1, Assistant=2, Tool=3
    pub content: String,                     // Message text
    pub tool_calls: Vec<ToolCallRef>,        // Inline tool invocations
    pub tool_results: Vec<ToolResultRef>,    // Inline tool results
    pub timestamp: rend::i64_le,             // Unix millis
    pub parent_id: Option<String>,           // Conversation threading
    pub channel: String,                     // Output channel (Chat, Telemetry)
}
```

**MemoryEntry** (semantic index unit):
```rust
pub struct MemoryEntry {
    pub id: rend::u64_le,                    // Unique memory ID
    pub session_id: String,                  // Source session
    pub category: String,                    // "fact", "preference", "observation"
    pub content: String,                     // Distilled memory text
    pub importance: u8,                      // 1-10 (used for compaction prioritization)
    pub tags: Vec<String>,                   // Filtering labels
    pub embedding: Vec<f32>,                 // 384-dim vector (FastEmbed AllMiniLML6V2)
    pub created_at: rend::i64_le,            // Creation timestamp
    pub updated_at: rend::i64_le,            // Last update
    pub shannon_entropy: rend::f32_le,       // Informational density
    pub last_accessed_at: rend::i64_le,      // Temporal heuristic
    pub hit_count: rend::u32_le,             // Access frequency
    pub related_to: Vec<rend::u64_le>,       // Relational edges to other memories
}
```

**Memory layers already defined in engine.rs:**
```rust
pub enum MemoryLayer {
    Episodic,    // L0: High-frequency transient logs
    Contextual,  // L1: Aggregated workspace and session state
    Semantic,    // L2: SIMD-accelerated long-term storage
}
```

### 2.3 Current Operations

| Operation | Implementation | What it does |
|-----------|---------------|--------------|
| `append_message()` | LsmStorageEngine | Appends to session transcript (rkyv serialized) |
| `fetch_session_tail()` | LsmStorageEngine | Returns last N messages for a session |
| `atomic_compact()` | LsmStorageEngine | Deletes old messages, inserts compacted batch |
| `consolidate()` | AsyncMemoryBackend | Auto-summarizes when session has 50+ messages, keeps last 20 |
| `index_memory()` | MemoryEngine | Stores MemoryEntry in vector index + LSM metadata |
| `semantic_search()` | SemanticVectorEngine | Cosine similarity k-NN search (384-dim) |
| `cull_low_entropy_memories()` | MemoryEngine | Removes entries below Shannon entropy threshold |
| `hydrate_session()` | MemoryEngine | Combines transcripts with semantic memories |
| `delete_session()` | MemoryEngine | Purges transcript + cascades vector cleanup |

### 2.4 Data Flow

```
User Input
    ↓
WebSocket (Axum gateway)
    ↓
AsyncMemoryBackend::store()
    ├── AgentMessage::from_chat() → convert to internal format
    ├── MemoryEngine::append_message() → write to Fjall LSM
    ├── EmbeddingService::embed() → generate 384-dim vector
    └── MemoryEngine::index_memory() → store in vector engine

Retrieval:
    ↓
AsyncMemoryBackend::retrieve()
    ├── EmbeddingService::embed(query) → query vector
    ├── MemoryEngine::semantic_search() → cosine similarity
    ├── MemoryEngine::fetch_session_tail() → recent messages
    └── Merge + deduplicate → return Vec<ChatMessage>
```

### 2.5 Consolidation Flow (Already Implemented)

```
When session has 50+ messages:
    1. Fetch last 500 messages
    2. Split: older messages → consolidate, last 20 → keep as-is
    3. Create summary of older messages (AgentMessage with role=System)
    4. atomic_compact() → delete old, insert [summary + recent 20]
```

This is the session compaction layer from the research. It already exists.

### 2.6 Token Budget (Already Implemented)

```rust
pub struct TokenBudget {
    pub limit: usize,      // Max tokens (default 8192)
    pub used: usize,       // Current usage
}

// Allocations: system 20%, recent 50%, semantic 20%, old 10%
// Triggers summarize at 80% usage
// Token estimation: 4 chars ≈ 1 token
```

---

## 3. Swarm Architecture

```
workspaces/
├── substrate/
│   ├── agent.json          ← System-level config
│   └── .env                ← API keys (Management key)
├── agents/
│   ├── agent-alpha/
│   │   ├── SOUL.md         ← Personality (MBTI, OCEAN, core laws)
│   │   ├── AGENTS.md       ← Operating instructions
│   │   └── .env            ← Agent-specific keys (optional)
│   └── agent-beta/
│       └── ...
└── workspace-Savant/       ← System agent workspace
```

**Memory sharing model:**
- Each agent has its own session_id
- Transcript storage is per-session (agents don't see each other's transcripts)
- Semantic memories are shared across the swarm (global vector index)
- The swarm can collaborate via IPC (blackboard, collective voting)

**Critical constraint:** Memory operations must be thread-safe across multiple concurrent agents.

---

## 4. What We Already Have (Confirmed)

| Research Layer | Savant Status | Implementation |
|---|---|---|
| Session Memory (Layer 1) | ✅ EXISTS | `MemoryEngine` + `consolidate()` + `atomic_compact()` |
| Semantic Search (Layer 2 partial) | ✅ EXISTS | `VectorEngine` + `EmbeddingService` (FastEmbed 384-dim) |
| Token Budget | ✅ EXISTS | `TokenBudget` (20/50/20/10 allocation, 80% summarize trigger) |
| Local-first storage | ✅ EXISTS | Fjall LSM-tree, no server |
| Entropy-based pruning | ✅ EXISTS | `cull_low_entropy_memories()` with Shannon entropy |
| Access frequency tracking | ✅ EXISTS | `hit_count` + `last_accessed_at` in MemoryEntry |
| Relational edges | ✅ EXISTS | `related_to: Vec<rend::u64_le>` in MemoryEntry |

---

## 5. Identified Gaps (Need Research)

### Gap 1: Auto-Recall / Pre-Prompt Context Injection

**Current state:** Agent must manually call `memory_search` to recall past context. If the agent doesn't think it needs to search, it hallucinates.

**What we want:** Before the LLM sees any prompt, the system automatically:
1. Embeds the user's query
2. Searches the semantic memory store
3. Injects relevant context into the system prompt silently
4. The agent sees the context as if it always knew it

**Research questions:**
- What are production implementations of auto-recall / pre-prompt context injection?
- How do they handle latency (<200ms target for sub-second agent response)?
- How do they handle context budget enforcement (don't overflow the system prompt)?
- What's the best way to structure a `<context_cache>` XML block in the system prompt?
- Should it be synchronous (block before prompt) or async (background enrichment)?
- How do you handle relevance scoring when the query is ambiguous?
- What are the failure modes? (stale context, conflicting facts, context bloat)

### Gap 2: Bi-Temporal Tracking

**Current state:** MemoryEntry has `created_at` and `updated_at` but no concept of "when this fact became true in reality" vs "when it was recorded."

**Example failure:** User says "budget is $500" then later "$2000." Both facts exist without resolution. Agent doesn't know which is current.

**What we want:**
- `valid_from`: When the fact became true in the real world
- `valid_to`: When the fact ceased to be true (infinity if current)
- `recorded_at`: When the agent stored it
- Contradiction detection: When new fact contradicts old, set old's `valid_to`
- Query filter by active validity

**Research questions:**
- Is full bi-temporal tracking (like Zep/Graphiti) worth the complexity for a coding agent?
- What's a simpler approach that still handles contradictions? (version chains? just latest-wins?)
- How do you detect contradictions automatically without LLM validation on every write?
- What's the performance overhead of maintaining temporal metadata?
- Are there Rust libraries for bi-temporal data structures?

### Gap 3: Knowledge Promotion (Transient → Canonical)

**Current state:** All memories live in the same vector store. No distinction between "this was said in passing" and "this is architectural truth."

**What we want:**
- After a session ends, analyze stored memories
- Identify durable facts (high `hit_count`, referenced across sessions, low entropy = high confidence)
- Promote them to a "canonical" layer with different retrieval priority
- Demote or archive low-value memories

**Research questions:**
- What algorithms work best for identifying durable vs ephemeral knowledge?
- Is access frequency alone sufficient, or do you need semantic analysis?
- How do production systems decide what to promote?
- What's the minimum viable promotion system? (hit_count + time_window?)
- How do you handle promotion of conflicting facts?

### Gap 4: Daily Operational Logs

**Current state:** No append-only log of execution state. Agent forgets what it tried yesterday.

**What we want:**
- Append-only log per agent per day: `memory/YYYY-MM-DD.md` or `.json`
- Records: what was attempted, what failed, what succeeded, current blockers
- Loaded on session start for immediate orientation
- Token-efficient (shouldn't consume more than 500 tokens)

**Research questions:**
- What format works best? Markdown for human readability? JSON for machine parseability?
- How do production agent systems structure daily logs?
- How much token budget should daily logs consume?
- How do you prevent daily logs from becoming stale or bloated?
- Should daily logs be cross-referenceable with the semantic memory store?

### Gap 5: Cross-Session Entity Tracking

**Current state:** We store individual messages but don't track entities (people, projects, API keys, services) across sessions.

**What we want:**
- When the agent mentions "the Stripe API key" or "Project Alpha," track as entities
- Entities have: name, type, relationships, temporal metadata, first/last seen
- Query: "What do I know about Project Alpha?" → returns all related memories

**Research questions:**
- What's the simplest effective entity extraction approach for agent memory?
- Is NER (Named Entity Recognition) worth the complexity, or can the LLM itself extract entities during write?
- How do you handle entity resolution (same entity, different mentions)?
- Do you need a full knowledge graph, or is a simpler entity registry sufficient?
- What are the Rust options for lightweight NER or entity extraction?

### Gap 6: Memory Quality Decay & Eviction

**Current state:** `cull_low_entropy_memories()` exists but uses only Shannon entropy. No time-based decay, no importance-weighted eviction.

**What we want:**
- Importance-weighted LRU: memories with low `importance` + high age + low `hit_count` get evicted first
- High-importance or frequently accessed memories survive regardless of age
- Evicted memories go to an archive (not deleted), can be restored

**Research questions:**
- What eviction policies work best for agent memory?
- Is pure LRU sufficient, or do you need semantic-aware eviction?
- What are the sweet spots for retention periods (7 days? 30 days? 90 days?)
- How do you balance memory freshness vs completeness?
- Should eviction be per-session or global?

### Gap 7: Context Assembly Order

**Current state:** `AsyncMemoryBackend::retrieve()` does semantic search + transcript tail. No prioritization or ordering.

**What we want:**
- When assembling context for the LLM, determine optimal order:
  1. System prompt (always first)
  2. Auto-recalled relevant memories (from semantic search)
  3. Recent session messages (from transcript)
  4. Daily log context (from operational logs)
  5. Canonical facts (if relevant)
- Budget allocation across these layers

**Research questions:**
- What's the optimal context assembly order for coding agents?
- How do you prevent context conflicts when different layers return contradictory info?
- What's the right budget split between semantic memories and recent messages?

---

## 6. Technical Constraints

**MUST:**
- Remain fully local-first (no cloud, no external APIs for memory)
- Use Fjall (committed, file locking model)
- Use FastEmbed (committed, 384-dim AllMiniLML6V2)
- Be thread-safe for concurrent agent access
- Integrate with existing `MemoryEngine`, `VectorEngine`, `EmbeddingService`
- Work with rkyv serialization (zero-copy, `#[repr(C)]` structs)

**MUST NOT:**
- Switch from Fjall to SQLite
- Add external vector databases (Pinecone, Weaviate, Qdrant)
- Add heavyweight dependencies (neo4j, surrealdb)
- Use cloud APIs for embedding (no OpenAI, no Cohere)
- Break existing `MemoryEntry` struct (backward compatibility with stored data)

---

## 7. Expected Output

Please provide:
1. **Recommended approach for each gap** with trade-offs
2. **Implementation priority order** (what delivers most value first)
3. **Complexity estimates** per feature (simple/medium/high)
4. **Integration points** with existing Savant architecture
5. **Code-level design patterns** for Rust/Tokio/Fjall
6. **Performance characteristics** (latency, memory overhead, storage growth)
7. **Failure modes** and how to handle them
8. **Production benchmarks** or case studies from similar systems

---

*Copy this entire document into Gemini 3 Pro Deep Research. The more specific the research, the better the implementation.*

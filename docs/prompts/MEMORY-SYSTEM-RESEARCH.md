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
- What data structures work best for entity storage in a local-first system?
- How do you handle entity merging when two references resolve to the same entity?

### Gap 8: Contradiction Resolution

**Current state:** No mechanism to detect or resolve contradictory facts.

**Example failures:**
1. User says "use port 3000" then later "switch to port 8080." Both stored. Agent doesn't know which is current.
2. User says "the API key is X" then "the API key is Y." Old key still indexed, might be used.
3. Code changes a config value. Old value still in memory. Agent uses stale config.

**What we want:**
- When new fact contradicts existing fact, automatically resolve (latest wins, or flag for review)
- Keep history of changes (don't delete old facts, just mark superseded)
- Query returns only current/active facts unless explicitly asked for history

**Research questions:**
- What's the most effective contradiction detection approach? Semantic similarity threshold? Rule-based? LLM validation?
- How do you handle contradictions across sessions? (Agent A says X, Agent B says Y)
- Is "latest wins" sufficient for most cases, or do you need more sophisticated resolution?
- How do you handle contradictions in code artifacts? (Config file says X, memory says Y)

### Gap 9: Memory Graph / Relationship Tracking

**Current state:** `MemoryEntry` has `related_to: Vec<rend::u64_le>` but it's never populated. No graph traversal.

**What we want:**
- When memories are related (same topic, same entity, causal chain), link them
- Query: "What's related to this memory?" → traverse graph
- Support reasoning chains: "I did X because of Y, which led to Z"

**Research questions:**
- What's the simplest graph structure that works for agent memory? Adjacency list? Edge list?
- How do you automatically detect related memories without expensive pairwise comparison?
- Is graph traversal worth the complexity, or is tag-based grouping sufficient?
- What's the memory overhead of maintaining relationship edges?

### Gap 10: Cross-Agent Memory Sharing

**Current state:** Each agent has its own session. Semantic memories are indexed globally but there's no explicit sharing protocol.

**What we want:**
- Agent A discovers something useful → can explicitly share with Agent B
- Shared memories are marked as such (not accidentally treated as personal context)
- Swarm-wide facts vs agent-specific facts

**Research questions:**
- What's the right sharing model? Push-based (agent decides what to share)? Pull-based (agent queries swarm memory)?
- How do you prevent context contamination (Agent A's personality leaking into Agent B)?
- What metadata is needed for shared memories? (source agent, confidence, scope)
- How do you handle conflicting shared memories from different agents?

---

## 8. Specific Failure Scenarios

These are real problems we've encountered that the research should address:

### Scenario 1: Long Coding Session Amnesia
**Setup:** Agent spends 3 hours debugging a Docker networking issue. Session has 200+ messages.
**Problem:** When the session compacts (consolidates), the summary loses critical details like "the issue was iptables rules on the host, not the container." Agent wastes 30 minutes re-discovering this.
**Question:** How do you preserve critical debugging context during compaction?

### Scenario 2: API Key Rotation
**Setup:** User rotates OpenRouter API key. Old key was stored in memory.
**Problem:** Agent sometimes tries the old key (from semantic memory) before getting the new one from config.
**Question:** How do you handle sensitive data that becomes invalid? TTL? Immediate invalidation?

### Scenario 3: Cross-Session Architecture Decision
**Setup:** Agent decides to use Fjall over SQLite in Session 1. In Session 2 (days later), user asks "why not SQLite?"
**Problem:** Agent doesn't remember the decision rationale. Semantic search might return unrelated results.
**Question:** How do you ensure architectural decisions persist across sessions?

### Scenario 4: Swarm Coordination Memory
**Setup:** Agent Alpha discovers a bug. Agent Beta later encounters the same bug.
**Problem:** Agent Beta has no way to know Agent Alpha already found and documented this.
**Question:** How do you propagate discovered knowledge across agents?

### Scenario 5: Stale Configuration Memory
**Setup:** Agent stores "model is gpt-4" in memory. User switches to "hunter-alpha" in config.
**Problem:** Memory still says gpt-4. Agent might use stale model info.
**Question:** How do you invalidate memory when external config changes?

---

## 9. Performance Requirements

| Metric | Target | Current |
|--------|--------|---------|
| Memory search latency | <100ms | ~50ms (local FastEmbed) |
| Context injection overhead | <200ms | Not implemented |
| Storage per message | <1KB | ~500 bytes (rkyv) |
| Vector index memory | <100MB for 10K entries | ~15MB (384 * 4 bytes * 10K) |
| Compaction time | <500ms for 500 messages | ~200ms (measured) |
| Embedding generation | <50ms per text | ~30ms (FastEmbed local) |
| Total memory footprint | <500MB | ~50MB (current) |

---

## 10. Existing Research References

The first-pass research document identified these systems as relevant:
- **Zep:** Bi-temporal knowledge graph for agent memory
- **Graphiti:** Neo4j-based knowledge graph memory
- **MemGPT:** Virtual context management (OS-inspired paging)
- **Lossless Claw (OpenClaw):** DAG-based session compaction
- **sqlite-memory:** Markdown-based agent memory with hybrid search
- **Hindsight:** Adaptive forgetting with importance weighting

We need the second pass to go deeper into:
- Actual implementation details (not just concepts)
- Performance benchmarks and trade-offs
- Failure modes and how they're handled
- Simpler alternatives that work in practice

---

## 11. Code-Level Integration Points

These are the exact functions that would need modification:

| Function | File | Current | Proposed Change |
|----------|------|---------|-----------------|
| `store()` | `async_backend.rs` | Appends + embeds | Add entity extraction, contradiction detection |
| `retrieve()` | `async_backend.rs` | Semantic + transcript | Add auto-recall, context assembly |
| `consolidate()` | `async_backend.rs` | Summarizes old messages | Add importance-aware compaction |
| `index_memory()` | `engine.rs` | Stores in vector index | Add validity tracking, promotion logic |
| `cull_low_entropy_memories()` | `engine.rs` | Entropy-based pruning | Add time-decay, importance weighting |
| `hydrate_session()` | `engine.rs` | Combines transcripts + semantic | Add daily log injection |
| `new()` | `MemoryEngine` | Initializes backends | Add daily log initialization |
| `embed()` | `EmbeddingService` | Single text embedding | Already works, used by auto-recall |

---

## 12. Expected Output Format

Please structure your response as:

### For each gap:
1. **Recommended approach** (with 2-3 alternatives and trade-offs)
2. **Data structures** (Rust structs with field types)
3. **Integration with existing code** (which functions to modify)
4. **Performance characteristics** (latency, memory, storage)
5. **Failure modes** (what goes wrong, how to handle)
6. **Implementation priority** (1-10, where 1 = most impactful)

### Overall:
1. **Recommended implementation order** (sequence of features)
2. **Complexity summary** (simple/medium/high per feature)
3. **Dependencies** (which features depend on others)
4. **Quick wins** (high value, low effort)
5. **Production benchmarks** (from similar systems, if available)

---

## 13. Agent Identity & Memory Context

Each Savant agent has a **SOUL.md** that defines its personality, knowledge boundaries, and operating principles. This interacts with memory in specific ways:

### SOUL.md Structure (18 sections):
```
1. Systemic Core & Origin (designation, version, role, core directive)
2. Psychological Matrix (MBTI type, OCEAN Big Five traits, moral compass)
3. Archival Lineage (history of how this agent was built)
4. Linguistic Architecture (voice principles, anti-mechanical mandate)
5. Zero-Trust Execution (capability grants, WASM sandboxing)
6. Memory Safety (Kani verification, WAL guarantees)
7. Core Laws (10 unbreakable rules — same as coding system)
8. Guardian Protocol (20-point compliance monitoring)
9. Flawless Protocol (12-step implementation methodology)
10. Nexus Flow (swarm orchestration rules)
11. Strategic Maxims (30 decision-making principles)
12. Savant Lexicon (agent-specific terminology)
13. Recursive Reflection (self-improvement protocol)
14. Interaction Loops (scenario-specific response patterns)
15. The Savant Creed (identity statement)
16. Moral Registry (ethical framework)
17. Personality Matrix (5 behavioral pillars)
18. Daily Operational Flow (work schedule, break patterns)
```

**Memory implication:** The agent's SOUL.md affects how it processes and stores memories. A "Security Paranoid" agent (high conscientiousness) might weight security-related memories higher. A "Creative Explorer" agent (high openness) might store more experimental observations.

**Research question:** Should the SOUL.md personality traits influence memory storage (e.g., importance weighting, categorization)? Or should memory be personality-agnostic?

### Agent Workspace Isolation:
```
workspaces/agents/agent-alpha/
├── SOUL.md           ← Personality (affects reasoning, NOT memory access)
├── AGENTS.md         ← Operating instructions
├── IDENTITY.md       ← Identity card
├── .env              ← Agent-specific API keys (optional)
├── memory/           ← Agent-specific memory (per-session)
│   └── 2026-03-19/   ← Daily logs (proposed)
└── projects/         ← Agent's working files
```

**Memory sharing model:**
- `./data/memory/` (Fjall) → Global semantic index (all agents share)
- `workspaces/agents/<name>/` → Agent-specific context (NOT shared)
- The swarm coordinator can query global memory across all agents

---

## 14. Fjall Partition Structure

The current Fjall storage uses these partitions:

| Partition | Contents | Access Pattern |
|-----------|----------|----------------|
| `transcripts:<session_id>` | Serialized AgentMessage bytes | Append + range scan |
| `metadata:<entry_id>` | Serialized MemoryEntry bytes | Random read + write |
| `dedup_hashes` | blake3 content hashes for dedup | Random read |
| `partition_counters` | Global monotonic counters | Atomic increment |

**Important:** Fjall partitions are logical key-value namespaces within a single database instance. Each partition is NOT a separate database — they share the same WAL and compaction.

**Vector storage** uses a separate Fjall instance at `./data/memory/vectors/` with:
- HNSW index (384-dim, cosine similarity)
- 32 neighbors per node, 200 connections per layer
- Persistent header + vector data

---

## 15. Embedding Pipeline Details

**Model:** FastEmbed AllMiniLML6V2 (384 dimensions)
- Downloaded on first use (~80MB), cached locally
- Runs on a dedicated thread (TextEmbedding is NOT Send in older versions, but IS Send in 5.12.1)
- LRU cache: 1000 entries, keyed by input text string
- Batch embedding: `embed_batch()` processes multiple texts in one inference call

**Embedding flow:**
```
Input text (e.g., "Deploy the API to production")
    ↓
Cache check (LRU, string key)
    ↓ (miss)
FastEmbed inference (384-dim vector)
    ↓
Cache insert
    ↓
Return Vec<f32>
```

**Research question:** For auto-recall, do we embed the full query or extract keywords first? Full query gives semantic context but is slower. Keywords are faster but might miss nuanced intent.

---

## 16. Consolidation Details (What Already Works)

The `consolidate()` function in `async_backend.rs` already implements session compaction:

```rust
async fn consolidate(&self, agent_id: &str) -> Result<(), SavantError> {
    // 1. Fetch last 500 messages from session
    let messages = engine.fetch_session_tail(&sid, 500);

    // 2. Skip if <50 messages (not worth compacting)
    if messages.len() < 50 { return Ok(()); }

    // 3. Split: older → consolidate, last 20 → keep
    let (to_consolidate, recent) = messages.split_at(messages.len() - 20);

    // 4. Create summary (lightweight text, not full transcript)
    let summary = create_conversation_summary(&to_consolidate);

    // 5. Atomic compact: delete old, insert [summary + recent 20]
    engine.atomic_compact(&sid, vec![summary_msg, ...recent])
}
```

**Limitations of current consolidation:**
1. Summary is lossy (creates one text block from all old messages)
2. No way to expand back to original messages (they're deleted)
3. No importance weighting (all messages treated equally)
4. No preservation of critical debugging context
5. Fixed threshold (50 messages) regardless of content

**Research question:** How do you make compaction reversible? DAG-based approach? Keep raw messages in archive layer?

---

## 17. Additional Research Questions

### On Architecture:
1. Should auto-recall be a separate crate or integrated into the memory crate?
2. What's the right crate boundary for memory subsystem components?
3. How do you test memory systems? What are the critical test cases?

### On Performance:
4. What's the real-world latency of FastEmbed inference for single queries?
5. How does Fjall LSM performance degrade as data grows? At what point do you need to shard?
6. What's the memory overhead of maintaining a 384-dim vector index for 100K entries?

### On Correctness:
7. How do you verify that memory operations are actually durable after a crash?
8. What are the consistency guarantees when multiple agents write simultaneously?
9. How do you detect and repair corruption in the vector index?

### On UX:
10. How should the dashboard display memory state to users? (memory browser, search, stats)
11. How do you let users see what the agent "remembers" about them?
12. How do you let users correct or delete memories they don't want stored?

---

*Copy this entire document into Gemini 3 Pro Deep Research. Include all sections for maximum research depth.*


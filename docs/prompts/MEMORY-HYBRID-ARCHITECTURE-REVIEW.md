# Deep Research Prompt: Savant 5-Layer Hybrid Memory System & Hive-Mind Architecture

> **Purpose:** Comprehensive architectural critique and optimization blueprint for implementing a new 5-Layer Hybrid Architecture for Savant.  
> **Model:** Gemini 3 Pro Deep Research  
> **Status:** Clean-Slate Design (No legacy data migration required).  
> **Core Philosophy:** "Both of Both Worlds" — SQLite for Relational/Structured Data, Fjall/Vectors for Semantic Search.

---

## 1. Project Context & Current Baseline

**Savant** is a Rust-native autonomous AI agent swarm orchestrator. It is an operating system for autonomous agents that execute coding tasks and collaborate via Axum-powered WebSockets.

### **1.1 The Baseline Primary Substrate (Current)**
We currently utilize **Fjall** (an embeddable LSM-tree) for persisting all transcript data and semantic indices.
- **Transcripts:** Stored in `transcripts:<session_id>` partitions using `rkyv` zero-copy serialization.
- **Semantic Vectors:** Managed by `ruvector-core` (HNSW) inside of a separate Fjall instance.
- **Embeddings:** FastEmbed (AllMiniLML6V2, 384-dim) running locally on a dedicated thread.

### **1.2 Core Data Structures (Rust)**

```rust
pub struct AgentMessage {
    pub id: String,                          // UUID v4
    pub session_id: String,                  // Agent session (sanitized)
    pub role: MessageRole,                   // System=0, User=1, Assistant=2, Tool=3
    pub content: String,                     // Message text
    pub tool_calls: Vec<ToolCallRef>,        // Inline tool invocations
    pub tool_results: Vec<ToolResultRef>,    // Inline tool results
    pub timestamp: rend::i64_le,             // Unix millis
}

pub struct MemoryEntry {
    pub id: rend::u64_le,                    // Unique memory ID
    pub session_id: String,                  // Source session
    pub content: String,                     // Distilled memory text
    pub importance: u8,                      // 1-10
    pub embedding: Vec<f32>,                 // 384-dim vector
    pub shannon_entropy: rend::f32_le,       // Informational gain
    pub hit_count: rend::u32_le,             // Access frequency
    pub related_to: Vec<rend::u64_le>,       // DAG edges to other memories
}

pub struct TemporalMetadata {
    pub valid_from: i64,                     // Reality-start epoch
    pub valid_to: Option<i64>,               // Reality-end epoch
    pub recorded_at: i64,                    // Agent-storage epoch
    pub superseded_by: Option<u64>,          // ID of the fact that replaced this one
    pub entity_name: String,                 // e.g., "api_port", "user_is_admin"
}
```

---

## 2. Proposed Architecture: The 5-Layer Hybrid System

We are building a new 5-Layer hybrid substrate. SQLite will displace Fjall for all relational/structured logic, while Fjall remains the high-speed backend for vector descriptors.

### **Layer 1: Lossless Context Management (LCM - The DAG)**
- **Concept:** Transition from sliding-window destructive summarization to Contextual Memory Virtualization.
- **Substrate:** `rusqlite` (Embedded SQLite).
- **Operation:** Transcripts are never deleted. They are hierarchically summarized into a tree structure.
  - **The "Tail":** Last 15-20 messages are raw text.
  - **The "Branch":** Messages 21-100 are summarized into a parent `DagNode`.
  - **The "Root":** High-level session objectives.
- **Reversibility:** Agents can call `expand_node(node_id)` to re-fetch raw historical tokens into their active context window for high-precision debugging.

### **Layer 2: Bi-Temporal Knowledge & Reciprocal Rank Fusion (RRF)**
- **Concept:** Cross-session semantic recall with exact temporal invalidation.
- **Substrate:** Hybrid SQLite FTS5 + Vector Engine.
- **Recall Engine:** Employs **RRF** to fuse results from:
  1. **SQLite FTS5:** Lexical matching (BM25) for specific keywords/IDs.
  2. **FastEmbed:** Semantic k-NN (Cosine Similarity) for conceptual meaning.
- **Bi-Temporal Invalidation:** When a new fact ("port is 8080") is stored, the system searches SQLite for highly similar entities ("port is 3000"). If a match exists, the old metadata's `valid_to` is updated, and the new entry is marked current.

### **Layer 3: Canonical Knowledge (PARA File System)**
- **Concept:** Indestructible, deterministic ground truth.
- **Structure:**
  ```text
  ./knowledge/
  ├── Projects/           ← Active project specs, architecture docs
  ├── Areas/              ← Long-term responsibilities (e.g., Security, UI)
  ├── Resources/          ← Technical docs, API references (Markdown)
  └── Archives/           ← Completed project debriefs
  ```
- **Operation:** Agents treat this as a "Read-Only Library". It is the source of truth that overrides all probabilistic vector model predictions.

### **Layer 4: Procedural Daily Logs**
- **Concept:** Situational orientation after a restart.
- **Format:** `memory/logs/YYYY-MM-DD.md`.
- **Content:** Milestones achieved, failed command attempts, current blockers, and "Next Steps" for tomorrow.
- **Boot Flow:** Upon agent initialization, the last 48 hours of logs are injected into the agent's hidden thought block (`<internal_reflection>`).

### **Layer 5: Orchestrator (Autonomous Push Recall)**
- **Concept:** Passive context enrichment.
- **Operation:** Middleware intercepts the user input → generates embedding → performs `tokio::join!` parallel execution across Layer 1 (Session) and Layer 2 (Semantic) → injects results into an XML block.
- **XML Structure:**
  ```xml
  <context_cache>
    <memory id="123" similarity="0.95" valid="current">
      User preferred dark mode in session alpha.
    </memory>
  </context_cache>
  ```

---

## 3. The Savant Hive-Mind (Swarm Collective Intelligence)

The 5-layer system is NOT per-agent; it is a shared intelligence plane.
- **Collective Memory:** All agents share the Layer 2 (Semantic) and Layer 3 (Canonical) substrates.
- **Discovery Broadcast:** When an agent updates a fact in SQLite with importance >= 7, it triggers a `broadcast` event.
- **Asynchronous Ingestion:** Other agents in the swarm receive this update via Tokio broadcast channels and update their internal "Belief State" without needing to re-index the data.

---

## 4. Deep Research Assignment & Critical Critique

Audit this blueprint relentlessly across these five domains:

### **Domain 1: Multi-Master Synchronization & Atomicity**
1. **The "Orphaned Vector" Problem:** If a write fails in SQLite but succeeds in Fjall (or vice versa), we have a corrupted memory state. Suggest a Robust Transactional Pattern (e.g., WAL-checkpointing, or a unified Write-Ahead Log) for a Dual-DB system in Rust.
2. **Global Atomicity:** How does the Hive-Mind handle "Distributed Brain Fog"? If Agent A and Agent B discover contradictory facts simultaneously, how does the 5-Layer SQLite/Vector system handle "Conflict Resolution" at a swarm level? Who is the "Truth Arbiter"?

### **Domain 3: Scaling & Performance Bottlenecks**
1. **Axum Parallelism vs SQLite Locking:** SQLite is single-writer. Under heavy Axum WebSocket traffic (multiple agents writing to the LCM DAG), how do we avoid `database is locked` errors? Compare `deadpool-sqlite` vs `sqlx` vs dedicated write-threads.
2. **Vector Retrieval Latency:** FastEmbed + RRF + SQLite FTS5. Can this really run in <200ms? Predict the latency curve as the database grows to 100,000 memories.

### **Domain 4: Contradiction Detection Security**
1. **"The Hallucinated Override":** If an agent misinterprets a user's statement and triggers a Bi-Temporal invalidation of a *true* fact, the system loses its anchor. How can we implement "Fact-Check Guards" (e.g., Confidence Thresholds, Entropy Checks) before allowing a `valid_to` update?
2. Critique the 0.92 Cosine Similarity threshold for contradictions. Is that too aggressive? Predict the risks.

### **Domain 5: Actionable Schema & Integration Recommendations**
1. Provide the exact SQL schema for the **Layer 1 DagNode** (Session Compaction).
2. Provide the recommended SQL schema for the **Layer 2 BiTemporalMemoryEntry**.
3. Propose a refined 7-step implementation roadmap, prioritizing the items that deliver the highest intelligence compounding with the lowest architectural risk.


### Domain 6: improvements to the 5-layer hybrid system
1. What are the top 10 improvements that can be made to the 5-layer hybrid system?
2. What are high impact ideas that i have missed that research shows would improve the system?
3. Should we add more layers to ensure total recall and the agent never forgets? 
4. should we combine other 0 cost technologies to improve the system?

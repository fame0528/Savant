# Savant Memory Trait Specification

> **Last Updated:** 2026-05-25 (v0.3.2)

## Overview

The `MemoryBackend` trait defines the operational contract for long-term and short-term memory persistence within the Savant substrate. It leverages **CortexaDB** LSM-tree for transactional durability and **Ruvector** for semantic similarity search.

## Trait Definition

```rust
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    /// Store a message in persistent session memory.
    /// Implementation must ensure atomic commitment to the transcripts keyspace.
    async fn store(&self, agent_id: &str, message: &ChatMessage) -> Result<(), SavantError>;

    /// Retrieve relevant context from memory using semantic similarity.
    /// Returns a vector of ChatMessages ordered by relevance and weight.
    async fn retrieve(
        &self,
        agent_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ChatMessage>, SavantError>;

    /// Finalize and optimize memory state (e.g., LlamaIndex compaction or IEG pruning).
    async fn consolidate(&self, agent_id: &str) -> Result<(), SavantError>;
}
```

## Memory Architecture (v0.3.2)

### Three-Layer Storage

| Layer | Technology | Purpose |
|:------|:-----------|:--------|
| **Hot** | SQLite WAL | Message history, agent state, configuration |
| **Warm** | CortexaDB LSM-Tree | Agent SOUL.md files, skill state, structured data |
| **Cold** | rkyv Vector Store | Semantic embeddings for memory retrieval |

### Dual-Enclave Architecture

| Enclave | Purpose | Access |
|:--------|:--------|:-------|
| **Private** | Per-agent memory (episodic, procedural) | Agent-only |
| **Collective** | Shared hive-mind memory (swarm-wide) | All agents |

### Subsystems

| Subsystem | Purpose |
|:----------|:--------|
| **BM25 Index** | Keyword search alongside vector search |
| **Reflective Memory** | 4-graph system (Semantic, Temporal, Causal, Entity) |
| **Procedural Memory** | Learned tool-call workflows |
| **Lessons** | Synthesized from repeated experiences |
| **Insights** | Higher-order concept cluster patterns |
| **Multimodal Store** | CLIP-embedded image references |
| **Audit Trail** | Application-level memory operation log |
| **Notification Channel** | Hive-mind broadcast for high-importance events |

## Glass House (Obsidian Bidirectional Sync)

Memory projected to Obsidian vault with bidirectional sync:

| Direction | Flow |
|:----------|:-----|
| **Outbound** | Memory → Vault (18 directories: Episodic, Semantic, Identity, Procedural, Lessons, Insights, Graphs, Dashboard, Themes, Working, Delegation, Multimodal, Retention) |
| **Inbound** | Vault → Memory (Semantic = Ground Truth, Personality = OCEAN updates, Episodic = Rejected/Corrections) |

All inbound edits pass through `scan_prompt()` injection defense.

## Learning Safety (v0.3.2 — Phase 1)

### Content-Hash Dedup

Rolling 10K entry window prevents duplicate learnings. Hash computed on normalized text.

### Per-Entry Length Cap

Maximum 2000 characters per learning entry. Entries exceeding the limit are truncated.

### LEARNINGS.md Rotation

File rotates at 100KB to `LEARNINGS-ARCHIVE-{timestamp}.md`. No data is lost.

### Trigger-Path Tagging

Every entry tagged with source (e.g., `memory_store`, `heartbeat_pulse`).

### Filtered Content Logging

Rejected entries logged to `FILTERED.jsonl` for human review with rejection reason.

## Security Considerations

1. **Isolation**: Memory must be partitioned strictly by `agent_id`. Under no circumstances should an agent be able to retrieve memories from a different identity unless explicitly shared via the `CollectiveBlackboard`.
2. **Entropic Pruning**: The implementation must utilize **Information-Entropy Gain (IEG)** to cull low-value memories, preventing context pollution and side-channel leakage.
3. **Preamble Injection**: Retrieval must be preceded by a **Sovereign Preamble v3** check to verify that context remains within its cognitive bounds.
4. **Injection Defense**: All inbound vault edits scanned via `scan_prompt()` before affecting agent state. Vault treated as potentially hostile data source.
5. **Memory Recall Deduplication**: Recalled memories injected into system prompt only (not duplicated in conversation history) to prevent attention waste.

---

*Documentation updated: 2026-05-25. Reflects v0.3.2 codebase.*

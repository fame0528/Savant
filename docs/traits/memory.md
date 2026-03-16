# Savant Memory Trait Specification [ADR-0004]

## Overview

The `MemoryBackend` trait defines the operational contract for long-term and short-term memory persistence within the Savant substrate. It leverages the **Fjall** LSM-tree for transactional durability and **Ruvector** for semantic similarity search.

## Trait Definition

```rust
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    /// Store a message in persistent session memory.
    /// Implementation must ensure atomic commitment to the transripts keyspace.
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

## Security Considerations [SECURITY-01]

1. **Isolation**: Memory must be partitioned strictly by `agent_id`. Under no circumstances should an agent be able to retrieve memories from a different identity unless explicitly shared via the `CollectiveBlackboard`.
2. **Entropic Pruning**: The implementation must utilize **Information-Entropy Gain (IEG)** to cull low-value memories, preventing context pollution and side-channel leakage.
3. **Preamble Injection**: Retrieval must be preceded by a **Sovereign Preamble v3** check to verify that context remains within its cognitive bounds.

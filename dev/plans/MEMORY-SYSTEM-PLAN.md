# 🧠 5-Layer Unified Cognitive Architecture Refactoring Plan

## 1. Goal
The primary objective of this phase is to refactor Savant's current Memory System (Fjall LSM-tree) to fully align with the **5-Layer Architecture** optimized by the Gemini Deep Research Blueprint.

This transition abandons the fragmented **SQLite + Fjall** pattern in favor of **CortexaDB**—a unified, Rust-powered local-first database that natively integrates Vector, Graph, and Temporal indexing under a single Write-Ahead Log (WAL), eliminating all multi-master synchronization hazards.

## 2. The 5-Layer Architecture Mapping

### **Layer 1 & 2: CMV & Episodic Memory (Unified on CortexaDB)**
- **Target Architecture**: **CortexaDB** (Native Vector + Graph + Time Search).
- **Implementation (Contextual Memory Virtualization)**: Transcripts are modeled as a Directed Acyclic Graph (DAG) using CortexaDB's graph capabilities. Provides "structurally lossless trimming."
- **Implementation (Hybrid Search)**: Querying Layer 2 instantly retrieves semantic matches (HNSW) and automatically traverses connected graph edges sorted by temporal recency.
- **Dependency Nuance (Identified in Perfection Loop)**: CortexaDB handles storage (HNSW/Graph), but we MUST retain `fastembed` for actual text-to-vector embedding generation.

### **Layer 3: Canonical Knowledge (Durable - PARA)**
- **Target Architecture**: The PARA (Projects, Areas, Resources, Archives) File System.
- **Implementation**: Complex architectural schemas, immutable API rules, and the agent's identity (`SOUL.md`) remain on the file system physically decoupled from probabilistic databases.

### **Layer 4: Procedural Logs (Daily Execution)**
- **Target Architecture**: Ephemeral Daily Extraction.
- **Implementation**: The agent distills procedural execution states organically into CortexaDB episodic memory.

### **Layer 5: Collective Hive-Mind (Arbitration & Privacy Boundary)**
- **Target Architecture**: **The Judge Node & Cryptographic Privacy Enclaves.**
- **Implementation (Privacy Boundaries)**: Cross-agent intelligence sharing must be mediated by JWT delegation. PII and raw transcripts stay in local Agent enclaves (`~/.savant/memory/{agent_id}.mem`).
- **Implementation (Arbitration)**: When Agent A and Agent B submit contradictory facts, the conflict triggers an **Arbiter Node**. All writes must pass a **Shannon Entropy Limit** (<1.5 bits). Uncertain LLM guesses are mathematically rejected.

## 3. Execution Roadmap

We will structure this refactoring sequentially across `crates/memory`:

1. **Phase 1: CortexaDB Substrate Migration**
   - Introduce `cortexadb` into `crates/memory/Cargo.toml` (Via Git).
   - Retain `fastembed` for embeddings. Remove `fjall` and `ruvector-core`.
   - Refactor `MemoryEngine` struct to initialize a single unified CortexaDB instance per agent (Private Enclave) and one Global Instance (Collective Graph).

2. **Phase 2: Contextual Memory Virtualization (DAG State)**
   - Refactor `models.rs`: Deprecate simple `AgentMessage` appendages.
   - Implement `DagNode` primitives utilizing CortexaDB's graph adjacency lists.
   - Rewrite `consolidate()` to perform structurally lossless trimming rather than destructive slicing.

3. **Phase 3: The Distillation Pipeline (Private -> Public)**
   - Introduce `jsonwebtoken` crate to `crates/memory/Cargo.toml`.
   - Implement a background `tokio::spawn` scanner that anonymizes Private Enclave data into Triplets (Subject -> Predicate -> Object).
   - Sign commits with JWT delegation tokens before writing to the Collective Graph to enforce the Strict Privacy Boundary.

4. **Phase 4: Entropy-Based Conflict Resolution**
   - Hook in the "Judge Pattern" logic: Create a dedicated internal Axum RPC route or background handler `spawn_arbiter_task()` that invokes an Arbiter LLM call when `recorded_at` timestamps conflict.
   - Calculate Shannon Entropy on generated memory text blocks to enforce the <1.5 bits cap.

5. **Phase 5: Hive-Mind Broadcast (Synchronized Sync)**
   - Wire `tokio::sync::broadcast` so that when the Collective Graph receives an Arbiter-approved memory commit, the internal `<context_cache>` XML block of all running agents is updated without blocking their inner Axum execution loops.

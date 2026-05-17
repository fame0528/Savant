# Memory System: Savant vs agentmemory — Side-by-Side Audit & Enhancement Report

**Date:** 2026-05-15
**Scope:** Complete architectural comparison with actionable enhancement recommendations

---

## 1. Architecture Overview

| Dimension | Savant | agentmemory |
|---|---|---|
| **Language** | Rust | TypeScript (Node.js) |
| **Runtime** | Native binary | iii-engine worker |
| **Storage Backend** | CortexaDB (custom LSM-tree + WAL + checkpoint + compaction) | iii-engine KV (file-based SQLite) |
| **Vector Index** | ruvector-core HNSW (ANN, SIMD, binary quantization) | In-memory Map (linear-scan cosine similarity) |
| **Graph Layer** | MAGMA 4-graph (semantic/temporal/causal/entity) in-memory | Knowledge graph with temporal edges in KV |
| **Embedding Providers** | fastembed (local, 384-dim), Ollama | Xenova (local, 384-dim), OpenAI, Gemini, Voyage, Cohere, OpenRouter, CLIP |
| **Search Strategy** | 3-tier: Semantic(HNSW) -> Transcript Tail -> Substring | Triple-stream RRF: BM25 + Vector + Graph |
| **Architecture Style** | Dual-enclave (private + collective), layered engine | Hook-driven observation pipeline, iii primitives |
| **Lines of Code** | ~7,000+ across memory crate + CortexaDB | ~21,800 LOC |
| **Test Coverage** | Rust tests (workspace) | 827 test files |

---

## 2. Data Model Comparison

| Concept | Savant | agentmemory | Gap Analysis |
|---|---|---|---|
| **Raw Input** | AgentMessage (role, content, tool_calls, DAG threading) | RawObservation (hook type, tool name, input/output, modality, image) | agentmemory captures richer metadata (modality, image refs, raw tool I/O) |
| **Processed Memory** | MemoryEntry (category, importance, entropy, hit_count, embedding) | CompressedObservation (type, title, facts[], narrative, concepts[], files[], importance, confidence) | agentmemory has structured fact extraction, concept tagging, file references |
| **Long-Term Memory** | MemoryEntry (same struct, different collections) | Memory (type: pattern/preference/architecture/bug/workflow/fact, version, supersedes[]) | agentmemory has explicit memory typing, versioning, supersession chains |
| **Session State** | SessionState + TurnState (rkyv, zero-copy) | Session (id, project, model, tags, firstPrompt, summary, observationCount) | agentmemory tracks model used, tags, first prompt — richer session metadata |
| **Knowledge Graph** | MAGMA: Concept + Relation (4 namespaces) | GraphNode (12 types) + GraphEdge (with temporal validity) | agentmemory has more node types, temporal edge validity (tcommit/tvalid/tvalidEnd) |
| **Episodic** | Daily logs (markdown append-only), Obsidian vault | RawObservation per-session, session summaries | Savant has better episodic archival (cold storage manager) |
| **Semantic** | Vector-indexed MemoryEntry with HNSW | SemanticMemory (fact + confidence + accessCount + strength) | agentmemory separates semantic facts from raw observations |
| **Procedural** | Not explicitly modeled | ProceduralMemory (name, steps[], triggerCondition, frequency) | **Savant has no procedural memory** — major gap |
| **Lessons/Insights** | Not explicitly modeled | Lesson (content, confidence, decayRate, reinforcements), Insight (synthesized from graph traversal) | **Savant has no lessons or insights layer** |
| **Actions/Work Items** | TaskState journal (WAL with XXH3) | Action (status, priority, tags, deps, result, sketchId, crystallizedInto) | agentmemory has full action lifecycle with dependency graphs |
| **Retention Scoring** | PromotionEngine (OCEAN personality-driven) | RetentionScore (Ebbinghaus: salience * exp(-λ*Δt) + reinforcement) | Different approaches — Savant uses personality, agentmemory uses cognitive model |
| **Audit Trail** | WAL (CortexaDB internal) | AuditEntry (30+ operation types, qualityScore, userId) | agentmemory has explicit application-level audit trail |

---

## 3. Storage & Indexing Comparison

| Dimension | Savant | agentmemory |
|---|---|---|
| **Persistence Model** | LSM-tree: WAL -> segments -> checkpoint -> compaction | SQLite-backed KV with debounced index persistence |
| **Crash Recovery** | WAL replay + state machine rebuild | SQLite ACID + index rebuild from raw data |
| **Serialization** | rkyv (zero-copy, `#[repr(C)]`, `check_bytes`) | JSON (via iii-engine KV) |
| **Vector Search** | HNSW (M=16, ef=200/50), AVX2/AVX-512/NEON SIMD, 32x binary quantization | Linear-scan cosine similarity over Float32Array map |
| **Keyword Search** | Substring filter (tier 3 fallback only) | Full BM25 inverted index (k1=1.2, b=0.75) with Porter stemmer, synonym expansion, CJK segmentation |
| **Index Persistence** | HNSW: atomic write on Drop; CortexaDB: WAL + checkpoint | BM25 + Vector: 5-second debounce to KV |
| **Tombstone Management** | Auto-compaction when tombstones > 20% | Not applicable (no deletions in vector index) |
| **Capacity Enforcement** | Deterministic eviction (oldest -> least important -> lowest ID) | Retention-based eviction (Ebbinghaus cold tier) |
| **Deduplication** | Not explicitly implemented | SHA-256 dedup with 5-minute window |

---

## 4. Retrieval & Search Comparison

| Dimension | Savant | agentmemory |
|---|---|---|
| **Primary Search** | HNSW semantic search (cosine distance) | Triple-stream RRF fusion (BM25 0.4 + Vector 0.6 + Graph 0.3) |
| **Temporal Filtering** | `semantic_search_temporal` + exponential decay | Temporal graph edges with tvalid/tvalidEnd, query expansion with temporal concretization |
| **Fallback** | Transcript tail -> substring match | BM25-only when no embedding provider |
| **Query Enhancement** | None | Query expansion (reformulation, temporal concretization, entity extraction), multi-query search |
| **Reranking** | None | Optional cross-encoder reranker (ms-marco-MiniLM) on top-20 |
| **Diversification** | None | Session diversification (max 3 results per session) |
| **Token Budget** | Auto-recall: 2000 tokens, 0.3 threshold, 5 results | Context injection: greedy selection by recency within budget |
| **Access Tracking** | hit_count + last_accessed_at on MemoryEntry | AccessLog with count, lastAt, last 20 timestamps |

---

## 5. Memory Lifecycle Comparison

| Operation | Savant | agentmemory |
|---|---|---|
| **Create** | append_message -> CortexaDB transcript; index_memory -> HNSW + metadata | Hook -> observe -> dedup -> privacy filter -> compress (LLM or synthetic) -> index |
| **Read** | retrieve() 3-tier; auto_recall() for context injection | search, smart-search, context, recall, graph-query, relations |
| **Update** | Metadata updates (embedding required if content changes); atomic compaction | Memory versioning (supersedes chains), strength adjustment, retention recomputation |
| **Delete** | delete_memory (vector + LSM); delete_session; capacity eviction | TTL expiry, contradiction detection, low-value eviction, retention-based eviction, governance-delete |
| **Forget** | Not explicitly modeled — "forever memory" philosophy | auto-forget: TTL + contradictions + Ebbinghaus decay + low-value eviction |
| **Consolidate** | Distillation pipeline: triplet extraction -> collective enclave -> SPO facts | 4-tier pipeline: Semantic -> Reflect -> Procedural -> Decay |
| **Background Tasks** | Distillation (5-min), Arbiter (10-min), notification broadcast | Consolidation (periodic), retention scoring (hourly), mesh sync, sentinels |

---

## 6. Multi-Agent & Coordination Comparison

| Feature | Savant | agentmemory |
|---|---|---|
| **Inter-Agent Context** | ContextPackage (448-byte fixed struct with collection keys) | Signals (inter-agent messages with threading, expiresAt) |
| **Shared Memory** | Collective enclave (hive-mind), CollectiveBlackboard | Team memory (namespaced shared/private), Mesh P2P sync |
| **Action Coordination** | TaskState journal (WAL) | Actions with dependency graphs, Leases (exclusive claims), Routines (workflow templates), Checkpoints (external gates), Sentinels (event watchers) |
| **Notification** | NotificationChannel (tokio broadcast, capacity 64, importance >= 7) | Event triggers, webhook sentinels |
| **Conflict Resolution** | Factual Arbiter (Shannon entropy-based contradiction resolution) | Contradiction detection (Jaccard > 0.9), governance-based deletion |

---

## 7. Quality & Engineering Comparison

| Dimension | Savant | agentmemory |
|---|---|---|
| **Error Handling** | MemoryError enum, Result propagation, Kani formal verification | Circuit breaker + fallback chain, self-healing diagnostics |
| **Privacy** | Not explicitly modeled | 15+ secret pattern regexes, `<private>` tag redaction at capture time |
| **Provider Resilience** | Single provider (fastembed or Ollama) | Fallback chain (Anthropic -> Gemini -> OpenRouter), noop provider |
| **Dimension Guards** | Implicit (embedding service provides dimension) | Explicit guards at write AND load time to prevent cross-dimension corruption |
| **Observability** | tracing crate | OpenTelemetry initialization, health monitor, 107 REST endpoints |
| **Developer Experience** | Cargo-based, Rust tooling | CLI with start/stop/demo/doctor, real-time viewer (port 3113), demo mode |
| **Data Portability** | CortexaDB format (proprietary) | Versioned export/import with 40+ migrations |

---

## 8. Enhancement Recommendations

### Priority 1: High Impact, Low-Medium Effort

#### 8.1 BM25 Keyword Search Index
**Source:** agentmemory `state/search-index.ts`
**Gap:** Savant's fallback is a naive substring match. agentmemory has a full BM25 inverted index with Porter stemmer, synonym expansion (45+ developer term groups), and CJK segmentation.
**Recommendation:** Add a BM25 index as a parallel search path in `vector_engine.rs`. When semantic search returns no results or embedding service is unavailable, use BM25 instead of substring matching.
**Files to modify:** `crates/memory/src/vector_engine.rs`, new `crates/memory/src/bm25_index.rs`
**Estimated effort:** Medium (2-3 days)

#### 8.2 SHA-256 Deduplication Window
**Source:** agentmemory `functions/dedup.ts`
**Gap:** Savant has no deduplication. Duplicate messages/memories can be stored repeatedly.
**Recommendation:** Add a 5-minute SHA-256 dedup window in `async_backend.rs::store()`. Hash the content + session_id, check against a recent-hash cache, skip if duplicate.
**Files to modify:** `crates/memory/src/async_backend.rs`
**Estimated effort:** Low (half day)

#### 8.3 Privacy Filter / Secret Redaction
**Source:** agentmemory `functions/privacy.ts`
**Gap:** Savant stores all content verbatim, including potential secrets.
**Recommendation:** Add a privacy filter module that scans content for 15+ secret patterns (API keys, tokens, JWTs, AWS keys, GitHub PATs, etc.) and redacts them before storage. Support `<private>` tag stripping.
**Files to modify:** New `crates/memory/src/privacy.rs`, integrate into `async_backend.rs::store()`
**Estimated effort:** Low (1 day)

#### 8.4 Access Tracking with Timestamp History
**Source:** agentmemory `functions/access-tracker.ts`
**Gap:** Savant tracks `hit_count` and `last_accessed_at` but not access history.
**Recommendation:** Extend `MemoryEntry` to track last N access timestamps (e.g., last 20). This feeds better retention scoring and temporal analysis.
**Files to modify:** `crates/memory/src/models.rs`, `crates/memory/src/async_backend.rs`
**Estimated effort:** Low (half day)

### Priority 2: High Impact, Medium-High Effort

#### 8.5 Procedural Memory Layer
**Source:** agentmemory `functions/consolidation-pipeline.ts` (Procedural tier)
**Gap:** Savant has no procedural memory — it cannot learn and store workflows, decision patterns, or step-by-step procedures.
**Recommendation:** Add a `ProceduralMemory` type with fields: `name`, `steps[]`, `trigger_condition`, `frequency`, `strength`, `tags`. Create a background consolidation task that identifies recurring tool-call patterns across sessions and extracts them as procedures.
**Files to modify:** `crates/memory/src/models.rs`, new `crates/memory/src/procedural.rs`, integrate into distillation pipeline
**Estimated effort:** High (1-2 weeks)

#### 8.6 Lessons & Insights Layer
**Source:** agentmemory `functions/lessons.ts`, `functions/reflect.ts`
**Gap:** Savant has no explicit lessons-learned or insight synthesis layer.
**Recommendation:** Add `Lesson` (content, confidence, decay_rate, reinforcements, source) and `Insight` (title, content, confidence, source_concept_cluster) types. Create a reflective consolidation task that traverses the MAGMA graph to synthesize higher-order insights from concept clusters.
**Files to modify:** `crates/memory/src/models.rs`, new `crates/memory/src/lessons.rs`, extend `crates/memory/src/reflective.rs`
**Estimated effort:** High (1-2 weeks)

#### 8.7 Triple-Stream RRF Search Fusion
**Source:** agentmemory `state/hybrid-search.ts`
**Gap:** Savant uses HNSW-only semantic search. agentmemory fuses BM25 + Vector + Graph with Reciprocal Rank Fusion (k=60), dynamic weight normalization, and session diversification.
**Recommendation:** When BM25 is added (8.1), implement RRF fusion between BM25 and HNSW scores. Add optional graph-score contribution from MAGMA. Implement session diversification (max 3 results per session).
**Files to modify:** `crates/memory/src/vector_engine.rs`, `crates/memory/src/async_backend.rs`
**Estimated effort:** Medium (3-5 days)

#### 8.8 Ebbinghaus Retention Scoring
**Source:** agentmemory `functions/retention.ts`
**Gap:** Savant's PromotionEngine uses OCEAN personality traits for scoring. agentmemory uses a cognitive model: `score = salience * exp(-λ*Δt) + σ * Σ(1/days_since_access)`.
**Recommendation:** Add an alternative retention scoring mode using the Ebbinghaus model. This is more principled for general-purpose memory decay. Keep OCEAN as an optional override. Add tier thresholds (hot >= 0.7, warm >= 0.4, cold >= 0.15).
**Files to modify:** `crates/memory/src/promotion.rs` (extend or add parallel engine)
**Estimated effort:** Medium (3-5 days)

### Priority 3: Medium Impact, Medium Effort

#### 8.9 Query Expansion & Reformulation
**Source:** agentmemory `functions/query-expansion.ts`
**Gap:** Savant queries are literal — no expansion, reformulation, or temporal concretization.
**Recommendation:** Add a lightweight query expansion module that handles: temporal expressions ("last week" -> date range), synonym expansion, and entity extraction from queries. Can be rule-based (no LLM required).
**Files to modify:** New `crates/memory/src/query_expansion.rs`
**Estimated effort:** Medium (2-3 days)

#### 8.10 Memory Versioning & Supersession Chains
**Source:** agentmemory `functions/remember.rs` (Jaccard similarity check)
**Gap:** Savant has no memory versioning. When a fact changes, the old version is lost or duplicated.
**Recommendation:** Add `version`, `parent_id`, `supersedes[]`, `is_latest` fields to `MemoryEntry`. When storing a new memory, check similarity with existing memories (Jaccard > 0.7). If similar, mark old as superseded, link to new.
**Files to modify:** `crates/memory/src/models.rs`, `crates/memory/src/async_backend.rs`
**Estimated effort:** Medium (2-3 days)

#### 8.11 Cross-Encoder Reranking
**Source:** agentmemory `state/reranker.ts`
**Gap:** Savant has no reranking — search results are returned in raw score order.
**Recommendation:** Add optional cross-encoder reranking (using a local model like ms-marco-MiniLM) on top-N search results. This significantly improves precision for the top results.
**Files to modify:** New `crates/memory/src/reranker.rs`, integrate into retrieval pipeline
**Estimated effort:** Medium (3-5 days)

#### 8.12 Application-Level Audit Trail
**Source:** agentmemory `functions/audit.ts`
**Gap:** Savant's WAL is internal to CortexaDB. There's no application-level audit of memory operations.
**Recommendation:** Add an `AuditEntry` type that logs: operation type (30+ types), timestamp, target memory IDs, quality score. Store in a dedicated CortexaDB collection. Useful for debugging and compliance.
**Files to modify:** New `crates/memory/src/audit.rs`
**Estimated effort:** Low-Medium (1-2 days)

### Priority 4: Lower Priority / Future

#### 8.13 Mesh P2P Sync
**Source:** agentmemory `functions/mesh.ts`
**Gap:** Savant's collective enclave is local-only. No P2P sync between Savant instances.
**Recommendation:** For multi-machine deployments, add optional P2P sync of collective enclave entries. Can use the existing A2A protocol as a foundation.
**Estimated effort:** High (2-3 weeks)

#### 8.14 Image/Multimodal Memory
**Source:** agentmemory `functions/vision-search.ts`, `functions/image-refs.ts`
**Gap:** Savant is text-only. agentmemory supports CLIP image embeddings and image reference counting.
**Recommendation:** If multimodal agents are planned, add image embedding support to the vector engine with CLIP or similar.
**Estimated effort:** High (2-3 weeks)

#### 8.15 Circuit Breaker + Provider Fallback Chain
**Source:** agentmemory `providers/circuit-breaker.ts`, `providers/fallback-chain.ts`
**Gap:** Savant has a single embedding provider. If it fails, semantic search is dead.
**Recommendation:** Add circuit breaker pattern around embedding service. Support fallback chain (e.g., fastembed -> Ollama -> no semantic).
**Files to modify:** `crates/core/src/utils/embeddings.rs`
**Estimated effort:** Medium (3-5 days)

---

## 9. What Savant Does Better (Keep As-Is)

| Feature | Why Savant Wins |
|---|---|
| **HNSW Vector Index** | agentmemory uses O(n) linear scan. Savant's HNSW with SIMD is orders of magnitude faster at scale. |
| **LSM-Tree Storage** | CortexaDB's WAL + checkpoint + compaction is more robust than debounced KV writes. Crash-safe by design. |
| **Zero-Copy Serialization** | rkyv with `#[repr(C)]` and `check_bytes` is superior to JSON for performance and safety. |
| **Dual-Enclave Architecture** | Private + collective separation is cleaner than agentmemory's flat KV scopes. |
| **Factual Arbiter** | Shannon entropy-based contradiction resolution is more principled than simple Jaccard comparison. |
| **MAGMA 4-Graph** | Four orthogonal namespaces (semantic/temporal/causal/entity) is more structured than agentmemory's single graph. |
| **Atomic Compaction** | Write-before-delete with tool pair integrity verification is more robust than agentmemory's linear versioning. |
| **Formal Verification** | Kani harnesses in `safety.rs` — agentmemory has no formal verification. |
| **Cold Storage Manager** | Episodic file archival with tombstone and file ceiling enforcement is more sophisticated. |
| **ContextPackage for A2A** | Fixed-size 448-byte struct with collection keys is more efficient than agentmemory's signal-based approach. |

---

## 10. Summary: Top 5 Immediate Actions

| # | Enhancement | Impact | Effort | Rationale |
|---|---|---|---|---|
| 1 | **SHA-256 Dedup Window** | High | Low | Prevents duplicate storage, zero risk, immediate benefit |
| 2 | **Privacy Filter** | High | Low | Security-critical, prevents secret leakage to disk |
| 3 | **BM25 Keyword Index** | High | Medium | Dramatically improves search when embeddings unavailable |
| 4 | **Access Tracking History** | Medium | Low | Enables better retention scoring, feeds other enhancements |
| 5 | **Memory Versioning** | High | Medium | Prevents knowledge loss when facts change, enables provenance chains |

---

## 11. Architectural Philosophy Comparison

| Aspect | Savant | agentmemory |
|---|---|---|
| **Memory Philosophy** | "Forever memory" — permanent by default, explicit deletion only | "Living memory" — decay, forget, reinforce, evolve |
| **Design Approach** | Bottom-up: storage engine first, then layers on top | Top-down: agent hooks first, then storage adapts |
| **Quality Bar** | Formal verification, zero-copy, AAA safety | Production-ready, 827 tests, circuit breakers |
| **Scale Target** | 1000+ agents, enterprise-grade | Single developer to small team |
| **LLM Dependency** | Optional (works with local embeddings) | Optional for core, required for advanced features |
| **Innovation Source** | Custom database engineering (CortexaDB) | Cognitive science (Ebbinghaus, OCEAN) + practical agent patterns |

**Conclusion:** Savant has superior infrastructure (HNSW, LSM-tree, zero-copy, formal verification) but is missing several practical memory management patterns that agentmemory has refined through production use. The highest-value enhancements are deduplication, privacy filtering, BM25 search, and memory versioning — all of which can be added without changing Savant's core architecture.

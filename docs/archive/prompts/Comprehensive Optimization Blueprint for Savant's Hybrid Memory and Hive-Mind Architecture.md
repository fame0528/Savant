# **Comprehensive Optimization Blueprint for Savant's Hybrid Memory and Hive-Mind Architecture**

## **The Strategic Imperative for Advanced Cognitive Architectures**

The paradigm of artificial intelligence has irrevocably shifted from stateless, single-turn interactions toward continuous, multi-agent frameworks operating over extended horizons. In these autonomous ecosystems, the primary limitation is no longer the generative capability of the underlying large language models (LLMs), but rather the infrastructural capacity to store, retrieve, synchronize, and validate state. As autonomous agents engage in complex reasoning tasks, they accumulate significant state—architectural mappings, trade-off decisions, and evolving domain conventions—within their context windows. Without a persistent, structured memory architecture, this cognitive accumulation is destroyed upon session termination or corrupted by lossy context window compaction.

For advanced systems like the Savant architecture, executing a true "Hive-Mind" requires more than parallel model execution; it demands a rigorous, highly optimized data substrate. Multi-agent systems inherently suffer from epistemic drift, lock contention, and localized hallucinations when operating on shared data. When agents operate without coordinated memory, they overwrite one another, retrieve stale information, and burn computational budgets repeatedly establishing context. Furthermore, statistical models seeking to minimize average prediction errors inevitably run into limits set by the entropy of real language, leading to a probabilistic gap between accuracy and creativity known as the entropy gap.

The following architectural critique and implementation blueprint exhaustively details the optimization of a 5-Layer Hybrid Memory System. It addresses the critical engineering challenges by shifting to a unified memory synchronization utilizing CortexaDB, mitigates the notorious concurrency and atomicity bottlenecks of legacy dual-database architectures, establishes mathematical frameworks for swarm conflict resolution, and engineers a strict cryptographic boundary to isolate private agent sessions from the collective Hive-Mind graph.

## **Part I: Deconstructing and Optimizing the 5-Layer Hybrid Memory System**

The standard approach to LLM memory—treating conversation history as a linear, ephemeral string and long-term memory as an undifferentiated pool of vectorized text chunks—fails at enterprise scale. Such "flat" Retrieval-Augmented Generation (RAG) architectures lack structural awareness, temporal understanding, and contradiction detection. A biologically inspired, 5-Layer Hybrid Memory System resolves these deficiencies by organizing state according to its temporal relevance, structural complexity, and access frequency.

This optimization blueprint mandates a rigorous architectural separation of concerns, mimicking modern operating system memory management to stratify agent cognition into specialized tiers.

### **The 5-Layer Memory Hierarchy Specification**

| Layer | Nomenclature | Function & Storage Substrate | Eviction / Lifecycle Policy |
| :---- | :---- | :---- | :---- |
| **Layer 1** | **Sensory I/O Buffer** | Ephemeral ingestion streams handling WebSockets and API payloads. Holds unparsed raw data before validation. | Dropped immediately post-processing or upon session timeout. |
| **Layer 2** | **Working Memory (Cache)** | The active LLM context window. Managed via Directed Acyclic Graph (DAG) state tracking for branching execution inside CortexaDB. | Structurally lossless trimming and hierarchical summarization. |
| **Layer 3** | **Episodic Memory** | Time-series interaction logs stored in CortexaDB temporal indices. The strict "autobiography" of the agent. | Compressed via LLM distillation; archived to persistent disk storage. |
| **Layer 4** | **Semantic & Procedural Memory** | CortexaDB native vector \+ graph embeddings for conceptual meaning and procedural user preferences. The agent's factual knowledge base. | Importance-weighted Least Recently Used (LRU) via Automatic Forgetting.1 |
| **Layer 5** | **Collective (Hive-Mind) Memory** | Shared global state accessible by all agents. Synthesized insights, vetted by arbitration protocols. | Eventual consistency via Event Sourcing; Native Temporal Invalidation.1 |

### **Optimizing Layer 2: Contextual Memory Virtualization (CMV)**

A critical bottleneck in standard agent design is the lossy compaction of the context window. Traditional systems utilize a sliding window that truncates older messages, permanently destroying the rationale behind earlier decisions and inducing immediate context degradation.

To optimize Working Memory (Layer 2), the system must implement Contextual Memory Virtualization (CMV). Borrowing from operating system virtual memory principles, CMV models the LLM session history as a Directed Acyclic Graph (DAG) rather than a linear string. This framework introduces formal snapshot, branch, and trim primitives. When a context window nears its token limit, a three-pass structurally lossless trimming algorithm is deployed.

Instead of deleting data, the algorithm parses the DAG to strip mechanical bloat—such as raw tool outputs, Base64 image encodings, and verbose API metadata—while preserving every user message and assistant response verbatim. The pruned data is persisted to the local CortexaDB database, organized by conversation and linked back to its source messages via graph edges.1 This DAG-based summarization enables context reuse across independent parallel sessions. If an agent spends 40 minutes generating an architectural understanding, that state can be snapshotted as a stable root commit. Subsequent specialized agents can branch from this root commit without repeating the computationally expensive context-building phase. Empirical evaluations across real-world coding sessions demonstrate that this trimming mechanism reduces token counts by a mean of 20%, and up to 86% for sessions with significant mechanical overhead, reaching break-even economics within 10 conversational turns.

## **Part II: Dual Memory Isolation: Private Enclaves vs. Shared Collective Graph**

In a Hive-Mind architecture, balancing collective intelligence with data privacy is a non-trivial distributed systems challenge. Agents require the ability to conduct highly sensitive, private 1-on-1 conversations with human operators or specific tenant APIs, while simultaneously contributing generalized insights to the Swarm's shared collective memory.

Operating a multi-agent system on a purely "Shared State" architecture results in catastrophic privacy leaks, where one agent might accidentally ingest and leak another's authentication tokens, proprietary logic, or personally identifiable information (PII). The integration of artificial intelligence into sensitive contexts such as legal proceedings, therapeutic settings, and proprietary enterprise workflows requires absolute data sovereignty. Conversely, a purely "Isolated State" architecture prevents agents from learning from one another, defeating the fundamental purpose of the swarm and resulting in duplicated effort and siloed intelligence.

### **Designing the Secure Boundary**

The optimization blueprint dictates a Hybrid Isolation Strategy, physically and logically separating the Private Enclave from the Collective Graph. This prevents scenarios where information from one customer interaction improperly influences another.

| Isolation Architecture | Storage Mechanism | Data Types Handled | Security Characteristics |
| :---- | :---- | :---- | :---- |
| **Private Enclave (Isolated State)** | Local CortexaDB instance per agent/tenant (\~/.savant/memory/{agent\_id}.mem).1 | Raw episodic transcripts, PII, session-specific context, raw tool outputs. | Hard boundaries; no cross-tenant visibility. Data is ephemeral to the session lifecycle or strictly vaulted. |
| **Collective Graph (Shared State)** | Centralized Database / Decentralized Knowledge Graph (DKG). | Generalized rules, vetted facts, architectural schemas, swarm-level metrics. | Cryptographically signed, heavily audited, completely anonymized. |

### **The Distillation and Promotion Protocol**

To bridge the Private Enclave and the Collective Graph safely, the architecture utilizes a one-way, LLM-mediated "Memory Distillation" pipeline. This prevents the "Hive-Mind" from becoming polluted with unstructured, private dialogue.

1. **Extraction & Sanitization:** A background asynchronous process (often operating during idle compute cycles) scans the agent's private local episodic memory. It extracts causal relationships, generalized patterns, and factual assertions.  
2. **Anonymization:** The distillation prompt is strictly instructed to strip all Personally Identifiable Information (PII), API keys, and session-specific metadata. The architecture enforces explicit data boundaries and least-privilege access to guarantee regulatory alignment.  
3. **Promotion:** The sanitized knowledge is transformed into a highly structured triplet (Subject \-\> Predicate \-\> Object) representing an entity relationship. It is then vectorized and published to the Shared State via an Event Sourcing protocol, ensuring that the reasoning behind each write is captured alongside the data itself.  
4. **Delegation Tokens:** Any read or write operation targeting the Collective Graph must be signed with JSON Web Tokens (JWT) that prove the complete custody path of the delegation chain. This ensures that malicious or compromised agents cannot poison the shared graph, as each delegation is cryptographically signed.

By enforcing this cryptographic and semantic boundary, the architecture achieves the localized security of an isolated microservice while maintaining the emergent intelligence and continuous learning capabilities of a unified swarm.

## **Part III: Unified Atomicity and Concurrency with CortexaDB**

A high-performance agentic node requires storage that can handle complex graph relationships, high-dimensional vector similarity searches, and temporal data simultaneously. Relying on a Dual-Database architecture (such as combining SQLite for relations and Fjall for blobs/vectors) introduces severe write-contention under load and extreme difficulties in maintaining cross-database atomic commits.

The blueprint mandates a unified architecture powered by **CortexaDB**, a Rust-powered, local-first database designed specifically to act as a "cognitive memory" for autonomous agents.1 By consolidating the storage substrate, we elegantly bypass the traditional multi-master synchronization hazards.

### **Lock-Free Concurrency and Asynchronous Handling**

Traditional embedded relational databases strictly limit operations to a single writer, requiring an EXCLUSIVE lock that causes severe thread starvation under high-throughput asynchronous frameworks like Axum and Tokio.

CortexaDB resolves this natively without needing an external Actor Pattern or dedicated write-threads. It employs arc-swap for highly efficient, lock-free reads. To ensure that high-dimensional vector operations do not block the event loop, CortexaDB utilizes the rayon crate to parallelize index building. This architecture supports high-frequency reads and asynchronous updates out-of-the-box, fitting perfectly within Savant's Axum-powered WebSocket orchestration.

### **Built-in Atomicity and Hard Durability**

Attempting to orchestrate a distributed Two-Phase Commit (2PC) or a complex Event Sourcing Saga across two separate local databases (e.g., SQLite and Fjall) creates the risk of "orphaned vectors" if the machine loses power mid-transaction.

CortexaDB solves the "Orphaned Vector" problem organically through its log-structured engine. Every single operation (whether updating a graph edge, adding a temporal tag, or storing a vector) is serialized and immediately written to a unified Write-Ahead Log (WAL) fortified with CRC32 checksums before it ever reaches the state machine.

If a crash occurs, CortexaDB utilizes a deterministic replay mechanism upon startup: it re-reads the WAL to flawlessly rebuild the in-memory HNSW (Hierarchical Navigable Small World) index for vectors and the adjacency lists for graphs. This yields **100% data integrity** and instant recovery, natively guaranteeing global atomicity across all memory types without custom saga middlewares.1

## **Part IV: Global Atomicity for Swarm Conflict Resolution**

When multiple autonomous agents operate over shared data within the Collective Graph, the phenomenon of "epistemic drift" occurs. Epistemic drift is a systemic failure mode where belief strengthens even as truth decays, resulting in growing confidence paired with fading accuracy. In naive architectures, if Agent A determines that a software bug is caused by a memory leak, while Agent B alters the code based on the assumption that it is a network timeout, the agents will engage in an infinite loop of overwriting each other's work. This violent burning of token budgets, known as review thrashing, prevents the system from achieving consensus.

The structural failure is systemic; there is no inherent mathematical mechanism built into language models to logically compromise when localized objectives actively conflict. To counter this, the architecture requires advanced conflict resolution algorithms and strict mathematical contradiction detection.

### **Hierarchical Arbitration: The Judge Pattern**

Decentralized, peer-to-peer consensus is notoriously fragile in LLM swarms. Studies show that multiple agents working in a completely self-organized stigmergic emergence approach fail to deliver correct outcomes up to 36% of the time due to communication inefficiencies and governance conflicts. The optimized blueprint utilizes the Judge Pattern, introducing a specialized, highly capable "Arbiter Node" within the DAG orchestration layer to enforce hierarchical arbitration.

When a shared memory block receives conflicting updates from subordinate agents, the Arbiter is invoked. Instead of applying simple timestamp-based overwrites—which blindly destroy data without accounting for semantic reasoning—the Arbiter agent executes semantic conflict resolution. It evaluates the reasoning traces behind both writes, synthesizes a coherent compromise that considers intent and meaning, and commits the finalized decision to the Collective Graph.

To prevent infinite loops, an aggressive Circuit Breaker is configured within the orchestration logic. If a specific memory block or document bounces between two squabbling agents more than a precisely defined threshold (e.g., twice), external routing is immediately halted, and peer-to-peer communication is severed. The Arbiter then unilaterally forces a resolution, breaking the deadlock.

### **Contradiction Security: Shannon Entropy and Fisher Information**

To programmatically detect when an agent is hallucinating or generating conflicting data before it enters the Shared Graph, the system utilizes advanced information-theoretic metrics. Rather than relying on simple, easily manipulated confidence scores emitted by the LLM, the system calculates the Shannon Entropy of the generated tokens.

The Shannon Entropy ![][image1] of a probability distribution, representing the level of uncertainty, is defined as:

![][image2]  
Where ![][image3] is the probability of the token ![][image4] being generated. High entropy indicates a flat probability distribution, meaning the model is highly uncertain and is effectively guessing. Low entropy indicates a sharp probability distribution, signifying confidence. By establishing strict entropy thresholds—such as setting an upper bound of 1.5 bits for factual memory commits—the architecture proactively blocks uncertain, potentially contradictory facts from entering the Shared Graph. If the entropy exceeds the threshold, the system forces the agent to request human clarification or trigger the Arbiter.

### **Native Temporal Knowledge Invalidation**

Even with entropy thresholds and Arbiters, facts change over time. A traditional database UPDATE statement destroys historical context, inducing "AI amnesia" by wiping out the system's ability to understand what it previously believed.

Because agents need more than "semantic similarity" and require a strict sense of time, CortexaDB supports Temporal indexing intrinsically.1 Instead of manually scripting bi-temporal schema triggers, the engine handles temporal boundaries natively. When an agent observes a new, conflicting fact, it does not overwrite the old memory. The old memory is temporally invalidated, preserving the historical timeline of what the agent believed in the past, while the new memory is recorded as current. This ensures time-travel queries are natively supported by the engine.

## **Part V: Performance Scaling and Hybrid Search Optimization**

To function as a low-latency, localized cognitive engine, the memory system must execute retrievals in milliseconds. Cloud-based vector databases introduce unacceptable network latency, privacy vulnerabilities, and serialization overhead that disrupt the fluid execution of the agent loop.

### **Unified Hybrid Search: Vector \+ Graph \+ Time**

Traditional local databases rely on duct-taping Full-Text Search (FTS) alongside brute-force vector search and manually blending the scores using Reciprocal Rank Fusion (RRF) at the application layer. While functional, it is computationally heavy and structurally unaware of relationships.

CortexaDB replaces this with a natively integrated **Hybrid Search** engine.1 It does not simply retrieve based on cosine similarity; it computes results using **Vector \+ Graph \+ Time** simultaneously.1

1. **Semantic Matching:** It locates the initial concept via HNSW vector similarity.  
2. **Graph Traversal:** Upon finding a semantic match, it immediately traverses adjacency list edges to find connected, structural "related thoughts" without requiring a secondary SQL JOIN or external graph query.  
3. **Temporal Prioritization:** It inherently factors in recency, ensuring the most up-to-date facts override obsolete matching vectors.1

### **Automated Lifecycle Management**

To prevent the active knowledge base from expanding infinitely and degrading performance, the architecture leverages CortexaDB's **Automatic Forgetting** features.1 Instead of writing custom cron jobs to purge old embeddings, the engine is configured with a capacity limit. It automatically utilizes an importance-weighted Least Recently Used (LRU) algorithm to gracefully evict old, irrelevant memories from the active cache.1 This biologically-inspired eviction perfectly mirrors how a real brain prunes unused synaptic connections, keeping the Hive-Mind focused, efficient, and relentlessly fast.

## **Strategic Conclusion**

The transition from isolated, stateless LLM calls to a fully autonomous, multi-agent Hive-Mind requires a radical reconceptualization of data infrastructure. By displacing the fragmented dual-database (SQLite \+ Fjall) concept with CortexaDB, this blueprint transitions the 5-Layer Hybrid Memory System from a theoretical framework into a highly concurrent, zero-config production architecture capable of operating at enterprise scale.1

By virtualizing the context window as a Directed Acyclic Graph, the system prevents the destructive loss of reasoning sequences and radically optimizes token consumption. By logically separating the Private Enclave via strictly sandboxed CortexaDB instances, and controlling the flow of generalized knowledge to the Collective Graph via cryptographic delegation and LLM-distillation, the architecture guarantees absolute tenant privacy while enabling emergent swarm intelligence.

Crucially, the severe engineering hazards of cross-database synchronization are eliminated. CortexaDB's unified WAL with CRC32 checksums ensures 100% atomic durability without orphans 1, and its arc-swap lock-free reads effortlessly scale beneath Axum's async runtime. Finally, the integration of Shannon Entropy limits, Hierarchical Arbitration, and native Hybrid Search (Vector \+ Graph \+ Time) ensures that the Hive-Mind retrieves highly accurate, contextual payloads instantly without blocking the event loop.1 Executing this optimized architecture transforms an erratic ensemble of agents into a unified, secure, and relentlessly logical cognitive engine.

#### **Works cited**

1. I built "SQLite for AI Agents" A local-first memory engine with hybrid Vector, Graph, and Temporal indexing \- Reddit, accessed March 19, 2026, [https://www.reddit.com/r/LocalLLM/comments/1rehu2k/i\_built\_sqlite\_for\_ai\_agents\_a\_localfirst\_memory/](https://www.reddit.com/r/LocalLLM/comments/1rehu2k/i_built_sqlite_for_ai_agents_a_localfirst_memory/)  
2. https://github.com/anaslimem/CortexaDB

[image1]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADIAAAAYCAYAAAC4CK7hAAACuUlEQVR4Xu2WS6hNURjH/wp5Jq8kSiQlcpUYMSAKRSJFMjJgoJTC9OpeY+8BkSgRJgp5DBwpKVNFSa6SAaHExPP+/+fbq9b+rHP22ffscyfur37tc9a3X+tb61t7AUP8P4ygE3xjG1R9v5bQA8/QZT7QBurIMbrZB1KspB/p38jPdCedQh/R31HsG71Kx+rijOH0NN0TtcXoGV+Rf8YbOo+Oobdc7AmdXL/SjndQIkHn6B+62gdgbYpdoMNcTKyhNRRPg22wF32AfCKmwl5+F2wUPGvpfTrOBzwT6TPaR2fkQ3UOwV5ghw+Q0fQu3esDCabRF/QLXZi16cVP0i3hpARKkDqqRDRlPv1Eb8CmSYz+q11xnedZQl8hHUvRDUuKjqETSkJqpGOOIP1+OZRp3VyZ92iE+mAjppHz7KaP6XgfaIBGQiPykh6lB1DcCbGOvqUzfSDmBP1FN9Hpzq2w+tA5KS5mtkoYYSVONZeqiRQa+Xd0qQ8EQn18p5foWadWl0b1oeKrwYa9DPtg97yCgqkSoaRqRDb4QKCd+ggdSU3JRmgF1JL+HvmiLyJ0JJXQOqE+9vsAiuujbEf0LbhOJyFf9K0QOpJ6zzr6fvyky30A1qZYo/oo05FZ9GZ2FKHotRxrWS6i6dRq5/shwtRTMpqhl79HF0dtulY1ovsXfh/IHNh01Or1D1oJVOSp+hhFb8O2Fl0uFqPR0nk636NldREsWetdTIRpfQ3Fq5dWq9dwtaqCU+90k+AHuh22t9EW4kcU0+/LsH2RZyN9DtuXxfQiv0fT/k2JCxyPYuEZ5+nI6JwYdbqGFrYpA2U27OOWqrGqCNOw27VXiqZPDz2V/e4Ec+nT7NhRtG14SBf4QAUoOYfpwex3x9E3QkVbtJUvywrYYlT1fZuyCrb9qAp9ErT9GdRODDFQ+gG3HaD4TNFcUwAAAABJRU5ErkJggg==>

[image2]: <data:image/png;base64,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>

[image3]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACoAAAAYCAYAAACMcW/9AAACy0lEQVR4Xu2WS8hNURTHlzwi8s4jJPImBqIUA8+YGBgpj74JpmKgMPAsE5J3IhmgUJLHSPpMJEopUh5FiZGUMpL4/+7a2zl32/e6ffcog+9fv277rH32WXuttde+Zt36f9RTDBE9UkMXNUj0Th+2KxY8KNakhjY0X1wyd7gybRdHrLpoRq0XZ6yiyM4UT8SE1FCB+olbVkGmiOCJQNXRjForHlqbJTBGvBLLUkOFmijeiIWpAQ0Uq8S4MO4vVoi55qc7CgffirGlZ2Uxl3d4lzWI+hSx3DytrYj37omdqYEFTgbDR3FIXBfrwu9ZK4p7h+gUA8K4LFJ1WWwTu8QzcdS8O5wXN0Tf37Ob66J5B6grL6LEwtPFZ3HOit3PE1/CHMQCON8rjKNY8IBYFMajxXtxRcw2X7fT8hvMKRuQDvMetlr8sPr6o06+m0cJ4SikGip2WxGxOeKreVbIxgYxI9iilgZywlE6CxfKHyJF78wPTNQW8dOKdtHI0VQ4+M28XnMiI3yP2s0JRzkLI1NDLOByWvllTNooC9SKo5TBBWsSkRaUTT2iJXwynxBFQ/8gTlvhPFFgQ2ysLA7ScbFZDBcvzJ2Nh4HIYUOjxCmxzxrfQI2+U6tPUrwnjPnAfvFcjA/PEKWAEzhTFq2N9w+LxeZ1HTfNJsjCJPN1O8xrmKZeXjuKOZz4Y6kBsQNO9yNxTTwwTzu7L4uae21FKUSRkafmH7gptprPoy3dFUvCPDIz2TwwV8M4FeXy2HxOnaiDTnHHvC0NC89yIjpshsOSihM/wooLIh1H4Rz9lqsypwXipWX+S+Tqs5n4h0OUWr1pUpENNjvLvG5jHaNYclB+XhOTqa+NYnBiy4lo37bGreVvmirumx+maYmNOuYQ5Wq3dkVGGqUjFQtRi9kFWxBlkV6ndAA6Rdt/8VKRwr2iT2roojaJlenDbv0r/QISWHd6b3Sh9gAAAABJRU5ErkJggg==>

[image4]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABEAAAAYCAYAAAAcYhYyAAABJUlEQVR4Xu3TsUtCURTH8SMlFAViROQURJpJ1NDcUGHQ0hxE4NQcLQ06RY1CELRFNFTQEi2O0hSCEDQ0OfkHuAhODvU9nfusHlfhjYE/+Az3nfvu5Z77nsgww0TLCNawjQnEkEEe47/m9U0C9zhGEe+4wDmu8YSx3mxPdMczrLtxCk08YAUtvGDS1b2ZQkl+dlpFG/uI4wA5Vwuy5fSNvtwR648vo2LH1F55o0e7QR3JUG1gtKmXOMQ0PsQW0gU1uqPWNLO4wqnYUXvZwSfK2EAXJ66mG9xiQWzRgljPXjHn5nxnHm+4wzOO0BC72go23TztRRq7eHTjP9GbmRH74HzjIPqifk97oeeRsoQalsX6FPQtUhZRFWtsNlSLFD3qwF/gn+cLIMkmCBVi84QAAAAASUVORK5CYII=>
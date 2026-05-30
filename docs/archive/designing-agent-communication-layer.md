# **Enterprise-Grade Agent-to-Agent Communication Layer for Savant**

The evolution of autonomous AI agent frameworks increasingly points toward decentralized, multi-agent orchestration. For Savant, a Rust-native desktop agent platform, the current reliance on text-based Large Language Model (LLM) commands for task delegation represents a critical structural fragility. While the existing architecture boasts sophisticated single-machine optimizations—such as the iceoryx2 zero-copy blackboard, CortexaDB Write-Ahead Log (WAL) persistence, and an Obsidian-backed memory graph—the connective tissue between agents remains untyped, asynchronous without structured lifecycle management, and prone to context bloat.

To elevate Savant into an enterprise-grade framework without sacrificing its single-machine, zero-copy philosophy, a comprehensive redesign of the Agent-to-Agent (A2A) communication layer is required. This analysis provides an exhaustive architectural blueprint for implementing a strictly typed, memory-aware, and formally verified A2A protocol. By synthesizing the theoretical strengths of the Google A2A specification with the practical realities of a lock-free, Rust-native shared memory environment, this blueprint establishes a paradigm where agents advertise capabilities via cryptographic metadata, coordinate via zero-copy IPC queues, and pass context as memory pointers rather than raw strings.

## **1\. Prioritized List of Architectural Improvements**

Transitioning a monolithic orchestrator model to a distributed, typed A2A protocol necessitates a sequenced integration strategy. The enhancements must map to existing Savant subsystems (such as the CollectiveBlackboard and SpeculativeDag) without disrupting crash recovery guarantees. The following prioritization matrix evaluates the required architectural improvements based on their operational impact relative to the implementation effort required within the existing \~75,000-line Rust codebase.

| Priority | Architectural Component | System Impact | Implementation Effort | Strategic Rationale and Core Value Proposition |
| :---- | :---- | :---- | :---- | :---- |
| **1** | **Typed Task Delegation Protocol** | High | Low | Replaces brittle regex parsing (/subagents spawn) with structured, rkyv-serialized C-structs. This immediately eliminates schema hallucinations and stabilizes the orchestrator-to-subagent handoff sequence. |
| **2** | **Task State Machine & WAL Integration** | High | Medium | Maps the Google A2A lifecycle (Submitted, Working, Input-Required, Completed) to Rust enums. Journaling these transitions to CortexaDB guarantees deterministic crash recovery for long-running multi-agent tasks. |
| **3** | **Memory-Aware Context Packages** | High | High | Leverages Savant's core differentiator (Obsidian graph and LSM store). Drastically reduces token bloat by passing memory offsets and HNSW vector IDs instead of raw conversation transcripts. |
| **4** | **Agent Card Advertisement** | High | Medium | Enables dynamic capability discovery. Orchestrators can query available agents and match tasks via vector similarity and pressure metrics rather than relying on hardcoded configuration arrays. |
| **5** | **Inter-Agent Result Routing** | Medium | Medium | Establishes a bidirectional IPC channel over iceoryx2. Subagents can deliver structured Artifact payloads back to the parent without blocking asynchronous tokio threads. |
| **6** | **Cancellation Cascade & Timeouts** | Medium | Low | Introduces a distributed CancellationToken pattern across the swarm. Prevents runaway subagent loops, enforces the 20-level max delegation depth, and respects strict token budgets. |
| **7** | **Cryptographic Audit Trail** | Low | Low | Implements structured logging of agent interactions, satisfying security-first constraints by journaling Ed25519-signed Capability Tokens (CCT) alongside task assignments. |

## **2\. Analysis of Contemporary A2A Implementations**

To design a best-in-class A2A layer for Savant, it is necessary to deconstruct how contemporary multi-agent protocols approach discovery, delegation, and state management. The Google A2A protocol, LangGraph, CrewAI, and OpenClaw offer contrasting methodologies that highlight both ideal design patterns and critical anti-patterns for single-machine deployments.

### **2.1. The Google A2A Protocol Specification**

Google's Agent-to-Agent (A2A) protocol 1 provides an open standard designed to enable interoperability between AI agents across varied providers. It acts as a universal messaging tier, focusing on horizontal agent collaboration rather than vertical tool access (which is handled by the Model Context Protocol, or MCP).1 The protocol is built upon standard web technologies—HTTP/HTTPS, JSON-RPC 2.0, and Server-Sent Events (SSE)—operating on a client-server paradigm where a "client agent" delegates requests to a "remote agent".1

While the transport mechanisms (HTTP and SSE) are wholly inappropriate for Savant's zero-copy, single-machine constraints, the underlying data model is exceptionally robust and serves as the optimal foundation for Savant's Rust structs. The specification revolves around four primary entities: AgentCard, Task, Message, and Artifact.3

The AgentCard acts as a machine-readable manifest, exposing the agent's identity, supported interaction modalities, and specific skills.5 The Task represents a fundamental unit of work with a strict lifecycle state machine.6 When a task concludes, the agent returns an Artifact—an immutable output composed of granular Parts (e.g., text, JSON data, or file references).2

A critical feature of the Google A2A state machine is its nuanced lifecycle, which includes states such as Submitted, Working, Input-Required, Completed, Failed, and Canceled.2 The Input-Required state is highly relevant to Savant; it allows an agent to pause execution and yield back to the orchestrator for clarification, rather than hallucinating an action. Savant should adapt this conceptual model entirely, mapping the JSON schemas to \#\[repr(C)\] memory layouts serialized via rkyv. The streaming update model defined by A2A (using TaskStatusUpdateEvent) 1 can be seamlessly transitioned from SSE to iceoryx2 publish-subscribe event buses, achieving nanosecond latency.

### **2.2. LangGraph Subgraph Delegation Patterns**

LangGraph approaches multi-agent orchestration through the lens of graph theory, where subagents are treated as nested subgraphs.8 LangGraph relies on a Pregel-like message-passing model where nodes run in discrete super-steps, firing state updates along outgoing edges.9

LangGraph exposes two primary patterns for subgraph communication: shared state keys and transformed state schemas. When a parent graph and subgraph share state keys (e.g., a common messages array), the subgraph is added directly as a node, reading from and writing to the parent's memory channels automatically.8 Alternatively, when schemas differ or privacy is required, a wrapper function invokes the subgraph, transforming the parent state to match the subgraph's input schema and mapping the results back upon completion.8

For Savant, the shared state approach presents severe risks. Allowing a subagent direct write access to the parent's SwarmSharedContext violates the principle of least privilege and risks blackboard pollution. Savant must adopt LangGraph's transformed state pattern, enforcing strict namespace isolation. A parent agent in Savant will generate a localized ContextPackage tailored specifically for the subagent, ensuring the subagent executes within an isolated memory enclave.

### **2.3. CrewAI Hierarchical Task Routing**

CrewAI treats A2A delegation as a first-class primitive, enabling agents to autonomously decide whether to execute a task using local capabilities or delegate it to a specialized remote agent.10 In CrewAI's hierarchical process, a manager agent coordinates the workflow, allocating tasks based on the declared roles and capabilities of the swarm members.12

CrewAI configuration utilizes an A2AClientConfig block that defines endpoints, timeouts, and a maximum number of conversational turns to prevent infinite loops.10 Furthermore, CrewAI implements a "fail-fast" resiliency toggle. When set to false, if a delegated agent fails to connect or crashes, the system does not halt; instead, it informs the orchestrating LLM of the failure, allowing the model to dynamically reroute the task or attempt local execution.10

This fail-fast autonomy is a crucial requirement for Savant. If a subagent in Savant encounters an emergency\_halt or exceeds its token budget, the crash must not panic the main orchestrator thread. The orchestrator must trap the failure via the iceoryx2 event bus, log the failure to the CollectiveBlackboard, and dynamically select a new agent from the pool.

### **2.4. OpenClaw Instabilities and Loopback Exhaustion**

Analyzing failures in other frameworks provides critical insight into what Savant must avoid. The OpenClaw framework has repeatedly struggled with subagent spawning, heavily documented in GitHub issues \#21445, \#60312, and \#24654.13

The primary failure mode in OpenClaw arises from attempting to spawn persistent subagent sessions over loopback network gateways (ws://127.0.0.1). These requests frequently fail with gateway closed (1008): pairing required or timeout due to socket exhaustion and asynchronous pairing blockages.13 Furthermore, OpenClaw models often fail to invoke actual tools, instead hallucinating XML tool-call syntax because the tool schemas are not properly injected into the subagent's local context boundary.16

Savant's architecture bypasses the networking gateway failures intrinsically by strictly prohibiting HTTP and WebSocket loopbacks in favor of iceoryx2 shared memory. However, the OpenClaw hallucination issue underscores the necessity of strict schema validation. Savant must ensure that when a subagent is spawned, the DelegationTask includes a cryptographic hash of the exact JSON schema expected for the output. If the LLM output fails rkyv validation against this schema, the NLP layer must intercept the error and trigger the Anti-Dwindle Continuation Engine to force a structured retry, rather than propagating the hallucination back to the parent.

## **3\. Community Top-Wanted Features in A2A**

A survey of developer communities (Reddit, GitHub discussions, and standard specification debates) highlights several highly requested features missing from initial A2A and MCP implementations.17 The following table evaluates the top ten most-requested capabilities and outlines their integration strategy within Savant's architecture.

| Rank | Requested Feature | Application to Savant's Single-Machine Architecture | Integration Strategy |
| :---- | :---- | :---- | :---- |
| **1** | **Structured Observability & Audit Trails** | High | Implement a standard JSON-based or rkyv logging format capturing agent identity, action classification, and trust levels. Hash-chain these records into CortexaDB WAL to ensure tamper-evident decision traces.19 |
| **2** | **Bounded Context / Memory Compression** | High | Solve "context bloat" by passing localized ContextPackage pointers. Use the Agent Cognitive Compressor (ACC) pattern to maintain bounded internal states rather than unbounded transcript replays.21 |
| **3** | **Dynamic Tool & Skill Discovery** | High | Agents frequently synthesize new WASM tools at runtime. Savant will allow agents to dynamically update their AgentCard bitmasks in the CollectiveBlackboard to advertise newly acquired capabilities without a framework restart.5 |
| **4** | **Standardized Cancellation Propagation** | Medium | A hierarchical cancellation cascade via tokio::util::sync::CancellationToken. If the Orchestrator cancels a primary task, the death signal propagates deterministically through the iceoryx2 event bus to all nested subtasks. |
| **5** | **Real-Time Streaming of Partial Artifacts** | Medium | Rather than waiting for a massive code generation task to complete, subagents can publish Part updates to an iceoryx2 pub-sub topic, allowing the parent to monitor and interrupt if the generation goes off-track.1 |
| **6** | **Zero-Trust Capability Sandboxing** | High | Inherit taint tracking across agent boundaries. A delegated task carries a Cryptographic Capability Token (CCT); subagents operate inside wasmtime sandboxes strictly bounded by the parent's authorization level. |
| **7** | **Conflict Resolution in Shared Memory** | High | When parallel agents update the shared Obsidian graph, conflicts are resolved via Savant's zstd compression density metric, promoting the memory node with the lowest Shannon entropy (highest information density).24 |
| **8** | **Memory-Aware Agent Selection** | High | Discard static role-based routing. Embed the task description into an HNSW vector and query the CortexaDB to find agents whose AgentCard descriptions have the highest semantic overlap with the task.25 |
| **9** | **Asynchronous Wait States (Yielding)** | Medium | Map the A2A Input-Required state to Savant's Continuation Engine. If an agent is blocked, it yields the tokio thread with an exponential backoff rather than burning CPU cycles in a spin-lock. |
| **10** | **Backpressure and Load Balancing** | High | Orchestrators read the pressure metric (0.0-1.0) from the CollectiveBlackboard prior to delegation. Highly loaded agents receive penalty modifiers during the semantic matching phase, naturally distributing the swarm's workload. |

## **4\. Single-Machine IPC and Inter-Agent Messaging Design**

The core philosophy of Savant relies on zero-copy shared memory to avoid the serialization overhead, kernel context switching, and socket exhaustion typical of network-based agent swarms. Savant currently implements iceoryx2 for a 128-byte SwarmSharedContext and a CollectiveBlackboard.27 To support a robust A2A protocol, this IPC layer must be significantly expanded.

### **4.1. Expanding the iceoryx2 Architecture**

iceoryx2 is a lock-free, zero-copy IPC middleware that operates entirely in user space, achieving sub-microsecond latency.28 It supports diverse messaging patterns, including publish-subscribe, events, and the newly implemented blackboard pattern.27

The A2A messaging layer will utilize a tripartite architecture mapping directly to iceoryx2 primitives:

1. **The Capability Blackboard (Key-Value Store):** The iceoryx2 Blackboard pattern is an ideal fit for capability advertisement. It acts as a highly concurrent key-value registry.31 The key is the deterministic agent\_id (a 32-byte FNV-1a hash), and the value is the serialized AgentCard. This allows the Orchestrator to scan the entire swarm's capabilities instantly without acquiring locks.  
1. **Task Delegation Queues (Request-Response Streams):** Each active agent instantiates an iceoryx2 request-response port to serve as its inbound task queue. When the Orchestrator delegates a task, it allocates memory in the shared payload segment, writes the DelegationTask struct, and pushes the memory offset to the target's queue. The target agent retrieves the offset and resolves it to a local pointer, achieving true zero-copy data transfer.32  
1. **Lifecycle Event Bus (Publish-Subscribe):** To track task states (TaskStatusUpdateEvent) and stream partial artifacts, a publish-subscribe topology is established. Agents publish state changes to a shared topic. The Executive Monitor subscribes to this topic, utilizing its adaptive tick rate (100ms active, exponential backoff during stillness) to aggregate results into the TaskMatrix.

### **4.2. Typed Message Queue Design**

Communication over these queues must be strongly typed. By utilizing rkyv, Savant can perform zero-copy deserialization—accessing fields directly from the shared memory buffer without allocating new Rust structs on the heap.

The foundational message envelope is defined as follows:

Rust

\#\[repr(u8)\]  
\#\[derive(Copy, Clone, PartialEq, Eq)\]  
pub enum A2AMessageType {  
    TaskDelegation \= 0,  
    StatusUpdate \= 1,  
    ArtifactDelivery \= 2,  
    Interruption \= 3,  
}

\#  
\#  
pub struct A2AEnvelope {  
    pub message\_type: A2AMessageType,  
    pub session\_id\_hash: u64,  
    pub source\_agent\_id: \[u8; 32\],  
    pub target\_agent\_id: \[u8; 32\],  
    pub payload\_offset: u32,       // Pointer to Task, Status, or Artifact payload  
    pub payload\_size: u32,  
    pub cct\_signature: \[u8; 64\],   // Ed25519 signature for authorization  
    pub trace\_id: \[u8; 16\],        // W3C TraceContext for audit trails  
}

### **4.3. Backpressure and Flow Control**

Deploying 100+ concurrent agents on a single machine requires strict flow control. If an Orchestrator rapidly spawns subtasks, the target agents' inbound queues will fill.

The iceoryx2 lock-free queues inherently support bounded capacities. If an orchestrator attempts to push to a full queue, the operation yields a Full error. In this scenario, the Orchestrator integrates with the Anti-Dwindle Continuation Engine. Instead of blocking the async tokio thread or panicking, the Orchestrator catches the Full error, calculates a penalty score based on the target agent's pressure metric from the CollectiveBlackboard, and applies an exponential backoff (tokio::sleep) before attempting to match the task against the next most capable agent in the semantic index.

## **5\. Agent Card and Capability Advertisement Design**

To transition away from static configuration files and regex parsing, Savant requires a dynamic capability advertisement system. Borrowing heavily from the Google A2A AgentCard specification 5, the architecture must define a strictly typed struct that resides in the iceoryx2 Blackboard.

### **5.1. The AgentCard Struct**

Because the AgentCard lives in a lock-free shared memory segment, its size must be deterministic. Variable-length data, such as human-readable descriptions, are not stored directly in the card; instead, the card stores a pointer to the HNSW vector embedding in the CortexaDB layer.

Rust

\#\[repr(C)\]  
\#  
pub struct AgentCard {  
    pub agent\_id: \[u8; 32\],                // Deterministic FNV-1a hash  
    pub name: \[u8; 32\],                    // Null-terminated human-readable name  
    pub description\_vector\_id: u64,        // Pointer to HNSW intent vector  
    pub allowed\_skills\_mask: u128,         // Bitmask mapping to 128 global WASM tools  
    pub input\_modes: u8,                   // Bitflags (0x01=Text, 0x02=MemoryGraph)  
    pub output\_modes: u8,                  // Bitflags (0x01=Text, 0x02=JSON, 0x04=Artifact)  
    pub pressure: f32,                     // Current load metric (0.0 to 1.0)  
    pub total\_successes: u32,  
    pub total\_failures: u32,  
    pub protocol\_version: u16,             // For backward compatibility (e.g., 0x0100)  
    pub is\_active: bool,  
    pub \_padding: \[u8; 5\],                 // Align to 128 bytes for optimal cache-line fetching  
}

### **5.2. Semantic Capability Matching**

When an orchestrator must delegate a task (e.g., "Analyze the zstd compression efficiency of these three files"), it does not address an agent by name. Instead, it utilizes memory-aware agent selection:

1. **Intent Vectorization:** The Savant NLP layer processes the task description through a lightweight embedding model, generating a high-dimensional query vector.  
1. **Similarity Search:** The orchestrator executes an AVX2-accelerated similarity search against the HNSW index in the Fjall LSM-tree, querying the description\_vector\_id of all agents currently marked is\_active in the CollectiveBlackboard.25  
1. **Heuristic Scoring:** Raw cosine similarity is insufficient, as highly relevant agents might be overloaded. The orchestrator computes a composite score: Score \= (Cosine\_Similarity \* 0.7) \+ ((1.0 \- AgentCard.pressure) \* 0.3).  
1. **Skill Verification:** Before finalizing the selection, the orchestrator performs a bitwise AND operation between the task's required tool mask and the winning agent's allowed\_skills\_mask. If the agent lacks the necessary WASM tool authorization, the next highest-scoring agent is selected.

### **5.3. Dynamic Capability Updates**

If a subagent successfully synthesizes a novel WASM tool at runtime, it invokes the arc-swap hot-swappable tool registry. The agent then generates a new description vector, stores it in CortexaDB, and atomically updates its description\_vector\_id and allowed\_skills\_mask in the iceoryx2 Blackboard. This makes the new capability instantly discoverable to the entire swarm without requiring a system restart.23

## **6\. Typed Task Delegation Protocol Design**

Replacing the fragile /subagents spawn \<agentId\> \<task\> text pattern requires a robust data model governing how tasks are initiated, executed, and validated.

### **6.1. Task Lifecycle State Machine**

Mirroring the Google A2A specification 7, Savant implements a deterministic state machine. Crucially, every transition in this state machine must be journaled to the CortexaDB WAL to ensure that long-running multi-agent tasks survive process crashes.

Rust

\#\[repr(u8)\]  
\#  
pub enum TaskState {  
    Submitted \= 0,     // Enqueued in target's request port  
    Working \= 1,       // Target has acquired the payload and is executing  
    InputRequired \= 2, // Target is blocked, waiting for parent clarification  
    Completed \= 3,     // Artifact successfully written to shared memory  
    Failed \= 4,        // Execution trapped by WASM sandbox or LLM hallucination  
    Canceled \= 5,      // Parent broadcast an Interruption token  
}

### **6.2. The DelegationTask Struct**

The DelegationTask replaces the limited 128-byte SwarmSharedContext. It is archived via rkyv directly into the iceoryx2 shared memory payload segment.

Rust

\#  
\#  
pub struct DelegationTask {  
    pub task\_id: \[u8; 16\],                // UUID v4  
    pub parent\_session\_hash: u64,  
    pub token\_budget: u32,  
    pub max\_delegation\_depth: u8,         // Enforces the 20-level cycle prevention limit  
    pub priority\_level: u8,  
    pub deadline\_timestamp: u64,          // Epoch timestamp for automated cancellation  
    pub cct\_token: \[u8; 64\],              // Ed25519 Cryptographic Capability Token  
    pub context\_package\_offset: u32,      // Pointer to the bundled ContextPackage  
    pub expected\_schema\_hash: \[u8; 32\],   // Hash of the JSON schema required for the Artifact  
    pub task\_description: String,         // rkyv dynamically sized string  
}

### **6.3. Artifacts and Result Routing**

When a subagent completes a task, it does not simply append text to a Markdown file. It generates an Artifact conforming to the Google A2A specification's concept of modular Parts.2 The Artifact is serialized into shared memory.

To deliver the result, the subagent pushes an A2AEnvelope of type ArtifactDelivery to the parent agent's inbound queue. The parent agent's asynchronous tokio runtime polls this queue. Upon receiving the artifact, the parent agent can immediately access the result without blocking.

If the parent utilized Speculative Parallel Execution to assign the same read-only task to three different subagents 24, it gathers the three resulting Artifacts. It applies the Informational Entropy Gain heuristic—compressing the payload via zstd to measure information density—and selects the artifact with the lowest Shannon entropy for integration into its main context.

### **6.4. Cancellation Cascades and Timeouts**

Delegation tasks must respect temporal bounds. The DelegationTask includes a deadline\_timestamp. If an agent determines that a task has exceeded this deadline, or if the parent orchestrator encounters an emergency\_halt condition, a cancellation cascade is triggered.

The parent utilizes a tokio::util::sync::CancellationToken. When triggered, the parent broadcasts an A2AMessageType::Interruption across the iceoryx2 pub-sub event bus. Because all subagents utilize wasmtime with epoch deadlines, the subagent's execution engine checks for cancellation tokens at every tick. Upon detecting the interruption, the subagent immediately terminates the WASM sandbox, updates its TaskState to Canceled in the WAL, and forwards the cancellation token to any of its own nested subagents, enforcing cycle prevention down the entire graph.

## **7\. Memory-Aware Context Passing Design**

The primary bottleneck in prolonged agentic workflows is context bloat.22 When orchestrators blindly concatenate full conversation histories and pass them as text strings to subagents, they exhaust LLM token limits, degrade reasoning ("lost in the middle" phenomena), and skyrocket inference latency.22

Savant's distinct advantage is its "Glass House" memory architecture, backed by an Obsidian graph and a Fjall LSM-tree. To make A2A communication memory-aware, Savant will adopt the principles of the Agent Cognitive Compressor (ACC) 21, replacing transcript replay with bounded state commitments.

### **7.1. The ContextPackage Concept**

Instead of passing raw text, the Orchestrator passes a ContextPackage—a strictly typed index into the shared memory graph. This grants the subagent precise, scoped read-access to the relevant data without duplicating memory.

Rust

\#  
\#  
pub struct ContextPackage {  
    pub episodic\_node\_ids: Vec\<u64\>,  // Pointers to conversation turns in Fjall  
    pub semantic\_node\_ids: Vec\<u64\>,  // Pointers to established Ground Truth facts  
    pub active\_tool\_outputs: Vec\<u32\>, // Offsets to recent tool execution results  
    pub taint\_tags: Vec\<f32\>,         // Data provenance tracking for security enclaves  
}

### **7.2. Context Hydration and Taint Tracking**

1. **Extraction:** When the Orchestrator formulates a DelegationTask, it queries the HNSW vector store to find the top-K relevant Semantic and Episodic nodes from the Fjall database related to the task intent.25  
1. **Bundling:** The u64 IDs of these nodes are packed into the ContextPackage vector and serialized into the shared memory payload.  
1. **Hydration:** The spawned subagent operates in an isolated namespace. It receives the ContextPackage and performs zero-copy reads against the Fjall LSM-tree using the provided IDs. The subagent *only* sees the specific subgraph relevant to its task, artificially bounding its context window and maximizing its token budget for pure reasoning.  
1. **Provenance Enforcement:** The ContextPackage explicitly carries taint\_tags. If a memory node originated from an untrusted source (e.g., external\_web=0.2), the subagent's execution environment inherits this taint. The wasmtime sandbox enforces these tags, categorically denying the subagent the ability to pass tainted data into high-security sinks, such as executing shell commands based on web context.

## **8\. Code Structure Recommendations**

To integrate this sophisticated A2A layer into Savant's existing 75,000 lines of code cleanly, a dedicated IPC protocol module must be established, maintaining the existing blackboard.rs for backward compatibility while expanding its functionality.

**New Modules to Create (crates/ipc/src/a2a/):**

* mod.rs: Central initialization and module exports for the A2A subsystem.  
* protocol.rs: Definitions for A2AEnvelope, DelegationTask, TaskState, and Artifact utilizing the rkyv macros for zero-copy validation.  
* agent\_card.rs: The AgentCard \#\[repr(C)\] struct definition and logic for capability bitmask operations.  
* queues.rs: Safe Rust wrappers around the iceoryx2 Request-Response lock-free queue primitives.  
* context.rs: The ContextPackage definition, linking the HNSW node IDs to the Fjall database query engine.

**Existing Files to Modify:**

* crates/ipc/src/blackboard.rs: Expand the static struct implementation to instantiate the iceoryx2 Key-Value Blackboard pattern 31, providing the highly concurrent registry for tracking AgentCard metadata.  
* crates/agent/src/orchestration/mod.rs: Deprecate and remove the text-based /subagents spawn regex parsing. Wire the LLM output parser to validate against the DelegationTask JSON schema.  
* crates/agent/src/orchestration/handoff.rs: Rewrite validate\_handoff(). It must execute the check\_agent\_card\_capabilities() similarity search against the Blackboard, mint the Ed25519 CCT token, and enqueue the task.  
* crates/agent/src/orchestration/continuation.rs: Hook the exponential backoff logic (tokio::sleep) into the new TaskState::InputRequired enum and iceoryx2 queue-full backpressure events.  
* crates/memory/src/lsm\_engine.rs: Inject journaling routines to sync TaskState transitions with the CortexaDB WAL, enabling deterministic crash recovery.

## **9\. Codebase Integration Points**

Precise function hooks must be established to weave the A2A layer into the orchestration engine without breaking speculative execution or crash recovery.

1. **Orchestrator NLP Router (crates/agent/src/nlp/mod.rs):**  
   * *Hook:* Update the system prompts sent to providers (Anthropic, OpenAI, Deepseek) to strictly enforce JSON output that maps to the DelegationTask schema.  
   * *Hook:* In the route\_command() function, detect the delegation JSON. Invoke rkyv deserialization. If validation fails, intercept the error and route it to the Continuation Engine for an automated retry, preventing the hallucination from entering the task queue.  
1. **Handoff Execution (crates/agent/src/orchestration/handoff.rs):**  
   * *Hook:* Replace simple Bloom filter checks with a call to calculate\_semantic\_match(task\_embedding, blackboard\_cards).  
   * *Hook:* Invoke the mint\_cct\_token() function to cryptographically sign the DelegationTask struct hash before writing it to shared memory.  
1. **Result Aggregation (crates/agent/src/orchestration/branching.rs):**  
   * *Hook:* Modify the speculative timeline collapse logic. Instead of parsing the flat Markdown Task Matrix, the engine will await A2AMessageType::ArtifactDelivery events on its inbound iceoryx2 queue, extracting the typed Parts for entropy evaluation.  
1. **Crash Recovery Engine (crates/memory/src/lsm\_engine.rs):**  
   * *Hook:* During process initialization, the recover\_from\_checkpoint() function must scan the WAL for any DelegationTask entries marked TaskState::Working. If found, the framework re-hydrates the task and re-queues it in the target agent's inbound port, achieving exactly-once execution semantics even after a hard fault.

## **10\. Testing Strategy**

Validating distributed, highly concurrent, lock-free IPC mechanisms requires specialized methodologies to prevent flaky, non-deterministic integration test suites.

1. **Formal Verification with Kani:** Apply the Kani Rust Verifier to crates/ipc/src/a2a/queues.rs. Because iceoryx2 relies on unsafe lock-free pointer arithmetic, Kani will mathematically prove the absence of data races, deadlocks, and out-of-bounds reads during task queue operations.  
1. **Concurrency Exploration with Loom:** Replace standard std::sync atomics with loom primitives within the TaskState transition logic. Run the Loom scheduler to exhaustively explore all possible thread interleavings when multiple subagents attempt to update the CollectiveBlackboard simultaneously.  
1. **Property-Based Testing (Proptest):** Utilize the proptest crate to generate thousands of randomized, malformed, or boundary-case DelegationTask and ContextPackage payloads. Pass these through the rkyv::check\_bytes validator to ensure the agent framework correctly rejects corrupted shared memory offsets without panicking the process.  
1. **Deterministic Crash Simulation:** Expand the existing CortexaDB WAL test suite. Create a test harness that spawns a parent and subagent, initiates a DelegationTask, and sends a SIGKILL to the subagent process while it is in the TaskState::Working phase. Verify that the parent agent detects the dropped connection via an iceoryx2 heartbeat failure, safely traps the error without crashing, and that the WAL successfully rehydrates the task state upon process restart.

## **Conclusion**

The transition from a text-based, monolithic orchestrator to a strictly typed, zero-copy Agent-to-Agent communication layer represents a critical evolution for the Savant framework. By discarding fragile string-matching handoffs and adopting the structural rigor of the Google A2A data model—Agent Cards, Task lifecycles, and Artifacts—Savant guarantees deterministic routing and lifecycle management. Adapting these concepts to Rust's high-performance, single-machine IPC primitives via iceoryx2 and rkyv ensures sub-microsecond latency. Furthermore, integrating memory-aware ContextPackages scoped by the Obsidian graph fundamentally resolves the context bloat and token exhaustion that plague traditional multi-agent systems. This architectural blueprint ensures that Savant remains highly secure, deterministically recoverable, and capable of seamlessly executing complex, long-horizon autonomous workflows.

### **Works cited**

1. What is A2A protocol (Agent2Agent)? \- IBM, accessed May 14, 2026, [https://www.ibm.com/think/topics/agent2agent-protocol](https://www.ibm.com/think/topics/agent2agent-protocol)  
1. A2A Protocol: An In-Depth Guide. The Need for Agent Interoperability | by Saeed Hajebi, accessed May 14, 2026, [https://medium.com/@saeedhajebi/a2a-protocol-an-in-depth-guide-78387f992f59](https://medium.com/@saeedhajebi/a2a-protocol-an-in-depth-guide-78387f992f59)  
1. Getting Started with Agent2Agent (A2A) Protocol: A Purchasing Concierge and Remote Seller Agent Interactions on Cloud Run and Agent Engine | Google Codelabs, accessed May 14, 2026, [https://codelabs.developers.google.com/intro-a2a-purchasing-concierge](https://codelabs.developers.google.com/intro-a2a-purchasing-concierge)  
1. Understanding A2A — The protocol for agent collaboration \- Google Developer forums, accessed May 14, 2026, [https://discuss.google.dev/t/understanding-a2a-the-protocol-for-agent-collaboration/189103](https://discuss.google.dev/t/understanding-a2a-the-protocol-for-agent-collaboration/189103)  
1. AgentCard – Agent2Agent Protocol \- The A2A Protocol Community, accessed May 14, 2026, [https://agent2agent.info/docs/concepts/agentcard/](https://agent2agent.info/docs/concepts/agentcard/)  
1. Artifact – Agent2Agent Protocol \- The A2A Protocol Community, accessed May 14, 2026, [https://agent2agent.info/docs/concepts/artifact/](https://agent2agent.info/docs/concepts/artifact/)  
1. Google A2A Protocol: Agent-to-Agent Communication Guide \- Digital Applied, accessed May 14, 2026, [https://www.digitalapplied.com/blog/google-a2a-protocol-agent-to-agent-communication-guide](https://www.digitalapplied.com/blog/google-a2a-protocol-agent-to-agent-communication-guide)  
1. Subgraphs \- Docs by LangChain, accessed May 14, 2026, [https://docs.langchain.com/oss/python/langgraph/use-subgraphs](https://docs.langchain.com/oss/python/langgraph/use-subgraphs)  
1. AI Agents XII — LangGraph graph-based framework \- Artificial Intelligence in Plain English, accessed May 14, 2026, [https://ai.plainenglish.io/ai-agents-xii-langgraph-graph-based-framework-b7b74e1fa5df](https://ai.plainenglish.io/ai-agents-xii-langgraph-graph-based-framework-b7b74e1fa5df)  
1. Agent-to-Agent (A2A) Protocol \- CrewAI Documentation, accessed May 14, 2026, [https://docs.crewai.com/en/learn/a2a-agent-delegation](https://docs.crewai.com/en/learn/a2a-agent-delegation)  
1. Introduction \- CrewAI Documentation, accessed May 14, 2026, [https://docs.crewai.com/en/introduction](https://docs.crewai.com/en/introduction)  
1. Hierarchical Process \- CrewAI Documentation, accessed May 14, 2026, [https://docs.crewai.com/en/learn/hierarchical-process](https://docs.crewai.com/en/learn/hierarchical-process)  
1. Bug: Sub-agents spawned from main session require manual device pairing approval · Issue \#24019 \- GitHub, accessed May 14, 2026, [https://github.com/openclaw/openclaw/issues/24019](https://github.com/openclaw/openclaw/issues/24019)  
1. \[Bug\] Subagent spawn fails with 'pairing required' after 2.19-2 update — devices already paired · Issue \#21445 \- GitHub, accessed May 14, 2026, [https://github.com/openclaw/openclaw/issues/21445](https://github.com/openclaw/openclaw/issues/21445)  
1. OpenClaw Bug Report: Subagent Spawning Fails with Local Models (ollama/\*) \- GitHub, accessed May 14, 2026, [https://github.com/openclaw/openclaw/issues/24654](https://github.com/openclaw/openclaw/issues/24654)  
1. Subagents spawned via sessions\_spawn cannot invoke tools (output as text instead of tool\_use) · Issue \#9857 \- GitHub, accessed May 14, 2026, [https://github.com/openclaw/openclaw/issues/9857](https://github.com/openclaw/openclaw/issues/9857)  
1. I analyzed 3 A2A approaches. 2 already failed. Here's what's actually missing. \- Reddit, accessed May 14, 2026, [https://www.reddit.com/r/artificial/comments/1synrp2/i\_analyzed\_3\_a2a\_approaches\_2\_already\_failed/](https://www.reddit.com/r/artificial/comments/1synrp2/i_analyzed_3_a2a_approaches_2_already_failed/)  
1. Feature: A2A (Agent-to-Agent) Protocol Support — Remote Agent Discovery, Communication & Interoperability · Issue \#514 · NousResearch/hermes-agent \- GitHub, accessed May 14, 2026, [https://github.com/NousResearch/hermes-agent/issues/514](https://github.com/NousResearch/hermes-agent/issues/514)  
1. Agent Audit Trail: A Standard Logging Format for Autonomous AI Systems \- IETF Datatracker, accessed May 14, 2026, [https://datatracker.ietf.org/doc/draft-sharif-agent-audit-trail/](https://datatracker.ietf.org/doc/draft-sharif-agent-audit-trail/)  
1. Decision Traces: Building Audit Trails for Autonomous AI Agents \- Streamkap, accessed May 14, 2026, [https://streamkap.com/resources-and-guides/decision-traces-ai-agents](https://streamkap.com/resources-and-guides/decision-traces-ai-agents)  
1. AI Agents Need Memory Control Over More Context \- arXiv, accessed May 14, 2026, [https://arxiv.org/html/2601.11653v1](https://arxiv.org/html/2601.11653v1)  
1. Architecting efficient context-aware multi-agent framework for production \- Google for Developers Blog, accessed May 14, 2026, [https://developers.googleblog.com/architecting-efficient-context-aware-multi-agent-framework-for-production/](https://developers.googleblog.com/architecting-efficient-context-aware-multi-agent-framework-for-production/)  
1. Agent Discovery, Naming, and Resolution \- the Missing Pieces to A2A | Solo.io, accessed May 14, 2026, [https://www.solo.io/blog/agent-discovery-naming-and-resolution---the-missing-pieces-to-a2a](https://www.solo.io/blog/agent-discovery-naming-and-resolution---the-missing-pieces-to-a2a)  
1. Announcing the Agent2Agent Protocol (A2A) \- Google for Developers Blog, accessed May 14, 2026, [https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)  
1. What Is Agent Memory Infrastructure? How Mem0 Beats OpenAI's Built-In Memory by 26%, accessed May 14, 2026, [https://www.mindstudio.ai/blog/agent-memory-infrastructure-mem0-vs-openai](https://www.mindstudio.ai/blog/agent-memory-infrastructure-mem0-vs-openai)  
1. Engineering Agent Memory | Blog of Ken W. Alger, accessed May 14, 2026, [https://www.kenwalger.com/blog/ai/engineering-agent-memory/](https://www.kenwalger.com/blog/ai/engineering-agent-memory/)  
1. iceoryx2 \- Rust \- Docs.rs, accessed May 14, 2026, [https://docs.rs/iceoryx2/latest](https://docs.rs/iceoryx2/latest)  
1. Eclipse iceoryx2™ \- true zero-copy inter-process-communication with a Rust core \- GitHub, accessed May 14, 2026, [https://github.com/eclipse-iceoryx/iceoryx2](https://github.com/eclipse-iceoryx/iceoryx2)  
1. Announcing iceoryx2 v0.5: Fast and Robust Inter-Process Communication (IPC) Library for Rust, C++, and C \- Reddit, accessed May 14, 2026, [https://www.reddit.com/r/programming/comments/1hktm3a/announcing\_iceoryx2\_v05\_fast\_and\_robust/](https://www.reddit.com/r/programming/comments/1hktm3a/announcing_iceoryx2_v05_fast_and_robust/)  
1. Blackboard \- The iceoryx2 Book \- GitHub Pages, accessed May 14, 2026, [https://ekxide.github.io/iceoryx2-book/main/getting-started/robot-nervous-system/blackboard.html](https://ekxide.github.io/iceoryx2-book/main/getting-started/robot-nervous-system/blackboard.html)  
1. Advanced Messaging Patterns \- Blackboard \- ekxide Blog, accessed May 14, 2026, [https://ekxide.io/blog/advanced-messaging-patterns-blackboard/](https://ekxide.io/blog/advanced-messaging-patterns-blackboard/)  
1. Implementing True Zero-Copy Communication with iceoryx2 \- ekxide Blog, accessed May 14, 2026, [https://ekxide.io/blog/how-to-implement-zero-copy-communication/](https://ekxide.io/blog/how-to-implement-zero-copy-communication/)  
1. accessed May 14, 2026, [https://letsdatascience.com/blog/a2a-protocol-agent-to-agent](https://letsdatascience.com/blog/a2a-protocol-agent-to-agent)

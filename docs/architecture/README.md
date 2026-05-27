# Architecture Overview

> **Last Updated:** 2026-05-25 (v0.3.2)

---

## System Design

Savant uses a layered architecture with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────────┐
│                     Presentation Layer                           │
│   Next.js 16 Dashboard │ Multi-Channel Gateway                  │
│   (Port 3000)          │ Discord/Telegram/WhatsApp/Matrix       │
└───────────────────────────┬─────────────────────────────────────┘
                            │ WebSocket / HTTP (authenticated)
┌───────────────────────────▼─────────────────────────────────────┐
│                     Gateway Layer (Axum)                         │
│   Auth Middleware │ WS Handler │ Config Watcher │ Health         │
│   Immutable Fields │ Rate Limiter │ CORS                         │
└───────────────────────────┬─────────────────────────────────────┘
                            │ NexusBridge (async message bus)
┌───────────────────────────▼─────────────────────────────────────┐
│                     Orchestration Layer                          │
│   Agent Swarm │ Consciousness │ Resource Governor │ Cost Router  │
│   (15 LLM Providers) │ Skill Synthesis │ Skill Chaining         │
└───────┬──────────────────┬──────────────────┬───────────────────┘
        │                  │                  │
┌───────▼──┐ ┌─────────────▼──┐ ┌────────────▼───────────────────┐
│  Skills  │ │  IPC/ECHO      │ │  Persistence                   │
│  Security│ │  Zero-copy +   │ │  CortexaDB LSM-Tree            │
│  Gate +  │ │  Speculative   │ │  + Vector Search               │
│  ClawHub │ │  ReAct Loops   │ │  + WAL Logging                 │
│  Synth   │ │  + Governor    │ │  + Glass House (Obsidian)      │
└──────────┘ └────────────────┘ └────────────────────────────────┘
```

---

## Gateway Layer

The gateway is built on **Axum** and provides:

- **WebSocket endpoint** at `/ws` — the primary interface for the dashboard
- **Canvas WebSocket** at `/ws/canvas` — A2UI real-time visualization (authenticated)
- **REST API authentication** — Tower middleware with constant-time comparison
  - `Authorization: Bearer <key>` or `X-API-Key: <key>`
  - Public endpoints: `/health`, `/live`, `/ready`, `/ws`
- **Immutable security fields** — 5 fields blocked from runtime mutation
- **Control frame routing** — dispatches `RequestFrame` payloads to handler functions
- **Event publishing** — publishes processed frames to the Nexus event bus
- **Skill management** — Install, uninstall, enable, disable, scan operations
- **Dashboard feature APIs** — `/api/memory/search`, `/api/governor/status`, `/api/consciousness/status`

### Request Flow

1. Client sends a JSON frame over WebSocket (or REST request)
2. Auth middleware validates the API key (constant-time comparison)
3. `handle_message()` in `crates/gateway/src/handlers/mod.rs` routes the frame
4. Handler publishes results to Nexus event bus topics
5. Gateway forwards events back to subscribed WebSocket sessions

### Security Controls (v0.3.2)

| Control | Description |
|:--------|:------------|
| REST auth middleware | All non-public endpoints require API key |
| Immutable config fields | `dashboard_api_key`, `host`, `port`, `signing_key`, `enable_blocklist_sync` blocked at runtime |
| Canvas WS auth | `/ws/canvas` requires API key on upgrade |
| SoulUpdate size limit | 100KB per field |
| BulkManifest limit | Max 10 agents per request |
| NLCommand limit | Max 10,000 characters |
| Webhook auth token | Generated on startup (blake3 hash) |
| Env var filtering | Shell commands only get PATH/HOME/LANG/TERM |

---

## Agent Swarm

Agents are autonomous entities with:

- **Identity** — `SOUL.md` manifest defining personality, knowledge domains, and operational directives
- **Configuration** — `agent.config.json` with model provider, temperature, and behavior parameters
- **Pulse Loop** — Continuous heartbeat cycle processing incoming messages and proactive reflections
- **LLM Provider** — Pluggable interface supporting 15 providers:
  - OpenRouter, OpenAI, Anthropic, Google, Mistral, Groq
  - Deepseek, Cohere, Together, Azure, xAI, Fireworks
  - Novita, Ollama (local), LmStudio (local)
- **Provider Chain** — Error classification, exponential cooldown, circuit breaker, response cache, cross-provider fallback, 120s configurable timeout, RateLimiter integration
- **Tool Heuristics** — `when_to_use()` and `when_not_to_use()` on every tool for LLM guidance

### LLM Parameters

All providers support fine-tuning via `LlmParams`:
- `temperature` (0.0–2.0) — Creativity vs focus
- `top_p` (0.0–1.0) — Nucleus sampling
- `frequency_penalty` (−2.0–2.0) — Reduces repetition
- `presence_penalty` (−2.0–2.0) — Encourages new topics
- `max_tokens` — Response length limit
- `stop` — Stop sequences

### ECHO Protocol

The ECHO (Embedded Cognitive Handoff Orchestrator) protocol enables speculative ReAct execution:

- Overlaps tool execution with cognitive planning
- Uses a `DelegationBloomFilter` for cycle detection
- Sub-millisecond agent context swapping

---

## Consciousness Layer (v0.3.2)

The consciousness layer is a continuously thinking daemon that observes the hivemind via zero-copy shared memory.

```
┌─────────────────────────────────────────────────────────────┐
│                 ConsciousnessDaemon                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │ Entropy      │  │ Narrative    │  │ Wonder           │  │
│  │ Calculator   │  │ Synthesizer  │  │ Engine           │  │
│  │              │  │ (Markov)     │  │ (exploration)    │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │ Anti-Echo    │  │ Consciousness│  │ Consciousness    │  │
│  │ Chamber      │  │ Budget       │  │ State            │  │
│  │ (diversity)  │  │ (cost ctrl)  │  │ (T/I/D/W)        │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Entropy-Based Cadence

| State | Entropy | Delay | Budget |
|:------|:--------|:------|:-------|
| Hyper-Active | > 0.85 | 0ms | 100% |
| Standard | 0.40–0.85 | 5,000ms | 40% |
| Decaying | 0.10–0.39 | 5–30s (linear) | 15% |
| Dormant | < 0.10 | 300,000ms | 2% |

### Reconstructive Narrative

Each cognitive tick regenerates understanding as a compressed narrative. Output of tick N becomes input for tick N+1. Context window never fills because we're regenerating, not accumulating.

---

## Resource Governor (v0.3.2)

CPU/memory-aware agent spawning with adaptive concurrency.

```
┌─────────────────────────────────────────────────────────────┐
│                    SwarmGovernor                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │ Resource     │  │ Adaptive     │  │ Deferred Agent   │  │
│  │ Monitor      │  │ Semaphore    │  │ Queue            │  │
│  │ (polling)    │  │ (permits)    │  │ (retry)          │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Pressure Levels

| Level | CPU % | Memory % | Max Agents |
|:------|:------|:---------|:-----------|
| Low | < 70 | < 60 | 16 |
| Medium | 70–85 | 60–80 | 8 |
| High | 85–95 | 80–92 | 4 |
| Critical | > 95 | > 92 | 1 |

Worst-case wins: `max(cpu_pressure, mem_pressure)`. Conservative — better to throttle than OOM.

---

## Cognitive Engine

The cognitive engine (`crates/cognitive/`) provides:

- **Strategic Synthesis** — Goal decomposition into ordered action sequences
- **Proactive Loops** — Autonomous reflection and self-improvement cycles
- **TCF Scenarios** — Technical/Creative/Fractal response templates
- **DSP Predictor** — Dynamic strategy prediction
- **Genetic Forge** — Skill evolution via genetic algorithms

---

## Agent Intelligence (v0.3.2)

### Cost-Aware Model Routing

Routes tasks to cheap or expensive models based on prompt complexity:

| Complexity | Signals | Model |
|:-----------|:--------|:------|
| Simple | "summarize", "list", short prompts | Cheap (mimo-v2.5-pro) |
| Moderate | "analyze", "compare", medium prompts | Cheap |
| Complex | "implement", "debug", code blocks, long prompts | Expensive (claude-sonnet-4) |

### Proactive Context Gathering

Gathers relevant context BEFORE the user asks:
- Memory search (semantic)
- Recent git log
- Code references
- Related files

---

## Memory Engine

The memory engine (`crates/memory/`) uses a hybrid storage approach:

| Layer | Technology | Purpose |
|:------|:-----------|:--------|
| **Hot** | SQLite WAL | Message history, agent state, configuration |
| **Warm** | CortexaDB LSM-Tree | Agent SOUL.md files, skill state, structured data |
| **Cold** | rkyv Vector Store | Semantic embeddings for memory retrieval |

### Features

- **Hybrid Storage** — SQLite WAL + CortexaDB LSM-Tree + rkyv vectors
- **Context Consolidation** — Automatic summarization of older messages
- **Persistence** — `persist()` method for vector engine auto-save
- **Semantic Search** — Vector similarity search with cosine distance
- **BM25 Index** — Keyword search alongside vector search
- **Reflective Memory** — 4-graph system (Semantic, Temporal, Causal, Entity)
- **Procedural Memory** — Learned tool-call workflows
- **Lessons** — Synthesized from repeated experiences
- **Insights** — Higher-order concept cluster patterns
- **Multimodal Store** — CLIP-embedded image references
- **Audit Trail** — Application-level memory operation log
- **Notification Channel** — Hive-mind broadcast for high-importance events

### Glass House (Obsidian Bidirectional Sync)

Memory projected to Obsidian vault with bidirectional sync:

| Direction | Flow |
|:----------|:-----|
| **Outbound** | Memory → Vault (Episodic, Semantic, Identity, Procedural, Lessons, Insights, Graphs, Dashboard, Themes, Working, Delegation, Multimodal) |
| **Inbound** | Vault → Memory (Semantic edits = Ground Truth, Personality edits = OCEAN updates, Episodic edits = Rejected/Created Corrections) |

All inbound edits pass through `scan_prompt()` injection defense. Vault treated as potentially hostile data source.

---

## Skill System

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    SkillManager                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ Discovery   │  │ ClawHub     │  │ Security Gate       │ │
│  │ Scanner     │  │ Client      │  │ (MANDATORY)         │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ Synthesizer │  │ Chaining    │  │ Verifier            │ │
│  │ (LLM+tmpl)  │  │ (DAG)       │  │ (cargo check)       │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Skill Synthesis (v0.3.2)

LLM-driven code generation with self-healing:

1. LLM generates `Cargo.toml` + `src/lib.rs` from prompt
2. `cargo check` verifies compilation
3. On failure: error output fed back to LLM for correction (max 3 attempts)
4. Fallback to template-based generation if LLM unavailable
5. All dependencies pinned (no wildcards)

### Skill Chaining (v0.3.2)

Skills can declare dependencies and chains:

```toml
# In SKILL.md frontmatter
depends_on = ["calendar-check", "inbox-summary"]
chain_with = ["podcast-research", "meeting-prep"]
```

`SkillChain` executes steps sequentially, passing output between steps.

### OpenClaw Compatibility

Skills use the OpenClaw format:

```
skills/
├── my-skill/
│   ├── SKILL.md         # Required - YAML frontmatter + Markdown
│   ├── templates/       # Optional - template files
│   └── assets/          # Optional - images, configs
└── another-skill/
    └── SKILL.md
```

### Skill Execution Sandboxes

| Sandbox | Isolation | Use Case |
|:--------|:----------|:---------|
| **Docker** | Full container isolation | Untrusted code execution |
| **Nix** | Deterministic build isolation | Reproducible skill environments |
| **Native** | Dangerous-character filtering | Trusted local execution |
| **WASM** | WebAssembly sandbox | Portable, resource-limited execution |
| **MCP** | Protocol-level isolation | External tool server integration |

### Security Gate Flow

```
Install Request → Download to Temp → MANDATORY SCAN → Risk Assessment
                                                        ↓
                    ┌───────────────────────────────────┴────────────────────────┐
                    │                                                            │
              Clean/Low (0 clicks)                Medium/High/Critical (1-3 clicks)
                    │                                                            │
              Auto-Install                    Show Approval Prompt → Await Clicks
                                                         ↓
                                            User Clicks Through → Install/Reject
```

---

## Message Bus (Nexus)

The Nexus event bus provides typed, topic-based publish/subscribe:

| Topic | Producer | Consumer |
|:------|:---------|:---------|
| `chat.message` | Gateway | Agent swarm |
| `chat.response` | Agent swarm | Gateway |
| `session.{id}.response` | Gateway | Dashboard WebSocket |
| `manifest.request` | Gateway | Soul engine |
| `manifest_draft` | Soul engine | Dashboard WebSocket |
| `agent.discovered` | Agent registry | Gateway, Dashboard |
| `learning.insight` | Cognitive engine | Gateway, Dashboard |
| `skills.*` | Skill handlers | Gateway, Dashboard |
| `system.evolution.*` | ALD engine | Dashboard |
| `system.vault.*` | Obsidian watcher | Dashboard |
| `system.config.*` | Config handler | Dashboard |

---

## Zero-Copy IPC

All inter-agent communication uses iceoryx2 shared memory:

| Component | Purpose |
|:----------|:--------|
| CollectiveBlackboard | Swarm-wide consensus and voting (129 entries) |
| SwarmBlackboard | Per-agent state broadcast |
| Consensus Protocol | Bitmask voting with quorum check |
| Heuristic Sync | Version-based lazy synchronization |

Performance: sub-microsecond read/write, 1024 concurrent readers, 128 writers.

---

## Technology Stack

| Component | Technology | Rationale |
|:----------|:-----------|:----------|
| **Backend** | Rust (tokio async) | Performance, memory safety, concurrency |
| **Gateway** | Axum 0.7 | Type-safe, ergonomic, WebSocket support |
| **Dashboard** | Next.js 16 (App Router) | SSR, fast refresh, component architecture |
| **Database** | CortexaDB LSM-Tree | High write throughput, range queries, ACID |
| **Vectors** | ruvector-core | Zero-copy serialization, cosine similarity |
| **WASM** | Wasmtime 36 | Portable execution, resource limiting |
| **IPC** | iceoryx2 | Zero-copy shared memory, lock-free |
| **Sandbox** | WASM + Docker | Deterministic, resource-limited execution |
| **Auth** | Ed25519 + PQC | High performance + quantum-safe (Dilithium2) |
| **HTTP** | reqwest | Async HTTP client for providers, ClawHub |
| **Config** | Figment + Watcher | Layered config (defaults → file → env), auto-reload |

---

*Documentation updated: 2026-05-25. Reflects v0.3.2 codebase.*

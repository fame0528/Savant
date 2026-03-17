# Architecture Overview

## System Design

Savant uses a layered architecture with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────┐
│                     Presentation Layer                       │
│   Next.js 16 Dashboard │ Multi-Channel Gateway              │
│   (Port 3000)          │ Discord/Telegram/WhatsApp/Matrix   │
└───────────────────────────┬─────────────────────────────────┘
                            │ WebSocket / HTTP
┌───────────────────────────▼─────────────────────────────────┐
│                     Gateway Layer (Axum)                     │
│   Auth Middleware │ WS Handler │ Config Watcher │ Health     │
└───────────────────────────┬─────────────────────────────────┘
                            │ NexusBridge (async message bus)
┌───────────────────────────▼─────────────────────────────────┐
│                     Orchestration Layer                      │
│   Agent Swarm │ Cognitive Engine │ Memory Engine             │
│   (15 LLM Providers)                                      │
└───────┬──────────────────┬──────────────────┬───────────────┘
        │                  │                  │
┌───────▼──┐ ┌─────────────▼──┐ ┌────────────▼───────────────┐
│  Skills  │ │  IPC/ECHO      │ │  Persistence               │
│  Security│ │  Zero-copy +   │ │  Fjall LSM-Tree            │
│  Gate +  │ │  Speculative   │ │  + Vector Search           │
│  ClawHub │ │  ReAct Loops   │ │  + WAL Logging             │
└──────────┘ └────────────────┘ └────────────────────────────┘
```

## Gateway Layer

The gateway is built on **Axum** and provides:

- **WebSocket endpoint** at `/ws` — the primary interface for the dashboard
- **Authentication middleware** — Ed25519-signed session tokens with nonce replay prevention
- **Control frame routing** — dispatches `RequestFrame` payloads to handler functions
- **Event publishing** — publishes processed frames to the Nexus event bus
- **Skill management** — Install, uninstall, enable, disable, scan operations

### Request Flow

1. Client sends a JSON frame over WebSocket
2. Auth middleware validates the session token
3. `handle_message()` in `crates/gateway/src/handlers/mod.rs` routes the frame
4. Handler publishes results to Nexus event bus topics
5. Gateway forwards events back to subscribed WebSocket sessions

### Skill Control Frames

| Frame | Description |
|:------|:------------|
| `SkillsList` | List all skills with security scan status |
| `SkillInstall` | Install from ClawHub with pre-scan |
| `SkillUninstall` | Remove skill directory |
| `SkillEnable` | Enable a disabled skill |
| `SkillDisable` | Disable a skill |
| `SkillScan` | Run security scan on existing skill |

## Agent Swarm

Agents are autonomous entities with:

- **Identity** — `SOUL.md` manifest defining personality, knowledge domains, and operational directives
- **Configuration** — `agent.config.json` with model provider, temperature, and behavior parameters
- **Pulse Loop** — Continuous heartbeat cycle processing incoming messages and proactive reflections
- **LLM Provider** — Pluggable interface supporting 15 providers:
  - OpenRouter, OpenAI, Anthropic, Google, Mistral, Groq
  - Deepseek, Cohere, Together, Azure, xAI, Fireworks
  - Novita, Ollama (local), LmStudio (local)

### LLM Parameters

All providers support fine-tuning via `LlmParams`:
- `temperature` (0.0-2.0) — Creativity vs focus
- `top_p` (0.0-1.0) — Nucleus sampling
- `frequency_penalty` (-2.0-2.0) — Reduces repetition
- `presence_penalty` (-2.0-2.0) — Encourages new topics
- `max_tokens` — Response length limit
- `stop` — Stop sequences

### ECHO Protocol

The ECHO (Embedded Cognitive Handoff Orchestrator) protocol enables speculative ReAct execution:

- Overlaps tool execution with cognitive planning
- Uses a `DelegationBloomFilter` for cycle detection
- Sub-millisecond agent context swapping

## Cognitive Engine

The cognitive engine (`crates/cognitive/`) provides:

- **Strategic Synthesis** — Goal decomposition into ordered action sequences
- **Proactive Loops** — Autonomous reflection and self-improvement cycles
- **TCF Scenarios** — Technical/Creative/Fractal response templates

## Memory Engine

The memory engine (`crates/memory/`) uses a hybrid storage approach:

| Layer | Technology | Purpose |
|:------|:-----------|:--------|
| **Hot** | SQLite WAL | Message history, agent state, configuration |
| **Warm** | Fjall LSM-Tree | Agent SOUL.md files, skill state, structured data |
| **Cold** | rkyv Vector Store | Semantic embeddings for memory retrieval |

### Features

- **Hybrid Storage** — SQLite WAL + Fjall LSM-Tree + rkyv vectors
- **Context Consolidation** — Automatic summarization of older messages
- **Persistence** — `persist()` method for vector engine auto-save
- **Semantic Search** — Vector similarity search with cosine distance

## Skill System

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    SkillManager                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ Discovery   │  │ ClawHub     │  │ Security Gate       │ │
│  │ Scanner     │  │ Client      │  │ (MANDATORY)         │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

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

## Technology Stack

| Component | Technology | Rationale |
|:----------|:-----------|:----------|
| **Backend** | Rust (tokio async) | Performance, memory safety, concurrency |
| **Gateway** | Axum 0.7 | Type-safe, ergonomic, WebSocket support |
| **Dashboard** | Next.js 16 (App Router) | SSR, fast refresh, component architecture |
| **Database** | Fjall LSM-Tree | High write throughput, range queries, ACID |
| **Vectors** | ruvector-core | Zero-copy serialization, cosine similarity |
| **WASM** | Wasmtime 36 | Portable execution, resource limiting |
| **IPC** | iceoryx2 | Zero-copy shared memory, lock-free |
| **Sandbox** | WASM + Docker | Deterministic, resource-limited execution |
| **Auth** | Ed25519 + PQC | High performance + quantum-safe (Dilithium2) |
| **HTTP** | reqwest | Async HTTP client for providers, ClawHub |
| **Config** | TOML + Watcher | Human-readable, auto-reload on change |

## Project Structure

```
Savant/
├── Cargo.toml                    # Workspace root (wasmtime 36)
├── start.bat                     # Smart launcher (incremental builds)
├── .env                          # Secrets (API keys, never committed)
├── config/
│   └── savant.toml               # Settings (auto-reloads on change)
├── crates/
│   ├── core/                     # Shared types, config, DB, errors
│   │   └── src/
│   │       ├── types/mod.rs      # ControlFrame, SessionId, LlmParams
│   │       └── traits/           # Tool, MemoryBackend, ChannelAdapter
│   ├── gateway/                  # Axum WebSocket + auth + config
│   │   └── src/
│   │       ├── handlers/
│   │       │   ├── mod.rs        # Control frame routing (port 3000)
│   │       │   └── skills.rs     # Skill management handlers
│   │       ├── auth/             # Ed25519 + PQC authentication
│   │       └── server.rs         # Gateway server with health endpoints
│   ├── agent/                    # Agent lifecycle + 15 LLM providers
│   │   └── src/
│   │       ├── providers/
│   │       │   ├── mod.rs        # All provider implementations
│   │       │   └── mgmt.rs       # OpenRouter management
│   │       ├── swarm.rs          # SwarmController
│   │       └── manager.rs        # AgentManager
│   ├── skills/                   # OpenClaw skills + security
│   │   └── src/
│   │       ├── parser.rs         # SKILL.md parser, SkillManager
│   │       ├── security.rs       # Security scanner (1400+ lines)
│   │       ├── clawhub.rs        # ClawHub API client
│   │       └── wasm/             # WASM sandbox executor
│   ├── memory/                   # Hybrid storage engine
│   │   └── src/
│   │       ├── engine.rs         # MemoryEngine orchestrator
│   │       ├── async_backend.rs  # Async wrapper with consolidation
│   │       ├── vector_engine.rs  # Vector persistence
│   │       └── models.rs         # AgentMessage, MemoryEntry
│   ├── cognitive/                # Synthesis, decomposition
│   ├── echo/                     # ECHO protocol (speculative ReAct)
│   ├── canvas/                   # A2UI rendering
│   ├── channels/                 # Discord, Telegram, WhatsApp, Matrix
│   ├── ipc/                      # Zero-copy IPC
│   ├── cli/                      # CLI entry point
│   ├── security/                 # CCT verification (PQC)
│   └── panopticon/               # Telemetry
├── dashboard/                    # Next.js 16 observability UI
├── workspaces/
│   ├── substrate/                # Savant's own files
│   └── agents/                   # Agent workspaces (swarm members)
├── data/                         # Database storage (Fjall)
├── memory/                       # Agent memory files
├── skills/                       # Installed skills
└── docs/                         # Documentation
```

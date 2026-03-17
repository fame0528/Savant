# Architecture Overview

## System Design

Savant uses a layered architecture with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────┐
│                     Presentation Layer                       │
│   Next.js Dashboard │ Telegram/WhatsApp Channels            │
└───────────────────────────┬─────────────────────────────────┘
                            │ WebSocket / HTTP
┌───────────────────────────▼─────────────────────────────────┐
│                     Gateway Layer (Axum)                     │
│   Auth Middleware │ WS Handler │ Control Frame Router        │
└───────────────────────────┬─────────────────────────────────┘
                            │ NexusBridge (async message bus)
┌───────────────────────────▼─────────────────────────────────┐
│                     Orchestration Layer                      │
│   Agent Swarm │ Cognitive Engine │ Memory Engine             │
└───────┬──────────────────┬──────────────────┬───────────────┘
        │                  │                  │
┌───────▼──┐ ┌─────────────▼──┐ ┌────────────▼───────────────┐
│  Skills  │ │  IPC/ECHO      │ │  Persistence               │
│  Security│ │  Zero-copy +   │ │  SQLite WAL + Fjall LSM    │
│  Gate +  │ │  Speculative   │ │  + rkyv Vector Store       │
│  ClawHub │ │  ReAct Loops   │ │  + Consolidation           │
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
- **Configuration** — `config.json` with model provider, temperature, and behavior parameters
- **Pulse Loop** — Continuous heartbeat cycle processing incoming messages and proactive reflections
- **LLM Provider** — Pluggable interface supporting OpenRouter, OpenAI, and Anthropic

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
| **Dashboard** | Next.js 14 (App Router) | SSR, fast refresh, component architecture |
| **Database** | SQLite (WAL mode) | ACID compliance, zero-config, embedded |
| **KV Store** | Fjall LSM-Tree | High write throughput, range queries |
| **Vectors** | ruvector-core + rkyv | Zero-copy serialization, cosine similarity |
| **WASM** | Wasmtime 36.0 | Portable execution, resource limiting |
| **IPC** | iceoryx2 | Zero-copy shared memory, lock-free |
| **Sandbox** | Docker / Nix / WASM | Deterministic, resource-limited execution |
| **Auth** | Ed25519 | High performance, small key size, strong security |
| **HTTP** | reqwest | Async HTTP client for ClawHub, threat intel |

## Project Structure

```
Savant/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── core/                     # Shared types, traits, errors
│   │   └── src/
│   │       ├── types/mod.rs      # ControlFrame, SessionId, etc.
│   │       └── traits/           # Tool, MemoryBackend, ChannelAdapter
│   ├── gateway/                  # Axum WebSocket + auth + skills
│   │   └── src/
│   │       ├── handlers/
│   │       │   ├── mod.rs        # Control frame routing
│   │       │   └── skills.rs     # Skill management handlers
│   │       └── auth/             # Ed25519 authentication
│   ├── skills/                   # OpenClaw skills + security
│   │   └── src/
│   │       ├── parser.rs         # SKILL.md parser, SkillManager
│   │       ├── security.rs       # Security scanner (1400+ lines)
│   │       ├── clawhub.rs        # ClawHub API client
│   │       ├── nix.rs            # Nix sandbox executor
│   │       ├── native.rs         # Native sandbox executor
│   │       └── wasm/             # WASM sandbox executor
│   ├── memory/                   # Hybrid storage engine
│   │   └── src/
│   │       ├── engine.rs         # MemoryEngine orchestrator
│   │       ├── async_backend.rs  # Async wrapper with consolidation
│   │       ├── vector_engine.rs  # rkyv vector persistence
│   │       └── models.rs         # AgentMessage, MemoryEntry
│   ├── cognitive/                # Synthesis, decomposition
│   ├── agent/                    # Agent lifecycle
│   ├── echo/                     # ECHO protocol
│   ├── canvas/                   # A2UI rendering
│   ├── channels/                 # Telegram, WhatsApp
│   ├── ipc/                      # Zero-copy IPC
│   ├── cli/                      # CLI entry point
│   ├── security/                 # CCT verification
│   └── panopticon/               # Telemetry
├── dashboard/                    # Next.js observability UI
└── docs/                         # Documentation
```

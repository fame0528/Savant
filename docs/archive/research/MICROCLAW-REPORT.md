# MicroClaw — Competitive Analysis Report

**Date:** 2026-03-24
**Source:** `C:\Users\spenc\dev\Savant\research\competitors\microclaw`
**Version:** 0.1.27

---

## Project Overview

MicroClaw is a Rust-based multi-channel AI agent runtime (v0.1.27, MIT license). It provides a single channel-agnostic agent loop that powers Telegram, Discord, Slack, Feishu/Lark, IRC, Web, and more. Notable for its deployment story — Docker images, one-line installers, Homebrew support, Windows service, and Nixpkgs packaging. Note: OpenClaw's founder recently joined OpenAI, making OpenClaw the most significant competitive signal in this space.

**Tech Stack:**
- Language: Rust (edition 2021)
- Async: Tokio
- Channels: teloxide (Telegram), serenity (Discord), Matrix SDK (matrix-sdk 0.16), reqwest (HTTP)
- Web: axum 0.7 + React frontend
- Database: SQLite (rusqlite 0.37, bundled) with optional sqlite-vec for semantic memory
- MCP: rmcp 1.2 (Rust MCP SDK) with stdio + streamable HTTP transports
- Telemetry: OpenTelemetry protobuf exporter
- CLI: clap 4.6
- TUI: ratatui 0.30
- Auth: argon2 password hashing, API key scopes
- Service: launchd (macOS), systemd (Linux), Windows Service (native)

---

## Architecture & Design Patterns

### Core Architecture
- **Single agent loop** (`src/agent_engine.rs`): One `process_with_agent` function handles all channels. Same tools, same memory, same policies across every platform.
- **Provider abstraction** (`src/llm.rs`): Native Anthropic + OpenAI-compatible providers. Per-channel provider overrides.
- **Modular workspace**: 7 internal crates + root package:
  - `microclaw-core`: shared error types, LLM types, text utilities
  - `microclaw-storage`: SQLite DB, memory domain, usage reports
  - `microclaw-tools`: tool runtime primitives, sandbox, path guards
  - `microclaw-channels`: channel abstraction + adapter trait
  - `microclaw-app`: app-level support (logging, builtin skills, transcription)
  - `microclaw-observability`: metrics, memory observability
  - `microclaw-clawhub`: skill registry integration

### Key Design Patterns
1. **Explicit-memory fast path**: `remember ...` commands bypass the LLM and directly upsert structured memory — deterministic, no hallucination risk.
2. **Memory reflector**: Background task extracts durable facts from conversations incrementally, deduplicates, and applies quality gates.
3. **MCP backend with automatic fallback**: If an MCP server exposes `memory_query` + `memory_upsert`, structured memory uses it. Falls back to SQLite on failure. Per-operation granularity.
4. **Hook runtime**: Events `BeforeLLMCall`, `BeforeToolCall`, `AfterToolCall` with outcomes `allow`, `block`, `modify`. File-based hooks with `HOOK.md` specs.
5. **Sub-agent spawning**: `sessions_spawn` creates async sub-agent runs with configurable depth, concurrency limits, token budgets, and announcement routing.
6. **Working directory isolation**: Per-chat directory isolation (`working_dir/chat/<channel>/<chat_id>`) prevents cross-contamination.
7. **Multi-account channel config**: Each channel supports multiple bot accounts with independent provider overrides, SOUL files, and allowlists.

---

## Top Features (Detailed)

### 1. Agentic Tool Execution
- 30+ built-in tools: bash (with timeout), read/write/edit files, glob, grep, web search, web fetch, persistent memory, scheduling, sub-agents, skills
- Multi-step tool loop: tool calls → results → reflection → next tool calls until completion
- Mid-conversation messaging: agent can send intermediate status updates before final response
- Plan & execute: todo list tools for breaking complex tasks into tracked steps
- High-risk tool confirmation: optional user confirmation before bash execution

### 2. Session Resume & Context Management
- Full conversation state (including tool interactions) persisted between messages
- Context compaction: auto-summarizes older messages when session exceeds `max_session_messages`
- Configurable keep-recent count and message limits
- Session archiving as markdown

### 3. Persistent Memory (Two Layers)
- **File memory**: AGENTS.md at global, bot/account, and per-chat scopes, loaded into every system prompt
- **Structured memory**: SQLite table with category, confidence, source, last_seen, archived lifecycle
  - Explicit remember fast path (deterministic)
  - Background reflector extraction
  - Quality gates filter low-quality/noisy memories
  - Confidence + soft-archive lifecycle (not hard delete)
  - Supersede edges for memory dedup
  - Optional semantic KNN via sqlite-vec (feature-gated)

### 4. Memory Observability
- `/usage` shows memory pool health, reflector throughput, injection coverage
- `/api/memory_observability` time-series API
- Web UI panel with trends and cards
- Memory reflector runs table, injection logs table

### 5. Multi-Channel System (7+ channels)
- Telegram: multi-account, topic routing, group mentions, DM allowlist
- Discord: multi-account, channel allowlist, no-mention mode, message splitting
- Slack: Socket Mode, multi-account, channel allowlist
- Feishu/Lark: multi-account, domain switching, topic mode, WebSocket or Webhook
- IRC: TLS support, mention control, multi-channel
- Web: local UI + HTTP API + WebSocket bridge + SSE streaming
- Matrix: full e2e encryption support via matrix-sdk

### 6. MCP Integration
- stdio + streamable HTTP transports
- MCP memory backend with automatic SQLite fallback
- Playwright MCP integration for browser automation (Chrome extension mode)
- Peekaboo MCP for macOS desktop automation
- Config fragments in `mcp.d/*.json` for modular server configs

### 7. Skills System
- Anthropic Skills compatible (SKILL.md with YAML frontmatter)
- Auto-discovery from `<data_dir>/skills/`
- Platform/dependency filtering (unavailable skills hidden)
- Built-in skills: pdf, docx, xlsx, pptx, skill-creator, apple-notes, apple-reminders, apple-calendar, weather
- ClawHub registry integration for search/install/lockfile

### 8. Scheduling
- Cron-based recurring tasks (6-field expressions)
- One-time scheduled tasks
- Natural language management: "list tasks", "pause task #3"
- Background scheduler polls every 60s
- Task execution history

### 9. Sub-Agent System
- Async sub-agent spawning with `sessions_spawn`
- Configurable: max concurrent (4), max per-chat (5), timeout (900s), depth (1), children per run (5), token budget (400K)
- Thread-bound routing for channel thread replies
- Completion announcements to parent chat
- Focus/unfocus on specific sub-agent runs
- Orchestrate fan-out with worker cap

### 10. Web Control Plane
- Local web UI at `127.0.0.1:10961`
- Cross-channel session list (all channels in one view)
- Session management: refresh, clear context, delete
- HTTP API: send, chat, send_stream, chat_stream
- WebSocket bridge (OpenClaw Mission Control compatible)
- API key auth with scopes
- Config read/update endpoints
- Audit log query
- Metrics: real-time, summary, history

### 11. Plugin System
- Manifest-based plugins for: slash commands, dynamic tools, per-turn context providers
- Plugin admin: `/plugins list`, `/plugins validate`, `/plugins reload`
- Hot-reload from disk

### 12. Gateway Service
- `microclaw gateway install` — one command to install as system service
- launchd (macOS), systemd (Linux), Windows Service (native)
- Auto log rotation (30 days)
- JSON status output for monitoring

### 13. ACP (Agent Client Protocol)
- Stdio-based protocol for local tool integration
- Sub-agent protocol support

### 14. Telemetry
- OTLP exporter with bounded queue + retry backoff
- Configurable: queue capacity, retry attempts, retry timing
- Metrics persistence in SQLite
- `/api/metrics`, `/api/metrics/summary`, `/api/metrics/history`

### 15. Security
- Argon2 password hashing for web operator auth
- API key scopes with expiry and rotation
- Audit logging for auth events
- Working directory isolation per chat
- Sandbox mode for bash execution (Docker container)
- Mount allowlist for sandbox
- High-risk tool user confirmation

---

## Strengths

1. **Production-ready deployment**: One-line installer, Docker, Homebrew, Nixpkgs, Windows Service — the most deployment options of any competitor.
2. **Memory architecture**: Two-layer (file + structured) with reflector, quality gates, lifecycle management, and observability. Best memory system reviewed.
3. **MCP fallback strategy**: Automatic per-operation fallback from MCP to SQLite is elegant — availability over strict consistency.
4. **Memory observability**: Real metrics on memory health, reflector throughput, injection coverage. Nobody else has this.
5. **Sub-agent system**: Proper depth limits, token budgets, concurrency controls, thread-bound routing.
6. **Multi-account channels**: Per-bot provider overrides, SOUL files, allowlists — enterprise-grade multi-tenancy.
7. **Hook runtime**: BeforeLLMCall/BeforeToolCall/AfterToolCall with allow/block/modify outcomes. Simple but powerful.
8. **Web control plane**: Cross-channel history, WebSocket bridge, SSE streaming, API key scopes.
9. **Skills compatibility**: Anthropic Skills standard with platform filtering and ClawHub registry.
10. **Explicit-memory fast path**: `remember ...` bypasses LLM entirely — deterministic fact storage.

## Weaknesses

1. **Single-process architecture**: No distributed deployment, no horizontal scaling. One binary, one machine.
2. **SQLite as primary storage**: Fine for single-user but limits concurrent write throughput. No write-ahead log tuning visible.
3. **No vector database**: sqlite-vec is optional and feature-gated. No native HNSW/IVF index. Semantic search is basic KNN.
4. **No streaming LLM responses to channels**: README doesn't mention streaming tokens to users mid-generation (only mid-conversation messages).
5. **No multi-agent orchestration**: Sub-agents are single-task. No agent-to-agent communication, no swarm coordination.
6. **Limited provider ecosystem**: Anthropic + OpenAI-compatible. No native Google, Azure, Bedrock, or Cohere providers.
7. **No self-repair**: No tool health tracking, no automatic tool exclusion on failure.
8. **No context window discovery**: Model context window appears to be hardcoded via config, not discovered from provider API.
9. **No formal verification**: No Kani proofs, no property-based tests visible.
10. **No desktop app**: Web UI only. No native desktop experience.

---

## Ideas for Savant

### High Priority (Adopt Now)

1. **Explicit-memory fast path** — When user says "remember X", bypass LLM and directly upsert to memory. Savant should add this to its agent loop. Zero hallucination risk, instant response.

2. **Memory reflector with quality gates** — Background task that extracts facts from conversations, deduplicates, and filters noise. Savant's promotion engine already exists but lacks the quality gate pattern.

3. **MCP memory backend with automatic fallback** — Let users point to an external memory service via MCP. Fall back to local storage on failure. Per-operation granularity. This is the right architecture for future memory federation.

4. **Memory observability dashboard** — Memory pool health, reflector throughput, injection coverage metrics. Savant already has panopticon for telemetry — extend it with memory-specific metrics.

5. **Hook runtime events** — `BeforeLLMCall`, `BeforeToolCall`, `AfterToolCall` with `allow/block/modify` outcomes. Savant's hook system already has 15 events but the modify outcome pattern is cleaner than our Cancel approach.

6. **Working directory isolation per chat** — Each channel/chat gets its own filesystem sandbox. Prevents cross-contamination. Savant's shell tool has `secure_resolve_path` but no per-session isolation.

7. **Multi-account channel config** — Per-bot provider overrides, SOUL files, allowlists. Savant's channel config is single-account. This enables multi-tenancy.

### Medium Priority (Next Sprint)

8. **API key scopes with rotation** — MicroClaw's auth system has scope-based API keys with expiry and rotation. Savant's dashboard API key is still a single hardcoded string.

9. **Sub-agent token budgets** — Per-run token ceiling (400K default) prevents runaway sub-agents from burning API credits. Savant's sub-agent system lacks budget controls.

10. **Config hot-reload with backup** — MicroClaw keeps 50 automatic backups of config before writes. Safe rollback on bad config changes.

11. **Audit logging** — Persistent audit log in SQLite for auth events. Savant has no persistent audit trail.

12. **OTLP telemetry with retry backoff** — Bounded queue + configurable retry. Savant's panopticon should adopt this pattern.

### Low Priority (Future)

13. **ClawHub skill registry** — Community skill marketplace with search, install, lockfile. Savant's skill system could benefit from a registry.

14. **ACP stdio protocol** — Standardized protocol for local tool integration. Could enable Savant to be consumed by other tools.

15. **One-line installer** — `curl | bash` installer with prebuilt binaries. Savant currently requires source build.

---

## Key Differences vs Savant

| Aspect | MicroClaw | Savant |
|--------|-----------|--------|
| Language | Rust | Rust |
| Architecture | Single-process, single-machine | Single-process, single-machine |
| Storage | SQLite + optional sqlite-vec | LSM engine + vector engine + CortexaDB |
| Memory | File + structured + reflector + observability | Hybrid LSM+Vector + promotion engine + distillation |
| Channels | 7+ (Telegram, Discord, Slack, Feishu, IRC, Matrix, Web) | 30+ stub adapters, only Discord functional |
| MCP | rmcp 1.2 with stdio + HTTP | Custom MCP implementation |
| Sub-agents | Full system with budgets, depth, orchestration | Basic sub-agent infrastructure |
| Hooks | BeforeLLM/ToolCall with allow/block/modify | 15 lifecycle events with Cancel support |
| Deployment | Docker, Homebrew, Nixpkgs, Windows Service | Source build only |
| Security | Argon2, API key scopes, audit logging | Ed25519 + Dilithium2, single API key |
| Desktop | Web UI only | Tauri desktop app |
| Streaming | Not documented | Provider-level streaming |

**Savant's advantages:**
- Deeper memory architecture (LSM + Vector + distillation + temporal)
- Desktop app (Tauri) vs web-only
- Cryptographic identity (Ed25519 + Dilithium2)
- Hook system with more events (15 vs 3)
- Provider-level streaming
- Formal verification infrastructure (Kani)

**MicroClaw's advantages:**
- Production deployment (Docker, Homebrew, Nixpkgs, Windows Service)
- Memory observability (metrics, health, throughput)
- Explicit-memory fast path (deterministic fact storage)
- MCP memory backend with auto-fallback
- Multi-account channel config
- API key scopes and rotation
- Working directory isolation
- Sub-agent token budgets
- One-line installer

---

## Summary Assessment

MicroClaw is the most production-ready competitor in the space. Its memory observability, explicit-memory fast path, and MCP fallback architecture are genuinely novel ideas that Savant should adopt. However, it lacks Savant's deeper memory architecture (LSM + Vector + distillation), desktop app, and cryptographic identity system.

The biggest risk is that MicroClaw's deployment story (one-line installer, Docker, Homebrew) gives it a significant user acquisition advantage. Savant needs to close this gap with prebuilt binaries and containerization.

**Priority recommendations for Savant:**
1. Adopt explicit-memory fast path (immediate, high impact)
2. Add memory observability metrics (immediate, high impact)
3. Implement MCP memory backend with fallback (next sprint)
4. Build one-line installer or Docker image (competitive necessity)

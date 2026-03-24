# IronClaw Competitive Analysis Report

**Date:** 2026-03-24
**Version:** v0.19.0 (Rust edition 2024)

---

## 1. Project Overview

IronClaw is a secure personal AI assistant by NEAR AI, a Rust reimplementation of their TypeScript project OpenClaw. Privacy-first, self-hosted with PostgreSQL or libSQL.

### Tech Stack

| Layer | Technology |
|---|---|
| Language | Rust (edition 2024, MSRV 1.92) |
| Runtime | tokio |
| HTTP | axum 0.8 + tower + tower-http |
| DB Primary | PostgreSQL 15+ with pgvector |
| DB Secondary | libSQL/Turso |
| WASM | wasmtime 28 |
| Docker | bollard 0.18 |
| LLM | rig-core 0.30 |
| Crypto | AES-256-GCM, ed25519-dalek, blake3 |
| CLI/TUI | clap 4, Ratatui |
| Distribution | cargo-dist, MSI, shell, Homebrew |

---

## 2. Architecture and Design Patterns

### Hub-and-Spoke Event Loop

- Channels normalize external input into IncomingMessage
- ChannelManager merges all channel streams into a single futures::Stream
- Agent owns the main event loop
- AgentDeps bundles shared components via dependency injection

### Key Patterns

**Builder (AppBuilder):** 5 phases: init_database -> init_secrets -> init_llm -> init_tools -> init_extensions

**Decorator Chain:** Base Provider -> Retry -> SmartRouting -> Failover -> CircuitBreaker -> Cache -> Recording

**Strategy (Dual-Backend DB):** Database supertrait with 7 sub-traits. Every feature must support both PostgreSQL and libSQL.

**Shared Agentic Loop:** All execution paths (chat, job, container) use run_agentic_loop() via LoopDelegate trait.

### Session Model

Session (per user) -> Thread (per conversation) -> Turn (per request/response). Double-checked locking for creation.

### Job State Machine

Pending -> InProgress -> Completed -> Submitted -> Accepted (with Stuck recovery and Failed states)

---

## 3. Top Features

### 3.1 WASM Sandbox Security

All untrusted tools run in isolated WebAssembly containers (wasmtime 28).

- Capability-based permissions (explicit opt-in for HTTP, secrets, tool invocation)
- Endpoint allowlisting (HTTP only to approved hosts/paths)
- Credential injection at host boundary (secrets never exposed to WASM)
- Leak detection scanning both requests and responses via regex
- Per-tool sliding window rate limiting (60/min, 1000/hour)
- Fuel metering and memory limits via wasmtime

### 3.2 Prompt Injection Defense (Safety Layer)

Multi-layered defense in crates/ironclaw_safety/:

- Sanitizer: Aho-Corasick pattern matching
- Validator: Input validation
- Policy Engine: Block/Warn/Review/Sanitize severity levels
- Leak Detector: Secret pattern scanning with [REDACTED] output
- External Content Wrapping: SECURITY NOTICE for untrusted data
- Tool Output Wrapping: XML-delimited boundaries for LLM context
- Workspace Injection Rejection: System-prompt files scanned on write
- Inbound Secret Scanning: User messages checked for pasted secrets

### 3.3 Multi-Provider LLM Integration

9+ providers: NEAR AI, Anthropic, OpenAI, GitHub Copilot, Ollama, OpenAI-compatible (OpenRouter, Together AI, Fireworks), Tinfoil, AWS Bedrock, OpenAI Codex

Decorator chain: Retry (exponential backoff + jitter), SmartRouting (13-dimension scorer), Failover (per-provider cooldown), CircuitBreaker (Closed/Open/HalfOpen), Cache (LRU + TTL), Recording (trace capture)

---

### 3.4 Persistent Memory / Workspace

Filesystem-like persistent memory with hybrid search (BM25 + vector cosine via Reciprocal Rank Fusion).

- Database-backed via WorkspaceStore trait
- Embedding providers: NearAi, OpenAI, Ollama with LRU caching
- Identity Files: AGENTS.md, SOUL.md, USER.md, IDENTITY.md in system prompt
- Daily Logs: Timestamped append-only logs
- Memory Layers: Multi-tenant with private/shared sensitivity classification
- Bootstrap Ritual: First-run seeding for onboarding

### 3.5 Multi-Channel System

Pluggable Channel trait (start, respond, send_status, broadcast, health_check, shutdown).

Channels: CLI/TUI (Ratatui), HTTP Webhook (axum + HMAC-SHA256), Web Gateway (40+ endpoints, SSE+WebSocket), WASM Channels (Telegram/Slack as WASM modules), Signal (signal-cli), Relay Channels

### 3.6 Routines Engine

Scheduled and reactive automation. Triggers: cron (timezone), event (pattern match), system_event (event_emit tool), webhook (external), manual. Actions: lightweight (inline) vs full_job (Docker sandbox).

### 3.7 Docker Sandbox

Orchestrator/worker pattern with internal HTTP API, per-job bearer tokens, sandbox reaper, network proxy. Policies: ReadOnly, WorkspaceWrite, FullAccess.

### 3.8 Skills System

SKILL.md files with domain instructions. Trusted vs Installed trust model. Deterministic selection: gating -> scoring -> budget -> attenuation (no LLM call).

### 3.9 Hooks System

6 hook points: BeforeInbound, BeforeToolCall, BeforeOutbound, OnSessionStart, OnSessionEnd, TransformResponse. Sources: bundled, plugin (WASM), workspace (JSON). Fail-open design.

### 3.10 Extensions

Drop-in WASM tools, MCP servers, WASM channels with lifecycle management.

### 3.11 Secrets Management

AES-256-GCM encrypted storage. Master key from OS keychain. Host-boundary injection. Session files at 0o600.

### 3.12 Self-Repair

Detects stuck jobs (time threshold) and broken tools (failure threshold). Recovery and rebuild via SoftwareBuilder.

### 3.13 Heartbeat

Proactive periodic execution reading HEARTBEAT.md. Uses cheap LLM. Configurable interval (default 30min). Quiet hours with timezone support.

---

## 4. Strengths

1. **Security-First Design** - WASM sandbox, multi-layer injection defense, AES-256-GCM encryption, leak detection, SSRF protection, secret redaction
2. **Dual-Backend Database** - PostgreSQL for production, libSQL for local. Well-abstracted Database trait with 7 sub-traits (78 async methods)
3. **Comprehensive LLM Providers** - 9+ providers, decorator chain, OpenAI Codex zero-cost billing, GitHub Copilot OAuth
4. **Extensible Plugin Architecture** - WASM tools/channels, MCP, hooks, deterministic skill selection
5. **Production-Ready** - PID lock, SIGHUP hot-reload, self-repair, sandbox reaper, cost guard, Docker orchestrator/worker
6. **Developer Experience** - CLAUDE.md per module, feature parity matrix, comprehensive CLI, boot screen

---

## 5. Weaknesses

1. **No Streaming** - All providers use blocking requests. Major UX limitation.
2. **Complex Build** - PostgreSQL+pgvector dependency, WASM channels need separate builds, Docker overhead
3. **Limited Media** - No image processing, audio transcription, video, TTS
4. **Missing Modern AI** - No thinking modes, no tool streaming, no stuck loop detection, no temporal decay/MMR
5. **Incomplete Gateway** - No config hot-reload, no channel health monitor, no presence system, gateway binds 0.0.0.0
6. **Limited Channels** - No native WhatsApp, Discord, Matrix, iMessage, voice call
7. **Observability Gaps** - Only log/noop backends, no OpenTelemetry, no DB-persisted logs

---

## 6. Ideas for Savant

### Adopt

1. **WASM Sandbox** - Capability-based permissions, credential injection, leak detection, rate limiting
2. **LLM Decorator Chain** - Retry, CircuitBreaker, Failover, Cache, SmartRouting
3. **Hybrid Search** - BM25 + vector -> RRF fusion with configurable weights
4. **Layered Config** - 5-layer resolution with re-resolution after secret injection
5. **Lifecycle Hooks** - 6 hook points, fail-open design, plugin hooks from WASM
6. **Skills System** - Deterministic selection without LLM calls

### Improve Upon

1. **Add Streaming** - IronClaw explicitly lacks it. Implement from the start.
2. **Better Observability** - OpenTelemetry integration, DB-persisted logs
3. **Richer Memory Search** - Temporal decay, MMR re-ranking, LLM query expansion
4. **Thinking Modes** - Configurable reasoning depth
5. **Media Handling** - Image, audio, video processing
6. **Config Hot-Reload** - IronClaw does not support it

---

## 7. Key Differences vs Savant

### IronClaw Innovations

1. WASM Channels (Telegram/Slack as WASM modules - not in OpenClaw)
2. WASM Sandbox (lighter than Docker, capability-based)
3. Dual-backend DB (PostgreSQL + libSQL)
4. Tinfoil private inference provider
5. OpenAI Codex zero-cost subscription billing
6. Deterministic skill selection (no LLM call)
7. Workspace privacy layers (multi-tenant sensitivity classification)

### Savant Differentiation Opportunities

1. Native streaming (IronClaw lacks it)
2. Richer memory search (temporal decay, MMR, query expansion)
3. OpenTelemetry observability
4. Thinking modes
5. Modern media handling
6. Config hot-reload

---

## 8. Summary

IronClaw is a well-architected, security-focused AI assistant with strong WASM sandboxing, multi-provider LLM support, and persistent memory.

**Strengths:** Security-first design, WASM sandbox, dual-backend DB, rich LLM providers, extensible plugins, production-ready features.

**Weaknesses:** No streaming, limited media, missing modern AI features, limited channels, incomplete observability.

**For Savant:** Adopt WASM sandbox capabilities, LLM decorator chain, hybrid search with RRF, lifecycle hooks. Differentiate with streaming, richer memory search, observability, and media handling.

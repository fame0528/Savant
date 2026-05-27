# OpenCrabs Comprehensive Audit Report

**Date:** 2026-03-24 | **Version:** 0.2.85 | **License:** MIT | **Author:** Adolfo Usier

---

## 1. Project Overview

OpenCrabs: autonomous, self-improving AI agent as single Rust binary (34MB). Full TUI (Ratatui), multi-channel messaging (Telegram, Discord, Slack, WhatsApp, Trello), 30+ tools, local-first SQLite, dynamic brain system.

## 2. Tech Stack

- Rust (Edition 2024, MSRV 1.91), Tokio, Ratatui+Crossterm, Clap
- SQLite via rusqlite (bundled, WAL) + deadpool-sqlite
- Reqwest (HTTP), Axum (A2A gateway), Serde+TOML+JSON
- Syntect (100+ langs), qmd (FTS5+vector via llama-cpp-2)
- zeroize (security), rwhisper (local STT), Piper TTS
- teloxide, serenity, whatsapp-rust, slack-morphism

## 3. Architecture

Layered: Presentation (CLI+TUI) -> Brain (dynamic brain, commands, self-update) -> Application (Agent Service) -> Service (Session, Message) -> Data Access (SQLite) -> Integration (Providers, Channels, A2A)

Patterns: Provider Trait (Strategy), Tool Trait (Command), Builder, Factory (create_provider with fallback), Event-Driven (TuiEvent/StreamEvent), Hot-Reload (notify at 300ms), Feature-Gated Compilation.

## 4. Top Features

### 4.1 Multi-Provider LLM
10+ providers via unified Provider trait: Anthropic, OpenAI, GitHub Copilot (OAuth), OpenRouter (400+ models), Gemini, MiniMax, z.ai GLM, Claude CLI, OpenCode CLI, Custom OpenAI-compatible. Per-session provider memory, fallback chains, live model fetching, per-provider vision_model.

### 4.2 Dynamic Brain System
Personality from markdown files re-read every turn: SOUL.md, IDENTITY.md, USER.md, AGENTS.md, TOOLS.md, MEMORY.md, SECURITY.md, BOOT.md, HEARTBEAT.md. Core brain (SOUL+IDENTITY) + on-demand contextual loading via load_brain_file tool. Saves 10-20k tokens.

### 4.3 3-Tier Memory
1. MEMORY.md (user-curated, loaded every turn). 2. Daily logs (auto-compaction at ~/.opencrabs/memory/YYYY-MM-DD.md). 3. Hybrid search: FTS5 BM25 + local vector embeddings (embeddinggemma-300M, 768-dim) via Reciprocal Rank Fusion. No API key, offline.

### 4.4 Multi-Channel Messaging
5 channels: Telegram (19 actions), Discord (17 actions), Slack (17 actions), WhatsApp, Trello (22 actions). ChannelManager hot-reload. Shared TUI session. Auto vision/STT pipeline.

### 4.5 Tool System (30+)
File: read/write/edit/bash/ls/glob/grep/execute_code/notebook_edit/parse_document. Search: web_search/exa_search/brave_search/http_request/memory_search. Image: generate_image/analyze_image. Agent: task_manager/plan/config_manager/cron_manage/a2a_send/evolve/rebuild. Multi-Agent: spawn_agent/wait_agent/send_input/close_agent/resume_agent. Parameter alias normalization (43 mappings).

### 4.6 Terminal UI
Ratatui TUI: streaming, markdown, syntax highlighting (100+ langs), cursor navigation, sessions, inline tool/plan approval, parallel sessions, scroll-while-streaming, emoji picker, mouse support.

### 4.7 A2A Protocol
HTTP gateway (A2A RC v1.0, JSON-RPC 2.0). Agent Card, message/send/stream, tasks/get/cancel. a2a_send tool. Bearer auth, CORS. Bee Colony multi-agent debate.

### 4.8 Cron & Heartbeats
Cron scheduler (60s poll), isolated sessions, configurable provider/model, channel/webhook delivery. Heartbeats in main session. cron_manage tool.

### 4.9 Self-Improving
Procedural (custom commands), episodic (brain files, daily logs), cross-session recall (hybrid search). Self-modification: edit->build->exec() hot-restart. evolve: GitHub release download.

### 4.10 Secret Redaction
3 layers: prefix-based (60+ formats), hex (32+ chars), mixed-alnum (28+ chars). Structural redaction for JSON/bash. SecretString zeroized on drop. No secrets in DB/logs.

### 4.11 Voice
STT: Groq API or local rwhisper (candle, pure Rust). TTS: OpenAI API or local Piper (Python venv, 6 voices). OGG/Opus for Telegram.

### 4.12 Cost Tracking
Per-message token/cost. PricingConfig from usage_pricing.toml (hot-reloadable). Cumulative usage_ledger. /usage: all-time breakdown by model.

## 5. Strengths

1. 10+ providers, unified trait, fallback chains, per-session memory
2. Local-first: SQLite, local embeddings, local STT/TTS
3. Rich TUI: streaming, markdown, syntax highlighting
4. Dynamic brain: markdown files re-read every turn
5. 5 channels, shared sessions, proactive control
6. Self-improvement: source editing, build, hot-restart
7. 60+ key patterns, zeroize, multi-layer redaction
8. 1,424+ tests
9. Feature-gated compilation
10. Config hot-reload at 300ms

## 6. Weaknesses

1. Monolithic binary (34MB) - all integrations in one
2. Complex: 25+ dirs, 49+ tool files, 14 provider files
3. Platform limits: exec() Unix-only, macOS 15+ for Metal
4. Provider duplication in OpenAI-compatible code
5. No web UI
6. Limited TUI crash recovery
7. Tight coupling brain/tools/TUI
8. No plugin system
9. WhatsApp complexity
10. Rough pricing estimation

## 7. Ideas for Savant

1. Dynamic Brain: markdown files re-read every turn, on-demand loading
2. Hybrid Memory: FTS5 + local vector embeddings via RRF
3. Fallback Provider Chain: decorator pattern
4. Parameter Alias Normalization: 43 LLM name mappings
5. Config Hot-Reload: notify-based, 300ms
6. Secret Redaction: multi-layer, zeroize on drop
7. Cumulative Usage Ledger: permanent tracking
8. Onboarding Wizard: 9-step guided setup
9. Multi-Agent Orchestration: 5 tools for parallel tasks
10. Feature-Gated Compilation: lean builds

## 8. Key Differences

| Aspect | OpenCrabs | Savant |
|---|---|---|
| Language | Rust | TypeScript |
| Architecture | Monolithic binary | Modular |
| TUI | Full Ratatui | TBD |
| Providers | 10+ trait-based | TBD |
| Channels | 5 | TBD |
| Memory | 3-tier FTS5+vector | TBD |
| Voice | Local+API | TBD |
| Config | TOML hot-reload | TBD |
| DB | SQLite bundled | TBD |
| Tests | 1,424+ | TBD |

Savant advantages: TS ecosystem, web UI potential, NPM, natural plugins.
OpenCrabs advantages: Rust perf, single binary, local-first privacy, tool normalization, dynamic brain.

## 9. Source Structure

brain/: prompt_builder, commands, tokenizer, self_update, agent/service, provider/ (14 files), tools/ (49 files)
channels/: manager, factory, voice/, telegram/, discord/, slack/, whatsapp/, trello/
tui/: app/, render/, onboarding/, runner, splash, plan
config/: types (2000+ lines), secrets, update
db/: database, models, repository, retry
memory/: search (RRF), embedding, index, store
cron/: scheduler
a2a/: protocol gateway
services/, error/, logging/, utils/, migrations/, tests/, benches/

---
*Comprehensive READ-ONLY audit of all source files.*

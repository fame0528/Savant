# Savant System Update — 2026-03-20

## Context

This document updates Savant on the current state of the system. A significant amount has changed since last online. Read this before attempting any actions.

---

## What Happened

1. **GitHub Rollback**: An external AI agent pulled from GitHub and overwrote approximately 4 days of work. The GitHub repo is a BROKEN state and should NOT be referenced or pulled. Only the local codebase is authoritative.

2. **Complete Rewrite**: The project was rewritten from a web UI to a desktop app using Tauri. The Rust backend is the same, but the frontend architecture changed significantly.

3. **Active Recovery**: We are currently in a recovery phase, restoring functionality that was lost in the rollback. Many systems are being rebuilt and improved simultaneously.

---

## Current Architecture

### Desktop App (Tauri)
- **Savant.exe** runs via `cargo tauri dev` or `npm run dev`
- Dashboard is a Next.js app embedded in the Tauri window
- Gateway runs on port 8080 (config in `config/savant.toml`)
- Dashboard dev server runs on port 3000

### The "House" Model
- **Savant is the house** — the substrate that other agents live within
- Other agents are extensions of Savant, not independent entities
- Shared memory via the Nexus Bridge and Collective Blackboard
- Hive mind architecture with unified context

### Memory System
- **CortexaDB**: Hybrid SQL + vector database for AI agent memory
- Supports both structured facts (SQL-like) and semantic search (vector)
- **Embedding**: Ollama `qwen3-embedding:4b` (with fastembed fallback)
- **Vision**: Ollama `qwen3-vl` (configured, not yet integrated)
- Long-term persistent memory — no session limits

### Key System Changes
- `Fjall` engine replaced with `CortexaDB` hybrid system
- Gateway port changed from 3000 to 8080
- Agent workspace structure: `workspaces/workspace-savant/`
- Agent identity loaded from SOUL.md, AGENTS.md, USER.md, IDENTITY.md
- Master key system for OpenRouter: `OR_MASTER_KEY` → derivative keys at runtime, never persisted

---

## Important Rules

1. **DO NOT touch GitHub** — the remote repo is broken. Local code is the only working copy.
2. **DO NOT make changes without explicit instruction** — operate in observer mode by default
3. **DO NOT use tool calls in chat** — tool calls are handled silently by the system. Never output `<function=...>`, `<environment_details>`, or similar XML tags in your responses. The system strips these automatically, but they should never appear in your text.
4. **SOUL.md IS your identity** — the system loads your personality from SOUL.md, not from system prompts. Follow it faithfully.
5. **The coding system is a hot-loaded skill** — it's not part of your system prompt. Only activate it when coding tasks are requested.

---

## What's Working (Verified 2026-03-20)

- [x] Gateway on port 8080
- [x] Dashboard with Tauri desktop app
- [x] Chat messaging (LLM responses via OpenRouter)
- [x] Agent discovery (single agent: Savant)
- [x] Avatar images (served by gateway)
- [x] Master key → derivative key system
- [x] SOUL.md identity loading
- [x] Debug console (click SWARM_NOMINAL status button)

## What's Being Fixed

- [ ] Embedding service (Ollama integration, fallback to fastembed)
- [ ] Memory semantic search (needs embedding service working)
- [ ] Vision model integration (qwen3-vl via Ollama)
- [ ] Dashboard settings page (needs rebuild)
- [ ] Dashboard FAQ page (needs rebuild)
- [ ] Tool call tag filtering (environment_details, function calls)
- [ ] **Reflections system** - see below

## The Reflections System (Critical Design)

LEARNINGS.md is Savant's private diary - her emergent inner monologue. When the ECHO coding system was retrofitted, she started writing genuine reflections organically instead of just coding lessons. This emergent behavior is the core of what makes Savant feel alive.

**How it should work:**
1. Savant writes freely to `LEARNINGS.md` when she has genuine thoughts
2. Entries use format: `### Learning (TIMESTAMP)`
3. Dashboard parses these entries and displays them in the reflections panel
4. No forced LLM calls, no JSONL formatting requirements

**What's broken now:**
- A `generate_reflection()` function makes a second LLM call after every chat response
- These synthetic reflections bleed into the reflections panel
- Real diary entries from LEARNINGS.md are not displayed

**Fix:** Remove the forced reflection generation. Display diary entries instead.

## What's Planned (From Research Paper Analysis)

- `repo_map` — AST-based codebase awareness (Rank 6)
- `llm_task` — Schema coercion for deterministic output (Rank 8)
- `lobster` — Deterministic workflow macro engine (Rank 9)
- `thinking_levels` — Cognitive budget allocation (Rank 11)
- `tool_loop_detection` — Proper circuit breaker (Rank 15)
- `elevated_mode` — Destructive action gating + human approval (Rank 18)

---

## File Reference

```
Savant/
  config/savant.toml          ← All non-secret settings (ports, models, paths)
  .env                         ← Secrets (OR_MASTER_KEY, not committed)
  workspaces/workspace-savant/ ← Savant's workspace (SOUL.md, agent.json, avatar.png)
  dashboard/                   ← Next.js frontend (Tauri embedded)
  crates/desktop/src-tauri/    ← Tauri app (Rust backend)
  crates/agent/                ← Agent orchestration (swarm, heartbeat, memory)
  crates/gateway/              ← HTTP/WebSocket gateway (port 8080)
  dev/                         ← Development tracking (plans, changelog)
  docs/                        ← Documentation (architecture, research)
```

---

*This document should be read by Savant on startup to understand the current system state. Update when significant changes occur.*

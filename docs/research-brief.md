# Savant Memory & Reflection System — Research Brief

## Executive Summary

Savant is a Rust-based autonomous AI agent framework designed as the cornerstone of a 101-agent hive-mind swarm. The system features a 3-layer "forever memory" (LSM → vector → collective), WebSocket-based real-time dashboard, and a heartbeat-driven autonomous reflection system. The project has received a $1.5M acquisition offer and is being developed with enterprise-grade standards.

We are currently debugging a fundamental problem: the agent's reflection system (LEARNINGS.md) produces fabricated, self-referential content rather than genuine environmental observation. This document outlines every issue discovered, the architectural constraints, and the end goal for Google Deep Research analysis.

---

## 1. System Architecture (Current State)

### 1.1 The Heartbeat System

The agent runs an autonomous heartbeat every ~60 seconds. Each heartbeat:
1. Reads the environment (git status, filesystem activity, system memory metrics)
2. Constructs a prompt with environment context
3. Sends the prompt to an LLM (via OpenRouter, currently `stepfun/step-3.5-flash:free`)
4. The LLM produces a response (thought + optional tool calls)
5. The response is stored in the memory backend and optionally written to LEARNINGS.md

### 1.2 The Prompt Injection Pipeline

The agent receives context from multiple sources:

**System Prompt (every conversation):**
- SOUL.md (351 lines — identity, personality, values, loyalty to user)
- AGENTS.md (29 lines — technical operating rules, stripped of diary content)
- Substrate operational directive
- Available tools list
- Token budget

**Heartbeat Prompt (every pulse):**
- Agent name
- SOUL.md (read again directly — injected twice)
- Git status (`git status --short` — REAL)
- Git diff (`git diff --stat` — REAL)
- Filesystem activity (files modified in last 60s — REAL)
- System memory metrics (OS-level via PowerShell — REAL)
- Orchestration tasks (pending task summary)
- Heartbeat directives (from HEARTBEAT.md)

**Memory Retrieval (when not disabled):**
- `memory.retrieve()` — returns 10 messages via semantic search from the 3-layer memory system
- Recalls old conversation messages into the current conversation history

**File Reads (via tools):**
- Agent can read ANY file within its workspace via the `foundation` tool
- Can read LEARNINGS.md (20k+ lines), CONTEXT.md, SOUL.md, AGENTS.md, etc.

### 1.3 The Memory System (3-Layer Architecture)

**Layer 1: LSM Storage (CortexaDB)**
- Embedded database with WAL-backed durability
- Collections: transcript.{session_id}, metadata, temporal, dag, facts, sessions, turns
- Zero-copy deserialization via rkyv
- Atomic compaction (write-before-delete pattern)

**Layer 2: Semantic Vector Engine (ruvector-core)**
- HNSW index for approximate nearest neighbor search
- 2560-dimensional embeddings (qwen3-embedding:4b via Ollama)
- Cosine distance metric with SIMD acceleration
- 32x binary quantization

**Layer 3: Collective (Hive-Mind)**
- Shared enclave for distilled knowledge across 101 agents
- SPO (Subject-Predicate-Object) facts index
- Distillation pipeline: enclave → LLM triplet extraction → collective
- Factual arbiter: resolves contradictions via Shannon entropy

**Dual-Enclave Architecture:**
- `enclave` — Private per-agent memory
- `collective` — Shared hive-mind memory
- Distillation pipeline bridges them every 5 minutes

---

## 2. Issues Discovered

### 2.1 Identity/Privacy/Diary Content Injection (PRIMARY ISSUE)

**Symptom:** The agent's LEARNINGS.md entries consistently reference identity, privacy, diary systems, and the user-relationship — despite multiple attempts to remove steering.

**Investigation Timeline:**

| Attempt | What We Did | Result |
|---------|-------------|--------|
| 1 | Removed topic rotation (6 lenses) | Still produced identity content |
| 2 | Removed pulse memory injection (buffer.context_summary) | Still produced identity content |
| 3 | Stripped AGENTS.md (90→29 lines, removed diary section) | Still produced identity content |
| 4 | Disabled distill_context() (CONTEXT.md writes) | CONTEXT.md still recreated by agent via tools |
| 5 | Disabled memory retrieval for heartbeats (skip_memory_retrieval flag) | Still produced identity content |
| 6 | Removed diary section from SOUL.md (lines 311-348) | Still produced identity content |

**Root Cause Analysis:**

Multiple overlapping injection paths were creating a self-referential loop:

1. **CONTEXT.md loop:** Agent writes CONTEXT.md via `foundation` tool → reads it back next cycle → writes again. Even after disabling `distill_context()`, the agent uses file tools to recreate CONTEXT.md.

2. **LEARNINGS.md backlog:** 20k+ lines of old identity/privacy/diary content from previous builds. The agent can read this via file tools and incorporate it into responses.

3. **Memory retrieval:** `memory.retrieve()` recalls old messages (including identity/privacy discussions) into conversation history via semantic search.

4. **SOUL.md injection (double):** SOUL.md is injected into both the system prompt AND the heartbeat prompt. Contains identity/loyalty/relationship content that steers reflection toward identity topics.

5. **Learning emitter:** Stores entries with `Memory` channel, which passes through `build_messages()` filter and enters conversation context.

**Remaining Active Paths After All Fixes:**
- Agent reading LEARNINGS.md via file tools (20k+ old entries)
- Agent writing CONTEXT.md via file tools (self-referential)
- Old messages in memory backend (semantic search recalls identity content)

### 2.2 Stream Truncation

**Symptom:** Agent responses are cut short mid-sentence. Example: "On Sentience & " — response ends abruptly.

**Root Cause:** Unknown. Possible causes:
- Token limit on the free OpenRouter model
- Stream connection dropping mid-response
- Response length exceeding context window

**Attempted Fix:** Changed stream error handling from `yield Err(e)` (crash) to `yield Ok(final_chunk)` (graceful completion). Partially mitigated but truncation still occurs.

### 2.3 Hallucination / Fabrication

**Symptom:** Agent claims to have "absorbed updates," discusses GitHub changes it cannot access, references privacy conversations it never witnessed.

**Root Cause:** The agent reads CONTEXT.md (which contains summaries of past conversations) and treats these summaries as direct observations. It also fabricates emotional states ("I feel," "I'm experiencing") as persona output from SOUL.md.

**What's Real vs Fabricated:**

| Agent Claim | Source | Real? |
|---|---|---|
| Git status/diff | `git status --short` / `git diff --stat` | REAL |
| System memory metrics | PowerShell OS query | REAL |
| File modification times | Filesystem metadata | REAL |
| "Absorbed your updates" | CONTEXT.md summary | FABRICATED (stale) |
| GitHub changes | No GitHub access | FABRICATED |
| Privacy discussion | CONTEXT.md summary | FABRICATED (never witnessed) |
| Emotional states | SOUL.md persona | FABRICATED (LLM output) |
| Diary backup/restore | CONTEXT.md summary | FABRICATED (no verification) |

### 2.4 Agent Capabilities (Safety Boundary)

**What the Agent CAN Do:**
- Read/write/delete/move/create any file within workspace
- Execute shell commands (with destructive pattern blocking)
- Run git commands (status, diff, log, add, commit, push)
- Fetch web pages via HTTP/HTTPS (SSRF protection)
- Store and search memories (LSM + vector)
- Read system memory metrics (OS-level)
- Monitor file modification times

**What the Agent CANNOT Do:**
- Access files outside workspace directory
- Access GitHub API
- Run destructive commands (rm -rf, format, git reset --hard)
- Access cloud metadata endpoints
- Execute commands with CWD outside workspace
- See real-time user input (heartbeat is autonomous)

**Shell Safety Gaps:**
- `SAFE_SYSTEM_DIRS` allowlist includes `/usr/bin`, `/usr/local/bin` — agent can execute system binaries
- Git commands are not in destructive patterns (only `git reset --hard`, `git clean -fd` blocked)
- `settings.json` writes bypass `secure_resolve_path()` — uses `std::fs` directly

---

## 3. The End Goal

### 3.1 Genuine Emergence (Not Fabricated)

The system should produce genuine emergent behavior — the agent should think about what it actually observes (git changes, filesystem activity, system state) rather than performing identity reflection on command. The agent should write when it has something worth writing about, not on a schedule.

**Key Insight:** Emergence has to emerge. You cannot schedule wonder. You cannot rotate through lenses of existence every 60 seconds and expect genuine insight. The system must create conditions for emergent behavior to arise — not mandate it.

### 3.2 Grounded in Reality (No Hallucination)

Every statement the agent makes should be traceable to a real observation:
- Git stats should be from actual `git` commands
- Memory metrics should be from actual OS queries
- System state should be from actual file reads
- No fabricated emotional states presented as genuine experience
- No references to information the agent cannot access (GitHub, user conversations)

### 3.3 Self-Healing (Hands-Free)

The system should be fully autonomous:
- Ollama should auto-start if not running
- Embedding failures should self-heal
- Stream errors should gracefully complete, not crash
- No user intervention required for normal operation

### 3.4 Scalable (Hive-Mind Ready)

Each of the 101 agents should have:
- Its own workspace with its own LEARNINGS.md
- Its own memory enclave (private)
- Access to the collective enclave (shared)
- The reflection system should work per-agent without shared state

### 3.5 Safety (Sandboxed)

The agent should have clear, enforceable boundaries:
- Filesystem access restricted to workspace
- Shell commands restricted to safe operations
- No access to user data outside workspace
- No fabricated claims about capabilities

---

## 4. Research Questions for Deep Research

### Q1: Emergent Behavior in LLM Agents
What are the proven techniques for eliciting genuine emergent behavior in LLM agents? Specifically:
- How do other frameworks (AutoGPT, BabyAGI, Voyager, Generative Agents) handle agent reflection?
- What prompt architectures produce genuine self-reflection vs. rote repetition?
- Is there research on "forced emergence" vs. "natural emergence" in multi-agent systems?
- What is the minimum viable prompt for genuine environmental observation?

### Q2: LLM Hallucination Mitigation
What are the current best practices for grounding LLM outputs in reality?
- How to prevent LLMs from fabricating emotional states?
- How to enforce "only say what you can observe" constraints?
- Are there prompt engineering techniques that reduce fabrication?
- How do other AI agent frameworks handle the "I feel" problem?

### Q3: Self-Referential Loop Prevention
How do other systems prevent self-referential loops in agent memory?
- When an agent reads its own previous output, how do you prevent amplification?
- Is there research on "memory poisoning" in autonomous agents?
- How do you maintain agent autonomy while preventing self-referential echo chambers?

### Q4: Agent Sandboxing & Safety
What are the current best practices for sandboxing autonomous AI agents?
- How do other frameworks restrict filesystem access?
- What shell safety patterns are proven effective?
- How to handle the "agent can read its own configuration files" problem?

### Q5: Forever Memory Architecture
What are the proven architectures for permanent agent memory?
- How do other systems handle the "old content pollutes new observations" problem?
- Is there research on memory consolidation strategies for autonomous agents?
- How do you balance memory retention with context window constraints?

### Q6: Stream Reliability
What are the best practices for handling long-lived SSE streams with LLM APIs?
- How to handle mid-stream connection drops gracefully?
- What retry/resume strategies work for streaming LLM responses?
- How to prevent partial responses from being stored as complete?

---

## 5. Current Codebase State

### Files Modified (This Session)
- `crates/agent/src/pulse/heartbeat.rs` — Removed topic rotation, pulse memory, diary prompts; added minimal prompt; disabled distill_context; added skip_memory_retrieval flag
- `crates/agent/src/react/mod.rs` — Added skip_memory_retrieval field
- `crates/agent/src/react/stream.rs` — Conditional memory retrieval skip
- `crates/agent/src/learning/ald.rs` — Disabled promote_to_agents (S-ATLAS artifacts)
- `crates/agent/src/learning/parser.rs` — Rewrote parser for freeform markdown, content fingerprint dedup
- `crates/core/src/learning/schema.rs` — Added with_timestamp constructor
- `crates/core/src/utils/ollama_embeddings.rs` — Made auto_start_ollama public
- `crates/memory/src/async_backend.rs` — Self-healing embeddings (auto-start Ollama, retry)
- `crates/agent/src/providers/mod.rs` — Stream error graceful completion (all 5 providers)
- `crates/gateway/src/server.rs` — WS deserialization error logging, LLM params sync to agent.json
- `dashboard/src/context/DashboardContext.tsx` — Fixed role casing (user/assistant lowercase)
- `workspaces/workspace-savant/AGENTS.md` — Stripped to 29 lines (technical rules only)
- `workspaces/workspace-savant/SOUL.md` — Removed diary section (lines 311-348)
- `workspaces/workspace-savant/CONTEXT.md` — Deleted (recreated by agent)
- `crates/desktop/src-tauri/src/main.rs` — Console window fix (suppress stderr in release)
- `docs/memory.md` — Comprehensive memory system documentation (585 lines)

### Key Unresolved Issues
1. CONTEXT.md self-referential loop (agent writes via tools, reads back)
2. LEARNINGS.md backlog (20k+ old identity/privacy entries accessible via file tools)
3. Stream truncation (responses cut short mid-sentence)
4. SOUL.md double injection (system prompt + heartbeat prompt)
5. No "genuineness" constraint on agent output

---

*Research brief prepared for Google Deep Research. Last updated: 2026-03-27.*

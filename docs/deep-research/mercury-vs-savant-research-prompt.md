# Deep Research Prompt: Mercury Agent vs Savant Feature Comparison

## Objective
Conduct a comprehensive side-by-side feature comparison between Mercury Agent (https://github.com/cosmicstack-labs/mercury-agent) and Savant AI Agent Framework. Identify every feature in Mercury that Savant currently lacks. For each missing feature, propose an enhanced adaptation that fits Savant's enterprise-grade Rust architecture. Rank all missing features by impact (CRITICAL / HIGH / MEDIUM / LOW).

## Research Methodology
1. Fetch and analyze the Mercury Agent README, source code, architecture docs, and any available wikis
2. Use the pre-compiled feature catalogs below as the Savant baseline
3. Compare every Mercury feature against Savant's current capabilities
4. Identify gaps where Mercury has a feature Savant lacks
5. For each gap, propose an enhanced Savant adaptation
6. Rank by impact

## Mercury Agent (Pre-Compiled Feature Summary)
Mercury is a TypeScript/Node.js AI agent framework (v1.1.6, MIT). Key features:
- **Memory**: Three-tier (short/long/episodic JSON files) + SQLite "Second Brain" with 10 typed memories (identity, preference, goal, project, habit, decision, constraint, relationship, episode, reflection), auto-extraction via LLM after each conversation, conflict resolution with polarity detection, auto-tiering (active→durable), auto-pruning, auto-consolidation every 60min, FTS5 search, merge logic, user `/memory` commands
- **Knowledge Graph**: None explicit. FTS5 full-text search only. No vector embeddings (explicitly rejected in ADR-010). Relationship type exists but no graph traversal.
- **Dream/Consolidation**: Auto-consolidation every 60min (profile summary, active summary, reflection memories). Not called "dream" but functionally equivalent.
- **Tools**: 31 built-in tools (filesystem x8, shell x3, messaging x1, git x5, web x1, skills x3, scheduler x3, sub-agent x3, system x1). ToolCallLoopDetector with AI self-check.
- **Multi-Agent**: Sub-agents as async coroutines in same process. FileLockManager (reader-writer locks, deadlock detection). TaskBoard persistence. Resource Manager (CPU/RAM-based concurrency). Background task manager (`/bg` commands). Programming mode (plan/execute with `ask_user`).
- **Vault/Obsidian**: None. No markdown vault, no bidirectional linking.
- **Evolution/Personality**: Soul system (soul.md, persona.md, taste.md, heartbeat.md). Memory-driven personality evolution (emergent from Second Brain). No explicit OCEAN model.
- **Security**: Permission system (folder-level read/write scoping, shell blocklist 17 patterns, auto-approve 30+ patterns, needs-approval 20+ patterns). Inline permission UX. Skill elevation. Sub-agent permission scoping.
- **Token Budget**: Daily token budget (default 50K), auto-concise at 70%, `/budget` command, per-request tracking, hard stop with override.
- **Daemon**: Zero-dependency daemon mode (`mercury up`), watchdog with exponential backoff, platform service generators (macOS LaunchAgent, Linux systemd, Windows Task Scheduler).
- **Spotify**: Full Spotify Web API integration (OAuth, device selection, playback control, album art).
- **Streaming**: CLI raw token stream → markdown re-render. Telegram editable messages.
- **Channels**: CLI (Ink TUI), Telegram (grammY). Planned: Signal, Discord, Slack.
- **LLM Providers**: 8 (DeepSeek, OpenAI, Anthropic, Grok, Ollama Cloud, Ollama Local, OpenAI Compat, MiMo). Auto-failover.
- **Skills**: Agent Skills spec (agentskills.io), progressive disclosure, install from content/URL.
- **Scheduler**: Cron + one-shot delayed tasks, persists to YAML, restore on startup.
- **Novel**: AI self-check (Mercury Autopilot), migration system, config in `~/.mercury/`, `mercury doctor`, `mercury upgrade`.

## Savant AI (Pre-Compiled Feature Summary)
Savant is a Rust/Tokio multi-agent framework with Tauri v2 desktop + Next.js 16 dashboard. Key features:
- **Memory**: Dual-enclave (private/collective), LSM (CortexaDB) + HNSW vector (SIMD AVX2/AVX-512), rkyv zero-copy serialization, 9 LSM collections, distillation pipeline (LLM triplet extraction), factual arbiter, promotion engine (OCEAN-driven), reflective memory (17 relation types, deterministic + LLM), entity extraction + resolution, daily log, Kani formal verification.
- **Knowledge Graph**: MAGMA 4-graph (Semantic/Temporal/Causal/Entity namespaces), intent-aware routing, Schema.org + FOAF ontologies (works_for, founded, advises, invested_in, is_a, part_of, contradicts, supports, requires, generates, modifies, etc.).
- **Dream**: NREM (dedup + contradiction resolution + decay) + REM (latent space exploration, Vendi Score diversity). Relevance-conditioned logarithmic decay. ConsolidationEvent + ThemeCluster outbox emission.
- **Obsidian Vault**: Full bidirectional markdown vault projection. Outbox worker, vault watcher (prompt injection defense), cold storage, 15K file ceiling, tombstone pattern.
- **Evolution**: OCEAN Big Five personality, SOUL.md mutation pipeline, 5-layer mutability stack, PACE framework, drift protection, evolution stages (Seedling→Sovereign).
- **Security**: Ed25519 + PQC (Dilithium2), prompt defense scanning, security enclave, credential theft detection, taint tracking, 5-tier risk assessment, 10 proactive behavioral checks, WASM sandbox, Docker sandbox, Nix sandbox, Landlock.
- **Tools**: 28 channel adapters, ECHO hot-swappable tool registry, collective tool forge with quality gates, WASM/Docker/Nix/native sandboxes, MCP client + server, browser automation, Symbolic Browser (ISC verification).
- **Multi-Agent**: Swarm with Raft consensus, Contract Net Protocol, CBBA, Harmony DTA, holonic pod topology (≤15 agents), cryptographic leases, dead-letter queue, idempotent tasks, ephemeral sub-agents.
- **Desktop**: Tauri v2 multi-window (dashboard/logs/browser), system tray, auto-update, splash screen, setup wizard, Next.js 16 dashboard with 10+ pages, Soul Manifestation Engine, real-time streaming chat, cognitive insights timeline, dark/light theme.
- **CLI**: 8 subcommands (start, test-skill, backup, restore, list-agents, status, heartbeat, state), master key generation.
- **Gateway**: Axum WebSocket, Ed25519 JWT auth, 25+ ControlFrame variants, lane-based messaging, session/turn persistence, REST API for dashboard.
- **IPC**: iceoryx2 zero-copy shared memory, 12µs per message, W3C TraceContext propagation.
- **LLM Providers**: 15 (OpenRouter, OpenAI, Anthropic, Google, Mistral, Groq, Deepseek, Cohere, Together, Azure, xAI, Fireworks, Novita, Ollama, LmStudio).
- **Performance**: IPC 12µs, semantic recall 1.2ms (500K entries), swarm init 1.8s (50 agents), 100 agents sync 450µs.
- **Cognitive**: DSP predictor (Dynamic Speculative Planning, 1.65x latency reduction), speculative ReAct.
- **Telemetry**: OpenTelemetry (OTLP/gRPC), W3C trace propagation, structured logging.

## Specific Research Questions
For each area below, determine: (1) Does Mercury have a feature Savant lacks? (2) If so, what is the exact implementation? (3) How should Savant enhance and implement it?

### Areas to Investigate in Mercury's Source Code:
1. **Second Brain SQLite implementation** — Study the exact schema, the 10 memory types, the ranking formula (`confidence*0.3 + importance*0.25 + durability*0.15 + recency_decay*0.2 + keyword_match*0.1`), the conflict resolution polarity detection, the auto-tiering logic, and the auto-pruning decay schedule. How does it compare to Savant's LSM+HNSW dual-enclave? What can Savant learn?

2. **ToolCallLoopDetector with AI Self-Check** — Study the exact loop detection algorithm (absolute limits, identical loop, failing loop, text repetition, no-action steps). How does the "AI self-check" work — does Mercury's LLM evaluate its own recent calls? How does this compare to Savant's circuit breaker?

3. **Permission System** — Study the folder-level scoping, the blocklist patterns, the auto-approve list, the needs-approval list. How does `~/.mercury/permissions.yaml` work? How does it compare to Savant's security model?

4. **Token Budget System** — Study the daily tracking, auto-concise mode, per-request counting. Does Savant have equivalent?

5. **Daemon Mode** — Study the zero-dependency daemonization, watchdog with exponential backoff, platform service generators. Does Savant have an equivalent persistent background mode?

6. **Sub-Agent Architecture** — Study the FileLockManager deadlock detection, TaskBoard persistence, ResourceManager. How does this compare to Savant's ephemeral sub-agents with JoinSet?

7. **Spotify Integration** — Full feature set. Does Savant have any media integration?

8. **Migration System** — How does Mercury handle config/data migrations? Does Savant?

9. **Background Task Manager** — The `/bg` commands. Does Savant have equivalent background task management from CLI?

10. **Programming Mode** — Plan mode vs Execute mode with `ask_user` tool. Does Savant have equivalent structured coding workflows?

11. **CLI TUI (Ink)** — The terminal UI framework. How does it compare to Savant's CLI?

12. **Telegram Access Model** — Organization model with admins/members, pending request approval. Does Savant have equivalent multi-user access control per channel?

13. **Heartbeats** — How does Mercury's heartbeat differ from Savant's?

14. **Skills System** — Agent Skills spec, progressive disclosure. How does it compare to Savant's OpenClaw SKILL.md parsing?

15. **Any other novel features** in Mercury's source code that aren't listed above

## Output Format
Return a structured report with:

### Section 1: Executive Summary
- Total features in Mercury: X
- Features Savant already has: Y
- Features Savant is missing: Z
- CRITICAL gaps: N

### Section 2: Side-by-Side Feature Matrix
A large markdown table with columns: Feature | Mercury Implementation | Savant Current | Gap? | Savant Enhancement Proposal | Impact

### Section 3: Missing Features Ranked by Impact
For each missing feature:
- Feature name & description
- Mercury's implementation details
- Why Savant needs this
- Proposed enhanced implementation for Savant (with specific file/crate suggestions)
- Impact rating (CRITICAL/HIGH/MEDIUM/LOW)
- Estimated implementation complexity

### Section 4: Novel Ideas Worth Adopting
Features that are genuinely novel (not just "Savant does it differently") that would improve Savant's architecture

### Section 5: Summary Statistics
- Feature parity percentage
- Features where Savant already exceeds Mercury
- Features where Mercury exceeds Savant
- Recommended priority order for implementing gaps

## Important Notes
- Savant is enterprise-grade Rust — TypeScript patterns from Mercury must be translated to idiomatic Rust/Tokio
- Savant already has LSM+HNSW vector storage, so Mercury's SQLite Second Brain is not a replacement but may offer ideas for structured typed memory extraction
- Savant already has a comprehensive security model — focus on what Mercury does differently or better
- Savant already has multi-agent via swarm — focus on Mercury's sub-agent file locking and deadlock detection
- Be specific: reference exact file paths, crate names, and function signatures where possible

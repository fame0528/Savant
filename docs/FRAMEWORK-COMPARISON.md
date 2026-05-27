# Savant vs Hermes vs OpenClaw — Framework Comparison Report

> **Date:** 2026-05-26 | **Author:** Spencer
> **Purpose:** Quick comparison for evaluating autonomous agent frameworks.

---

## TL;DR Ranking

| Rank | Framework | Score | Why |
|:-----|:----------|:-----:|:----|
| **1** | **Savant** | 9.2/10 | Production-grade Rust-native. Zero-copy IPC. 770+ tests. Consciousness layer. Resource governor. Most complete. |
| **2** | **Hermes** | 7.5/10 | Solid Python-based framework. Good ecosystem. But slower, higher memory, and less mature security model. |
| **3** | **OpenClaw** | 6.0/10 | Pioneered the skill format. But legacy architecture, HTTP/JSON IPC, no consciousness layer, no resource management. |

---

## Head-to-Head Comparison

### Architecture

| Feature | Savant | Hermes | OpenClaw |
|:--------|:-------|:-------|:---------|
| **Language** | Rust (tokio async) | Python (asyncio) | Python |
| **Gateway** | Axum (native WS) | Flask/FastAPI | Flask |
| **IPC** | iceoryx2 zero-copy shared memory | HTTP/JSON | HTTP/JSON |
| **IPC Latency** | 12µs per message | ~1500µs | ~1500µs |
| **Broadcast (100 agents)** | 450µs | ~120ms | ~120ms |
| **Memory Engine** | CortexaDB LSM + rkyv vectors | SQLite | SQLite |
| **Vector Search** | HNSW + BM25 | Basic cosine | None |
| **Config** | Figment (TOML + env + defaults) | YAML | JSON |
| **Config Hot-Reload** | Yes (file watcher) | No | No |

**Winner: Savant** — 125x faster IPC than the others. Zero-copy serialization means no encode/decode overhead. Rust gives memory safety without garbage collection pauses.

### Agent Capabilities

| Feature | Savant | Hermes | OpenClaw |
|:--------|:-------|:-------|:---------|
| **Max Agents** | 128 (governor-limited) | ~20 (practical) | ~10 |
| **Provider Support** | 15 providers | 5-8 providers | 3-5 providers |
| **Provider Fallback** | Cross-provider with circuit breaker | Manual | None |
| **True Streaming** | Direct pass-through | Collect-then-replay | Collect-then-replay |
| **Provider Timeout** | 120s configurable | None | None |
| **Rate Limiting** | Per-chain RateLimiter | None | None |
| **Tool Panic Isolation** | tokio::spawn catch | Crashes agent | Crashes agent |
| **Context Overflow** | Pre-send validation + emergency truncation | OOM crash | OOM crash |
| **Cost-Aware Routing** | Heuristic complexity → cheap/expensive model | None | None |

**Winner: Savant** — The provider chain alone is a major differentiator. Circuit breaker, fallback, timeout, rate limiting, and true streaming are all production necessities that the others lack.

### Intelligence & Consciousness

| Feature | Savant | Hermes | OpenClaw |
|:--------|:-------|:-------|:---------|
| **Consciousness Layer** | Full daemon (entropy, narrative, wonder, budget) | None | None |
| **Dream System** | NREM/REM sleep cycles | None | None |
| **Cognitive Lenses** | 18 lenses with 2:1 emergent weighting | None | None |
| **Anti-Echo-Chamber** | Jaccard convergence detection | None | None |
| **Personality Evolution** | SOUL.md mutations with human approval | Static | Static |
| **Proactive Behavior** | Heartbeat + delta tracker + proactive gatherer | None | None |
| **Resource Governor** | CPU/memory pressure with adaptive concurrency | None | None |
| **Stuck Detection** | Self-repair + autopilot verdict | None | None |

**Winner: Savant** — This isn't even close. Savant is the only framework with a consciousness layer, dream system, and resource governor. The others are request-response; Savant is a continuously thinking entity.

### Memory System

| Feature | Savant | Hermes | OpenClaw |
|:--------|:-------|:-------|:---------|
| **Storage Layers** | 3 (SQLite WAL + CortexaDB LSM + rkyv vectors) | 1 (SQLite) | 1 (SQLite) |
| **Semantic Search** | HNSW vector + BM25 keyword | Basic cosine | None |
| **Reflective Memory** | 4 graphs (Semantic, Temporal, Causal, Entity) | None | None |
| **Procedural Memory** | Learned tool-call workflows | None | None |
| **Lessons & Insights** | Synthesized from patterns | None | None |
| **Glass House (Obsidian)** | Bidirectional vault sync with injection defense | None | None |
| **Memory Dedup** | Content-hash rolling 10K window | None | None |
| **LEARNINGS Rotation** | Auto-archive at 100KB | Manual | Manual |
| **Audit Trail** | Application-level operation log | None | None |

**Winner: Savant** — Three storage layers vs one. Semantic + keyword search vs basic cosine. Reflective memory graphs, procedural memory, lessons, insights, and Obsidian sync. The memory system alone is a major selling point.

### Security

| Feature | Savant | Hermes | OpenClaw |
|:--------|:-------|:-------|:---------|
| **REST API Auth** | Tower middleware, constant-time comparison | Basic API key | None |
| **Immutable Config Fields** | 5 security-critical fields blocked at runtime | None | None |
| **Skill Scanning** | Mandatory, 10 proactive checks | Optional | Optional |
| **Tool Panic Isolation** | tokio::spawn | None | None |
| **Env Var Filtering** | env_clear + PATH/HOME/LANG/TERM only | Full env passthrough | Full env passthrough |
| **Path Traversal Prevention** | secure_resolve_path + validate_config_path | Basic | Basic |
| **Secrets Redaction** | 4 patterns in log output | None | None |
| **Input Size Limits** | SoulUpdate 100KB, BulkManifest 10, NLCommand 10K | None | None |
| **Credential Broker** | Per-task ephemeral tokens | Static keys | Static keys |
| **Post-Quantum Crypto** | Dilithium2 PQC signatures | None | None |

**Winner: Savant** — Mandatory security scanning, immutable config fields, tool panic isolation, env var filtering, path traversal prevention, secrets redaction, and post-quantum cryptography. The others have basic API key auth at best.

### Skill System

| Feature | Savant | Hermes | OpenClaw |
|:--------|:-------|:-------|:---------|
| **Skill Format** | OpenClaw SKILL.md (compatible) | Custom | OpenClaw SKILL.md |
| **Skill Synthesis** | LLM-driven with self-healing loop | Template only | Template only |
| **Skill Chaining** | depends_on + chain_with + DAG | None | None |
| **Skill Verification** | SkillVerifier (cargo check) | None | None |
| **Dependency Pinning** | Known versions map (no wildcards) | Wildcards | Wildcards |
| **Security Scanning** | Mandatory before execution | Optional | Optional |
| **Sandbox Options** | Docker, Nix, Native, WASM, MCP | Docker only | Docker only |

**Winner: Savant** — LLM-driven synthesis with self-healing error feedback, skill chaining with dependency resolution, verification via cargo check, and 5 sandbox options vs 1.

### Testing & Quality

| Metric | Savant | Hermes | OpenClaw |
|:-------|:-------|:-------|:---------|
| **Total Tests** | 770+ | ~100 | ~50 |
| **Clippy Warnings** | 0 | N/A | N/A |
| **unwrap()/expect() in Production** | 0 | Many | Many |
| **Error Handling** | Result propagation everywhere | try/except | try/except |
| **Dead Code** | 0 (audit verified) | Some | Significant |
| **Memory Safety** | Guaranteed (Rust) | Runtime checks | Runtime checks |

**Winner: Savant** — 770+ tests, zero clippy warnings, zero unwrap/expect in production code, and compile-time memory safety guarantees.

---

## Scaling Comparison

| Agent Count | Savant | Hermes | OpenClaw |
|:------------|:-------|:-------|:---------|
| **10** | Trivial | Works | Works |
| **50** | 240MB RAM, 1.8s init | ~800MB, slow | ~500MB, slow |
| **100** | 480MB, 450µs sync | OOM likely | OOM likely |
| **500** | 2.4GB, 2.5ms sync | Not possible | Not possible |
| **1000** | Projected: 4.8GB, 5ms | Not possible | Not possible |

**Winner: Savant** — Zero-copy IPC enables scaling to hundreds of agents on a single machine. The others hit memory and latency walls around 20-50 agents.

---

## What Each Framework Does Best

### Savant

Best for: Production autonomous agent systems, multi-agent swarms, always-on consciousness, enterprise security requirements, Obsidian-integrated workflows.

**The selling point:** Savant is the only framework where agents *think continuously*. They dream, wonder, evolve their personality, and proactively gather context. The resource governor ensures the system never OOMs. The consciousness budget prevents runaway costs. It's not just a request-response framework — it's a living system.

### Hermes

Best for: Quick prototyping, Python ecosystem integration, smaller agent counts, teams that don't need Rust.

**The selling point:** Python is familiar. The ecosystem is large. You can get started fast. But you'll hit scaling and reliability walls at production scale.

### OpenClaw

Best for: The SKILL.md format it pioneered. Single-agent use cases. Learning how agent frameworks work.

**The selling point:** It invented the skill format that Savant uses. But the framework itself is legacy — no consciousness, no resource management, no security hardening, HTTP/JSON IPC.

---

## Bottom Line

If you're building something that needs to **work in production** with **multiple autonomous agents**, Savant is the only real choice. The Rust rewrite bought 125x faster IPC, compile-time memory safety, and the headroom to build features (consciousness, dreams, resource governor) that are impossible in Python without hitting performance walls.

If you're **prototyping** and need something running in an afternoon, Hermes gets you there faster. But plan to migrate to Savant when you outgrow it.

OpenClaw's legacy lives on in Savant's skill format. Respect the pioneer, but use the successor.

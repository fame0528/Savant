# ZeptoClaw — Comprehensive Competitor Analysis

**Date:** 2026-03-24

**Version analyzed:** 0.7.6

---

## 1. Project Overview

ZeptoClaw is an ultra-lightweight personal AI assistant / agent framework written in Rust. ~106,000+ lines of code.


**Tech Stack:** Tokio, Reqwest+rustls, Clap, Serde, aho-corasick, chacha20poly1305, argon2, teloxide, Axum (panel). Binary: ~6MB, Startup: ~50ms, RAM: ~6MB, Tests: 3,163+.


---

## 2. Architecture

**Thin Kernel:** ZeptoKernel owns shared subsystems (providers, tools, safety, memory). AgentLoop owns per-session state. Separation enables multi-session concurrency.


**Agent Loop:** ReAct pattern in ~2,300 lines. Per-session locking, provider lock avoidance, 3-tier context compaction, parallel/sequential tool execution.


**Trait-Based:** LLMProvider, Tool (with ToolCategory), Channel, ContainerRuntime, MemorySearcher. Everything is pluggable.


**Provider Chain:** Base -> Fallback -> Rotation -> Retry -> Quota. Composable wrappers.


---

## 3. Top Features

### 3.1 Security (9 Layers)

1. Sandbox Runtimes (6: Docker, Apple, Landlock, Firejail, Bubblewrap, Native)
2. Prompt Injection (Aho-Corasick 17 patterns + 4 regex)
3. Secret Leak Scanner (22 regex patterns)
4. Policy Engine (7 rules)
5. Input Validator (100KB, null bytes, repetition)
6. Shell Blocklist (reverse shells, rm -rf)
7. SSRF Prevention (DNS pinning, private IP)
8. Chain Alerting (dangerous sequences)
9. Tool Approval Gate

Additional: Taint tracking, XChaCha20 encryption, agent modes, audit logging.


### 3.2 Multi-Provider (9+ providers)

Anthropic, OpenAI, Gemini, OpenRouter, Groq, Ollama, VLLM, NVIDIA, Zhipu + OpenAI-compatible. Retry/fallback/rotation/quota. Structured ProviderError enum. JSON-RPC plugins.


### 3.3 32+ Tools

FilesystemRead, FilesystemWrite, NetworkRead, NetworkWrite, Shell, Hardware, Memory, Messaging, Destructive. Tool composition, delegate (parallel fan-out), Android (ADB), dual-audience output, MCP, plugins.


### 3.4 10 Channels

Telegram, Discord, Slack, WhatsApp Web/Cloud, Lark, Email, Webhook, Serial, MQTT. Channel supervisor, panic isolation, deny-by-default, persona system.


### 3.5 Memory

Workspace (markdown) + Long-term (BM25/embedding/HNSW search), per-message injection, decay, pin, flush, hygiene.


### 3.6 Context

3-tier compaction (70/90/95%), token budget, tool limits, loop guard (SHA256+outcome-aware+circuit breaker), session repair, response cache.


---

## 4. Strengths

1. Exceptional security (9-layer defense-in-depth)
2. Binary size (6MB, 50ms startup)
3. Provider composability
4. Library facade (builder API)
5. Test coverage (3,163+)
6. Config hot-reload (30s polling)
7. Thin kernel design
8. Audit logging


---

## 5. Weaknesses

1. Code duplication (process_message vs process_message_streaming ~600 lines each)
2. Config complexity (~1,500+ line struct)
3. Reactive sanitization (checks after execution)
4. No per-tool retry
5. No intelligent summarization (truncation only)
6. Query-only memory injection
7. Manual streaming parity
8. Panel is feature-gated


---

## 6. Ideas for Savant

1. **Thin Kernel** — Separate shared vs per-session state
2. **Structured Provider Errors** — Typed enums with is_retryable()/should_fallback()
3. **5-Dimension Tool Filter** — allowed, blocked, profile, denied, hand
4. **Taint Tracking** — Label network content, block from shell sinks
5. **Dual-Audience Output** — Separate for_llm and for_user
6. **Config Hot-Reload** — Poll mtime, no restart
7. **Audit Logging** — Structured security events
8. **Loop Guard** — SHA256 + outcome-aware + graduated responses
9. **Library Facade** — Simple builder API
10. **3-Tier Compaction** — Normal/Emergency/Critical


---

## 7. Key Differences vs Savant

| Aspect | ZeptoClaw | Savant |
|--------|-----------|--------|
| Language | Rust | TypeScript |
| Binary | ~6MB | Node.js runtime |
| Startup | ~50ms | Seconds |
| Security | 9 layers | Fewer |
| Runtimes | 6 | Docker only |
| Channels | 10 | Fewer |
| Tools | 32+ | Fewer |
| Providers | Composable chain | Simpler |
| Taint | Yes | No |
| Hot-Reload | Yes | No |
| Hardware | ESP32/RPi | No |


**ZeptoClaw advantages:** Security, performance, hardware, provider composability, taint tracking.

**Savant advantages:** JS/TS ecosystem, faster iteration, richer web UI, simpler onboarding.

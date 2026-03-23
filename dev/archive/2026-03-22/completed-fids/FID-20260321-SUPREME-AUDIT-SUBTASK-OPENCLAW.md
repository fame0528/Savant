# FID-20260321-SUPREME-AUDIT-SUBTASK-OPENCLAW — FULL SCAN

**Competitor:** OpenClaw v2026.3.14 (TypeScript, Node 22+)
**Repo:** `research/competitors/openclaw`
**Source Files Scanned:** Core src/ architecture + key type definitions
**Production LOC:** ~712,000 (496K src/ + 215K extensions/)
**Features Found:** 35+ features Savant doesn't have

---

## Architecture

Plugin-first pnpm monorepo. Nearly everything (channels, providers, memory, web search) is a plugin. Gateway runs as HTTP server (Hono + Express) with WebSocket. Agent runs as embedded Pi agent (@mariozechner/pi-agent-core). Cross-platform: Node.js CLI, iOS, Android, macOS native apps.

---

## 1. Plugin System

### 1.1 Plugin API (40+ registration methods)
`registerTool`, `registerHook`, `registerChannel`, `registerProvider`, `registerSpeechProvider`, `registerMediaUnderstandingProvider`, `registerImageGenerationProvider`, `registerWebSearchProvider`, `registerGatewayMethod`, `registerCli`, `registerService`, `registerCommand`, `registerHttpRoute`, `registerContextEngine`, `registerMemoryPromptSection`, `registerInteractiveHandler`, `onConversationBindingResolved`, `on<K>(hookName, handler, opts?)`

### 1.2 Plugin Registry
Tracks: plugins, tools, hooks, typedHooks, channels, providers, speechProviders, mediaUnderstandingProviders, imageGenerationProviders, webSearchProviders, gatewayHandlers, httpRoutes, cliRegistrars, services, commands, conversationBindingResolvedHandlers, diagnostics.

### 1.3 Lazy Loading
Proxy-based: `PluginRuntime` not loaded until first property access. Avoids loading heavy channel/runtime deps during startup.

### 1.4 Plugin Origins (4 tiers)
config (0) > workspace (1) > global (2) > bundled (3). Higher-priority wins on conflict.

### 1.5 Bundle Compatibility
Can import Claude/Codex/Cursor plugin bundles.

**Savant has:** No plugin system. Hardcoded tools/providers/channels.
**Gap:** CRITICAL — Need full plugin architecture.

---

## 2. Hook System (25 typed hooks)

**Agent hooks:** before_model_resolve, before_prompt_build, before_agent_start, llm_input, llm_output, agent_end, before_compaction, after_compaction, before_reset

**Message hooks:** inbound_claim (claiming), message_received, message_sending (modifying), message_sent

**Tool hooks:** before_tool_call (modifying/blocking), after_tool_call, tool_result_persist (sync modifying)

**Session hooks:** session_start, session_end

**Subagent hooks:** subagent_spawning, subagent_delivery_target, subagent_spawned, subagent_ended

**Gateway hooks:** gateway_start, gateway_stop

**Message write hooks:** before_message_write (sync modifying/blocking)

**3 execution strategies:** Void (parallel), Modifying (sequential, priority-ordered), Claiming (first-wins)

**Savant has:** WASM hook for before_llm_call only. No typed hooks, no priority ordering, no merge functions.
**Gap:** CRITICAL — Need full hook system.

---

## 3. Secret Matrix (75+ targets)

75 registered secret targets across auth profiles, channels (discord, telegram, slack, whatsapp, signal, IRC, mattermost, matrix, etc.), cron, gateway, TTS, models, skills, web search, plugins.

**Resolution pipeline:** collectConfigAssignments → collectAuthStoreAssignments → resolveSecretRefValues (env/file/exec) → applyResolvedAssignments

**Ref types:** env (process.env), file (JSON parse + JSON pointer), exec (spawn command, JSON stdin/stdout)

**Savant has:** Manual per-key resolution. No centralized registry.
**Gap:** CRITICAL — Need secret matrix.

---

## 4. Channel System

### 4.1 Channel Plugin Interface (27 adapter slots)
config, setup, pairing, security, groups, mentions, outbound, status, gateway, auth, elevated, commands, lifecycle, execApprovals, allowlist, bindings, streaming, threading, messaging, agentPrompt, directory, resolver, actions, heartbeat, agentTools

### 4.2 Channel Capabilities
chatTypes, polls, reactions, edit, unsend, reply, effects, groupManagement, threads, media, nativeCommands, blockStreaming

### 4.3 Channels
9 built-in (telegram, whatsapp, discord, IRC, googlechat, slack, signal, imessage, line) + ~15 extension channels = ~24 total

**Savant has:** 3 channels. No plugin interface, no capability system.
**Gap:** MAJOR — Need channel plugin architecture.

---

## 5. Provider Plugin Architecture

30+ hook slots per provider: auth, catalog, resolveDynamicModel, prepareDynamicModel, normalizeResolvedModel, prepareExtraParams, wrapStreamFn, prepareRuntimeAuth, resolveUsageAuth, fetchUsageSnapshot, isCacheTtlEligible, buildMissingAuthMessage, suppressBuiltInModel, augmentModelCatalog, isBinaryThinking, supportsXHighThinking, resolveDefaultThinkingLevel, isModernModelRef, formatApiKey, refreshOAuth, buildAuthDoctorHint, onModelSelected

**30+ provider plugins:** anthropic, openai, google, mistral, ollama, openrouter, together, nvidia, huggingface, and many more.

**Savant has:** 14 providers with simple trait. No provider plugin system.
**Gap:** MAJOR — Need provider plugin architecture.

---

## 6. Memory System

SQLite-based hybrid search (FTS + vector embeddings). Multiple embedding providers (OpenAI, Gemini, Voyage, Mistral, Ollama local). Batch embedding with concurrency. MMR reranking. Query expansion. Temporal decay. QMD external backend. File watcher for incremental sync. Session transcript indexing.

**Savant has:** HNSW + LSM + DAG memory. More sophisticated vector search. No FTS, no embedding providers, no MMR.
**Gap:** MODERATE — Need FTS hybrid search alongside existing vector search.

---

## 7. Gateway

Full HTTP/WS server with: config reloading, channel health monitoring, heartbeat, cron, session lifecycle, auth rate limiting, exec approvals, secrets runtime activation, WebSocket handlers, control UI, model catalog, discovery service, tailscale exposure, mobile node tracking.

**Savant has:** Axum WebSocket gateway. No HTTP REST API, no session lifecycle, no auth rate limiting.
**Gap:** MAJOR — Need full HTTP gateway with REST API.

---

## 8. ACP Server (Agent Client Protocol)

Full ACP server with session management, thinking levels, tool call translation. Maps ACP sessions to OpenClaw session keys.

**Savant has:** No ACP support.
**Gap:** LOW — Protocol adoption depends on ecosystem.

---

## 9. Cron System

at/every/cron schedules. Isolated agent runs with full config propagation. Delivery modes (announce/webhook). Failure alerts with cooldown. Session reaping.

**Savant has:** Heartbeat with cron. No isolated runs, no delivery modes, no failure alerts.
**Gap:** MODERATE — Need full cron with delivery and failure handling.

---

## 10. Config System

978-line Zod schema across 36 modules. Sensitive field marking. Per-channel types. Plugin config validation. Session management. Config IO with backup rotation.

**Savant has:** figment-based TOML config. No Zod schema, no backup rotation.
**Gap:** MODERATE — Need typed config schema.

---

## 11. Multi-Platform Apps

iOS (SwiftUI @Observable), Android, macOS native apps. Shared protocol: OpenClawKit.

**Savant has:** Tauri desktop. No mobile.
**Gap:** LOW — Platform-specific.

---

## 12. Canvas / A2UI

HTTP server with WebSocket, live reload via chokidar, A2UI bundle hosting, MIME detection.

**Savant has:** Canvas crate (crates/canvas/) for A2UI rendering. Different approach.
**Gap:** LOW — Both have canvas support.

---

## ALL FEATURES CATALOGUED

| # | Feature | LOC | Priority | Savant Status |
|---|---------|-----|----------|---------------|
| 1 | Plugin system (40+ API methods) | ~25,000 | P1 | Missing |
| 2 | Hook system (25 typed hooks) | ~2,000 | P1 | Missing |
| 3 | Secret matrix (75+ targets) | ~9,700 | P1 | Missing |
| 4 | Channel plugin architecture (27 slots) | ~15,000 | P2 | Missing |
| 5 | Provider plugin (30+ hook slots) | ~5,000 | P2 | Missing |
| 6 | Hybrid memory search (FTS + vector) | ~11,900 | P2 | Partial |
| 7 | HTTP gateway with REST API | ~49,700 | P2 | Partial |
| 8 | ACP server | ~3,000 | P3 | Missing |
| 9 | Cron with delivery modes | ~5,000 | P2 | Partial |
| 10 | Typed config schema (Zod, 36 modules) | ~29,000 | P2 | Missing |
| 11 | Lazy plugin loading (Proxy) | ~500 | P2 | Missing |
| 12 | Bundle compatibility (Claude/Codex/Cursor) | ~500 | P3 | Missing |
| 13 | Conversation bindings | ~300 | P3 | Missing |
| 14 | Session transcript indexing | ~1,000 | P2 | Missing |
| 15 | MMR reranking | ~200 | P2 | Missing |
| 16 | Query expansion | ~200 | P3 | Missing |
| 17 | Temporal decay scoring | ~200 | P3 | Missing |
| 18 | Embedding providers (5 backends) | ~1,000 | P2 | Missing |
| 19 | Batch embedding with concurrency | ~300 | P2 | Missing |
| 20 | Auth rate limiting | ~200 | P2 | Missing |
| 21 | Exec approval forwarding | ~200 | P2 | Missing |
| 22 | Discovery service | ~300 | P3 | Missing |
| 23 | Tailscale exposure | ~200 | P3 | Missing |
| 24 | Mobile node tracking | ~200 | P3 | Missing |
| 25 | Control UI (web) | ~5,000 | P3 | Partial |
| 26 | Device pairing (QR codes) | ~500 | P3 | Missing |
| 27 | Voice call support | ~2,000 | P3 | Missing |
| 28 | Speech providers (TTS/STT) | ~1,000 | P3 | Missing |
| 29 | Image generation providers | ~500 | P3 | Missing |
| 30 | Web search providers (7 bundled) | ~1,000 | P2 | Missing |
| 31 | Sandbox (Docker-based) | ~1,000 | P2 | Partial |
| 32 | Pi coding agent integration | ~5,000 | P3 | Missing |
| 33 | Config backup rotation | ~200 | P2 | Missing |
| 34 | Zod schema validation | ~1,000 | P2 | Missing |
| 35 | OAuth auth profiles | ~2,000 | P3 | Missing |

---

*Exhaustive scan of core src/ architecture. Plugin system, hooks, secrets, channels, providers, memory, gateway, ACP, cron, config all catalogued.*
*Generated 2026-03-21*

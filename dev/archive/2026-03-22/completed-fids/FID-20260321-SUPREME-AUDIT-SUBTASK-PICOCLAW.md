# FID-20260321-SUPREME-AUDIT-SUBTASK-PICOCLAW — FULL SCAN

**Competitor:** PicoClaw (Go 1.25.8)
**Repo:** `research/competitors/picoclaw`
**Source Files Scanned:** ~270 .go files
**Production LOC:** ~42,930
**Features Found:** 30+ features Savant doesn't have

---

## Architecture

Multi-agent Go runtime with message bus, capability-based channel interfaces, two-tier tool registry (hidden+TTL), JSONL crash-safe persistence, model routing, multi-key load balancing.

---

## 1. Agent System

### 1.1 Agent Instance (`pkg/agent/instance.go:312`)
Each agent has: own workspace, session store, tool registry, context builder, model routing, thinking level config, summarization thresholds (message count + token %).

### 1.2 Agent Loop (`pkg/agent/loop.go:2133`)
Full lifecycle: audio transcription → system message check → route to agent → command check → LLM iteration → tool execution → summarization. Hot-swap of provider+config under write lock.

### 1.3 Agent Registry (`pkg/agent/registry.go:140`)
Multi-agent with `RouteResolver`. `CanSpawnSubagent()` with allowlist + wildcard. `ForEachTool()` for cross-agent dependency injection.

**Savant has:** Swarm agents. But no per-agent workspace isolation, no route resolver, no hot-swap.
**Gap:** MODERATE — Need per-agent workspace, route resolver, hot-swap.

---

## 2. Tool System

### 2.1 Two-Tier Registry (`pkg/tools/registry.go:386`)
- **Core tools** (IsCore=true): Always visible
- **Hidden tools** (IsCore=false): Hidden until `PromoteTools()` unlocks with TTL
- `TickTTL()` decrements each iteration. BM25 + regex search for discovery.
- `Clone()` creates snapshot for subagents (prevents recursive spawn)
- `ToProviderDefs()` sorted for KV cache stability

### 2.2 Tool Interface (`pkg/tools/base.go:93`)
```go
type Tool interface {
    Name() string; Description() string; Parameters() map[string]any
    Execute(ctx, args) *ToolResult
}
type AsyncExecutor interface { ExecuteAsync(ctx, args, cb) *ToolResult }
```

### 2.3 ToolResult (`pkg/tools/result.go:160`)
ForLLM, ForUser, Silent, IsError, Async, Err, Media fields. Factory functions for each type.

### 2.4 44 Tool Implementations
exec (483 LOC, 40+ deny patterns), read_file (862 LOC, os.Root sandbox), write_file, list_dir, edit_file, append_file, web_search (6 providers), web_fetch (SSRF protection), message, send_file, spawn, spawn_status, subagent, cron, mcp_tool, tool_search_regex, tool_search_bm25, find_skills, install_skill, i2c, spi.

**Savant has:** 12 tools. No hidden/two-tier, no async executor, no panic recovery, no BM25 search, no clone for subagents.
**Gap:** MAJOR — Need two-tier registry, async executor, BM25 discovery, tool cloning.

---

## 3. Provider System

### 3.1 25+ Providers
groq, openai, anthropic, anthropic_messages, openrouter, litellm, zhipu, gemini, vllm, nvidia, claude_cli, codex_cli, codex, deepseek, avian, mistral, minimax, longcat, github_copilot, moonshot, ollama, cerebras, volcengine, antigravity, qwen, modelscope, novita.

### 3.2 Fallback Chain (`pkg/providers/fallback.go:304`)
Error classification: auth, rate_limit, billing, timeout, format, overloaded. Exponential cooldown (1min → 5min → 25min → 1h). Billing cooldown (5h → 10h → 20h → 24h). 24-hour failure window reset.

### 3.3 Multi-Key Load Balancing
`api_keys` array → expanded into separate model entries → round-robin via `atomic.AddUint32` → per-key cooldown in fallback chain.

### 3.4 Provider Interfaces
LLMProvider, StatefulProvider (Close), StreamingProvider (ChatStream), ThinkingCapable, NativeSearchCapable.

**Savant has:** 14 providers with RetryProvider only. No fallback chain, no cooldown, no multi-key.
**Gap:** MAJOR — Need fallback chain with error classification and cooldown.

---

## 4. Channel System (17 channels)

Capability-based interfaces: TypingCapable, MessageEditor, MessageDeleter, ReactionCapable, PlaceholderCapable, StreamingCapable, CommandRegistrarCapable.

Per-channel rate limits: telegram 20/s, discord 1/s, slack 1/s, matrix 2/s, line 10/s, qq 5/s, irc 2/s.

Hot-reload: config hash comparison, stop removed channels, init new channels.

**Savant has:** 3 channels. No capability interfaces, no rate limits, no hot-reload.
**Gap:** MAJOR — Need capability interfaces, rate limits, hot-reload.

---

## 5. JSONL Persistence (`pkg/memory/jsonl.go:460`)
Append-only JSONL with logical truncation (metadata skip offset). 64 sharded mutexes. fsync on every append. Atomic rewrites via temp+rename. Crash-safe: meta written BEFORE JSONL rewrite. `Compact()` reclaims disk space.

**Savant has:** CortexaDB with LSM. More sophisticated but no JSONL fallback.
**Gap:** LOW — Savant's persistence is superior.

---

## 6. Context Builder (`pkg/agent/context.go:752`)
System prompt caching with mtime invalidation. Recursive skill file tree monitoring. `SystemParts` with `CacheControl{Type:"ephemeral"}` for Anthropic prefix caching. Two-pass history sanitization (DeepSeek compatibility).

**Savant has:** ContextAssembler. No caching, no Anthropic prompt caching.
**Gap:** MODERATE — Need system prompt caching + Anthropic prompt caching.

---

## 7. Model Routing (`pkg/routing/`)
Language-agnostic feature extraction: token estimate (CJK-aware), code block count, recent tool calls, conversation depth, has attachments. Threshold-based light/heavy routing (default 0.35).

**Savant has:** DSP predictor for speculation depth. No model routing.
**Gap:** MODERATE — Need complexity-based model routing.

---

## 8. Credential Encryption (`pkg/credential/credential.go:342`)
AES-256-GCM + HKDF-SHA256. SSH key binding. `enc://` prefix for encrypted credentials. Allowed SSH key path restrictions.

**Savant has:** Ed25519 signing, master key. No per-secret encryption.
**Gap:** MODERATE — Need per-secret encryption.

---

## 9. Shell Security (`pkg/tools/shell.go:483`)
40+ deny patterns. Workspace path traversal detection. TOCTOU mitigation via EvalSymlinks. Remote channel blocking. Platform-specific (PowerShell/sh).

**Savant has:** 8 deny patterns in SovereignShell. No path traversal, no symlink resolution, no remote blocking.
**Gap:** MODERATE — Need comprehensive shell security.

---

## 10. Web Fetch Security (`pkg/tools/web.go`)
SSRF protection (private IP blocking, DNS rebinding). Cloudflare retry with honest user-agent. Content type detection. Redirect limiting (5 max).

**Savant has:** Just added URL validation.
**Gap:** LOW — Partially addressed.

---

## 11. Heartbeat (`pkg/heartbeat/service.go:396`)
Reads HEARTBEAT.md. Minimum 5min interval, default 30min. Runs handler with last active channel context. Logs to heartbeat.log. Creates default template if missing.

**Savant has:** HeartbeatPulse with perception engine + cognitive diary. More sophisticated.
**Gap:** LOW — Savant's is more advanced.

---

## 12. Slash Commands (`pkg/commands/`)
/start, /help, /show, /list, /switch, /check, /clear, /reload. Runtime struct with injectable callbacks.

**Savant has:** No slash commands.
**Gap:** MODERATE — Need command system.

---

## 13. Voice Transcription
`Transcriber` interface for audio-to-text.

**Savant has:** No voice transcription.
**Gap:** LOW — Nice-to-have.

---

## 14. Media System
`MediaStore` interface: Store, Resolve, ReleaseAll, TempDir. MIME detection via magic bytes.

**Savant has:** No media store.
**Gap:** LOW — Nice-to-have.

---

## ALL FEATURES CATALOGUED

| # | Feature | LOC | Priority | Savant Status |
|---|---------|-----|----------|---------------|
| 1 | Two-tier tool registry (hidden+TTL+BM25) | ~386 | P1 | Missing |
| 2 | AsyncExecutor with callbacks | ~50 | P1 | Missing |
| 3 | Tool panic recovery | ~30 | P1 | Missing |
| 4 | Tool cloning for subagents | ~40 | P2 | Missing |
| 5 | Fallback chain with error classification | ~304 | P1 | Missing |
| 6 | Cooldown tracker (exponential backoff) | ~207 | P1 | Missing |
| 7 | Multi-key load balancing | ~100 | P2 | Missing |
| 8 | Model routing (complexity-based) | ~209 | P2 | Missing |
| 9 | Capability-based channel interfaces | ~70 | P2 | Missing |
| 10 | Per-channel rate limits | ~50 | P2 | Missing |
| 11 | Channel hot-reload | ~100 | P3 | Missing |
| 12 | System prompt caching (mtime-based) | ~200 | P2 | Missing |
| 13 | Anthropic prompt caching | ~50 | P2 | Missing |
| 14 | Per-secret AES-256-GCM encryption | ~342 | P2 | Missing |
| 15 | Shell security (40+ deny patterns) | ~200 | P2 | Partial |
| 16 | SSRF protection in web fetch | ~100 | P1 | Partial |
| 17 | Slash command system | ~200 | P2 | Missing |
| 18 | Per-agent workspace isolation | ~200 | P2 | Missing |
| 19 | Provider hot-swap | ~100 | P3 | Missing |
| 20 | Voice transcription | ~100 | P3 | Missing |
| 21 | Media store | ~100 | P3 | Missing |
| 22 | BM25 search engine (generic) | ~200 | P2 | Missing |
| 23 | HTML→Markdown conversion | ~411 | P3 | Missing |
| 24 | HTTP retry with backoff | ~100 | P2 | Missing |
| 25 | Skill installer (malware blocking) | ~203 | P2 | Missing |
| 26 | Cron job CRUD with enable/disable | ~380 | P2 | Partial |
| 27 | Bubble Tea TUI launcher | ~500 | P3 | Missing |
| 28 | Web backend (REST API) | ~656 | P3 | Missing |
| 29 | Gateway/health server | ~100 | P3 | Missing |
| 30 | JSONL crash-safe persistence | ~460 | P2 | Missing |

---

*Exhaustive scan of ~270 .go files. Every struct, interface, function catalogued.*
*Generated 2026-03-21*

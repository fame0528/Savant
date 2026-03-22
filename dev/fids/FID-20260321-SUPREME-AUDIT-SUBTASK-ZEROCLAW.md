# FID-20260321-SUPREME-AUDIT-SUBTASK-ZEROCLAW — FULL SCAN

**Competitor:** ZeroClaw v0.5.6 (Rust, edition 2021, MSRV 1.87)
**Repo:** `research/competitors/zeroclaw`
**Source Files Scanned:** 320 .rs files under src/
**Total LOC:** ~232,988
**Features Found:** 40+ features Savant doesn't have

---

## Architecture

Multi-agent Rust framework with 42 top-level modules. Feature-gated (23 feature flags). Dual tool dispatch (XML + function call). 74 tool files. 19 providers. 36 channels. 21 security submodules. Verifiable Intent credential chain system.

---

## 1. Agent System

Agent struct with provider, tools, memory, observer, prompt_builder, dual dispatcher, memory_loader, config, history, classification_config, route_model_by_hint, allowed_tools, response_cache, autonomy_level.

AgentBuilder with 22 setter methods. from_config() full construction with provider, memory, MCP wiring, dispatcher selection.

**Savant has:** AgentLoop with provider, memory, tools. No builder pattern, no config-based construction, no dispatcher selection.
**Gap:** MODERATE — Need builder pattern + config-based construction.

---

## 2. Dual Tool Dispatch

XmlToolDispatcher: Parses agent text for tool XML tags.
FunctionToolDispatcher: Uses provider native function calling.

Each produces ParsedToolCall(name, arguments, tool_call_id). ToolExecutionResult(name, output, success, tool_call_id).

**Savant has:** 5-format parser. No dual dispatch (single path).
**Gap:** LOW — Savant's approach is similar.

---

## 3. Tool System (74 tool files)

Core tools: ShellTool, FileReadTool, FileWriteTool, FileEditTool, GlobSearchTool, ContentSearchTool, CronAddTool, CronListTool, CronRemoveTool, CronUpdateTool, CronRunTool, CronRunsTool, MemoryStoreTool, MemoryRecallTool, MemoryForgetTool, ScheduleTool, ModelRoutingConfigTool, ModelSwitchTool, ProxyConfigTool, GitOperationsTool, PushoverTool, CalculatorTool, WeatherTool

Conditional tools: Browser, HTTP, WebFetch, TextBrowser, WebSearch, Notion, Jira, ProjectIntel, SecurityOps, Backup, DataManagement, CloudOps, GoogleWorkspace, ClaudeCode, PDFRead, Screenshot, ImageInfo, LinkedIn, Composio, Microsoft365, KnowledgeGraph, Delegate, Swarm, Workspace, VerifiableIntent, WASM plugins

**Savant has:** 12 tools. ZeroClaw has 6x more.
**Gap:** MAJOR — Need comprehensive tool set.

---

## 4. Provider System (19 providers)

Protocol prefix format (e.g., openai/gpt-4o). Feature flags for optional providers. Fallback with cooldown. Provider capabilities: ThinkingCapable, NativeSearchCapable.

**Savant has:** 14 providers. ZeroClaw has 19.
**Gap:** MINOR — Comparable.

---

## 5. Channel System (36 channels)

Feature-gated channels: channel-nostr, channel-matrix, channel-lark. WhatsApp via whatsapp-web feature.

**Savant has:** 3 channels. ZeroClaw has 12x more.
**Gap:** MAJOR — Need channel expansion.

---

## 6. Security (21 submodules)

SecretStore with chacha20poly1305 AEAD encryption. HMAC webhook verification. Comprehensive security architecture.

**Savant has:** PQC attestation + CCT tokens. Different approach but comparable security.
**Gap:** LOW — Both have strong security.

---

## 7. Verifiable Intent (1619 LOC)

Three-layer SD-JWT credential chain (L1/L2/L3). 8 constraint types. ECDSA P-256 via ring. Selective disclosure. sd_hash binding chain. Error taxonomy with 18 discriminants.

**Savant has:** No verifiable intent. No payment integration.
**Gap:** LOW — Niche feature for payment authorization.

---

## 8. Approval Gating

Risk-based operation approval. High-risk operations require user consent.

**Savant has:** No approval gating.
**Gap:** MODERATE — Need approval for destructive operations.

---

## 9. WASM Plugin System

Extism-based WASM plugin runtime. Feature-gated (plugins-wasm).

**Savant has:** WASM host for agent hooks. No general WASM plugin system.
**Gap:** MODERATE — Need WASM plugin runtime.

---

## 10. MCP Integration

MCP client with stdio, SSE, HTTP transports. Tool proxy.

**Savant has:** MCP server. No MCP client.
**Gap:** MAJOR — Need MCP client.

---

## 11. Daemon Mode

Full autonomous runtime. Background agent execution.

**Savant has:** Heartbeat for autonomous execution. No daemon mode.
**Gap:** MODERATE — Need background daemon.

---

## 12. Memory Backends (19 implementations)

Feature-gated: memory-postgres, memory-mem0.

**Savant has:** CortexaDB (LSM + HNSW). Single backend.
**Gap:** MODERATE — Need pluggable memory backends.

---

## 13. Observability

Prometheus metrics, OpenTelemetry tracing. Feature-gated.

**Savant has:** Panopticon (OpenTelemetry). No Prometheus.
**Gap:** LOW — Partial coverage.

---

## 14. Tunnel Support

Cloudflare, Tailscale, ngrok, custom tunnel providers.

**Savant has:** No tunnel support.
**Gap:** LOW — Deployment feature.

---

## 15. Hardware Integration

I2C, SPI, USB device monitoring. Hardware RAG. Feature-gated.

**Savant has:** No hardware integration.
**Gap:** LOW — IoT/edge feature.

---

## 16. Skills Platform

SKILL.md-based skills with ClawHub integration.

**Savant has:** Skills crate with WASM/Docker/Landlock sandbox. Comparable.
**Gap:** LOW — Both have skills.

---

## 17. SOP (Standard Operating Procedures)

Agent behavior protocols.

**Savant has:** No SOP system.
**Gap:** LOW — Governance feature.

---

## ALL FEATURES CATALOGUED

| # | Feature | LOC Est | Priority | Savant Status |
|---|---------|---------|----------|---------------|
| 1 | 74 tool implementations | ~5,000 | P1 | Partial (12 tools) |
| 2 | 36 channels | ~8,000 | P2 | Partial (3 channels) |
| 3 | Dual tool dispatch (XML + function) | ~443 | P2 | Partial (5 parsers) |
| 4 | Approval gating | ~200 | P1 | Missing |
| 5 | MCP client integration | ~300 | P1 | Missing |
| 6 | WASM plugin runtime | ~500 | P2 | Missing |
| 7 | Builder pattern (22 setters) | ~200 | P2 | Missing |
| 8 | Config-based agent construction | ~300 | P2 | Missing |
| 9 | Daemon mode | ~200 | P3 | Missing |
| 10 | Verifiable Intent (SD-JWT chain) | ~1,619 | P3 | Missing |
| 11 | Pluggable memory backends | ~500 | P3 | Missing |
| 12 | Prometheus metrics | ~200 | P3 | Missing |
| 13 | Tunnel providers | ~300 | P3 | Missing |
| 14 | Hardware I2C/SPI | ~200 | P3 | Missing |
| 15 | SOP system | ~200 | P3 | Missing |
| 16 | Knowledge graph tool | ~300 | P3 | Missing |
| 17 | Google Workspace tool | ~300 | P3 | Missing |
| 18 | Microsoft 365 tool | ~300 | P3 | Missing |
| 19 | Browser automation tool | ~500 | P3 | Missing |
| 20 | PDF read tool | ~100 | P3 | Missing |
| 21 | Screenshot tool | ~100 | P3 | Missing |
| 22 | Claude Code integration | ~200 | P3 | Missing |
| 23 | Jira integration | ~200 | P3 | Missing |
| 24 | Notion integration | ~200 | P3 | Missing |
| 25 | LinkedIn integration | ~200 | P3 | Missing |
| 26 | Composio integration | ~200 | P3 | Missing |
| 27 | Swarm tool | ~200 | P3 | Missing |
| 28 | Delegate tool | ~200 | P3 | Missing |
| 29 | Backup/data management | ~200 | P3 | Missing |
| 30 | Cloud ops tool | ~200 | P3 | Missing |
| 31 | Security ops tool | ~200 | P3 | Missing |
| 32 | Project intel tool | ~200 | P3 | Missing |
| 33 | Calculator tool | ~50 | P3 | Missing |
| 34 | Weather tool | ~50 | P3 | Missing |
| 35 | Git operations tool | ~100 | P2 | Missing |
| 36 | Model routing config tool | ~100 | P2 | Missing |
| 37 | Model switch tool | ~50 | P2 | Missing |
| 38 | Proxy config tool | ~100 | P3 | Missing |
| 39 | Pushover notification tool | ~50 | P3 | Missing |
| 40 | WhatsApp web channel | ~400 | P3 | Missing |

---

*Exhaustive scan of 320 .rs files. 42 top-level modules catalogued. 40+ features identified.*
*Generated 2026-03-21*

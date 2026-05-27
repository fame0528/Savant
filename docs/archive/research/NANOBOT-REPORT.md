# Nanobot Project Comprehensive Audit Report
**Date**: 2026-03-24 | **Version**: v0.1.4.post5 | **Source**: research/competitors/nanobot

---

## 1. Project Overview

Nanobot is an ultra-lightweight personal AI assistant framework in Python >=3.11 by HKUDS. Inspired by OpenClaw, delivers 99% of functionality in a much smaller codebase. Tech Stack: Python 3.11+, LiteLLM + direct providers, Pydantic v2, Typer + Rich + prompt_toolkit, asyncio, httpx. Channels: Telegram, Discord, Slack, Feishu, DingTalk, QQ, WhatsApp, Email, Matrix, WeCom, MoChat. Search: Brave, Tavily, Jina, SearXNG, DuckDuckGo. MCP: mcp SDK (stdio/SSE/HTTP). Cron: croniter. Tokens: tiktoken. ~50 source files, ~47 test files.

---

## 2. Architecture and Design Patterns

Layered: Channels -> MessageBus (async queue) -> AgentLoop -> Provider -> Tools. Key patterns: (A) Message Bus: async queue pair decoupling channels from agent. (B) Provider Registry: frozen dataclass ProviderSpec as single source of truth, 2-step addition. (C) Channel Plugins: Python entry_points, BaseChannel ABC. (D) Progressive Skills: summaries in prompt, full content on demand. (E) Token-Based Memory: MemoryConsolidator with LLM summarization. (F) Two-Phase Heartbeat: cheap skip/run decision then execution.

---

## 3. Top Features

3.1 Multi-Provider LLM (20+): ProviderSpec registry drives auto-detection. 3.2 Chat Channels (10+): WebSocket options without public IPs. 3.3 MCP: Three transports, schema normalization. 3.4 Tools: read/write/edit file, exec (safety guards), web search/fetch, message, spawn, cron. 3.5 Subagents: isolated tools, 15 iteration limit. 3.6 Token-Aware Memory: MEMORY.md + HISTORY.md, forced save_memory tool. 3.7 Cron: one-shot/interval/cron-expr with timezone. 3.8 Heartbeat: two-phase with notification evaluation. 3.9 CLI: prompt_toolkit + Rich. 3.10 Channel Plugins: entry_points. 3.11 Multi-Instance: --config/--workspace. 3.12 Wizard: interactive Pydantic editor.

---

## 4. Strengths

1. Simplicity: intentionally small, readable. 2. Provider Registry: 2-step, no if-elif. 3. Message Bus: clean decoupling. 4. Channel Breadth: 10+ platforms. 5. Token-Aware Memory: smarter than sliding windows. 6. Safety: SSRF, command guards, sandboxing. 7. MCP: three transports. 8. Plugins: entry_points. 9. Progressive Loading: summary + lazy load. 10. Heartbeat: two-phase avoids no-op waste.

---

## 5. Weaknesses

1. Global Lock: serializes all messages. 2. No Streaming: full response before send. 3. LiteLLM Dependency: heavy with quirks. 4. Unbounded History: no hard cap. 5. Stateless Tools. 6. Limited Error Recovery. 7. No Multi-Modal Output. 8. WhatsApp Bridge complexity (Node.js). 9. No agent loop integration tests. 10. Dict-based channel config loses type safety. 11. LLM-dependent memory fallback. 12. Linear sessions only.

---

## 6. Ideas for Savant

1. Provider Registry: frozen dataclass for 2-step addition. 2. Token-Aware Context: estimate tokens, archive with LLM summary. 3. MCP Support: three transports, growing ecosystem. 4. Plugin System: entry_points. 5. Subagent Spawning: background delegation. 6. SSRF + Sandboxing: private IP blocking. 7. Message Bus: async queue decoupling. 8. Progressive Loading: summary + on-demand. 9. Heartbeat: two-phase proactive tasks. 10. Smart Edit: fuzzy matching + diff.

---

## 7. Key Differences vs Savant

Nanobot: Python, personal AI assistant, LiteLLM, 10+ channels, MCP, file-based memory, cron+heartbeat, entry_points plugins, asyncio. Savant: Rust (presumably), developer tooling, unknown LLM/channels/MCP/memory. Nanobot optimizes for simplicity; Savant for performance/safety. Transferable patterns: registry, bus, progressive loading, MCP, plugins.

---

## Summary

Nanobot is well-engineered and intentionally minimal. Core strengths: provider registry, message bus, token-aware memory, channel breadth. Weaknesses: Python limitations, missing streaming/multi-modal/branching. Most transferable to Savant: provider registry, token-aware context, MCP integration, message bus, plugin architecture.

---

## Detailed Architecture

### Agent Loop (agent/loop.py - 599 lines)
The AgentLoop is the core engine. It consumes messages from the MessageBus, builds context via ContextBuilder, calls the LLM via provider.chat_with_retry(), executes tool calls via ToolRegistry.execute(), and sends responses back. It supports up to 40 iterations per turn. Slash commands (/stop, /restart, /status, /new, /help) are handled directly. Progress updates are streamed to channels during tool execution. A global async lock serializes message processing across sessions.

### Tool System (agent/tools/)
All tools extend Tool ABC with name, description, parameters (JSON Schema), and execute(). ToolRegistry handles registration, OpenAI-format schema generation, parameter casting (schema-driven type coercion), validation (required, types, enums, min/max), and execution with error hints. The base Tool class supports complex JSON Schema features including nullable union types, nested objects, and arrays.

### Provider System (providers/)
LLMProvider ABC defines chat() and get_default_model(). chat_with_retry() adds retry logic (delays: 1, 2, 4 seconds) for transient errors. LiteLLMProvider wraps LiteLLM with provider-specific model prefixing, env var setup, cache control, and model overrides. CustomProvider uses AsyncOpenAI directly. AzureOpenAIProvider uses httpx with Azure-specific headers (api-key, max_completion_tokens). OpenAICodexProvider uses OAuth + SSE streaming via the Responses API.

### Config System (config/)
Pydantic v2 models with camelCase alias support. Config extends BaseSettings for env var overrides (NANOBOT_ prefix). Provider matching logic in _match_provider() follows priority: explicit provider > keyword match > local fallback > gateway fallback > any with key. Config migration handles field renames.

### Session System (session/manager.py)
Sessions stored as JSONL files with metadata header. Session.messages is append-only for LLM cache efficiency. get_history() returns unconsolidated messages aligned to legal tool-call boundaries (_find_legal_start ensures every tool result has a matching tool_calls message). Legacy session migration from global ~/.nanobot/sessions/ to workspace-based.

### Security (security/network.py)
SSRF protection via DNS resolution + private IP blocking (10.x, 127.x, 172.16-31.x, 192.168.x, 169.254.x, link-local IPv6, unique-local IPv6). Both pre-fetch validation and post-redirect validation. Internal URL detection in shell commands.

### Evaluator (utils/evaluator.py)
Post-run LLM evaluation for background tasks (heartbeat/cron). Uses a virtual evaluate_notification tool call to decide if the result warrants user notification. Falls back to True (notify) on failure.


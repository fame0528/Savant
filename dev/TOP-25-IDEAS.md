# Top 25 Ideas From Competitive Analysis

> Ranked by impact on Savant. Each idea sourced from a specific framework review.

---

## Tier 1 — Critical (Implement Now)

| Rank | Idea | Source | Impact |
|------|------|--------|--------|
| 1 | **Multi-format tool call parser** | ZeroClaw | Tools currently broken. Parser strips XML tags instead of executing them. ZeroClaw parses 7+ formats. This is the single highest-impact fix. |
| 2 | **Native function calling from providers** | IronClaw | OpenRouter/Anthropic/Ollama return tool_calls in provider response. Savant ignores them entirely. Captures ~40% of tool calls that currently fail silently. |
| 3 | **Credential proxy pattern** | NanoClaw | Real API keys never enter agent environment. HTTP proxy injects auth transparently. Eliminates entire class of security vulnerability. |
| 4 | **Virtual tool-call pattern for heartbeat** | NanoBot | Force structured tool call for decisions (skip/run) instead of parsing free-text. More reliable across all providers. Stops heartbeat from flooding chat with mechanical output. |
| 5 | **Token-based memory consolidation** | NanoBot | Replace message-count limits with token estimation. Consolidation at user-turn boundaries preserves tool-call chains. Prevents context overflow crashes. |

## Tier 2 — High Impact (Next Sprint)

| Rank | Idea | Source | Impact |
|------|------|--------|--------|
| 6 | **Smart model routing (13 dimensions)** | IronClaw | Route simple queries to cheap models. 50-70% cost reduction. IronClaw's Flash/Standard/Pro/Frontier tiers with cascade mode. |
| 7 | **Tool name aliasing** | ZeroClaw | Normalize "bash" → "shell", "fileread" → "foundation.read". LLMs generate inconsistent tool names. Aliasing prevents silent failures. |
| 8 | **Tool result deduplication** | ZeroClaw | Canonical signature (name + sorted args) prevents duplicate execution. Same tool called twice with same args = skip second. |
| 9 | **Error poisoning prevention** | NanoBot | Don't persist error responses to session history. Prevents permanent 400 loops from bad tool calls. |
| 10 | **ToolDomain separation** | IronClaw | Orchestrator (safe) vs Container (filesystem/shell, require approval). Clear security boundary per tool. |
| 11 | **LoopDelegate trait** | IronClaw | Single agentic loop engine with pluggable strategy. Chat, Heartbeat, Speculative all share same loop. Reduces code duplication. |
| 12 | **External tamper-proof security config** | NanoClaw | Security rules stored outside project root. Agents cannot modify their own security rules. |

## Tier 3 — Medium Impact (Next Quarter)

| Rank | Idea | Source | Impact |
|------|------|--------|--------|
| 13 | **TTL-based tool visibility** | PicoClaw | Hidden tools auto-discover when relevant, auto-hide after use. Prevents system prompt bloat. |
| 14 | **Dual-channel tool results** | PicoClaw | ForLLM (context) vs ForUser (display). Clean separation of agent-facing vs user-facing content. |
| 15 | **Hybrid search (BM25 + vector RRF)** | IronClaw | Reciprocal Rank Fusion combines keyword + semantic search. Proven pattern for better recall. |
| 16 | **Circuit breaker for LLM providers** | IronClaw | Closed → Open → HalfOpen state machine. Fast-fail when provider is degraded. Prevents cascading failures. |
| 17 | **Post-run evaluation gate** | NanoBot | LLM evaluates if background task results warrant notification. Prevents notification fatigue. |
| 18 | **Tool intent nudge** | IronClaw | When LLM says "let me search..." without calling a tool, inject a nudge. Improves tool utilization. |
| 19 | **Credential scrubbing on tool output** | ZeroClaw | Scan tool results for secrets before injecting into LLM context. Prevents credential leakage. |
| 20 | **6 lifecycle hooks** | IronClaw | BeforeInbound, BeforeToolCall, BeforeOutbound, OnSessionStart/End, TransformResponse. Extensibility without coupling. |

## Tier 4 — Longer Term

| Rank | Idea | Source | Impact |
|------|------|--------|--------|
| 21 | **Skills trust model with attenuation** | IronClaw | Trusted vs Installed skills. Lower-trust restricts tool access. Prevents privilege escalation through skill mixing. |
| 22 | **Self-repair system** | IronClaw | Detect stuck jobs + broken tools. Auto-recover. Rebuild broken tools. Operational resilience. |
| 23 | **Progressive skills loading** | NanoBot | Summary in prompt, full content loaded on-demand via read_file. Reduces system prompt size. |
| 24 | **EMA learning for estimation** | IronClaw | Cost/time prediction improves over time via Exponential Moving Average. Better budget planning. |
| 25 | **WASM sandbox with leak detection** | IronClaw | Capability-based permissions, endpoint allowlisting, request/response scanning for secret exfiltration. Defense in depth. |

---

## Quick Wins (< 1 day each)

- Tool name aliasing (#7)
- Error poisoning prevention (#9)
- Max tool iterations config
- Credential scrubbing on output (#19)

## Medium Effort (1-3 days each)

- Multi-format parser (#1)
- Native function calling (#2)
- Virtual tool-call for heartbeat (#4)
- Token-based consolidation (#5)
- ToolDomain separation (#10)

## Major Effort (1-2 weeks each)

- Smart model routing (#6)
- LoopDelegate trait (#11)
- Hybrid search (#15)
- WASM sandbox (#25)

---

*Compiled from reviews of OpenClaw, ZeroClaw, NanoClaw, PicoClaw, NanoBot, IronClaw.*
*2026-03-20.*

# FID-20260324-AGENT-HOOK-SYSTEM

**Date:** 2026-03-24 (prepared night of 2026-03-23)
**Status:** CERTIFIED
**Protocol:** Perfection Loop + Checkpoint Gates (brain surgery protocol)
**Source:** Agent loop delegates (ChatDelegate, HeartbeatDelegate, SpeculativeDelegate) — un-integrated hooks CRITICAL for agent functionality
**Standard:** $1M+ enterprise valuation — agents cannot function without these hooks
**Competitor Research:** zeroclaw (two-tier + panic catching), openfang (DashMap registry), microclaw (file-based discovery), memU (reverse-order after-interceptors)
**Perfection Loop:** Iteration 2 — certified (consolidated into existing HookRegistry)

---

## CRITICAL CONTEXT

The agent loop has 3 no-ops (`ChatDelegate`, `HeartbeatDelegate`, `SpeculativeDelegate`) that are extensibility hooks never wired in. Without these hooks, agents cannot:
- Monitor health (heartbeat)
- Handle errors gracefully (self-repair)
- Inject context dynamically (before_llm_call)
- Be paused/resumed (check_signals)
- Adapt identity per conversation (build_identity)

An existing `HookRegistry` exists in `core/hooks/mod.rs` with 6 events + VoidHookHandler + ModifyingHookHandler. But it has only 3 built-in logger hooks and is never wired into the agent loop.

**The fix: Extend the existing HookRegistry, add missing events, wire into agent loop, replace no-op delegates with real hook implementations.**

---

## FIX MATRIX (7 fixes)

| # | Issue | File | Action | Cross-Impact |
|---|-------|------|--------|-------------|
| H1 | HookResult missing Cancel | `core/hooks/mod.rs` | Add `Cancel(String)` variant to `HookResult` — approval gating, blocking | All hook consumers |
| H2 | Missing events | `core/hooks/mod.rs` | Add 9 events: `BeforeLlmCall`, `AfterLlmCall`, `CheckSignals`, `BuildIdentity`, `ToolError`, `LlmError`, `SessionError`, `HeartbeatTick`, `TurnStart`, `TurnEnd` | Hook registry, agent loop |
| H3 | ModifyingHandler needs cancel support | `core/hooks/mod.rs` | `ModifyingHookHandler::handle()` returns `HookResult` with `Cancel` — first `Cancel` stops chain | All modifying hooks |
| H4 | Not wired into agent loop | `agent/react/mod.rs` | Add `HookRegistry` field to `AgentLoop`, pass through to stream.rs | Agent loop structure |
| H5 | Agent loop needs hook dispatch | `agent/react/stream.rs` | Add dispatch calls at 8 integration points: before_llm_call, after_llm_call, before_tool_call, after_tool_call, check_signals, on_tool_error, on_turn_start, on_turn_end | Agent loop execution |
| H6 | Replace no-op delegates | `agent/react/mod.rs` | Remove `ChatDelegate`, `HeartbeatDelegate`, `SpeculativeDelegate`. Implement as real hooks: `MessageFormatterHook`, `HealthMonitorHook`, `CacheWarmerHook` | Agent extensibility |
| H7 | Hook execution safety | `core/hooks/mod.rs` | Add `catch_unwind` around hook dispatch (zeroclaw pattern) — hooks must never crash agent | Hook reliability |

---

## INTEGRATION POINTS (8 locations in stream.rs)

| Location | Hook | What It Does |
|----------|------|-------------|
| Before LLM call (line ~198) | `before_llm_call` | Inject context, warm caches, log |
| LLM stream start (line ~255) | `on_llm_input` | Telemetry, metrics |
| LLM chunk received (line ~349) | `on_llm_output` | Logging, streaming metrics |
| Tool matched (line ~481) | `before_tool_call` | Approval gating, validation |
| Tool result received (line ~580) | `after_tool_call` | Modify result, logging |
| Heuristic error (line ~585) | `on_tool_error` | Recovery, rollback |
| Turn start (line ~130) | `on_turn_start` | Telemetry, setup |
| Turn end (line ~637) | `on_turn_end` | Metrics, cleanup |

---

## PERFECTION LOOP ITERATIONS

| Iteration | Changes |
|-----------|---------|
| 1 | Initial FID — 7 items, generic hook system design |
| 2 | Deep audit found existing HookRegistry in core/hooks/mod.rs. Consolidated: extend existing system instead of creating new module. Added missing events, Cancel variant, catch_unwind. Certified. |

---

*FID certified via Perfection Loop (Iteration 2). Awaiting implementation.*

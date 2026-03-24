# FID-20260324-BATCH-5-FINAL-SESSION

**Date:** 2026-03-24 (1:40 AM — final batch before Spencer sleeps)
**Status:** CERTIFIED
**Protocol:** Perfection Loop + Checkpoint Gates (brain surgery protocol)
**Source:** Remaining audit items after 87+ fixes
**Standard:** $1M+ enterprise valuation — zero tolerance for stubs, data loss, security gaps
**Perfection Loop:** Iteration 1 — certified (items well-scoped from audit)

---

## FIX MATRIX (4 items)

### Item 1: Agent Hook System — Implement Delegates (H6 from FID-20260324-AGENT-HOOK-SYSTEM)

| # | Issue | File | Action | Cross-Impact |
|---|-------|------|--------|-------------|
| 1.1 | `ChatDelegate` no-op | `agent/react/mod.rs:69-97` | Implement as `MessageFormatterHook` (ModifyingHookHandler on `BeforeLlmCall`): log message count, inject context summary | Agent loop, hook system |
| 1.2 | `HeartbeatDelegate` no-op | `agent/react/mod.rs:99-127` | Implement as `HealthMonitorHook` (VoidHookHandler on `HeartbeatTick`): log health metrics, track session duration | Agent loop, monitoring |
| 1.3 | `SpeculativeDelegate` no-op | `agent/react/mod.rs:128-157` | Implement as `CacheWarmerHook` (VoidHookHandler on `TurnStart`): pre-fetch embedding for user input, warm tool schemas | Agent loop, performance |
| 1.4 | Replace no-op delegates | `agent/react/mod.rs` | Remove ChatDelegate/HeartbeatDelegate/SpeculativeDelegate structs. Replace with real hook implementations. Wire into AgentLoop constructor. | Agent extensibility |

### Item 2: PromotionEngine Instantiation

| # | Issue | File | Action | Cross-Impact |
|---|-------|------|--------|-------------|
| 2.1 | PromotionEngine never instantiated | `memory/engine.rs` | Add `promotion_engine: PromotionEngine` field to `MemoryEnclave`. Initialize in `MemoryEngine::new()`. | Memory engine |
| 2.2 | Promotion cycle not triggered | `memory/engine.rs` | Add `run_promotion_cycle()` method that scans all entries and archives low-scoring ones. Wire into a periodic task or explicit call. | Memory maintenance |

### Item 3: Remaining `let _ =` in Gateway

| # | Issue | File | Action | Cross-Impact |
|---|-------|------|--------|-------------|
| 3.1 | Error swallowing in gateway handlers | `gateway/handlers/mod.rs` | Replace `let _ =` with `if let Err(e) = { tracing::warn!(...) }` for all persistence calls | Gateway reliability |
| 3.2 | Error swallowing in gateway server | `gateway/server.rs` | Same pattern — log all persistence failures | Gateway reliability |

### Item 4: Dashboard URL Construction Unification

| # | Issue | File | Action | Cross-Impact |
|---|-------|------|--------|-------------|
| 4.1 | 3 different URL implementations | `dashboard/src/app/page.tsx`, `dashboard/src/lib/tauri.ts` | Extract gateway URL construction to a single utility function. Use `process.env.NEXT_PUBLIC_GATEWAY_PORT` consistently. | Dashboard consistency |

---

## EXECUTION ORDER

```
1.1-1.4: Implement delegates as hooks + wire into AgentLoop
  ↓ CHECKPOINT: cargo check -p savant_agent + cargo check --workspace
2.1-2.2: Wire PromotionEngine into memory engine
  ↓ CHECKPOINT: cargo check -p savant_memory + cargo check --workspace
3.1-3.2: Fix remaining let _ = in gateway
  ↓ CHECKPOINT: cargo check -p savant_gateway + cargo check --workspace
4.1: Unify dashboard URL construction
  ↓ CHECKPOINT: npx tsc --noEmit + cargo check --workspace
Final: commit + push
```

---

## PERFECTION LOOP

**Deep Audit:** All 4 items well-scoped from previous audit. No cross-impact risks.
**Validate:** Hook system H4 already wired in — H6 adds implementations.
**Certify:** Batch 5 is certified.

---

*FID certified. Awaiting implementation.*

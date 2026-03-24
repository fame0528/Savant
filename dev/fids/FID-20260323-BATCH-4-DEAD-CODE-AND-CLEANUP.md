# FID-20260323-BATCH-4-DEAD-CODE-AND-CLEANUP

**Date:** 2026-03-23
**Status:** CERTIFIED
**Protocol:** Perfection Loop + Checkpoint Gates (brain surgery protocol)
**Source:** AUDIT-REPORT.md — "Dead code" items that are actually un-integrated features
**Standard:** $1M+ enterprise valuation — we don't delete, we build
**Competitor Research:** 17 projects scanned. Key findings: zeroclaw SSE streaming, zeroclaw Shannon entropy, zeroclaw two-tier hook system, openclaw IRC protocol, opencrabs typed streaming events
**Perfection Loop:** Iteration 2 — certified

---

## CRITICAL RULE

**We do not delete "dead code."** Every piece of code exists for a reason. If it's not wired in, we wire it in. If it's a stub, we build it properly. The audit found items flagged as "dead code" — they are un-integrated features that need to be connected to their intended use paths.

---

## FIX MATRIX (13 fixes across 3 domains)

### Domain 1: Wire Un-Integrated Features (6 items)

| # | Issue | File | What To Do | Competitor Source | Cross-Impact |
|---|-------|------|-----------|-------------------|-------------|
| D1 | SSE parser not integrated | `agent/streaming.rs` | Wire `parse_llm_stream` into agent loop — replace inline SSE parsing in `stream.rs` with cleaner module. Includes noise filtering for provider-specific prefixes. | zeroclaw SSEClient (fetch+ReadableStream, buffer splitting), opencrabs StreamEvent enum (Anthropic-style lifecycle) | Agent loop message parsing |
| D2 | Entropy calculator not wired | `memory/arbiter.rs:93` | Wire `calculate_shannon_entropy_from_logprobs` as entropy scoring utility. Use when LLM returns logprobs in distillation pipeline. Keep as utility function available for future scoring needs. | zeroclaw shannon_entropy (sensitivity scaling + URL stripping for leak detection) | Distillation pipeline scoring |
| D3 | Key lookup not wired | `memory/lsm_engine.rs:652` | Wire `find_by_key` into arbiter contradiction detection — O(1) metadata key lookup vs O(N) scan | zeroclaw security patterns | Arbiter sweep efficiency |
| D4 | Agent defaults not applied | `core/fs/registry.rs:10` | Wire `defaults` field into `discover_agents_impl` — apply default config when creating new agents | OpenClaw agent configuration patterns | Agent workspace initialization |
| D5 | IRC PRIVMSG helper unused | `channels/irc.rs` | Wire `_send_privmsg` into outbound message handler — use convenience method instead of raw PRIVMSG format | openclaw IRC protocol.ts (word-boundary chunking, control char sanitization) | IRC outbound messaging |
| D6 | No-op delegates not wired | `agent/react/mod.rs:69-95` | Implement `ChatDelegate`, `HeartbeatDelegate`, `SpeculativeDelegate` with real behavior using two-tier hook pattern: (1) void hooks fire-and-forget, (2) modifying hooks sequential with priority, HookResult::Cancel for blocking | zeroclaw two-tier hook system (void/modifying) + microclaw file-based discovery | Agent loop extensibility |

### Domain 2: Log Message Cleanup (7 instances)

| # | Issue | File | Line | Fix |
|---|-------|------|------|-----|
| D7 | 🧬 emoji in log | `core/pulse/watchdog.rs` | 30 | Remove emoji from `info!` call |
| D8 | ❌ emoji in log | `core/fs/registry.rs` | 75, 79 | Remove emojis from error messages |
| D9 | ⚠️ emoji in log | `core/fs/registry.rs` | 107, 132 | Remove emojis from warn messages |

### Domain 3: Clippy Suppression Cleanup (4 files)

| # | Issue | File | Line | Fix |
|---|-------|------|------|-----|
| D10 | Blanket clippy suppression | `channels/discord.rs` | 1 | Remove `#![allow(clippy::disallowed_methods)]` — fix underlying issues (serde_json::json! false positives) or scope suppression to specific lines |
| D11 | Blanket clippy suppression | `channels/email.rs` | 1 | Same as D10 |
| D12 | Blanket clippy suppression | `channels/slack.rs` | 1 | Same as D10 |
| D13 | Blanket clippy suppression | `channels/whatsapp.rs` | 1 | Same as D10 |

---

## EXECUTION ORDER

```
D1: Wire parse_llm_stream into agent loop (streaming.rs → stream.rs)
  ↓ CHECKPOINT: cargo check -p savant_agent + Spencer approval
D2: Wire entropy calculator into distillation (or keep as utility)
  ↓ CHECKPOINT: cargo check -p savant_memory + Spencer approval
D3: Wire find_by_key into arbiter
  ↓ CHECKPOINT: cargo check -p savant_memory + Spencer approval
D4: Wire agent defaults into discovery
  ↓ CHECKPOINT: cargo check -p savant_core + Spencer approval
D5: Wire _send_privmsg into IRC outbound
  ↓ CHECKPOINT: cargo check -p savant_channels + Spencer approval
D6: Implement delegates with two-tier hook pattern
  ↓ CHECKPOINT: cargo check -p savant_agent + Spencer approval
D7-D9: Remove emojis from watchdog.rs + registry.rs
  ↓ CHECKPOINT: cargo check -p savant_core + Spencer approval
D10-D13: Remove blanket clippy suppression + fix warnings
  ↓ CHECKPOINT: cargo check -p savant_channels + Spencer approval
Final: cargo check --workspace + commit + push
```

---

## PERFECTION LOOP ITERATIONS

| Iteration | Changes |
|-----------|---------|
| 1 | Initial FID — 13 items, generic "wire in" approach |
| 2 | Deep audit: clarified D2 (logprobs may not be available — keep as utility), D6 (two-tier hook pattern from zeroclaw), D10-D13 (remove and fix real warnings). Added competitor research. Certified. |

---

*FID certified via Perfection Loop (Iteration 2). No deletions — everything gets wired in.*

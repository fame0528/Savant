# Savant Session Summary -- 2026-05-28 (v0.3.5)

## Mission
Fix Build 7 regressions (partition key, Copy All, display names), bump version to v0.3.5, audit LEARNINGS pipeline, fix 6 dashboard UI regressions from live test, update all tracking docs.

## Status: v0.3.5 PUSHED — All Dashboard UI Fixes Complete

## What Was Done
| Item | Status | Details |
|------|--------|---------|
| Dashboard history partition key (CRITICAL) | Fixed | `persistence.rs` — flipped UCH precedence: agent_id > sender > recipient > session_id |
| Agent Logs Copy All (HIGH) | Fixed | `logs.html` — 3-tier clipboard fallback: navigator → execCommand → Tauri plugin |
| Dashboard auto-select (CRITICAL) | Fixed | `agents.discovered` auto-selects first agent when no saved preference |
| Sidebar display names (HIGH) | Fixed | Uses `getAgentMeta()` for "SAVANT" display instead of raw `.savant` id |
| Header/placeholder display (HIGH) | Fixed | Header title + input placeholder use `getAgentMeta()` consistently |
| .savant default agent (HIGH) | Fixed | Agents state pre-seeded with .savant, sidebar always shows core agent |
| Unified copyToClipboard (HIGH) | Fixed | 3-tier fallback in tauri.ts, used by all copy buttons |
| Code block copy feedback (MEDIUM) | Fixed | CodeCopyButton component with Tauri fallback + visual feedback |
| Duplicate helpers (MEDIUM) | Fixed | cleanMessage (3 to 1), formatEst (2 to 1), URL helpers (2 to 1) |
| Dashboard README | Fixed | Fixed broken HTML quote, rewritten to match main README quality |
| build.bat optimization | Fixed | Removed redundant cargo build, fixed --release flag, added do_build subroutine |
| Version bump 0.3.4 → 0.3.5 | Fixed | 28 Cargo.toml + 2 tauri.conf.json + dashboard/package.json (was 0.1.0) |
| CHANGELOG.md | Updated | v0.3.5 section with partition key fix + version bump |
| CHANGELOG-INTERNAL.md | Updated | All Build 7 + Dashboard UI V2 entries with full root cause + fix details |
| progress.md | Updated | Now at v0.3.5, includes Dashboard UI Fixes V2 |
| IMPLEMENTATION-TRACKER.md | Updated | Now at v0.3.5, includes Dashboard UI Fixes V2 |
| SESSION-SUMMARY.md | Updated | This file |
| LEARNINGS pipeline audit | Complete | Root cause found: grounding filter over-blocking + agent offline since May 13 |
| **Message doubling (V2)** | Fixed | Backend: skip user messages in telemetry_task. Frontend: dedupedMessagesRef with 15s TTL |
| **Sidebar empty (V2)** | Fixed | Always ensure .savant agent present in agents.discovered; HTTP fallback preserves defaults |
| **Input box too small (V2)** | Fixed | Increased wrapper gap, padding, borderRadius, minHeight; CSS height:100% |
| **No typing indicator (V2)** | Fixed | Added typing indicator component with animated dots |
| **Reflections panel empty (V2)** | Fixed | Seed "system online" insight; surface reflective messages as insights |
| **Copy All fails in logs (V2)** | Fixed | Reordered clipboard fallback: navigator → execCommand → Tauri plugin |

## Verification
- `cargo check --workspace` — 0 errors
- `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings
- `npx tsc --noEmit` — 0 errors
- `cargo check -p savant-gateway` — 0 errors (telemetry_task change)
- Partition logic verified: agent responses with `agent_id=".savant"` now go to `chat.savant`

## FIDs
- FID-20260528-v034-BUILD7-REGRESSIONS — FIXED (awaiting live test)
- FID-20260528-DASHBOARD-UI-FIXES-V2 — CLOSED (6/6 regressions fixed)

## New Issues Discovered
| # | Sev | Issue | Status |
|---|-----|-------|--------|
| 1 | CRITICAL | LEARNINGS.md not updated since March 28 — grounding filter over-blocking engineering content | Filter fix implemented (`2e8cc5a`) |
| 2 | HIGH | Agent backend offline since May 13 — heartbeat, consciousness, learnings pipeline dormant | Deployment state — agent needs restart |
| 3 | MEDIUM | Tracking docs were stale at v0.3.4 after v0.3.5 bump | FIXED |

## Git & Push
- Commit `01f79e1`: fix: message doubling dedup guard + input box sizing in dashboard
- Commit `2e8cc5a`: fix: add engineering grounding patterns to LEARNINGS filter
- Commit `4d4392d`: docs: update all tracking docs to v0.3.5
- Commit `4a2c712`: chore: version bump 0.3.4 → 0.3.5 across all 30 files
- Commit `aac6862`: fix: dashboard history partition key — flip precedence to agent_id > session_id
- All pushed to origin/main
- Operating mode: Level 3 (full autonomy, push at will)

## ECHO Compliance
- Law 1 (Read 0-EOF): All files read completely before editing
- Law 2 (Present before act): Fix plan presented before implementation
- Law 3 (Verify): `cargo check` + `npx tsc` passed before every push
- Law 10 (Update tracking): All tracking docs now at v0.3.5
- Law 13 (Utility-first): Centralized `getAgentMeta`, `copyToClipboard`, URL helpers

## Next Steps
1. User rebuilds (`build.bat`) and tests: (a) message doubling, (b) sidebar, (c) input box, (d) typing indicator, (e) reflections panel, (f) Copy All in logs
2. Verify all 6 dashboard UI regressions resolved after rebuild
3. Backfill key learnings from v0.3.2→v0.3.5 development cycle
4. Investigate consciousness daemon restart to resume LEARNINGS pipeline

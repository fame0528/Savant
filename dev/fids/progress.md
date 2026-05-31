# FID Progress Tracking

> **Last Updated:** 2026-05-30 20:41 EDT (v0.4.0)
> **Active FIDs:** 0
> **Closed FIDs:** 95 in `dev/fids/archived/`

---

## Status: v0.4.0 — All FIDs Closed, Ready for Push

| Metric | Count |
|:-------|:-----:|
| Total FIDs this session | 13 |
| FIDs closed this session | 13 |
| FIDs active | 0 |
| Issues fixed this session | 29 (27 previous + 2 memory enclave) |
| Clippy warnings | 0 |

---

## Build State (2026-05-29 v0.4.0)

- Version: 0.4.0 — single source in `VERSION` file + `Cargo.toml [workspace.package]`
- All 28 crates use `version.workspace = true` (workspace inheritance)
- `cargo check --workspace` — 0 errors
- `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings
- `cargo fmt --check` — 0 violations
- `npx tsc --noEmit` — 0 errors

---

## Commits (pushed to origin/main)

| Hash | Summary |
|------|---------|
| `01f79e1` | fix: message doubling dedup guard + input box sizing in dashboard |
| `2e8cc5a` | fix: add engineering grounding patterns to LEARNINGS filter — 10 patterns + 26 tests |
| `4d4392d` | docs: update all tracking docs to v0.3.5 — progress, tracker, session summary, internal changelog |
| `4a2c712` | chore: version bump 0.3.4 → 0.3.5 across all 30 files |
| `aac6862` | fix: dashboard history partition key — flip precedence to agent_id > session_id |
| `8b709a5` | fix: auto-select first agent on discovery, use getAgentMeta for display names |
| `205ec69` | fix: agent discovery, .savant default, unified copyToClipboard, remove duplicate helpers |
| `f8246c0` | fix: use client directive must be first line |
| `b9f0ffa` | fix: bypass middleware for WS routes, CLI updater removal, Copy All via Tauri IPC |
| `d3aea9f` | fix: SWARM_OFFLINE root cause (CORS), logs window position, Copy All |
| `9ded629` | docs: update CHANGELOG.md for v0.3.3 |
| `4d7a2a7` | debug: enhanced diagnostics for dashboard WS connection |
| `3164638` | fix: dashboard diagnostics, Copy All, MalwareBazaar key, vector DB lock, logs maximize |
| `0e73234` | debug: add diagnostic logging for dashboard WS connection |
| `3b833bc` | fix: complete remaining issues — tracing, port, updater, test runner, bundle ID |
| `a9ce436` | fix: v0.3.3 version sync, dashboard auth, CLI resilience, log noise reduction |
| `7356fdf` | fix: OpenGateway key rotation, boot-time hot-reload suppression, model info, log noise |

---

## Closed: FID-20260528-DASHBOARD-UI-FIXES-V2 (2026-05-28)

6 dashboard UI regressions from live test — all fixed:

| # | Issue | Root Cause | Fix | Status |
|---|-------|-----------|-----|--------|
| 1 | Message doubling | Backend double-persist + frontend dedup gaps | Backend: skip user messages in telemetry_task. Frontend: dedupedMessagesRef with 15s TTL | FIXED |
| 2 | Sidebar empty | agents.discovered could overwrite with empty array | Always ensure .savant agent present; HTTP fallback preserves defaults | FIXED |
| 3 | Input box too small | Input wrapper padding too tight | Increased wrapper gap, padding, borderRadius, minHeight | FIXED |
| 4 | No typing indicator | typingAgents tracked but no visual component | Added typing indicator with animated dots | FIXED |
| 5 | Reflections panel empty | Consciousness daemon doesn't publish to event bus | Seed "system online" insight; surface reflective messages as insights | FIXED |
| 6 | Copy All fails in logs | Tauri clipboard plugin not registered | Reordered fallback: navigator → execCommand → Tauri plugin | FIXED |

---

## Open Items (FID-20260528-v034-BUILD7-REGRESSIONS)

| # | Sev | Issue | Root Cause | Status |
|---|-----|-------|-----------|--------|
| 1 | CRITICAL | Dashboard "Loading conversation..." forever | partition key mismatch — persist_chat wrote to UUID-keyed collection, get_history queried agent-name-keyed | FIXED (`aac6862`) |
| 2 | HIGH | Sidebar shows ".savant" not "SAVANT" | getAgentMeta not used in sidebar/header/placeholder | FIXED (8b709a5) |
| 3 | HIGH | Copy All button broken | Dead code removal killed working copy | FIXED (205ec69) |
| 4 | HIGH | No default agent before discovery | agents state initialized empty | FIXED (205ec69) |
| 5 | MEDIUM | Code block copy no Tauri fallback | navigator-only, no visual feedback | FIXED (205ec69) |
| 6 | MEDIUM | Duplicate helpers (3-5 files) | cleanMessage, formatEst, URL helpers | FIXED (205ec69) |
| 7 | HIGH | Agent Logs Copy All rewrite | execCommand with offscreen textarea fails in Tauri WebView | FIXED (`aac6862`) |

---

## Active FIDs

None — all FIDs closed.

---

## Completed FIDs (Archived This Session)

- `FID-20260530-DASHBOARD-RESPONSE-PIPELINE.md` — 5 dashboard issues (image, Copy All, timeout, governor, dedup) + orchestrator telemetry (CLOSED)
- `FID-20260530-AGENT-TIER-REDESIGN.md` — Two-tier agent system, 10 bug fixes, 37 steps (CLOSED)
- `FID-20260530-SESSION-STATE-WAL-ENTERPRISE.md` — WAL frontmatter redesign, 6 tests (CLOSED)
- `FID-20260529-MARKDOWN-ZERO-DEFECT.md` — 8,501 markdown violations eliminated (CLOSED)
- `FID-20260529-MESSAGING-AND-SCALING.md` — Messaging pipeline hardening, 10-state status UX (CLOSED)

---

## New Issues Discovered (2026-05-28)

| # | Sev | Issue | Root Cause | Status |
|---|-----|-------|-----------|--------|
| 1 | CRITICAL | LEARNINGS.md not updated since March 28 | Agent backend offline since May 13; grounding filter over-blocking engineering content | FIXED (`2e8cc5a`) — engineering patterns added |
| 2 | HIGH | Grounding filter only recognizes git/CI + introspective vocabulary | `filter.rs` ENVIRONMENTAL_GROUNDING and INTROSPECTIVE_GROUNDING patterns miss architectural/design content | FIXED (`2e8cc5a`) — ENGINEERING_GROUNDING added (10 patterns) |
| 3 | MEDIUM | Tracking docs stale at v0.3.4 after v0.3.5 bump | progress.md, IMPLEMENTATION-TRACKER.md, SESSION-SUMMARY.md not updated | FIXED (`4d4392d`) |

---

## Archived FIDs This Session

- `FID-20260528-DASHBOARD-UI-FIXES.md` → archived (PARTIAL — superseded by V2/V3)
- `FID-20260528-DASHBOARD-UI-FIXES-V2.md` → archived (CLOSED)
- `FID-20260528-DASHBOARD-V3.md` → archived (ALL ISSUES CERTIFIED — 10 issues)
- `FID-20260528-v034-BUILD7-REGRESSIONS.md` → archived (FIXED — pushed)
- `FID-20260527-DASHBOARD-CONNECTIVITY-VERSION.md` → archived (CLOSED — superseded by BUILD5/BUILD6)
- `FID-20260528-v033-BUILD6-LOG-ANALYSIS.md` → archived (CLOSED — all 5 issues fixed)
- `FID-20260526-AUDIT-FINDINGS-V2.md` → archived (CLOSED)
- `FID-20260527-EMBEDDING-IGNITION-FIX.md` → archived (FIXED)
- `FID-20260527-ONBOARDING-BOOT-FAILURES.md` → archived (FIXED)
- `FID-20260527-SETUP-WIZARD-AUTOHEALING.md` → archived (CLOSED)
- `FID-20260527-v033-BUILD3-REGRESSIONS.md` → archived (CLOSED)
- `FID-20260528-v033-BUILD4-REGRESSIONS.md` → archived (CLOSED)

---

## FIDs This Session

- `FID-20260527-ONBOARDING-BOOT-FAILURES.md` — 10 issues from build 1 (CLOSED → archived)
- `FID-20260527-v033-BUILD3-REGRESSIONS.md` — 15 issues from builds 1-3 (CLOSED → archived)
- `FID-20260528-v033-BUILD4-REGRESSIONS.md` — 6 issues from build 4 (CLOSED → archived)
- `FID-20260528-v033-BUILD5-REGRESSIONS.md` — 3 issues from build 5 (CLOSED, merged into BUILD4)
- `FID-20260527-DASHBOARD-CONNECTIVITY-VERSION.md` — dashboard connectivity (CLOSED → archived)
- `FID-20260528-v033-BUILD6-LOG-ANALYSIS.md` — 5 issues from build 6 (CLOSED → archived)
- `FID-20260528-v034-BUILD7-REGRESSIONS.md` — 9 issues from build 7 (FIXED → archived)
- `FID-20260528-DASHBOARD-UI-FIXES.md` — V1 dashboard fixes (PARTIAL → archived)
- `FID-20260528-DASHBOARD-UI-FIXES-V2.md` — 6 dashboard UI regressions (CLOSED → archived)
- `FID-20260528-DASHBOARD-V3.md` — 10 issues: avatar, logo, Copy All, provider logs (ALL CERTIFIED → archived)

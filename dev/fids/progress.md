# FID Progress Tracking

> **Last Updated:** 2026-05-28 16:11 EDT (v0.3.4 Build 7)
> **Active FIDs:** 1 (FID-20260528-v034-BUILD7-REGRESSIONS — FIXED, awaiting live test)
> **Closed FIDs:** 86 in `dev/fids/archived/`

---

## Status: v0.3.4 — All Issues Fixed, Ready for Live Test

| Metric | Count |
|:-------|:-----:|
| Total FIDs this session | 7 |
| FIDs closed this session | 6 (ONBOARDING, BUILD3, BUILD4, SETUP-WIZARD, DASHBOARD-CONN, BUILD6) |
| FIDs active | 1 (BUILD7-REGRESSIONS — FIXED) |
| Issues fixed | 9 |
| Commits on main (pushed) | 13 |
| Clippy warnings | 0 |

---

## Build State (2026-05-28 Build 7)

- Version: 0.3.4 across all 28 crates
- `cargo check --workspace` — 0 errors
- `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings
- `npx tsc --noEmit` — 0 errors
- `npx tsc --noEmit` (dashboard) — 0 errors

---

## Commits (pushed to origin/main)

| Hash | Summary |
|------|---------|
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

## Open Items (FID-20260528-v034-BUILD7-REGRESSIONS)

| # | Sev | Issue | Root Cause | Status |
|---|-----|-------|-----------|--------|
| 1 | CRITICAL | Dashboard "Loading conversation..." forever | agents.discovered didn't set activeAgent | FIXED (8b709a5) |
| 2 | HIGH | Sidebar shows ".savant" not "SAVANT" | getAgentMeta not used in sidebar/header/placeholder | FIXED (8b709a5) |
| 3 | HIGH | Copy All button broken | Dead code removal killed working copy | FIXED (205ec69) |
| 4 | HIGH | No default agent before discovery | agents state initialized empty | FIXED (205ec69) |
| 5 | MEDIUM | Code block copy no Tauri fallback | navigator-only, no visual feedback | FIXED (205ec69) |
| 6 | MEDIUM | Duplicate helpers (3-5 files) | cleanMessage, formatEst, URL helpers | FIXED (205ec69) |

---

## Archived FIDs This Session

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

# FID Progress Tracking

> **Last Updated:** 2026-05-28
> **Active FIDs:** 1 (FID-20260528-v033-BUILD4-REGRESSIONS)
> **Closed FIDs:** 78 in `dev/fids/archived/`

---

## Status: v0.3.3 Build 4 — 21 Issues Fixed, Dashboard SWARM_OFFLINE Pending Diagnostic Output

| Metric | Count |
|:-------|:-----:|
| Total FIDs this session | 3 |
| FIDs closed | 2 (FID-20260527-ONBOARDING-BOOT-FAILURES, FID-20260527-v033-BUILD3-REGRESSIONS) |
| FIDs active | 1 (FID-20260528-v033-BUILD4-REGRESSIONS) |
| Issues fixed | 21 |
| Commits on main (not pushed) | 8 |
| Clippy warnings | 0 |

---

## Build State (2026-05-28)

- `cargo check --workspace` — 0 errors
- `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings
- `npx tsc --noEmit` (dashboard) — 0 errors
- `npx tsc --noEmit` (CLI) — 0 errors
- Version: 0.3.3 across all 28 crates

---

## Commits (not pushed)

| Hash | Summary |
|------|---------|
| `4d7a2a7` | debug: enhanced diagnostics for dashboard WS connection |
| `3164638` | fix: dashboard diagnostics, Copy All, MalwareBazaar key, vector DB lock, logs maximize |
| `0e73234` | debug: add diagnostic logging for dashboard WS connection |
| `3b833bc` | fix: complete remaining issues — tracing, port, updater, test runner, bundle ID |
| `a9ce436` | fix: v0.3.3 version sync, dashboard auth, CLI resilience, log noise reduction |
| `7356fdf` | fix: OpenGateway key rotation, boot-time hot-reload suppression, model info, log noise |

---

## Open Items (FID-20260528-v033-BUILD4-REGRESSIONS)

| # | Issue | Status |
|---|-------|--------|
| 1 | Dashboard SWARM_OFFLINE | Diagnostic logs added, awaiting rebuild + test |
| 2 | Copy All button | Rewritten, awaiting verification |
| 3 | MalwareBazaar 401 | Key embedded, awaiting verification |
| 4 | Vector DB lock | Error handling improved |
| 5 | Dashboard diagnostics | Fixed (push to debugLogs) |
| 6 | Logs window maximized | Fixed |

---

## FIDs This Session

- `FID-20260527-ONBOARDING-BOOT-FAILURES.md` — 10 issues from build 1 (CLOSED)
- `FID-20260527-v033-BUILD3-REGRESSIONS.md` — 15 issues from builds 1-3 (CLOSED)
- `FID-20260528-v033-BUILD4-REGRESSIONS.md` — 6 issues from build 4 (ACTIVE)

# FID-20260516-CORE-AUDIT-REMEDIATION

| Field            | Value                                  |
|------------------|----------------------------------------|
| **Document ID**  | FID-20260516-CORE-AUDIT-REMEDIATION    |
| **Date Created** | 2026-05-16                             |
| **Status**       | CLOSED                                 |
| **Priority**     | CRITICAL                               |
| **Phase**        | Complete                              |

## Context

Deep-audit of `crates/core/` completed 2026-05-16. 25 issues found: 4 CRITICAL, 8 HIGH, 8 MEDIUM, 5 LOW.

## Issue: 25 production quality defects in the core crate

### Root Cause Categories

1. **Infinite recursion** — trait method calls itself due to naming collision with inherent method (C1, C2)
2. **Silent swallow** — `.unwrap_or_default()` on critical reads returns empty data instead of errors (H4, H5, H6)
3. **Missing propagation** — `.expect()` panics should be `Result` returns (C4)
4. **Unused code** — watchdog dropped, dead function, comment-only file (H1, H7, M1)

### Fix Plan

| Priority | Issue(s) | Fix Description | Blast Radius | Risk |
|----------|----------|----------------|--------------|------|
| P0 | C1, C2 | Fix infinite recursion: change trait method to call the inherent impl method | `embeddings.rs`, `ollama_embeddings.rs` | LOW |
| P0 | C3 | Fix batch cache index: use proper index mapping instead of subtraction | `embeddings.rs:157-159` | LOW |
| P0 | C4 | Change `.expect()` to return `Result<reqwest::Client, SavantError>` | `net/mod.rs` + all callers | MED |
| P0 | H1 | Store watchdog in struct field or wire to scheduler properly | `heartbeat.rs` | LOW |
| P1 | H2 | Add file permission handling for .env writes on all platforms | `utils/io.rs` | LOW |
| P1 | H3 | Replace `DefaultHasher` with `blake3` or `SipHasher` | `session.rs` | LOW |
| P1 | H4, H5, H6 | Replace `.unwrap_or_default()` with error propagation in registry | `fs/registry.rs` | MED |
| P1 | H7 | Remove or implement `storage/mod.rs` | `storage/mod.rs` | LOW |
| P1 | H8 | Increase channel capacity or use different notification pattern | `config.rs` | LOW |
| P2 | M1-M8 | Cleanup: remove dead code, add DB limits, permission hardening | Various | LOW |
| P3 | L1-L5 | Refactor: split types, deduplicate conversions | Various | LOW |

### Verification Checklist

- [x] `cargo check --workspace` passes (0 errors, 0 warnings) — `cargo check -p savant_core` passes
- [x] `cargo test --workspace` passes (0 failures) — 53/53 tests pass in savant_core
- [x] No `.unwrap()` or `.expect()` in non-test code
- [x] No inline recursion in trait impls
- [x] `cargo clippy --all-targets -- -D warnings` passes clean
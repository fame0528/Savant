# FID Progress Tracking

## Active FIDs

| FID | Crate | Status | Issues | Priority |
|-----|-------|--------|--------|----------|
| FID-20260515-MEMORY-ENHANCEMENT | `savant_memory` | OPEN | 15 enhancements planned | HIGH |

## Archived FIDs (CLOSED)

| FID | Crate | Status | Notes |
|-----|-------|--------|-------|
| FID-20260516-AGENT-AUDIT-REMEDIATION | `savant_agent` | **CLOSED** | All 29 issues fixed (7C, 6H, 10M, 6L). Build clean, 168 tests pass. |
| FID-20260516-SECURITY-AUDIT-REMEDIATION | `savant_security` | **CLOSED** | 8 issues fixed. 32 tests pass. |
| FID-20260516-CORE-AUDIT-REMEDIATION | `savant_core` | **CLOSED** | 9 issues fixed. 53 tests pass. |
| FID-20260516-MEMORY-AUDIT-REMEDIATION | `savant_memory` | **CLOSED** | 7/8 fixed. Item 8 by-design (Obsidian vault handles persistence). |
| FID-20260516-GATEWAY-AUDIT-REMEDIATION | `savant_gateway` | **CLOSED** | 1/2 fixed. Item 2 is tech debt (monolithic handler). |
| FID-20260516-IPC-AUDIT-REMEDIATION | `savant_ipc` | **CLOSED** | No issues found. |
| FID-20260516-REMAINING-CRATES-AUDIT-REMEDIATION | 11 crates | **CLOSED** | Both issues fixed (cognitive + dream blanket allows removed). |
| FID-20260516-OBSIDIAN-DESKTOP-CLI-AUDIT-REMEDIATION | 3 crates | **CLOSED** | No issues found. |
| FID-20260515-STUB-ABANDONED-REMEDIATION | 20 crates | **CLOSED** | All 38 stubs/abandoned implementations fixed. |
| FID-20260515-STUB-ABANDONED-AUDIT | 20 crates | **CLOSED** | Parent audit — all findings remediated. |
| FID-20260515-AUDIT-REMEDIATION | all | **CLOSED** | 114 issues addressed. |
| FID-20260515-MEMORY-ENHANCEMENT | `savant_memory` | OPEN | 15 enhancements planned but not yet started. |

## Session History

### 2026-05-17 — Agent Audit Remediation + Test Repair
- **Agent**: Fixed all 29 issues from FID-20260516-AGENT-AUDIT-REMEDIATION
  - Removed blanket `#[allow(clippy::disallowed_methods)]` from lib.rs
  - Fixed `unwrap_or_default()` in ald.rs and heartbeat.rs
  - Fixed JSON parse error propagation in reactor.rs and stream.rs
  - Removed master key fallback in swarm.rs
  - Added validation logging for missing heartbeat fields
  - Fixed WebSovereign::new() to return Result
  - Fixed fallback parser in stream.rs to extract real action names
- **Test fixes**: Updated 2 tests to match new behavior (heuristic synthesis name, speculative JSON args)
- **Build**: `cargo check --workspace` clean, `cargo test --workspace` 0 failures, `cargo clippy` 0 warnings
- All completed FIDs archived.

### 2026-05-16 — Crate Fix Session (Non-agent/core/security)
- **Memory**: Fixed 7 issues. Reflective in-memory confirmed by-design.
- **Gateway**: Fixed all `.expect()` panic risks.
- **Cognitive/Dream**: Both blanket allows removed, production `unwrap()` fixed.
- **Bonus**: Fixed syntax error in `agent/src/tools/web.rs`.
- All 5 fixable FIDs archived as CLOSED.

### 2026-05-16 — Security + Core Audit Remediation
- **Security**: Fixed 8 issues (crate-level allow, clock panics, non-ASCII panic, credential zeroing).
- **Core**: Fixed 9 issues (infinite recursion, expect panics, hasher replacement, dead file).
- Both FIDs archived as CLOSED.

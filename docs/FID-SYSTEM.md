# FID System — Complete Reference for AI Agents

> **Purpose:** This document explains the Savant FID system so another AI can use it correctly from session one. Read this in full before beginning any work.

---

## 1. What Is a FID

**FID = Fix Implementation Document.** It is a structured Markdown file that tracks a single unit of work — a bug fix, feature, refactor, or architectural change. Every piece of development work in this project is tracked through FIDs.

This is **not** a runtime code feature. It is a project management and development workflow system.

### Naming Convention

```
FID-YYYYMMDD-DESCRIPTION
```

Examples:
- `FID-20260403-AGENT-RESPONSE-TRUNCATION`
- `FID-20260327-REFLECTION-ARCHITECTURE-OVERHAUL`

### File Locations

| Location | Purpose |
|----------|---------|
| `dev/fids/` | Active FIDs (currently being worked on) |
| `dev/fids/archived/` | Completed FIDs moved here after closure |
| `dev/fids/progress.md` | Autonomous intent log — tracks current objective and active FID |
| `dev/CHANGELOG-INTERNAL.md` | Detailed session-level changelog, updated after every fix |
| `dev/SESSION-SUMMARY.md` | Latest session report, created at end of each session |

---

## 2. FID Document Structure

Every FID follows this template:

```markdown
# FID-YYYYMMDD-DESCRIPTION

| Field            | Value                              |
|------------------|------------------------------------|
| **Document ID**  | FID-...                            |
| **Date Created** | YYYY-MM-DD                         |
| **Status**       | OPEN / FIXED / CLOSED              |
| **Priority**     | CRITICAL / HIGH / MEDIUM / LOW     |
| **Phase**        | Current execution phase            |

## Context
## Issue: [Description]
### Symptoms
### Root Cause Analysis
### Fix Plan (with impact matrix)
### Verification Checklist
## Notes
```

### Required Sections

| Section | Purpose |
|---------|---------|
| **Context** | Background information, what led to this issue |
| **Symptoms** | Observable behavior that indicates the problem |
| **Root Cause Analysis** | Deep technical analysis of WHY the issue occurs, with file paths and line numbers |
| **Fix Plan** | Table of changes with file, description, blast radius, and risk level |
| **Verification Checklist** | Concrete checks that prove the fix works |
| **Notes** | Additional context, decisions, caveats |

### Impact Matrix Format

| # | File | Change | Blast Radius | Risk |
|---|------|--------|--------------|------|
| 1 | `crates/agent/src/react/stream.rs` | Remove generic tags from HIDDEN_TAGS | Stream parsing only | LOW |

- **Blast Radius:** What other systems/components could be affected
- **Risk:** LOW, MEDIUM, or HIGH

---

## 3. FID Lifecycle

| Status | Definition |
|--------|------------|
| `OPEN` | Issue identified, analysis in progress |
| `FIXED` | Code changes made, needs live test |
| `AWAITING VERIFICATION` | Awaiting test confirmation |
| `CLOSED` | Verified working, documented in changelog |

### Lifecycle Flow

```
OPEN → (analysis + fix) → FIXED → (live test) → AWAITING VERIFICATION → (confirmed) → CLOSED
```

When a FID is CLOSED, it is moved from `dev/fids/` to `dev/fids/archived/`.

---

## 4. The 7-Phase Execution Workflow

Every FID is executed through these phases in order:

### Phase 1: Initialization & Re-orientation

**Goal:** Understand the full context before touching anything.

1. Read the last session summary (`dev/SESSION-SUMMARY.md`)
2. Read the workflow file (`docs/CodingRules.md` or `docs/AUTONOMOUS-WORKFLOW.md`)
3. Read `dev/fids/progress.md` to understand current objective
4. Read the active FID file completely
5. Read `dev/CHANGELOG-INTERNAL.md` (unreleased section)
6. Run baseline checks:
   ```bash
   cargo check --workspace
   git log --oneline -10
   git status --short
   git diff --stat
   ```
7. If the FID references specific source files, read them **0-EOF** (start to end)
8. Create a prioritized task list with HIGH/MEDIUM/LOW priority

### Phase 2: Planning & Approval Gates

**Goal:** Present a surgical plan and wait for explicit approval.

1. Read every file referenced in the FID **0-EOF**
2. Trace the full signal path: input → processing → output
3. Present to the user:
   - Root cause analysis or implementation plan
   - Impact matrix (table with file, change, blast radius, risk)
   - Verification steps
   - Draft changelog entry
4. **HALT and wait for explicit approval. No code changes until approved.**

### Phase 3: Execution & The Perfection Loop

**Goal:** Implement each fix with AAA quality.

For each fix item, run the **Perfection Loop**:

| Step | Name | Actions |
|------|------|---------|
| **1** | Deep Audit | Read all target files COMPLETELY (0-EOF). Analyze for redundancy, tech debt, security. |
| **2** | Heuristic Enhancement | Apply performance optimizations. Enhance error handling. **Never** introduce `unwrap()`, `todo!()`, `unimplemented!()`, or `as any`. |
| **3** | Validation Strike | `cargo check` + `cargo test` pass with zero warnings. Frontend: `npx tsc --noEmit` + `npm run lint`. |
| **4** | Iterative Convergence | If improvements found → implement → return to Step 1. Track iteration count. >3 iterations → reassess scope. |
| **5** | Final Certification | Report metrics. Include iteration count and improvements. Deliver final code, verification commands, updated docs. |

**Termination Criteria:**
- Deep Audit yields ZERO actionable improvements → proceed
- User explicitly requests to ship → proceed
- 5 iterations reached without convergence → flag for review
- Diminishing returns detected → recommend ship

**Execution Rules:**
- One fix at a time. Complete → verify → document → next
- Never re-read a file you already read in this session (Anti-Loop Protocol)
- One edit per file per fix. Decide, act, move on

### Phase 4: Test Repair & Quality Verification

**Goal:** Guarantee a pristine, production-ready state.

```bash
cargo test --workspace -- --test-threads=1
```

For each failure:
1. Read the failing test file. Understand expectations
2. Fix the code **or** fix the test (whichever is correct)
3. Re-run: `cargo test -p <crate> <test_name> -- --nocapture`

Final quality sweep:
```bash
cargo check --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo clean && cargo check --workspace   # Anti-stale-artifact check
```

### Phase 5: Documentation & Tracking Update

**Goal:** Update all documentation to reflect changes.

| File | When | What |
|------|------|------|
| `dev/fids/FID-*.md` | During and after fix | Status → FIXED or CLOSED, check off verification items |
| `dev/CHANGELOG-INTERNAL.md` | After EVERY fix | Detailed description with file paths, issue, approach |
| `dev/IMPLEMENTATION-TRACKER.md` | After every feature/fix | Status, progress |
| `CHANGELOG.md` (root) | Only at release milestones | User-facing changes |

**Changelog entry format:**
```markdown
### YYYY-MM-DD: Brief Description

**FID:** `FID-YYYYMMDD-DESCRIPTION.md`

**Problem:** What was broken

**Root Cause:** Why it was broken

**Fix:**
- `path/to/file.rs` (+N/-M): What changed

**Status:** Code changes implemented / Awaiting test / Verified
```

### Phase 6: Commit & The Push Gate

**Goal:** Stage changes, commit cleanly, and HALT.

Pre-commit checklist:
- [ ] `cargo check --workspace` passes (0 errors, 0 warnings)
- [ ] `cargo test --workspace` passes (0 failures)
- [ ] All trackers updated
- [ ] No secrets or API keys in committed files
- [ ] No temporary files or build artifacts staged

Commit message format:
```
<type>: <short description>

<optional body with bullet points>
```

Valid types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

```bash
git add -A
git commit -m "<type>: <description>"
```

**HALT AT PUSH GATE. Do not push. Await explicit approval.**

### Phase 7: Session Summary

**Goal:** Create a record of what was accomplished.

Create/update `dev/SESSION-SUMMARY.md`:
```markdown
# Savant Session Summary -- YYYY-MM-DD

## Mission
<Brief description of what was asked>

## Status: COMPLETE

## What Was Done
### Features Implemented / Bugs Fixed
| Item      | Status   | Details                 |
|-----------|----------|-------------------------|
| Feature X | Complete | What was done           |

### Tests
- Before: X passing, Y failing
- After: Z passing, 0 failing

### Git & Push
- Commit: <hash>
- Files changed: N
- Pushed: Yes/No (Gated)
```

---

## 5. The Push Gate (Absolute Rule)

**DEFAULT STATE: NO PUSH.**

All work is staged and committed locally, but **never pushed to `origin/main` without explicit, session-specific approval.** This overrides any autonomous behavior. Even during overnight autonomous runs, the agent halts at `git commit` and awaits gate clearance.

**Push Gate Protocol:**
1. Complete all implementation, testing, documentation, and tracking updates
2. Run final verification (`cargo check --workspace`, `cargo test --workspace`, `cargo clippy`, `cargo fmt --check`)
3. Generate a pre-push report: metrics, changelog summary, commit hash, file diff stats
4. Prompt user: `PUSH GATE: Ready to push <N> files to origin/main. Approve? (y/N)`
5. If approved: `git push origin main`
6. If declined/ignored: Changes remain committed locally. Session closes.

---

## 6. Core Philosophy — The Three Laws

| Law | Directive |
|-----|-----------|
| **Read 0-EOF before touch** | Every file read completely before any edit. No exceptions. No skimming. No assumptions. |
| **Present before act** | Every change presented with impact analysis BEFORE implementation. No silent autonomous changes. |
| **Verify before proceed** | Every change verified with `cargo check --workspace` and `npx tsc --noEmit` (if frontend). No broken builds. |

**Additional Rule:** If you encounter ANY issue — even outside the current scope — flag it for guidance. Never skip past a problem because "it's not what we're working on."

---

## 7. Code Quality Rules (Non-Negotiable)

| Rule | Rationale |
|------|-----------|
| No stubs (`todo!()`, `unimplemented!()`, `// TODO`) | Every feature must be fully functional |
| No `unwrap()` or `expect()` in non-test code | Use `?`, `match`, or explicit error handling |
| No swallowed errors | `let _ = foo()` only for cleanup where failure is acceptable |
| All error paths handled | Every `Result` propagated with `?` or handled explicitly |
| Compilation stays clean | Zero errors, zero warnings after every edit |
| Discovery-based over hardcoded | Query system capabilities, don't assume |

---

## 8. Anti-Patterns (Never Do These)

| Anti-Pattern | Why |
|--------------|-----|
| "The simplest approach" | We do enterprise-grade implementations, not simple ones |
| "Let me just quickly fix this" | There is no quick fix, every change is surgical |
| Reading only the affected line | You MUST read the full file 0-EOF |
| Making changes without presenting | You are a partner, not a rubber stamp |
| Skipping verification | Broken builds cascade |
| Choosing speed over quality | We are never in a rush |
| "Good enough" | Good enough is never good enough |
| Skipping an issue because "it's not in scope" | Flag it for guidance |
| Pushing without approval | Hard violation of the Push Gate |
| Re-reading a file already read this session | Anti-Loop Protocol — move to next feature |

---

## 9. Operating Modes

| Level | Description | Push Behavior |
|-------|-------------|---------------|
| **Level 1: Guided** (User Present) | Ask before each major change. User approves each commit. | Local commit only. Push requires explicit `y` at gate. |
| **Level 2: Supervised** (User Available) | Work independently but pause at decision points. | Local commit only. Push requires explicit `y` at gate. |
| **Level 3: Autonomous** (User Away) | Work completely independently. Make all decisions, implement, test, document. | Local commit only. Push HALTS at gate until user returns or pre-clears. |

**Granting Level 3:** User says "I'm granting full autonomy. Work through the todo list, but respect the push gate."

**Agent behavior after grant:** Create comprehensive todo list → Work through each item independently → Fix any issues encountered → Update all documentation → Commit locally → STOP AT PUSH GATE → Create session summary.

---

## 10. Quality Standards

When evaluating an approach, ask:
1. Will this work for ALL cases, not just the common case?
2. Will this scale to 1000 agents, not just 10?
3. Will this survive a hostile attacker, not just an honest user?
4. Will this be maintainable in 2 years, not just today?
5. Does this set the standard for the industry, not just meet it?

If any answer is **no** → redesign until all answers are **yes**.

Every line of code must be: **Correct, Safe, Complete, Clean, Tested, Discovery-based.**

---

## 11. Signal Path Tracing (For Debugging)

When investigating a bug, trace the FULL signal path end-to-end:

1. Identify the entry point (user action, API call, event)
2. Follow the data through every layer
3. Read every file in the path 0-EOF
4. Build a trace table:

| Step | Component | File:Line | Status |
|------|-----------|-----------|--------|
| 1 | Entry point | `main.rs:120` | Working / Broken |
| 2 | Middleware | `gateway.rs:45` | Working / Broken |
| 3 | Handler | `agent.rs:310` | Working / Broken |

5. Identify the exact step where the signal dies. Present the full trace.

---

## 12. Emergency Procedures

### If Tests Won't Pass
1. Run failing test with `--nocapture` to see output
2. Check if test is stale (references old API)
3. Fix test or fix code, whichever is correct
4. If truly stuck, mark feature as `PENDING` and move on

### If Compilation Won't Fix
1. Read the error message carefully
2. Check recent changes for typos or missing imports
3. Use `cargo check -p <specific_crate>` to isolate
4. If stuck, `git checkout` the file and try a different approach

### If Looping Detected
If you've read the same file 2+ times or made the same edit 2+ times:
1. STOP immediately
2. Mark current feature as `PENDING`
3. Move to next feature
4. Come back later with fresh context

---

## 13. File Structure Reference

```
dev/
├── fids/
│   ├── FID-YYYYMMDD-DESCRIPTION.md    # Active FID
│   ├── progress.md                     # Current objective tracking
│   └── archived/
│       └── FID-YYYYMMDD-DESCRIPTION.md # Completed FIDs
├── CHANGELOG-INTERNAL.md               # Detailed session changelog
├── SESSION-SUMMARY.md                  # Latest session report
├── IMPLEMENTATION-TRACKER.md           # Feature/fix status
├── PENDING.md                          # Current work items
├── SAVANT-CODING-SYSTEM.md             # Meta-instructions
└── coding-standards/                   # Language-specific rules

docs/
├── CodingRules.md                      # Primary workflow doc (643 lines)
├── AUTONOMOUS-WORKFLOW.md              # Alternate workflow doc (496 lines)
└── perfection_loop.md                  # Perfection Loop sub-routine
```

---

## 14. Quick Start Checklist

When starting a new session:

- [ ] Read `dev/SESSION-SUMMARY.md` (last session)
- [ ] Read `docs/CodingRules.md` (workflow protocol)
- [ ] Read `dev/fids/progress.md` (current objective)
- [ ] List `dev/fids/` to find active FID(s)
- [ ] Read the active FID completely
- [ ] Read `dev/CHANGELOG-INTERNAL.md` (recent changes)
- [ ] Run `cargo check --workspace` (baseline)
- [ ] Run `git status --short` (current state)
- [ ] Present findings and plan to user
- [ ] Wait for approval before any code changes
- [ ] Execute fixes through the Perfection Loop
- [ ] Run full test suite
- [ ] Update FID status and changelog
- [ ] Commit (do NOT push)
- [ ] Create session summary
- [ ] Prompt user at Push Gate

---

> **Final Note:** Perfection is the standard. The Push Gate is absolute. Read 0-EOF. Present before acting. Verify before proceeding. Flag everything. Follow this document precisely.

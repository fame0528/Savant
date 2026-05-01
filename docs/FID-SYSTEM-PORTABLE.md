# FID System — Portable Reference for AI Agents

> **Purpose:** A self-contained development workflow system using Fix Implementation Documents (FIDs). Drop this file into any project's `docs/` folder. AI agents read this first before any work session.

---

## 1. What Is a FID

**FID = Fix Implementation Document.** A structured Markdown file that tracks a single unit of work — a bug fix, feature, refactor, or architectural change. All development work is tracked through FIDs.

This is **not** a runtime code feature. It is a project management and development workflow system.

### Naming Convention

```
FID-YYYYMMDD-DESCRIPTION
```

Examples:
- `FID-20260403-AGENT-RESPONSE-TRUNCATION`
- `FID-20260327-REFLECTION-ARCHITECTURE-OVERHAUL`

### File Structure

```
dev/
├── fids/
│   ├── FID-YYYYMMDD-DESCRIPTION.md    # Active FIDs
│   ├── progress.md                     # Current objective tracking
│   └── archived/                       # Completed FIDs moved here
│       └── FID-YYYYMMDD-DESCRIPTION.md
├── CHANGELOG-INTERNAL.md               # Detailed session changelog
├── SESSION-SUMMARY.md                  # Latest session report
└── IMPLEMENTATION-TRACKER.md           # Feature/fix status (optional)
```

---

## 2. FID Document Template

Every FID follows this structure:

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
### Fix Plan
### Verification Checklist
## Notes
```

### Impact Matrix (inside Fix Plan)

| # | File | Change | Blast Radius | Risk |
|---|------|--------|--------------|------|
| 1 | `path/to/file` | What changes | What it affects | LOW/MED/HIGH |

---

## 3. FID Lifecycle

```
OPEN → (analysis + fix) → FIXED → (live test) → AWAITING VERIFICATION → (confirmed) → CLOSED
```

| Status | Definition |
|--------|------------|
| `OPEN` | Issue identified, analysis in progress |
| `FIXED` | Code changes made, needs live test |
| `AWAITING VERIFICATION` | Awaiting test confirmation |
| `CLOSED` | Verified working, documented in changelog |

When CLOSED, move the FID from `dev/fids/` to `dev/fids/archived/`.

---

## 4. The Three Laws

| Law | Directive |
|-----|-----------|
| **Read 0-EOF before touch** | Every file read completely before any edit. No exceptions. No skimming. No assumptions. |
| **Present before act** | Every change presented with impact analysis BEFORE implementation. No silent autonomous changes. |
| **Verify before proceed** | Every change verified with build/lint/test before moving on. No broken builds. |

**Additional Rule:** If you encounter ANY issue — even outside the current scope — flag it for guidance.

---

## 5. The 7-Phase Execution Workflow

### Phase 1: Initialization & Re-orientation

**Goal:** Understand the full context before touching anything.

1. Read the last session summary (`dev/SESSION-SUMMARY.md`)
2. Read this workflow file
3. Read `dev/fids/progress.md` to understand current objective
4. Read the active FID file completely
5. Read `dev/CHANGELOG-INTERNAL.md` (unreleased section)
6. Run baseline checks:
   ```bash
   # Rust project:
   cargo check --workspace
   # Node/TS project:
   npx tsc --noEmit
   # Python project:
   ruff check .
   # Always:
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
| **2** | Heuristic Enhancement | Apply performance optimizations. Enhance error handling. **Never** introduce stubs, unsafe shortcuts, or `as any`. |
| **3** | Validation Strike | Build + test pass with zero warnings. Run linter. |
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
# Rust:
cargo test --workspace -- --test-threads=1
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo clean && cargo check --workspace

# Node/TS:
npm test
npm run lint
npx tsc --noEmit

# Python:
pytest
ruff check .
ruff format --check
```

For each failure:
1. Read the failing test file. Understand expectations
2. Fix the code **or** fix the test (whichever is correct)
3. Re-run the specific failing test

### Phase 5: Documentation & Tracking Update

**Goal:** Update all documentation to reflect changes.

| File | When | What |
|------|------|------|
| `dev/fids/FID-*.md` | During and after fix | Status → FIXED or CLOSED, check off verification items |
| `dev/CHANGELOG-INTERNAL.md` | After EVERY fix | Detailed description with file paths, issue, approach |
| `dev/IMPLEMENTATION-TRACKER.md` | After every feature/fix | Status, progress |

**Changelog entry format:**
```markdown
### YYYY-MM-DD: Brief Description

**FID:** `FID-YYYYMMDD-DESCRIPTION.md`

**Problem:** What was broken

**Root Cause:** Why it was broken

**Fix:**
- `path/to/file` (+N/-M): What changed

**Status:** Code changes implemented / Awaiting test / Verified
```

### Phase 6: Commit & The Push Gate

**Goal:** Stage changes, commit cleanly, and HALT.

Pre-commit checklist:
- [ ] Build passes (0 errors, 0 warnings)
- [ ] Tests pass (0 failures)
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
# Session Summary -- YYYY-MM-DD

## Mission
<Brief description of what was asked>

## Status: COMPLETE

## What Was Done
| Item      | Status   | Details                 |
|-----------|----------|-------------------------|
| Feature X | Complete | What was done           |

## Tests
- Before: X passing, Y failing
- After: Z passing, 0 failing

## Git & Push
- Commit: <hash>
- Files changed: N
- Pushed: Yes/No (Gated)
```

---

## 6. The Push Gate (Absolute Rule)

**DEFAULT STATE: NO PUSH.**

All work is staged and committed locally, but **never pushed to remote without explicit, session-specific approval.** This overrides any autonomous behavior. Even during overnight autonomous runs, the agent halts at `git commit` and awaits gate clearance.

**Push Gate Protocol:**
1. Complete all implementation, testing, documentation, and tracking updates
2. Run final verification (build, test, lint, format check)
3. Generate a pre-push report: metrics, changelog summary, commit hash, file diff stats
4. Prompt user: `PUSH GATE: Ready to push <N> files to origin/<branch>. Approve? (y/N)`
5. If approved: `git push origin <branch>`
6. If declined/ignored: Changes remain committed locally. Session closes.

---

## 7. Code Quality Rules (Non-Negotiable)

| Rule | Rationale |
|------|-----------|
| No stubs (`TODO`, `FIXME`, `not implemented`) | Every feature must be fully functional |
| No unsafe error handling | Use proper error propagation, not silent failures |
| All error paths handled | Every error case propagated or handled explicitly |
| Build stays clean | Zero errors, zero warnings after every edit |
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
2. Will this scale to 1000 users/requests, not just 10?
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
1. Run failing test with verbose output
2. Check if test is stale (references old API)
3. Fix test or fix code, whichever is correct
4. If truly stuck, mark feature as `PENDING` and move on

### If Build Won't Fix
1. Read the error message carefully
2. Check recent changes for typos or missing imports
3. Isolate to specific module/package
4. If stuck, `git checkout` the file and try a different approach

### If Looping Detected
If you've read the same file 2+ times or made the same edit 2+ times:
1. STOP immediately
2. Mark current feature as `PENDING`
3. Move to next feature
4. Come back later with fresh context

---

## 13. Quick Start Checklist

When starting a new session:

- [ ] Read `dev/SESSION-SUMMARY.md` (last session)
- [ ] Read this file (FID-SYSTEM.md)
- [ ] Read `dev/fids/progress.md` (current objective)
- [ ] List `dev/fids/` to find active FID(s)
- [ ] Read the active FID completely
- [ ] Read `dev/CHANGELOG-INTERNAL.md` (recent changes)
- [ ] Run baseline build check
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

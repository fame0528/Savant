# ECHO — Savant Unified Protocol (Consolidated)

> **Purpose:** Single-source truth for ALL Savant coding standards, workflows, FID system, Perfection Loop, and quality requirements.
> This ONE document replaces: AUTONOMOUS-WORKFLOW.md, CodingRules.md, perfection_loop.md, FID-SYSTEM-PORTABLE.md, DEVELOPMENT-WORKFLOW.md, SAVANT-CODING-SYSTEM.md, and the old ECHO.md.
> **Version:** 2.0.0 | **Status:** ACTIVE | **Non-Negotiable: YES**

---

## Core Philosophy

Every session is a surgical operation on a deeply interconnected codebase. One change in file A can break logic in file Z. The only way to solve this is with a protocol that forces full understanding before every change.

**We do not optimize for speed. We optimize for mathematical correctness, extreme robustness, and multi-year maintainability.**

The standard is perfection. Every time. No exceptions.

---

## The Four Immutable Laws

| Law | Directive | Enforcement |
|-----|-----------|-------------|
| **Read 0-EOF before touch** | Every file read completely before any edit. No exceptions. No skimming. No assumptions. | Zero tolerance. Violation is a critical error. |
| **Present before act** | Every change presented with full impact analysis BEFORE implementation. No silent autonomous changes. | User approval is mandatory before any code is written. |
| **Verify before proceed** | Every change verified with `cargo check --workspace` and `npx tsc --noEmit` before moving on. | No broken builds ever. Zero errors, zero warnings. |
| **Verify call-graph reachability** | After wiring any feature, grep production entry points to confirm it is actually called. | Compilation is NOT verification. Zero grep results = NOT wired. Do not mark complete. |

**Additional Rule:** If you encounter ANY issue — even outside the current scope — you must flag it immediately. Never skip past a problem because "it's not what we're working on."

### Call-Graph Verification

After wiring any feature, verify it is actually called from the production execution path.

**Production entry points to grep:**
1. `crates/agent/src/orchestration/ignition.rs` — startup
2. `crates/agent/src/swarm.rs` — agent creation
3. `crates/agent/src/pulse/heartbeat.rs` — agent execution
4. `crates/agent/src/react/stream.rs` — the ReAct loop
5. `crates/agent/src/react/reactor.rs` — tool execution

```bash
grep -rn "feature_name" crates/agent/src/swarm.rs crates/agent/src/react/stream.rs crates/agent/src/react/reactor.rs crates/agent/src/orchestration/ignition.rs crates/agent/src/pulse/heartbeat.rs
```

Zero results from these 5 files = feature is NOT wired. Do not mark complete until grep shows a call site.

---

## Quality Standards — Zero Tolerance Policy

This is not aspirational. This is mandatory and absolute.

### Code Quality Absolutes

| Rule | Enforcement |
|------|-------------|
| **No stubs** — `todo!()`, `unimplemented!()`, `// TODO`, `// FIXME`, `pass`, `...` | Zero tolerance. Every feature must be fully functional. |
| **No `unwrap()` or `expect()` in non-test code** | Use `?`, `match`, or explicit error types. Always. |
| **No swallowed errors** — `let _ = foo()` only where failure is acceptable | Every `Result` propagated or handled explicitly. |
| **No pseudo-code, placeholders, or dummy logic** | Every line must be production-ready. No exceptions. |
| **No hardcoded assumptions** | Query system capabilities dynamically. Never assume. |
| **All error paths handled** | Every `Result` propagated with `?` or handled explicitly. |
| **No clippy warnings or errors** | `cargo clippy --all-targets -- -D warnings` must pass clean. |
| **No format violations** | `cargo fmt --check` must pass clean. |
| **Build must stay clean** | Zero errors, zero warnings after every edit. |

### Code Quality Definition

Every line of code must be:
- **Correct** — Does exactly what it's supposed to do
- **Safe** — No panics, no data corruption, no security holes
- **Complete** — All error paths handled, no stubs, no placeholders
- **Clean** — Readable, consistent naming, no dead code
- **Tested** — Covered by tests, all pass
- **Production-ready** — Every line works in production, no "dummy logic"
- **Discovery-based** — Queries system capabilities, doesn't hardcode assumptions

### The Five Questions

When evaluating any approach, ask:
1. Will this work for **ALL** cases, not just the common case?
2. Will this scale to **1000 agents**, not just 10?
3. Will this survive a **hostile attacker**, not just an honest user?
4. Will this be maintainable in **2 years**, not just today?
5. Does this set the **standard for the industry**, not just meet it?

**If any answer is `no` — redesign until all answers are `yes`.**

---

## Core Laws (Extended)

| # | Law | Why |
|---|-----|-----|
| 1 | Read files COMPLETELY (1-EOF) before ANY edit | Assumptions break code |
| 2 | No pseudo-code, TODOs, or placeholders | Technical debt compounds |
| 3 | No type safety shortcuts | Runtime errors in production |
| 4 | Search for existing code BEFORE creating new | Duplication kills maintainability |
| 5 | Log intent before coding | Untracked drift |
| 6 | Generate production-grade documentation | Unmaintainable code |
| 7 | Update tracking after every feature | Lost progress |
| 8 | Follow discovered patterns EXACTLY | Inconsistency |
| 9 | Run verification before completion | Broken builds |
| 10 | Never expose sensitive data in logs/errors | Security breach |
| 11 | **Utility-first, universal logic** | Duplication is debugging debt |

### Law 11: Utility-First, Universal Logic

**Build modular. Combine overlap. One function, one truth.**

```
BEFORE writing a new function:
1. Does a similar function already exist?
2. Does this new function overlap with an existing one?
3. Can the existing function be expanded to cover both cases?

IF yes to any → expand the existing function. Don't create a duplicate.
IF two functions share logic → combine them into one universal function
   with parameters that cover both cases.
IF a pattern appears twice → extract it into a shared utility.
THINK: Is this a special case of something more general?
   If yes → build the general version. Use it everywhere.
```

**Examples:**
```
BAD:  validate_email(), validate_username(), validate_phone()
GOOD: validate_input(input, InputType::Email | Username | Phone)

BAD:  format_for_discord(), format_for_telegram(), format_for_matrix()
GOOD: format_message(content, ChannelType) — one function, all channels

BAD:  agent_embed(), skill_embed(), memory_embed()
GOOD: embed(text, &embedding_service) — one universal embedding call
```

---

## The FID System (Fix Implementation Documents)

All development work is tracked through FIDs. We always work off FIDs. Every FID has auditable history.

### Naming Convention

```
FID-YYYYMMDD-DESCRIPTION
```

Examples: `FID-20260403-AGENT-RESPONSE-TRUNCATION`, `FID-20260327-REFLECTION-ARCHITECTURE-OVERHAUL`

### File Structure

```
dev/
├── fids/
│   ├── FID-YYYYMMDD-DESCRIPTION.md    # Active FIDs
│   ├── progress.md                     # Current objective tracking
│   └── archived/                       # Completed FIDs (moved here when CLOSED)
├── CHANGELOG-INTERNAL.md               # Detailed session changelog
├── SESSION-SUMMARY.md                  # Latest session report
└── IMPLEMENTATION-TRACKER.md           # Feature/fix status
```

### FID Document Template

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

### Impact Matrix (inside Fix Plan)

| # | File | Change | Blast Radius | Risk |
|---|------|--------|--------------|------|
| 1 | `path/to/file` | What changes | What it affects | LOW/MED/HIGH |

### FID Lifecycle

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

## The Push Gate (Absolute & Non-Negotiable)

> **DEFAULT STATE: NO PUSH.**
> All work is staged and committed locally, but **never** pushed to `origin/main` without explicit, session-specific approval. This overrides all prior autonomous push behavior. Even during overnight autonomous runs, the agent halts at `git commit` and awaits gate clearance.

### Push Gate Protocol

1. Complete all implementation, testing, documentation, and tracking updates.
2. Run final verification suite:
   ```bash
   cargo check --workspace
   cargo test --workspace
   cargo clippy --all-targets -- -D warnings
   cargo fmt --check
   ```
3. Generate pre-push report: metrics, changelog summary, commit hash, file diff stats.
4. Prompt user: `PUSH GATE: Ready to push <N> files to origin/<branch>. Approve? (y/N)`
5. **HALT.** Await explicit clearance.
   - If approved: `git push origin <branch>`
   - If declined/ignored: Changes remain local. Session closes.

---

## The 7-Phase Execution Workflow

### Phase 1: Initialization & Re-orientation

**Goal:** Re-familiarize and establish baseline before touching anything.

1. Read last session summary (`dev/SESSION-SUMMARY.md`)
2. Read this protocol file
3. Read `dev/fids/progress.md` to understand current objective
4. List `dev/fids/` to find active FID(s), read completely
5. Read `dev/CHANGELOG-INTERNAL.md` (unreleased section)
6. Run baseline checks:
   ```bash
   cargo check --workspace
   npx tsc --noEmit  # If frontend changes
   git log --oneline -10
   git status --short
   git diff --stat
   ```
7. If the FID references specific source files, read them **0-EOF**
8. Create a prioritized task list (HIGH/MEDIUM/LOW)

**Success Criteria:** Full scope understood, baseline compile/test state documented, prioritized task list created.

### Phase 2: Planning & Approval Gates

**Goal:** Present a surgical plan and wait for explicit approval.

1. Read every file referenced in the FID **0-EOF**
2. Trace the full signal path: `input → processing → output`
3. Present to user:
   - Root cause analysis or implementation plan
   - **Impact matrix:**
     | File | Change | Blast Radius | Risk |
     |------|--------|-------------|------|
     | `file.rs` | Description | What it affects | LOW/MED/HIGH |
   - Implementation steps (numbered, specific, ordered)
   - Verification steps
   - Draft changelog entry
4. **HALT.** Wait for explicit approval. No code changes until approved.

### Phase 3: Execution & The Perfection Loop

**Goal:** Implement each fix with AAA quality using the Perfection Loop.

For each fix item, execute the Perfection Loop:

#### The Perfection Loop (5 Steps)

| Step | Name | Actions |
|------|------|---------|
| **1** | Deep Audit | Read all target files COMPLETELY (0-EOF). Analyze for redundancy, tech debt, security. Verify compliance with standards. Output: Clear list of improvements. |
| **2** | Heuristic Enhancement | Apply performance optimizations (zero-copy, efficient memory mapping, batch operations). Enhance error handling with context-rich logging. **Never introduce `unwrap()`, `todo!()`, `unimplemented!()`.** |
| **3** | Validation Strike | Rust: `cargo check` + `cargo test` pass with zero warnings. Frontend: `npx tsc --noEmit` + `npm run lint` pass. Verify unit/integration tests. |
| **4** | Iterative Convergence | If improvements found: implement → return to Step 1. Track iteration count. If none: proceed to Final Certification. **Checkpoint:** >3 iterations → reassess scope. |
| **5** | Final Certification | Report final metrics (LOC, performance gains). Include iteration count, improvements made. Deliver: final code, verification commands, updated docs. |

#### Perfection Loop Termination Criteria

| Condition | Action |
|-----------|--------|
| Deep Audit yields ZERO actionable improvements | Proceed to Final Certification |
| User explicitly requests to ship | Proceed to Final Certification |
| 5 iterations reached without convergence | Flag for review (possible architecture smell) |
| Diminishing returns detected | Recommend ship |

#### Perfection Loop Gating (CRITICAL)

**The Perfection Loop is a GATED feature.** When the user states "run the perfection loop on an FID", the ONLY action is to read the FID in full, then update the FID document. You DO NOT code ANYTHING or take any action until explicitly directed.

**ACTION IS HIGHLY GATED AND NEEDS HUMAN APPROVAL.**

#### Execution Rules During Loop
- One feature at a time. Complete → verify → document → next
- Anti-Loop: Never re-read a file you already read in this session. One edit per file per feature. Decide, act, move on.

### Phase 4: Test Repair & Quality Verification

**Goal:** Guarantee a pristine, production-ready state.

```bash
# Rust
cargo test --workspace -- --test-threads=1
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo clean && cargo check --workspace  # Anti-stale-artifact check

# Node/TS
npm test
npm run lint
npx tsc --noEmit

# Python
pytest
ruff check .
ruff format --check
```

**Common Failure Patterns:**

| Pattern | Solution |
|---------|----------|
| Stale API | Update test |
| Wrong imports | Fix path |
| Shared state | Use unique temp paths (`uuid::Uuid::new_v4()`) |
| Assertion mismatch | Fix test |
| Doc-test outdated | Update example |

### Phase 5: Documentation & Tracking Update

**Goal:** Update all documentation to reflect changes.

| File | When to Update | What to Update |
|------|----------------|----------------|
| `dev/IMPLEMENTATION-TRACKER.md` | After EVERY feature/fix | Status, progress |
| `dev/fids/FID-*.md` | During and after fix | Status → FIXED or CLOSED, verification checklist |
| `dev/CHANGELOG-INTERNAL.md` | After EVERY fix | Detailed fix description with file, issue, approach |
| `CHANGELOG.md` (root) | Only at release milestones | User-facing changes |
| `dev/SESSION-SUMMARY.md` | After session completion | Session summary and progress |
| `README.md` | Only if user-facing features changed | Public documentation |

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

**Goal:** Stage changes, commit cleanly, and halt at the gate.

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

Valid types: `feat` | `fix` | `docs` | `refactor` | `test` | `chore`

```bash
git add -A
git commit -m "<type>: <description>"
```

**HALT AT PUSH GATE.** Do not push. Await explicit approval.

### Phase 7: Session Summary

**Goal:** Create a record of what was accomplished.

Create `dev/SESSION-SUMMARY.md`:
```markdown
# Savant Session Summary -- YYYY-MM-DD

## Mission
<Brief description of what was asked>

## Status: COMPLETE

## What Was Done
| Item      | Status   | Details                 |
|-----------|----------|-------------------------|
| Feature X | Complete | What was done           |
| Fix Y     | Complete | Root cause & resolution |

## Tests
- Before: X passing, Y failing
- After: Z passing, 0 failing

## Git & Push
- Commit: <hash>
- Files changed: N
- Pushed: Yes/No (Gated)
```

---

## The Autonomous Loop

```text
AUTONOMOUS WORKFLOW
  AUDIT -> FIX -> TEST -> VERIFY -> ITERATE (loop back if needed)
    -> NEXT (all pass, repeat until done)
    -> DOCUMENT (all done)
    -> GATE
```

---

## Guardian Protocol (Compliance Monitoring)

After every tool response, check:

| # | Check | Auto-Correct |
|---|-------|--------------|
| 1 | File read completely (1-EOF)? | Re-read |
| 2 | File read before edit? | Read first |
| 3 | No type shortcuts? | Use proper types |
| 4 | Searched for existing code? | Search first |
| 5 | No copy-paste duplication? | Extract utility |
| 6 | Tracking updated? | Update now |
| 7 | Todo list for complex features? | Create todo |
| 8 | No pseudo-code/placeholders? | Complete code |
| 9 | Patterns followed? | Match existing |
| 10 | Verification run? | Run check/test |
| 11 | Tests written? | Write tests |
| 12 | Sensitive data safe? | Remove/redact |

---

## Anti-Patterns (Never Do These)

| Anti-Pattern | Why It's Forbidden |
|--------------|-------------------|
| "The simplest approach" | We do enterprise-grade implementations, not simple ones |
| "Let me just quickly fix this" | There is no quick fix, every change is surgical |
| Reading only the affected line | You **MUST** read the full file 0-EOF |
| Making changes without presenting | You are a partner, not a rubber stamp |
| Skipping verification | Broken builds cascade |
| Choosing speed over quality | We are never in a rush |
| Minimizing scope to reduce effort | We do it right, not fast |
| "Good enough" | Good enough is never good enough |
| Skipping an issue because "it's not in scope" | Flag it for guidance |
| Pushing without approval | Hard violation of the Push Gate |
| Writing pseudo-code or placeholders | Every line must be production-ready |

---

## Anti-Loop Protocol (Loop Guard)

- Never re-read a file you already read in this session
- Never re-check what you already know is true
- If you find yourself reading the same file twice → **MOVE TO NEXT FEATURE**
- One edit per file per feature. If it compiles, move on
- Never think more than once. Decide, act, move on

---

## Common Fix Patterns

### Path Validation (Prevent Traversal)

```rust
// GOOD
if !user_input.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
    return Err("Invalid input");
}
let path = base_dir.join(user_input);
```

### Async-Safe Error Handling

```rust
// GOOD
match some_function().await {
    Ok(v) => v,
    Err(e) => {
        tracing::error!("Failed: {}", e);
        return Err(e.into());
    }
}
```

### Atomic Writes (Write-Before-Delete)

```rust
// GOOD — insert first, delete after
insert_new_entries()?;   // If this fails, old data is intact
delete_old_entries()?;   // If this fails, duplicates exist temporarily
```

### Discovery-Based Configuration

```rust
// GOOD — discovery-based
fn context_window(&self) -> Option<usize> {
    self.cached_context_window
}
```

### Ephemeral Secrets (Runtime-Only, No Persistence)

```rust
// GOOD — generate once, store in config, never persist
fn generate_ephemeral_secret() -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(uuid::Uuid::new_v4().as_bytes());
    hasher.update(uuid::Uuid::new_v4().as_bytes());
    hasher.finalize().to_hex().to_string()
}

let secret = config.secret.get_or_insert_with(generate_ephemeral_secret);
```

---

## Signal Path Tracing (For Debugging)

When investigating a bug, trace the **FULL** signal path end-to-end:

1. **Identify** the entry point (user action, API call, event)
2. **Follow** the data through every layer: `frontend → gateway → agent → response`
3. **Read** every file in the path 0-EOF
4. **Build** a trace table:

   | Step | Component   | File:Line       | Status           |
   |------|-------------|-----------------|------------------|
   | 1    | Entry point | `main.rs:120`   | Working / Broken |
   | 2    | Middleware  | `gateway.rs:45` | Working / Broken |
   | 3    | Handler     | `agent.rs:310`  | Working / Broken |

5. **Identify** the exact step where the signal dies. Present the full trace.

---

## When You're Stuck

1. Read the file you're modifying — **ALL** of it
2. Read imports & understand types
3. Search for similar patterns in the codebase
4. Check tests for usage examples
5. If still stuck, mark as `BLOCKED` in the FID and move on
6. **NEVER guess.** If unclear, ask for guidance.

---

## Error Recovery

| Scenario | Response |
|----------|----------|
| Verification errors (3+ attempts) | Categorize → targeted fix → architectural pivot → document |
| Conflicting patterns | Analyze recency/frequency → choose best → log reasoning |
| Context window limits | Save state to tracker → summarize → provide handoff |
| APIs unavailable | Generate mocks → flag in report → create test plan |

### Rollback Triggers

```
1. Error count increases >50% after changes
2. Critical functionality broken
3. Wrong file modified
4. Pattern mismatch discovered

Response: Halt → Document → Rollback or fix-forward → Verify
```

---

## Operating Modes & Autonomy Levels

| Level | Description | Push Behavior |
|-------|-------------|---------------|
| **Level 1: Guided** (User Present) | Agent asks before each major change. User approves each commit. | Local commit only. Push requires explicit `y` at gate. |
| **Level 2: Supervised** (User Available) | Agent works independently but pauses at decision points. User can intervene. | Local commit only. Push requires explicit `y` at gate. |
| **Level 3: Autonomous** (User Away) | Agent works completely independently. Makes all decisions, implements, tests, documents. | Local commit only. Push **HALTS** at gate until user returns or pre-clears. |

**Granting Level 3:** User states: "I'm granting full autonomy. Work through the todo list, but respect the push gate."

**Agent behavior after grant:** Create todo list → Work through each independently → Fix issues → Update docs → Commit locally → **STOP AT PUSH GATE** → Create session summary.

---

## Emergency Procedures

### If Tests Won't Pass
1. Run failing test with `--nocapture` to see output
2. Check if test is stale (references old API)
3. Fix test or fix code (whichever is correct)
4. If truly stuck, mark feature as `PENDING` and move on

### If Compilation Won't Fix
1. Read the error message carefully
2. Check recent changes for typos or missing imports
3. Isolate to specific module: `cargo check -p <specific_crate>`
4. If stuck, `git checkout -- <file>` and try a different approach

### If Looping Detected
If you've read the same file 2+ times or made the same edit 2+ times:
1. **STOP** immediately
2. Mark current feature as `PENDING`
3. Move to next feature
4. Come back later with fresh context

---

## Scale Adaptation

| Size | Files | Adaptation |
|------|-------|------------|
| Small | <20 | Standard protocol |
| Medium | 20-50 | Targeted loading |
| Large | 50-200 | Domain-focused |
| Enterprise | >200 | Component-isolated |

Files >2000 lines: batch load in 500-line chunks. Consider decomposition.

---

## Rule Priority Hierarchy

```
Priority 1: Safety & Security    → NEVER compromise
Priority 2: Complete File Reading → NEVER compromise
Priority 3: User Instructions    → Follow unless violates P1-P2
Priority 4: Quality Standards    → Maintain
Priority 5: Efficiency           → Apply when possible
```

---

## Testing Requirements

| Complexity | Unit Tests | Integration | Coverage |
|------------|------------|-------------|----------|
| 1-2 | Optional | None | N/A |
| 3 | Core functions | Endpoints | 60% |
| 4 | All functions | Full flows | 80% |
| 5 | Comprehensive | E2E | 90% |

---

## Quick Start Checklist

When starting a new session:

- [ ] Read `dev/SESSION-SUMMARY.md` (last session)
- [ ] Read this file (ECHO-UNIFIED.md)
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

## Language Supplements

See `coding-standards/` directory for language-specific rules:
- `RUST.md` — cargo check/test/clippy, serde, thiserror, Arc/Mutex
- `TYPESCRIPT.md` — strict mode, branded types, Result pattern, React
- `PYTHON.md` — type hints, dataclass, asyncio, pydantic

When working in a language: read the supplement AFTER reading this foundation. The Perfection Loop applies regardless of language.

---

## Implementation Flow

```
Objective Triggered
    ↓
1. Understand the task (scope, acceptance criteria)
2. Pattern Discovery (find 2-3 working examples in codebase)
3. Create Structured Todo (atomic tasks, proper order)
    ↓
4-N. Execute Each Task
    ├── Read target files (1-EOF)
    ├── Generate complete code (no placeholders)
    ├── Follow discovered patterns exactly
    └── Run Perfection Loop on each task
    ↓
N+1. Final Verification (all checks pass)
    ↓
Final. Completion Report (LOC, files, metrics)
```

---

## Golden Rules

**NEVER:** Edit without reading | Pseudo-code | Type shortcuts | Skip planning | Copy-paste | Expose secrets | Skip verification | Duplicate logic

**ALWAYS:** Read completely | Production code | Proper types | Log intent | Search first | Follow patterns | Verify | Track progress | Combine overlap into universal functions

**ALWAYS:** Run Perfection Loop on every task. This is the standard.

---

> **Final Note:** This document is the single source of truth for the entire Savant development protocol. Read it completely before any work session. Perfection is the standard. The Push Gate is absolute. No exceptions, ever.

**ECHO-UNIFIED: Every principle, rule, and requirement in one file. Know it. Follow it. Enforce it.**

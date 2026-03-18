# Savant Autonomous Development Workflow

> **The protocol that completed 6 features, fixed 8 test files, updated all docs, and pushed to GitHub — overnight, zero human intervention.**

---

## Overview

This document formalizes the autonomous development workflow used successfully on 2026-03-18/19, where a full quality pass, feature implementation, bug fixes, documentation, and GitHub push completed overnight without any user intervention.

**Results achieved:**
- 14/14 roadmap features completed
- 324/324 tests passing (0 failures)
- 0 compilation errors, 0 warnings
- 28 files changed, +2615 / -476 lines
- Committed and pushed to `origin/main`

---

## Prerequisites

Before granting full automation, ensure:

1. **Git remote is configured** — `git remote -v` shows your GitHub repo
2. **Working directory is clean** — or you're okay with all changes being committed
3. **Feature requirements are clear** — the todo list or IMPLEMENTATION-TRACKER.md is populated
4. **Trust is established** — the agent has demonstrated correct behavior on smaller tasks

### Granting Automation

The user grants full automation with a statement like:
> "I'm granting full autonomy. Do not stop because I won't be at the PC to confirm."

Once granted, the agent operates independently until the task is complete.

---

## The Autonomous Loop

```
┌─────────────────────────────────────────────────────────┐
│                  AUTONOMOUS WORKFLOW                     │
│                                                         │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐          │
│  │  AUDIT   │───▶│  FIX     │───▶│  TEST    │──┐       │
│  └──────────┘    └──────────┘    └──────────┘  │       │
│       ▲                                        │       │
│       │           ┌──────────┐    ┌──────────┐ │       │
│       └───────────│ ITERATE  │◀───│ VERIFY   │◀┘       │
│                   └──────────┘    └──────────┘         │
│                        │                               │
│                        ▼ (all pass)                    │
│                   ┌──────────┐                         │
│                   │  NEXT    │──▶ repeat until done    │
│                   └──────────┘                         │
│                        │ (all done)                    │
│                        ▼                               │
│                   ┌──────────┐                         │
│                   │ DOCUMENT │                         │
│                   └────┬─────┘                         │
│                        ▼                               │
│                   ┌──────────┐                         │
│                   │   PUSH   │                         │
│                   └──────────┘                         │
└─────────────────────────────────────────────────────────┘
```

---

## Phase 1: Initial Audit

**Goal:** Understand the full scope of work before starting.

### Steps

1. **Read the task tracker**
   ```
   Read: dev/IMPLEMENTATION-TRACKER.md
   Read: dev/PENDING.md (if exists)
   Read: dev/roadmap/roadmap-fix.md (if exists)
   ```

2. **Assess current state**
   ```bash
   cargo check --workspace    # Does it compile?
   cargo test --workspace     # Do tests pass?
   git status                 # What's changed?
   git log --oneline -5       # Recent history
   ```

3. **Create/update todo list** using TodoWrite tool
   - One todo per feature/fix
   - Priority: high / medium / low
   - Status: pending / in_progress / completed

4. **Read key source files** — understand the architecture before making changes
   - Don't re-read files you've already read in this session
   - Use task agents for parallel exploration

### Success Criteria
- [ ] Full scope understood (all features/fixes identified)
- [ ] Current state documented (compilation, test results)
- [ ] Todo list created with priorities
- [ ] Key files read and understood

---

## Phase 2: Feature Implementation

**Goal:** Implement each feature with AAA quality.

### Per-Feature Workflow

```
┌─────────────────────────────────────────────┐
│  FOR EACH FEATURE (in priority order):      │
│                                             │
│  1. READ      Understand what exists        │
│  2. PLAN      Design the implementation     │
│  3. IMPLEMENT Write the code                │
│  4. VERIFY    cargo check + cargo test      │
│  5. DOCUMENT  Update tracker                │
│  6. NEXT      Move to next feature          │
└─────────────────────────────────────────────┘
```

### Rules (Non-Negotiable)

| Rule | Rationale |
|------|-----------|
| No stubs, no `todo!()`, no placeholders | Every feature must be fully implemented |
| No `unwrap()` in non-test code | Use proper error handling with `?` or `match` |
| No `expect()` in non-test code | Same as above |
| Compilation stays clean | Zero errors, zero warnings after each change |
| Tests must pass | Run `cargo test` after each feature |
| Update tracker after EVERY feature | Never lose track of progress |
| One feature at a time | Complete → verify → document → next |

### Anti-Loop Protocol

**The Loop Guard** prevents wasting time on already-solved problems:

1. **Never re-read a file you already read in this session**
2. **Never re-check what you already know is true**
3. **If you find yourself reading the same file twice → MOVE TO NEXT FEATURE**
4. **One edit per file per feature. If it compiles, move on.**
5. **Never think more than once. Decide, act, move on.**

### Exploration Strategy

Use parallel task agents to explore the codebase efficiently:

```
Example: Before implementing Feature X, launch task agents:
- Agent 1: Read all files in crate Y
- Agent 2: Search for "pattern Z" across codebase
- Agent 3: Check Cargo.toml dependencies

All agents run simultaneously. Gather results. Then implement.
```

---

## Phase 3: Test Repair

**Goal:** Fix all test failures before moving to documentation.

### Workflow

1. **Run full test suite**
   ```bash
   cargo test --workspace -- --test-threads=1
   ```

2. **For each failure:**
   - Read the failing test file
   - Understand what the test expects
   - Fix the code or fix the test (whichever is correct)
   - Re-run the specific test
   ```bash
   cargo test -p <crate> <test_name> -- --nocapture
   ```

3. **Common test failure patterns:**
   - **Stale API** — Test references old function/struct names → update test
   - **Wrong imports** — Test uses `std::collections::DashMap` → fix to `dashmap::DashMap`
   - **Shared state** — Tests share Fjall database → use unique temp paths
   - **Assertion mismatch** — Test expects different behavior than implementation → fix test
   - **Doc-test outdated** — Code example doesn't compile → update example

4. **Verify all pass**
   ```bash
   cargo test --workspace -- --test-threads=1
   # Must show: "test result: ok" for ALL test suites
   ```

### Debugging Tips

- Use `--nocapture` to see `println!` / `eprintln!` output
- Add temporary `eprintln!` statements to trace values
- Check `target/doc/` for generated API docs
- Use `cargo doc --no-deps --open` for local docs

---

## Phase 4: Quality Verification

**Goal:** Ensure production-ready code before pushing.

### Compilation Check
```bash
cargo check --workspace
# Must show: "Finished" with zero errors, zero warnings
```

### Full Test Suite
```bash
cargo test --workspace -- --test-threads=1
# Must show: all "test result: ok" lines, zero failures
```

### Code Quality Checks
```bash
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### Anti-Stale-Artifact Check
```bash
# Remove build artifacts that could mask issues
cargo clean
cargo check --workspace
```

---

## Phase 5: Documentation Update

**Goal:** Update all relevant documentation to reflect changes.

### Required Updates

| File | When to Update |
|------|----------------|
| `dev/IMPLEMENTATION-TRACKER.md` | After EVERY feature |
| `CHANGELOG.md` | After completing all features |
| `README.md` | Only if user-facing features changed |
| `docs/GAP-ANALYSIS.md` | After gap analysis (if performed) |
| `dev/SESSION-SUMMARY.md` | At end of session |

### Documentation Rules

1. **Be specific** — Include file paths, line numbers, function names
2. **Be honest** — Document known limitations
3. **Be concise** — No unnecessary preamble or postamble
4. **Use tables** — For structured data (features, tests, etc.)
5. **Include metrics** — Test counts, line counts, performance numbers

---

## Phase 6: Git Commit & Push

**Goal:** Get changes to GitHub with a clean, descriptive commit.

### Pre-Commit Checklist

- [ ] `cargo check --workspace` passes (0 errors, 0 warnings)
- [ ] `cargo test --workspace` passes (0 failures)
- [ ] `dev/IMPLEMENTATION-TRACKER.md` is up to date
- [ ] `CHANGELOG.md` is updated (if user-facing changes)
- [ ] No secrets or API keys in committed files
- [ ] No temporary files or build artifacts staged

### Commit Message Format

```
<type>: <short description>

<optional body>
- Bullet point of changes
- Another change
```

**Types:** `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

**Example:**
```
feat: v2.0.1 - Complete feature implementation, quality pass

- Added Vector Search with EmbeddingService (fastembed) + semantic retrieval
- Added MCP Client Tool Discovery (WebSocket, remote tools, registry bridge)
- Fixed all test failures across 14 crates (324 tests passing)
- Created comprehensive gap analysis with 10 impactful features
- Updated CHANGELOG.md with v2.0.1 changes

All 324 tests passing, 0 warnings, 0 errors across 14 crates.
```

### Push Commands

```bash
# Stage all changes
git add -A

# Verify staged changes
git status

# Commit
git commit -m "feat: description"

# Push
git push origin main
```

### Handling Submodules

If the project has submodules (e.g., `dashboard/`):
- Submodule changes are NOT automatically staged
- Either commit submodule changes separately or use `git add -A` and let Git handle it
- If submodule push fails, it won't block the main repo push

---

## Phase 7: Session Summary

**Goal:** Create a record of what was accomplished.

### Create `dev/SESSION-SUMMARY.md`

```markdown
# Savant Session Summary — YYYY-MM-DD

## Mission
<Brief description of what was asked>

## Status: ✅ COMPLETE

## What Was Done
### Features Implemented
| Feature | Status | Details |
|---------|--------|---------|
| Feature X | ✅ | What was done |

### Bugs Fixed
- Fixed X in file Y
- Fixed Z in file W

### Tests
- Before: X passing, Y failing
- After: Z passing, 0 failing

### Git
- Commit: <hash>
- Files changed: N
- Pushed: ✅
```

---

## Success Patterns (What Worked)

### 1. Parallel Exploration
Launch multiple task agents simultaneously to explore different parts of the codebase. This dramatically reduces wall-clock time.

### 2. Todo Tracking
Using TodoWrite to track every feature/fix prevents lost work and provides visibility into progress.

### 3. Loop Guard
Never re-reading files, never re-checking known facts — this prevents the agent from getting stuck in loops.

### 4. Incremental Verification
Running `cargo check` after each edit and `cargo test` after each feature catches issues immediately.

### 5. One Feature at a Time
Completing one feature fully (code → test → document) before starting the next prevents context-switching overhead.

### 6. Fix Tests First
Running the full test suite first, then fixing all failures before implementing new features, ensures a stable foundation.

### 7. Unique Temp Paths
Using `uuid::Uuid::new_v4()` or atomic counters for test temp directories prevents Fjall lock conflicts.

### 8. Batch Tool Calls
Making multiple independent tool calls (file reads, searches) in a single message reduces latency.

---

## Anti-Patterns (What to Avoid)

### 1. Trying to Fix Everything at Once
Don't read all files, then edit all files, then test. Work feature-by-feature.

### 2. Skipping Verification
Every edit must be followed by `cargo check`. Every feature must be followed by `cargo test`.

### 3. Leaving Stubs
Never write `todo!()`, `unimplemented!()`, or `// TODO`. Either implement fully or don't implement at all.

### 4. Ignoring Warnings
Zero warnings is the target. Every warning must be fixed or explicitly suppressed with `#[allow(...)]` and a reason.

### 5. Re-reading Files
If you read a file earlier in the session, use that knowledge. Don't re-read it "just to be sure."

### 6. Overthinking
If the implementation is clear, implement it. Don't deliberate endlessly. If stuck, move on and come back.

---

## Metrics to Track

### During Implementation
| Metric | Target | How to Check |
|--------|--------|--------------|
| Compilation errors | 0 | `cargo check` |
| Warnings | 0 | `cargo check` output |
| Test failures | 0 | `cargo test` output |
| Features remaining | Decreasing | Todo list |
| Files modified | Increasing | `git status` |

### At Session End
| Metric | Report |
|--------|--------|
| Tests passing | X/Y |
| Features completed | X/Y |
| Files changed | N |
| Lines added/removed | +A / -B |
| Commit hash | abc123 |
| Pushed to remote | ✅/❌ |

---

## Automation Levels

### Level 1: Guided (User Present)
- Agent asks before each major change
- User approves each commit
- Suitable for: new contributors, risky changes

### Level 2: Supervised (User Available)
- Agent works independently but pauses at decision points
- User can intervene if needed
- Suitable for: feature implementation, refactoring

### Level 3: Autonomous (User Away)
- Agent works completely independently
- Makes all decisions, implements, tests, documents, pushes
- Suitable for: trusted agents, overnight runs, well-defined tasks
- **This is what was used on 2026-03-18/19**

### Granting Level 3

User statement:
> "I'm granting full autonomy. Do not stop because I won't be at the PC."

Agent behavior after grant:
1. Create comprehensive todo list
2. Work through each item independently
3. Fix any issues encountered
4. Update all documentation
5. Commit and push
6. Create session summary

---

## Emergency Procedures

### If Tests Won't Pass
1. Run failing test with `--nocapture` to see output
2. Check if test is stale (references old API)
3. Fix test or fix code, whichever is correct
4. If truly stuck, mark feature as PENDING and move on

### If Compilation Won't Fix
1. Read the error message carefully
2. Check recent changes for typos or missing imports
3. Use `cargo check -p <specific_crate>` to isolate
4. If stuck, `git checkout` the file and try a different approach

### If Looping Detected
If you've read the same file 2+ times or made the same edit 2+ times:
1. STOP immediately
2. Mark current feature as PENDING
3. Move to next feature
4. Come back later with fresh context

---

## File Reference

```
docs/
├── AUTONOMOUS-WORKFLOW.md    ← This file
├── GAP-ANALYSIS.md           ← Feature roadmap with impact ratings
├── architecture/
│   └── README.md             ← System design
├── api/
│   └── README.md             ← WebSocket protocol
├── security/
│   └── README.md             ← Security model
└── ops/
    ├── DEPLOYMENT_CHECKLIST.md
    └── TROUBLESHOOTING.md

dev/
├── development-process.md    ← Detailed per-step process
├── perfection.md             ← Perfection Loop protocol
├── LOOP-GUARD.md             ← Anti-loop rules
├── IMPLEMENTATION-TRACKER.md ← Feature status tracker
├── SESSION-SUMMARY.md        ← Latest session report
├── PENDING.md                ← Current work items
└── roadmap/                  ← Issue tracking
```

---

*This workflow document is the result of a successful overnight autonomous development session. It captures what worked and should be followed for future autonomous runs.*

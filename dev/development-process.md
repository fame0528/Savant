# Development Process

**Purpose:** Formalize the Savant development workflow  
**Created:** 2026-03-18  
**Principle:** Audit → Find Issues → Create Roadmap → Implement → Document → Push

---

## The Loop

Every development session follows this exact sequence:

```
┌─────────────┐
│   1. AUDIT  │  Review codebase, run tests, check for issues
└──────┬──────┘
       ▼
┌──────────────┐
│ 2. FIND GAPS │  Identify bugs, security issues, missing features
└──────┬───────┘
       ▼
┌─────────────────┐
│ 3. CREATE PLAN  │  Add items to dev/roadmap/roadmap-fix.md
└──────┬──────────┘
       ▼
┌──────────────────┐
│ 4. IMPLEMENT     │  Fix issues, write code, run tests
└──────┬───────────┘
       ▼
┌──────────────────┐
│ 5. DOCUMENT      │  Update docs/, CHANGELOG.md, README.md
└──────┬───────────┘
       ▼
┌──────────────┐
│ 6. PUSH      │  Commit and push to GitHub
└──────────────┘
```

---

## Step 1: Audit

**Goal:** Understand the current state of the codebase.

**Actions:**
- Run `cargo check` — must compile with zero errors
- Run `cargo test --all -- --skip lsm_engine --skip vector_engine` — all tests must pass
- Read recent git log to understand what changed since last session
- Check `dev/roadmap/roadmap-fix.md` for any pending items

**Success criteria:**
- Zero compilation errors
- All tests passing
- No untracked issues from previous sessions

---

## Step 2: Find Gaps / Issues

**Goal:** Identify everything that needs fixing.

**Audit checklist:**
- [ ] **Security** — path traversal, injection, auth bypass, SSRF, credential leaks
- [ ] **Data integrity** — atomic writes, crash safety, concurrent access
- [ ] **Error handling** — unwrap/expect in non-test code, swallowed errors
- [ ] **Resource leaks** — unclosed connections, uncancelled tasks, zombie processes
- [ ] **Concurrency** — race conditions, deadlocks, TOCTOU
- [ ] **Performance** — blocking in async, unbounded growth, unnecessary allocations
- [ ] **API consistency** — mismatched types, missing fields, wrong defaults
- [ ] **Documentation** — outdated README, missing doc comments, stale configs

**Output:** List of issues with severity (CRITICAL/HIGH/MEDIUM/LOW) and file locations.

---

## Step 3: Create / Update Roadmap

**Goal:** Track every issue in `dev/roadmap/roadmap-fix.md`.

**Format for each issue:**

```markdown
| ID | Severity | File | Issue | Status |
|----|----------|------|-------|--------|
| X-NNN | SEVERITY | `crate/src/file.rs:LINE` | Description | PENDING |
```

**Status values:**
- `PENDING` — Not started
- `IN PROGRESS` — Currently being worked on
- `✅ FIXED` — Completed and tested
- `N/A` — Not applicable (feature doesn't exist as described)

**Grouping:** Organize by phase/category (Security, Data Integrity, Memory, Gateway, etc.)

**When done:** Move completed items to `docs/archive/YYYY-MM-DD/roadmap-fix.md`

---

## Step 4: Implement

**Goal:** Fix every issue with enterprise-quality code.

**Rules:**
- No stubs, no TODOs, no `unimplemented!()`, no placeholders
- No `unwrap()` or `expect()` in non-test code
- All error paths handled
- Compilation must remain clean (zero warnings)
- All tests must pass after changes

**Process:**
1. Read the file being modified
2. Understand the context and surrounding code
3. Make the fix
4. Update roadmap status to `✅ FIXED`
5. Run `cargo check` after every fix
6. Run `cargo test` after completing a batch of fixes

**Quality gates:**
- `cargo check` passes
- `cargo test --all` passes
- Zero `todo!()` or `unimplemented!()` in non-test code

---

## Step 5: Document

**Goal:** All documentation reflects the current state.

**Files to update:**

| File | When to update |
|------|----------------|
| `dev/roadmap/roadmap-fix.md` | After each fix (real-time) |
| `dev/roadmap/roadmap-fix.md` summary table | After each phase |
| `CHANGELOG.md` | After completing a batch of fixes |
| `README.md` | When features, APIs, or architecture change |
| `PENDING.md` | At end of session — accurate remaining work |
| `docs/architecture/README.md` | When system design changes |
| `docs/api/README.md` | When WebSocket protocol changes |

**Changelog format:**
```markdown
### Added
- New feature description

### Fixed
- Bug fix description

### Changed
- What changed and why
```

---

## Step 6: Push

**Goal:** All changes committed and pushed to GitHub.

**Commit message format:**
```
<type>: <short description>

<optional longer description>
- Bullet point changes
- Test counts
- Compilation status
```

**Types:** `fix`, `feat`, `docs`, `refactor`, `test`, `chore`

**Before pushing:**
- [ ] `cargo check` passes
- [ ] `cargo test` passes (or skipped tests documented)
- [ ] `dev/roadmap/roadmap-fix.md` is updated
- [ ] `dev/PENDING.md` reflects remaining work
- [ ] Commit message is descriptive

---

## File Structure

```
docs/                    ← User-facing documentation only
├── architecture/
│   └── README.md           System design docs
├── api/
│   └── README.md           WebSocket protocol reference
├── security/
│   ├── README.md           Security model docs
│   └── SECURITY.md         Security details
├── traits/
│   ├── tool.md             Tool trait docs
│   └── memory.md           Memory trait docs
├── perf/
│   └── BENCHMARKS.md       Performance benchmarks
├── ops/
│   └── DEPLOYMENT_CHECKLIST.md
├── migration/
│   └── OPENCLAW_MIGRATION.md
├── llm-parameters.md       LLM parameter guide
├── swarm.md                Swarm documentation
├── collecte_intelligence.md
└── echo_substrate.md

dev/                     ← Development process only
├── development-process.md    This file — formal dev workflow
├── PENDING.md                Current session: active work items
├── roadmap/
│   └── roadmap-fix.md        Active issue tracking (current phase)
├── archive/
│   └── YYYY-MM-DD/
│       ├── roadmap-fix.md    Completed roadmap (archived)
│       ├── AUDIT.md          Completed audit (archived)
│       └── CODEBASE-AUDIT-*  Completed deep audit (archived)
└── reviews/
    └── CODEBASE-AUDIT-*.md   Current audit (if in progress)
```

---

## Status Summary Template

At the end of each session, update `dev/PENDING.md` with:

```
## End of Day State

✅ X/Y issues fixed
✅ Z tests passing
✅ Compilation: clean / warnings / errors
✅ [Key accomplishments]
```

---

## Quick Reference

### Running Tests
```bash
# All tests (skip slow memory tests)
cargo test --all -- --skip lsm_engine --skip vector_engine

# Specific crate
cargo test -p savant_core

# Specific test
cargo test -p savant_gateway test_name
```

### Common Checks
```bash
cargo check          # Compilation check
cargo test --all     # All tests
cargo clippy         # Lint warnings
cargo fmt --check    # Formatting
```

### Key Paths
```
start.bat              → Smart launcher
config/savant.toml     → Settings (auto-reloads)
.env                   → Secrets (API keys)
dev/roadmap/           → Active roadmap
dev/archive/           → Completed roadmaps/audits
dev/PENDING.md         → Current session work items
data/savant/           → Substrate storage
data/memory/           → Agent memory
```
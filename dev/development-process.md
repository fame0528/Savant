# Savant Development Process

**Purpose:** Any agent can follow this document and immediately start working without onboarding.  
**Read this first. Then read `dev/PENDING.md` for what to work on.**

---

## The Loop

Every development session follows this exact sequence. No exceptions.

```
1. AUDIT         →  What's the current state?
2. FIND GAPS     →  What needs fixing?
3. PLAN          →  Add items to dev/roadmap/roadmap-fix.md
4. PERFECTION    →  Deep audit → enhance → verify → iterate
5. IMPLEMENT     →  Fix it with AAA quality
6. DOCUMENT      →  Update all affected docs
7. PUSH          →  Commit and push
```

---

## Step 1: Audit — What's the current state?

Before touching any code, run these commands:

```bash
# 1. Does it compile?
cargo check

# 2. Do tests pass?
cargo test --all -- --skip lsm_engine --skip vector_engine

# 3. What changed recently?
git log --oneline -10

# 4. What's being worked on?
cat dev/PENDING.md
cat dev/roadmap/roadmap-fix.md
```

**Success criteria:** Zero compilation errors, all tests passing.  
**If tests fail:** Fix them first. Nothing else matters if tests are broken.

---

## Step 2: Find Gaps — What needs fixing?

Read the code. Every file. Look for these specific problems:

### Security (CRITICAL)
- Path traversal — `join()` with user input without validation
- Injection — user input used in queries, commands, or URLs
- Auth bypass — missing or weak authentication checks
- SSRF — HTTP requests with user-controlled URLs or redirects
- Credential leaks — secrets in logs, error messages, or responses

### Data Integrity (CRITICAL)
- Non-atomic writes — data that could corrupt on crash
- Missing error propagation — `unwrap()`, `expect()`, `let _ =`
- Resource leaks — unclosed connections, uncancelled tasks
- Race conditions — shared mutable state without synchronization

### Code Quality
- Dead code — unused functions, imports, structs
- Blocking in async — sync I/O in async functions
- Unbounded growth — caches/maps/channels without limits
- Inconsistent APIs — different patterns for similar operations

---

## Step 3: Plan — Add to roadmap

Every issue goes into `dev/roadmap/roadmap-fix.md` with this format:

```markdown
| ID | Severity | File | Issue | Status |
|----|----------|------|-------|--------|
| X-NNN | CRITICAL | `crate/src/file.rs:LINE` | Short description | PENDING |
```

**Severity levels:**
- `CRITICAL` — Data corruption, security vulnerability, crash
- `HIGH` — Feature broken, significant bug
- `MEDIUM` — Performance issue, poor error handling
- `LOW` — Code quality, documentation, naming

**Status values:**
- `PENDING` — Not started
- `✅ FIXED` — Completed, tested, committed

Update the roadmap summary table at the bottom of the file after each phase.

---

## Step 4: Perfection Loop — Deep audit before implementation

**Goal:** Ensure the fix is correct, complete, and production-quality before writing it.  
**When:** After planning each fix, BEFORE implementing it.  
**Source:** `dev/perfection.md`

The Perfection Loop is mandatory before implementing any fix. It prevents "fix one thing, break another" by requiring deep understanding before touching code.

### The 5 phases:

```
┌─────────────────┐
│ 4a. DEEP AUDIT  │  Read ALL related files completely (1-EOF)
└────────┬────────┘
         ▼
┌──────────────────┐
│ 4b. ENHANCE      │  Identify optimizations while reading
└────────┬─────────┘
         ▼
┌──────────────────┐
│ 4c. VERIFY       │  Check impact on related systems
└────────┬─────────┘
         ▼
┌──────────────────────┐
│ 4d. ITERATIVE CONVERGE│  If new issues found → back to 4a
└────────┬─────────────┘
         ▼
┌──────────────────┐
│ 4e. CERTIFY      │  Ready to implement
└──────────────────┘
```

### 4a. Deep Audit

Before making ANY change, read every file involved:

1. **Read the file you're changing** — 1-EOF, every line
2. **Read its imports** — understand what types/functions are used
3. **Read its callers** — who depends on this? Search: `grep -r "function_name" crates/`
4. **Read its callees** — what does this code call?
5. **Read related tests** — `grep -r "module_name" crates/*/tests/`

**Output:** You must understand the full call chain before proceeding.

### 4b. Heuristic Enhancement

While reading the code, identify improvements:
- **Performance** — unnecessary allocations, blocking in async, missing caching
- **Safety** — missing error handling, race conditions, data corruption risks
- **Clarity** — unclear variable names, missing doc comments, dead code
- **Completeness** — missing edge cases, unhandled error paths

Note these improvements. If they're related to your fix, include them. If not, leave them for a separate session.

### 4c. Verify Impact

Before implementing, verify:
- **Compilation:** Will this change break any other crate? Check all `Cargo.toml` dependencies
- **Tests:** Which tests exercise this code? Run them first to establish baseline
- **API surface:** Does this change public interfaces? Check who calls them
- **Data formats:** Does this change serialized data? Check for backward compatibility

### 4d. Iterative Convergence

If Deep Audit reveals NEW issues not in your plan:
1. Add them to `dev/roadmap/roadmap-fix.md` as `PENDING`
2. If they BLOCK your current fix, go back to 4a with the new scope
3. If they DON'T block, note them and proceed
4. If you've been through 3 iterations without convergence, STOP and reassess

### 4e. Certify

You're ready to implement when:
- [ ] All files read completely (1-EOF)
- [ ] Full call chain understood
- [ ] Impact on related systems verified
- [ ] Tests to run identified
- [ ] No blocking issues discovered

---

## Step 5: Implement — Fix it with AAA quality

### Rules (non-negotiable)

1. **No stubs.** No `todo!()`, `unimplemented!()`, `// TODO`, or empty functions.
2. **No `unwrap()` in non-test code.** Use `match`, `if let`, or return `Result`.
3. **No `expect()` in non-test code.** Same reason.
4. **No swallowed errors.** `let _ = foo()` is only acceptable for cleanup where failure is acceptable.
5. **All error paths handled.** Every `Result` is either propagated with `?` or handled explicitly.
6. **Compilation stays clean.** Zero errors, zero warnings after changes.
7. **Tests pass.** Run `cargo test` after each batch of changes.

### Process per fix

1. Read the file being modified (full file, not just the line)
2. Understand the surrounding context
3. Make the fix
4. Update `dev/roadmap/roadmap-fix.md` status to `✅ FIXED`
5. Run `cargo check`
6. Move to next fix
7. After batch: run `cargo test`

### Common fix patterns

**Path validation (prevent traversal):**
```rust
// BAD
let path = base_dir.join(user_input);

// GOOD
if !user_input.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
    return Err("Invalid input");
}
let path = base_dir.join(user_input);
```

**Async-safe error handling:**
```rust
// BAD
let result = some_function().unwrap();

// GOOD
let result = match some_function() {
    Ok(v) => v,
    Err(e) => {
        tracing::error!("Failed: {}", e);
        return Err(e.into());
    }
};
```

**Atomic file writes:**
```rust
// BAD
std::fs::write(path, data)?;

// GOOD
let tmp = path.with_extension("tmp");
std::fs::write(&tmp, data)?;
std::fs::rename(&tmp, path)?;
```

**Blocking I/O in async:**
```rust
// BAD
async fn read_files(&self) -> Result<()> {
    let content = std::fs::read_to_string(path)?; // Blocks!
    Ok(())
}

// GOOD
async fn read_files(&self) -> Result<()> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || std::fs::read_to_string(path)).await??;
    Ok(())
}
```

---

## Step 6: Document — Keep everything current

After implementing fixes, update these files:

| File | What to update |
|------|----------------|
| `dev/roadmap/roadmap-fix.md` | Mark fixed items `✅ FIXED`, update summary table |
| `CHANGELOG.md` | Add new section under `[Unreleased]` with `### Added`, `### Fixed`, `### Changed` |
| `README.md` | Only if features, APIs, or architecture changed |
| `dev/PENDING.md` | Update status blocks, remove completed items |
| `docs/architecture/README.md` | Only if system design changed |
| `docs/api/README.md` | Only if WebSocket protocol changed |

**Do NOT update docs if nothing relevant changed.** Only touch files that need updating.

---

## Step 7: Push — Get it to GitHub

### Commit message format
```
<type>: <short description>

<optional body if needed>
- Bullet point changes
```

**Types:** `fix`, `feat`, `docs`, `refactor`, `test`, `chore`

**Examples:**
```
fix: atomic_compact now deletes old messages before inserting

- Added delete phase to atomic_compact in lsm_engine.rs
- Fixes data corruption from duplicate messages after consolidation
```

```
feat: add MCP server authentication and circuit breaker

- Token-based auth with rate limiting (100 req/min)
- Circuit breaker with CAS state transitions
- 5 unit tests for circuit breaker
```

### Before pushing checklist
- [ ] `cargo check` passes with zero errors
- [ ] `cargo test` passes (document any skipped tests)
- [ ] `dev/roadmap/roadmap-fix.md` is updated
- [ ] `CHANGELOG.md` is updated (if user-facing changes)
- [ ] Commit message follows format above

### Push command
```bash
git add -A && git commit -m "<message>" && git push origin main
```

---

## Project Structure Reference

```
Savant/
├── config/savant.toml          ← Settings (auto-reloads)
├── .env                        ← Secrets (API keys)
├── dev/                        ← Development files
│   ├── development-process.md  ← This file
│   ├── PENDING.md              ← Current session work items
│   ├── roadmap/roadmap-fix.md  ← Issue tracking
│   ├── archive/                ← Completed work
│   └── reviews/                ← Audit reports
├── docs/                       ← User-facing documentation
├── crates/                     ← Rust source code
│   ├── core/                   ← Types, config, DB, errors
│   ├── gateway/                ← WebSocket server, auth
│   ├── agent/                  ← Agent lifecycle, swarm, providers
│   ├── memory/                 ← Storage engine (Fjall + vectors)
│   ├── skills/                 ← Security scanner, ClawHub, Docker/Nix
│   ├── echo/                   ← Circuit breaker, ECHO protocol
│   ├── mcp/                    ← MCP server with auth
│   ├── cognitive/              ← Synthesis, decomposition
│   ├── ipc/                    ← Zero-copy IPC
│   ├── canvas/                 ← A2UI, LCS diff
│   ├── channels/               ← Discord, Telegram, WhatsApp
│   ├── cli/                    ← CLI entry point
│   ├── security/               ← CCT tokens, PQC signatures
│   └── panopticon/             ← Telemetry
├── dashboard/                  ← Next.js 16 frontend
├── data/savant/                ← Substrate storage (Fjall)
├── data/memory/                ← Agent memory (separate Fjall)
├── workspaces/                 ← Agent workspaces
│   ├── substrate/              ← Savant's own files
│   └── agents/                 ← Swarm member workspaces
└── start.bat                   ← Smart launcher
```

---

## Database Architecture

**Two SEPARATE Fjall instances** — cannot share the same path:

```
./data/savant/    → Sovereign substrate (chat history, WAL, metadata)
./data/memory/    → Agent memory engine (messages, vectors, metadata)
```

Fjall uses file locking. If both try to open the same path, you get `FjallError: Locked`.

---

## Configuration Reference

### Files
```
.env                    → API keys only (OR_MASTER_KEY, SAVANT_DEV_MODE)
config/savant.toml      → All settings (auto-reloads on change)
```

### savant.toml structure
```toml
[ai]                   → provider, model, temperature, max_tokens
[server]               → port (3000), host (0.0.0.0), dashboard_api_key
[system]               → db_path, memory_db_path, substrate_path, agents_path
[proactive]            → enabled, heartbeat interval
```

### Environment variables
```
SAVANT_DEV_MODE=1      → Auto-generate master keys (no API key needed)
OR_MASTER_KEY=<key>    → OpenRouter API key
RUST_LOG=info          → Log level (trace, debug, info, warn, error)
```

---

## Testing Reference

```bash
# All tests (skip slow memory tests)
cargo test --all -- --skip lsm_engine --skip vector_engine

# Specific crate
cargo test -p savant_core
cargo test -p savant_gateway
cargo test -p savant_agent

# Specific test by name
cargo test -p savant_memory test_name_here

# Clippy (lint)
cargo clippy --all-targets -- -D warnings

# Format check
cargo fmt --check
```

**Test counts (as of 2026-03-17):** 157 passing, 1 ignored (Kani-dependent).

---

## Health Endpoints

```
GET http://localhost:3000/live    → "OK" if gateway running
GET http://localhost:3000/ready   → "OK" if gateway ready
WS  ws://localhost:3000/ws        → Dashboard WebSocket
```

---

## CLI Flags

```
--config <path>     → Load config from custom path
--keygen            → Generate master key pair and print
--skip              → Skip build (start.bat only)
--force             → Force rebuild (start.bat only)
```

---

## Agent Domain Map

If multiple agents are working simultaneously, respect these boundaries:

| Agent | Domain | Crates |
|-------|--------|--------|
| Prometheus | Architecture & DSP | cognitive, echo, mcp, skills |
| Hephaestus | Implementation & Ops | ipc, gateway, agent, cli |
| Athena | Security & Review | security, panopticon, all code review |

**WIP tags:** Before editing a file, check for `// WIP:` tags from other agents. If a file is tagged, wait or coordinate.

---

## Quality Standard: "AAA"

This is not negotiable. Every line of code must be:

- **Correct** — Does what it's supposed to do
- **Safe** — No panics, no data corruption, no security holes
- **Complete** — All error paths handled, no stubs
- **Clean** — Readable, consistent naming, no dead code
- **Tested** — Covered by tests, tests pass

If you find yourself writing `unwrap()`, `todo!()`, or `// TODO` — stop and fix it properly.

---

## When You're Stuck

1. Read the file you're modifying — all of it, not just the line
2. Read the imports and understand the types
3. Search for similar patterns in the codebase: `grep -r "pattern" crates/`
4. Check tests for usage examples: `grep -r "function_name" crates/*/tests/`
5. If still stuck, leave the item as `PENDING` and move to the next one

---

## End of Session

Update `dev/PENDING.md` with:

```
## End of Day State

✅ X/Y issues fixed
✅ Z tests passing
✅ Compilation: clean
✅ [What was accomplished]
```

Then commit and push everything.

---

*Read this file. Then read `dev/PENDING.md`. Then start working.*

# Development Workflow

> **Version:** 0.0.1  
> **Purpose:** Step-by-step development process. Follow this exactly.  
> **Read this first, then check `dev/IMPLEMENTATION-TRACKER.md` for what to work on.**

---

## The Development Loop

Every session follows this sequence. No exceptions.

```
1. AUDIT         →  What's the current state?
2. FIND GAPS     →  What needs fixing?
3. PLAN          →  Add items to IMPLEMENTATION-TRACKER.md
4. PERFECTION    →  Run Perfection Loop (from coding system skill)
5. IMPLEMENT     →  Fix it with AAA quality
6. DOCUMENT      →  Update all affected docs
7. PUSH          →  Commit and push
```

---

## Step 1: Audit — What's the current state?

Before touching any code, run these commands:

```bash
# 1. Does it compile?
cargo check --workspace

# 2. Do tests pass?
cargo test --workspace -- --test-threads=1

# 3. What changed recently?
git log --oneline -10

# 4. What's being worked on?
# Check dev/IMPLEMENTATION-TRACKER.md
```

**Success criteria:** Zero compilation errors, all tests passing.  
**If tests fail:** Fix them first. Nothing else matters if tests are broken.

---

## Step 2: Find Gaps — What needs fixing?

Read the code. Look for these specific problems:

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
- Duplicate logic — overlapping functions that should be combined (Law 11)

---

## Step 3: Plan — Track in IMPLEMENTATION-TRACKER.md

Every task goes into `dev/IMPLEMENTATION-TRACKER.md`:

| # | Task | Status | Details |
|---|------|--------|---------|
| 1 | Short description | PENDING | File: `crate/src/file.rs`, severity: CRITICAL |

**Severity levels:**
- `CRITICAL` — Data corruption, security vulnerability, crash
- `HIGH` — Feature broken, significant bug
- `MEDIUM` — Performance issue, poor error handling
- `LOW` — Code quality, documentation, naming

**Status values:**
- `PENDING` — Not started
- `IN PROGRESS` — Currently being worked on
- `COMPLETE` — Shipped, tested, documented
- `BLOCKED` — Cannot proceed (external dependency)

---

## Step 4: Perfection Loop — Quality audit before implementation

**Source:** Loaded from `skills/savant-coding-system/SKILL.md`

The Perfection Loop is mandatory before implementing any fix. See the coding system skill for the full protocol.

Quick summary:
1. Deep Audit — read ALL related files (1-EOF)
2. Enhance — identify optimizations while reading
3. Validate — check impact on related systems
4. Iterate — if new issues found, go back to step 1 (max 5 iterations)
5. Certify — ready to implement

---

## Step 5: Implement — Fix it with AAA quality

### Rules (non-negotiable)

1. **No stubs.** No `todo!()`, `unimplemented!()`, `// TODO`, or empty functions.
2. **No `unwrap()` in non-test code.** Use `match`, `if let`, or return `Result`.
3. **No `expect()` in non-test code.** Same reason.
4. **No swallowed errors.** `let _ = foo()` only for cleanup where failure is acceptable.
5. **All error paths handled.** Every `Result` is either propagated with `?` or handled explicitly.
6. **Compilation stays clean.** Zero errors, zero warnings after changes.
7. **Tests pass.** Run `cargo test` after each batch of changes.
8. **Combine overlap.** If two functions share logic, combine them into one universal function (Law 11).

### Process per fix

1. Read the file being modified (full file, not just the line)
2. Understand the surrounding context
3. Make the fix
4. Update `dev/IMPLEMENTATION-TRACKER.md` status to `COMPLETE`
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

**Combining overlapping functions (Law 11):**
```rust
// BAD — three functions doing similar things
fn validate_email(input: &str) -> Result<()> { ... }
fn validate_username(input: &str) -> Result<()> { ... }
fn validate_phone(input: &str) -> Result<()> { ... }

// GOOD — one universal function
enum InputType { Email, Username, Phone }
fn validate_input(input: &str, input_type: InputType) -> Result<()> { ... }
```

---

## Step 6: Document — Keep everything current

After implementing, update these files:

| File | When to update | What |
|------|---------------|------|
| `dev/IMPLEMENTATION-TRACKER.md` | After EVERY feature | Mark status, add details |
| `dev/SESSION-SUMMARY.md` | End of EVERY session | Archive old, write new |
| `dev/CHANGELOG-INTERNAL.md` | Significant features | Add entries |
| `CHANGELOG.md` (root) | At release time | User-facing changes |
| `README.md` | User-facing changes | Only if relevant |

**Do NOT update docs if nothing relevant changed.** Only touch files that need updating.

---

## Step 7: Push — Get it to GitHub

### Commit message format
```
<type>: <short description>

<optional body>
- Bullet point changes
```

**Types:** `fix`, `feat`, `docs`, `refactor`, `test`, `chore`, `sync`

### Before pushing checklist
- [ ] `cargo check --workspace` passes with zero errors
- [ ] `cargo test --workspace` passes
- [ ] `dev/IMPLEMENTATION-TRACKER.md` is updated
- [ ] `CHANGELOG.md` updated (if user-facing changes)
- [ ] `README.md` updated if any changes were made
- [ ] Commit message follows format

### Push command
```bash
git add -A && git commit -m "<message>" && git push origin main
```

---

## End of Session

1. Archive current `dev/SESSION-SUMMARY.md` to `dev/archive/YYYY-MM-DD/`
2. Write new `dev/SESSION-SUMMARY.md` with:
   - Features completed
   - Bugs fixed
   - Test results
   - Files changed
   - Git commit hash
3. Update `dev/IMPLEMENTATION-TRACKER.md`
4. Update `dev/CHANGELOG-INTERNAL.md` if significant
5. Commit and push

---

## When You're Stuck

1. Read the file you're modifying — all of it, not just the line
2. Read the imports and understand the types
3. Search for similar patterns in the codebase
4. Check tests for usage examples
5. If still stuck, mark as `BLOCKED` with reason and move to the next item

---

## Quality Standard: "AAA"

Every line of code must be:

- **Correct** — Does what it's supposed to do
- **Safe** — No panics, no data corruption, no security holes
- **Complete** — All error paths handled, no stubs
- **Clean** — Readable, consistent naming, no dead code
- **Tested** — Covered by tests, tests pass
- **Universal** — No duplicate logic, overlap combined into one function (Law 11)

If you find yourself writing `unwrap()`, `todo!()`, or `// TODO` — stop and fix it properly.

---

*Read this file. Check `dev/IMPLEMENTATION-TRACKER.md`. Load the coding system skill. Then start working.*

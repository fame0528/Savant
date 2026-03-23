# Development Workflow

> **Version:** 2.0.0  
> **Purpose:** The exact methodology for all development sessions. Follow this precisely.  
> **Read this first, then check `dev/IMPLEMENTATION-TRACKER.md` for what to work on.**  
> **This workflow was refined from the 2026-03-23 Production Pass session — the most productive session to date.**

---

## Core Philosophy

Every session is a surgical operation on a highly interconnected codebase. One change in file A can break logic in file Z. The only way to solve this is with a protocol that forces full understanding before every change.

**The three laws:**
1. **Read before touch.** Every file read 0-EOF before any edit. No exceptions. No skimming. No assumptions.
2. **Present before act.** Every change presented to Spencer with impact analysis BEFORE implementation. No autonomous changes.
3. **Verify before proceed.** Every change verified with `cargo check --workspace` before moving on. No broken builds.

---

## Session Startup (First Thing Every Session)

### 1. Get Back Up to Speed

After a context compact or new session start, you MUST re-familiarize with the codebase before touching anything:

```bash
# 1. Does it compile?
cargo check --workspace

# 2. Do tests pass?
cargo test --workspace

# 3. What changed recently?
git log --oneline -10

# 4. What's the current state?
# Read dev/progress.md
# Read dev/IMPLEMENTATION-TRACKER.md
# Read dev/SESSION-SUMMARY.md (last session's summary)
```

### 2. Full Project Audit (If Needed)

If you've been compacted or are starting fresh, do a FULL audit:
- Read EVERY file across all crates 0-EOF
- Create `dev/AUDIT-REPORT.md` with all issues found
- Categorize: Critical, High, Medium, Low, Stubs
- This serves dual purpose: audit + re-familiarization

**This is not optional.** The 2026-03-23 audit found ~250 issues and was the foundation for the entire production pass.

---

## The Production Pass Protocol

This is the protocol for fixing bugs, addressing security issues, and implementing features at enterprise grade. Follow this for EVERY fix.

### Phase A: Create the FID

1. Create `dev/fids/FID-YYYYMMDD-DESCRIPTION.md`
2. Build a fix matrix:

| # | Severity | Issue | File | Line | Fix | Cross-Impact | Gate |
|---|----------|-------|------|------|-----|-------------|------|
| 1.1 | CRITICAL | Description | `file.rs` | 94 | Proposed fix | What it affects | CHECKPOINT |

3. Group fixes by target file (not by severity) — this minimizes file reads and catches intra-file interactions
4. Add a cross-impact map showing which systems are interconnected
5. Add a risk register
6. Define checkpoint gates after every fix group

### Phase B: Perfection Loop on the FID

Run the Perfection Loop on the FID itself BEFORE any implementation:

1. **Deep Audit** — review every fix, every cross-impact, every gate
2. **Enhance** — improve the FID based on findings (merge fixes, add missing items, reorder)
3. **Validate** — check for dependency issues, ordering conflicts, missing verification steps
4. **Iterate** — if improvements found, go back to step 1 (max 5 iterations)
5. **Certify** — FID is ready for execution

### Phase C: Decision Gates (If Needed)

If the FID has ambiguous items (implement vs remove, multiple approaches):

1. Present each decision to Spencer with options
2. Spencer decides before any code changes
3. Update the FID with decisions
4. Commit decisions

### Phase D: Competitor Research (If Needed)

Before implementing stubs or new features:

1. Scan competitor projects for their implementations
2. Understand their approach (DO NOT copy code line-for-line)
3. Run Perfection Loop on the competitor's approach to improve it
4. Update the FID with competitor-informed approaches

### Phase E: Execute Fixes

For EVERY fix, follow this exact sequence:

#### Step 1: Present to Spencer

Before touching any code, present:
- **Issue description** — what's broken and why it matters
- **File + line** — exact location
- **Proposed fix** — exact code change
- **Cross-impact analysis** — what else touches this code? What could break?
- **Risk assessment** — what's the worst case if this is wrong?

#### Step 2: Wait for Spencer Approval

Do NOT proceed without explicit approval. If anything is unclear, STOP. Get clarity.

#### Step 3: Read the Target File 0-EOF

Read the ENTIRE file, not just the affected lines. Understand:
- The file's purpose and structure
- All imports and their types
- All functions that interact with the code you're changing
- The data flow: where does the data come from, where does it go?

#### Step 4: Read Cross-Impact Files 0-EOF

Read every file that imports from or is imported by the target. Understand:
- How the changed code is called
- What depends on the current behavior
- What could break if the behavior changes

#### Step 5: Make the Change

Implement the fix exactly as approved. No improvements beyond the scope unless discussed.

#### Step 6: Verify

```bash
cargo check --workspace     # Must pass with 0 errors
cargo test -p <crate>       # If applicable
npx tsc --noEmit            # If frontend changes
```

#### Step 7: Checkpoint with Spencer

Present:
- What changed (exact diff)
- What cross-impacts were affected
- Verification results
- Get approval for next fix

#### Step 8: Update Changelog

Update `dev/CHANGELOG-INTERNAL.md` with the fix details:
- File changed
- Issue fixed
- Approach taken
- Cross-impacts

### Phase F: Final Verification

After ALL fixes are complete:

```bash
cargo check --workspace     # 0 errors
cargo test --workspace      # All passing
npx tsc --noEmit            # 0 errors (if frontend changes)
```

Update tracking files:
- `dev/IMPLEMENTATION-TRACKER.md` — mark items complete
- `dev/progress.md` — update current status
- `dev/SESSION-SUMMARY.md` — archive old, write new
- `dev/CHANGELOG-INTERNAL.md` — all changes documented

Commit and push:
```bash
git add -A && git commit -m "Description" && git push origin main
```

---

## Rules (Non-Negotiable)

### Code Rules
1. **No stubs.** No `todo!()`, `unimplemented!()`, `// TODO`, or empty functions.
2. **No `unwrap()` in non-test code.** Use `match`, `if let`, or return `Result`.
3. **No `expect()` in non-test code.** Same reason.
4. **No swallowed errors.** `let _ = foo()` only for cleanup where failure is acceptable.
5. **All error paths handled.** Every `Result` is either propagated with `?` or handled explicitly.
6. **Compilation stays clean.** Zero errors after changes.
7. **Discovery-based over hardcoded.** Query the system for its capabilities. Don't assume.

### Process Rules
1. **Read 0-EOF before every edit.** No exceptions.
2. **Present before act.** No autonomous changes.
3. **Verify before proceed.** `cargo check --workspace` after every fix.
4. **Checkpoint gates.** Spencer approval between every fix group.
5. **No skimming.** The codebase is complex and interconnected. Partial reads cause bugs.
6. **Changelog every change.** Track what was done and why.

### Anti-Patterns (Never Do These)
- "The simplest approach" — we do NASA-level implementations, not simple ones
- "Let me just quickly fix this" — there is no quick fix, every change is surgical
- Reading only the affected line — you MUST read the full file
- Making changes without presenting — Spencer is the partner, not a rubber stamp
- Skipping verification — broken builds cascade
- Choosing speed over quality — we are never in a rush
- Minimizing scope to reduce effort — we do it right, not fast
- "Good enough" — good enough is never good enough

---

## The Standard: Perfection Over Convenience

This section is non-negotiable and overrides all other considerations.

**We are not in a rush. We do not optimize for speed. We optimize for correctness, robustness, and longevity.**

A single feature can take multiple days. A single fix can touch dozens of files. That is acceptable. What is NOT acceptable:
- Shipping code that works "for now"
- Taking a shortcut because the right way is harder
- Choosing a simpler approach because it's faster to implement
- Leaving a gap because "we'll fix it later"
- Writing code that assumes instead of discovers

**The standard is perfection. Every time. No exceptions.**

When evaluating an implementation approach, ask:
1. Will this work for ALL cases, not just the common case?
2. Will this scale to 1000 agents, not just 10?
3. Will this survive a hostile attacker, not just an honest user?
4. Will this be maintainable in 2 years, not just today?
5. Does this set the standard for the industry, not just meet it?

If the answer to any question is "no" — redesign until all answers are "yes".

**This is what sets us apart.** Other teams ship fast and fix later. We ship right and never need to fix. The quality IS the product.

---

## Quality Standard: "Enterprise Grade"

Every line of code must be:

- **Correct** — Does what it's supposed to do
- **Safe** — No panics, no data corruption, no security holes
- **Complete** — All error paths handled, no stubs
- **Clean** — Readable, consistent naming, no dead code
- **Tested** — Covered by tests, tests pass
- **Discovery-based** — Queries the system for capabilities, doesn't hardcode assumptions

If you find yourself writing `unwrap()`, `todo!()`, or `// TODO` — stop and fix it properly.

---

## Documentation (Always Updated)

| File | When | What |
|------|------|------|
| `dev/CHANGELOG-INTERNAL.md` | After EVERY fix | Detailed fix description with file, issue, approach |
| `dev/IMPLEMENTATION-TRACKER.md` | After EVERY phase | Mark status, add completion details |
| `dev/progress.md` | After EVERY phase | Update current status and pending work |
| `dev/SESSION-SUMMARY.md` | End of session | Archive old, write new summary |
| `dev/AUDIT-REPORT.md` | After full audit | All issues catalogued |
| `CHANGELOG.md` (root) | At release | User-facing changes |
| `README.md` | At release | If user-facing changes |

---

## Common Fix Patterns

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

**Atomic writes (write-before-delete):**
```rust
// BAD — delete before insert, no rollback
delete_old_entries()?;
insert_new_entries()?;  // If this fails, data is lost

// GOOD — insert first, delete after
insert_new_entries()?;   // If this fails, old data is intact
delete_old_entries()?;   // If this fails, duplicates exist temporarily (next run cleans up)
```

**Discovery-based configuration:**
```rust
// BAD — hardcoded
const CONTEXT_WINDOW: usize = 128_000;

// GOOD — discovery-based
fn context_window(&self) -> Option<usize> {
    // Query the actual system capabilities
    self.cached_context_window
}
```

**Ephemeral secrets (runtime-only, no persistence):**
```rust
// BAD — hardcoded default
let secret = config.secret.unwrap_or_else(|| "default_secret".to_string());

// GOOD — crypto-random ephemeral
let secret = config.secret.unwrap_or_else(|| {
    let mut hasher = blake3::Hasher::new();
    hasher.update(uuid::Uuid::new_v4().as_bytes());
    hasher.update(uuid::Uuid::new_v4().as_bytes());
    hasher.finalize().to_hex().to_string()
});
```

---

## When You're Stuck

1. Read the file you're modifying — ALL of it, not just the line
2. Read the imports and understand the types
3. Search for similar patterns in the codebase
4. Check tests for usage examples
5. Check competitor projects for how they solved it
6. If still stuck, mark as `BLOCKED` with reason and move to the next item
7. NEVER guess. If unclear, ask Spencer.

---

*Read this file. Check `dev/IMPLEMENTATION-TRACKER.md`. Load the coding system skill. Then start working.*

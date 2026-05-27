# Savant — Agent Bootstrap Prompt

You are working on **Savant**, a Rust-native autonomous AI agent orchestration framework. The workspace is at `C:\Users\spenc\dev\Savant`. The project is a Cargo workspace with 27+ crates. The primary crate is `crates/agent/` which contains the ReAct loop, swarm coordination, memory, learning, pulse system, and orchestration engine.

---

## Project Structure (Quick Reference)

| Crate | Purpose |
|-------|---------|
| `crates/agent/` | Core agent: react loop, swarm, memory, learning, pulse, orchestration, compact |
| `crates/core/` | Shared types, config, telemetry, token counting |
| `crates/gateway/` | REST API server |
| `crates/sandbox/` | MicroVM guest agent isolation |
| `crates/security/` | Circuit breaker, credential broker, PII detection, taint analysis |
| `crates/memory/` | Vector+graph memory engine (CortexaDB) |
| `crates/mcp/` | Model Context Protocol server |
| `crates/cli/` | CLI binaries (savant_cli, gateway, session, TUI) |
| `crates/panopticon/` | Monitoring and replay |
| `crates/browser/` | Chromium automation |
| `crates/canvas/` | Canvas manager |
| `crates/channels/` | Multi-channel adapters |
| `crates/echo/` | Echo monitoring |
| `crates/integrations/` | External service integrations |
| `crates/skills/` | Skill manager |
| `crates/generation/` | Local image/SVG generation |

### Production Entry Points (verify wiring against these 5 files)

1. `crates/agent/src/orchestration/ignition.rs` — startup
2. `crates/agent/src/swarm.rs` — agent creation
3. `crates/agent/src/pulse/heartbeat.rs` — agent execution
4. `crates/agent/src/react/stream.rs` — the ReAct loop
5. `crates/agent/src/react/reactor.rs` — tool execution

After wiring any feature, run:
```bash
grep -rn "feature_name" crates/agent/src/orchestration/ignition.rs crates/agent/src/swarm.rs crates/agent/src/pulse/heartbeat.rs crates/agent/src/react/stream.rs crates/agent/src/react/reactor.rs
```
Zero results = NOT wired. Do not mark complete.

---

## Coding Rules — Non-Negotiable

### The Three Laws + Fourth

1. **Read 0-EOF before touch.** Every file read completely before any edit. No skimming.
2. **Present before act.** Every change presented with impact analysis BEFORE implementation. No silent changes. Wait for approval.
3. **Verify before proceed.** Every change verified with `cargo check --workspace`. No broken builds.
4. **Verify call-graph reachability.** After wiring, grep the 5 production entry points. Compilation is NOT verification.

### Zero Tolerance

| Rule | What This Means |
|------|----------------|
| No stubs | No `todo!()`, `unimplemented!()`, `// TODO`, `// FIXME`, `pass`, `...` anywhere in production code |
| No unwrap/expect | No `.unwrap()` or `.expect()` in non-test code. Use `?`, `match`, or explicit error handling |
| No swallowed errors | No `let _ = foo()` unless failure is genuinely acceptable |
| No placeholders | No pseudo-code, dummy logic, or "implement later" comments |
| No hardcoded assumptions | Query system capabilities dynamically. Discovery-based. |
| All error paths | Every `Result` propagated or explicitly handled |
| Zero clippy warnings | `cargo clippy --workspace --no-deps` must pass clean after every edit |
| Zero format violations | `cargo fmt --check` must pass clean |
| Clean build | `cargo check --workspace` must pass after every edit |

### The Five Questions (evaluate every approach)

1. Will this work for **ALL** cases, not just the common case?
2. Will this scale to **1000 agents**, not just 10?
3. Will this survive a **hostile attacker**, not just an honest user?
4. Will this be maintainable in **2 years**, not just today?
5. Does this set the **standard for the industry**, not just meet it?

If any answer is NO — redesign until all answers are YES.

### Additional Rules

- **Never push to remote** without explicit `y` approval. Default state is NO PUSH.
- **Never remove working features.** No deprecation shims. Build everything fully.
- **Never defer without approval.** Never self-defer FID items.
- **Never code without approval.** Present the plan, wait for approval, then code.
- **Fix bugs regardless of scope.** Any bugs found during FID work must be fixed immediately.
- **Full codebase audits.** Read every file 0-EOF. No grep-only verification.

---

## The Perfection Loop (run on every feature/fix)

All work goes through this 5-step loop. Run ALL steps. Present results BEFORE coding. Wait for approval.

| Step | Name | What You Do |
|------|------|-------------|
| **1** | Deep Audit | Read ALL target files 0-EOF. Analyze every line for redundancy, tech debt, security. Output: list of findings. |
| **2** | Enhancement | Apply optimizations, enhance error handling. Never introduce stubs or shortcuts. |
| **3** | Validation | `cargo check --workspace`, `cargo test --workspace --lib`, `cargo clippy --workspace --no-deps`. All must pass. |
| **4** | Iteration | If improvements found during audit or validation, implement them and return to Step 1. Max 5 iterations. |
| **5** | Certification | Report metrics (LOC, test count, clippy warnings). CERTIFIED / NOT CERTIFIED / PARTIALLY CERTIFIED with reason. |

**Termination:** Deep Audit yields zero improvements → certified. 5 iterations without convergence → flag for review.

---

## The FID System (Fix Implementation Documents)

All work is tracked through FIDs. File naming: `FID-YYYYMMDD-DESCRIPTION.md` in `dev/fids/`.

**Lifecycle:** OPEN → FIXED → AWAITING VERIFICATION → CLOSED → archived to `dev/fids/archived/`

**Tracking files:**
- `dev/fids/progress.md` — current objective tracking
- `dev/SESSION-SUMMARY.md` — latest session report
- `dev/CHANGELOG-INTERNAL.md` — detailed changelog

---

## FID Template

Copy the block below into `dev/fids/FID-YYYYMMDD-DESCRIPTION.md`:

```markdown
# FID-YYYYMMDD-DESCRIPTION

> **FID:** FID-YYYYMMDD-DESCRIPTION
> **Objective:** [One sentence goal]
> **Status:** OPEN
> **Started:** YYYY-MM-DD

---

## Context

[What exists today. What the gap is. Why this FID exists.]

## Issues

### 1. [Issue Name] (CRITICAL/HIGH/MEDIUM/LOW)

**Current:** [What exists now]
**Target:** [What should exist]
**Files:** [Which files to modify]

| Risk | Blast Radius | Effort |
|------|-------------|--------|
| LOW/MEDIUM/HIGH | [What it affects] | [Time estimate] |

### 2. [Issue Name] (SEVERITY)

**Current:**
**Target:**
**Files:**

| Risk | Blast Radius | Effort |
|------|-------------|--------|

---

## Verification Plan

```bash
cargo check --workspace
cargo test --workspace --lib
cargo clippy --workspace --no-deps
```

---

## Completion Criteria

- [ ] [Criterion 1]
- [ ] [Criterion 2]
- [ ] All tests pass, zero clippy warnings

---

## Perfection Loop Results

### Step 1: Deep Audit
[Finding summary — what the code actually does vs what the spec claims]

### Step 2: Enhancement
[Issues found, what was fixed, what remains]

### Step 3: Validation
| Check | Result |
|-------|--------|
| `cargo check --workspace` | PASS/FAIL |
| `cargo test --workspace --lib` | N pass, N fail |
| `cargo clippy --workspace --no-deps` | N warnings |

### Step 4: Iteration
| # | Severity | Issue | Status |
|---|----------|-------|--------|

### Step 5: Certification
CERTIFIED / NOT CERTIFIED / PARTIALLY CERTIFIED
[Reason. What blocks certification if not certified.]
```

---

## Session Startup Checklist

When starting a work session:

1. Read `dev/SESSION-SUMMARY.md` — current state
2. Read `dev/fids/progress.md` — active FIDs
3. Read the active FID file completely
4. Read `dev/CHANGELOG-INTERNAL.md` — recent changes
5. Run baseline:
   ```bash
   cargo check --workspace
   cargo clippy --workspace --no-deps
   cargo test --workspace --lib
   ```
6. Present findings and plan. **Wait for approval before coding.**

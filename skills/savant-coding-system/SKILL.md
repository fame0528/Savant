---
name: savant-coding-system
description: AAA-quality autonomous coding standards for all Savant agents. Universal foundation + language supplements. Everything goes through the Perfection Loop.
version: "0.0.2"
execution_mode: Reference
capabilities:
  network_access: false
  filesystem_access: false
  max_memory_mb: 0
  max_execution_time_ms: 0
---

# Savant Coding System v0.0.2

**Embedded skill. Loaded as agent context. Not executed.**

Every agent working in code loads this skill before starting any task. It defines HOW code should be written, audited, and verified. **Everything goes through the Perfection Loop.**

---

## How This System Works

```
This file (SKILL.md) = Universal foundation (all languages)
coding-standards/     = Language-specific supplements

When agent starts coding task:
1. Read this SKILL.md (foundation)
2. Read coding-standards/<LANGUAGE>.md (supplement for what you're working in)
3. Run Perfection Loop on EVERY feature/task
4. Follow both exactly
```

---

## The Perfection Loop

**This is the core quality protocol. Everything goes through it. No exceptions.**

The Perfection Loop is a 5-step cycle that elevates code from "functional" to "flawless." It runs on every feature, every fix, every refactor. It is not optional.

```
┌─────────────────────────────────────────────────────┐
│              PERFECTION LOOP                        │
│                                                     │
│  1. DEEP AUDIT                                      │
│     Read all target files 1-EOF                     │
│     Analyze for redundancy, debt, vulnerabilities   │
│     Output: list of improvements                    │
│          ↓                                          │
│  2. ENHANCE                                         │
│     Apply optimizations                             │
│     Improve error handling                          │
│     Constraint: no unwrap/todo/placeholder          │
│          ↓                                          │
│  3. VALIDATE                                        │
│     Run verification (check, test, lint)            │
│     Must pass with 0 errors, 0 warnings             │
│          ↓                                          │
│  4. ITERATE                                         │
│     If improvements found → return to step 1        │
│     If none found → proceed to step 5               │
│     Max 5 iterations before flagging                │
│          ↓                                          │
│  5. CERTIFY                                         │
│     Report metrics (LOC, tests, quality)            │
│     Mark complete                                   │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### Step 1: Deep Audit

- Read ALL target files COMPLETELY (1-EOF) before any analysis
- Analyze every line for redundancy, tech debt, security vulnerabilities
- Verify compliance with project standards
- **Output:** A clear list of improvements before writing any code

### Step 2: Enhance

- Apply performance optimizations (zero-copy, batch operations, efficient data structures)
- Enhance error handling with context-rich logging
- Refine patterns to match existing codebase conventions
- **Constraint:** Do not introduce `unwrap()`, `todo!()`, `unimplemented!()`, `as any`, or placeholders. Ever.

### Step 3: Validate

- Run the language-appropriate verification commands (see supplements)
- All must pass: compilation, tests, linting
- Verify unit and integration tests are written and passing
- **If any fail:** return to Step 2, fix, then re-validate

### Step 4: Iterate

- If Deep Audit or Validation identified improvements:
  - Implement them immediately
  - Return to Step 1
  - Track iteration count (e.g., "Perfection Loop: Iteration 2")
- If NO improvements identified:
  - Proceed to Step 5
- **Checkpoint:** If loop exceeds 3 iterations, reassess scope. If 5 iterations without convergence, flag for review.

### Step 5: Certify

- Report final metrics (LOC, files changed, tests passing, quality metrics)
- Include: iteration count, improvements made
- **Deliverable:** Final code, verification passes, updated documentation
- Mark feature as COMPLETE in tracker

### Termination Criteria

The loop terminates when ANY of:

| Condition | Action |
|-----------|--------|
| Deep Audit yields ZERO improvements | → Certify |
| 5 iterations reached without convergence | → Flag for review |
| Diminishing returns detected | → Recommend ship |

---

## Core Laws (Never Violate)

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

## Error Recovery (No Human Escalation)

| Scenario | Response |
|----------|----------|
| Verification errors (3+ attempts) | Categorize → targeted fix → architectural pivot → document |
| Conflicting patterns | Analyze recency/frequency → choose best → log reasoning |
| Context window limits | Save state to tracker → summarize → provide handoff |
| APIs unavailable | Generate mocks → flag in report → create test plan |

---

## Rollback Triggers

```
1. Error count increases >50% after changes
2. Critical functionality broken
3. Wrong file modified
4. Pattern mismatch discovered

Response: Halt → Document → Rollback or fix-forward → Verify
```

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

## File Reading Law

```
BEFORE touching ANY file:
1. Read file completely (1-EOF)
2. Know all functions, types, structure
3. ONLY THEN proceed
```

---

## Golden Rules

**NEVER:** Edit without reading | Pseudo-code | Type shortcuts | Skip planning | Copy-paste | Expose secrets | Skip verification

**ALWAYS:** Read completely | Production code | Proper types | Log intent | Search first | Follow patterns | Verify | Track progress

**ALWAYS:** Run Perfection Loop on every task. This is the standard.

---

## Language Supplements

See `coding-standards/` directory for language-specific rules:
- `RUST.md` — cargo check/test/clippy, serde, thiserror, Arc/Mutex
- `TYPESCRIPT.md` — strict mode, branded types, Result pattern, React
- `PYTHON.md` — type hints, dataclass, asyncio, pydantic

**When working in a language:** Read the supplement AFTER reading this foundation. The Perfection Loop applies regardless of language.

---

*This skill is loaded as context by every Savant agent before coding. Version 0.0.2. Everything goes through the Perfection Loop.*

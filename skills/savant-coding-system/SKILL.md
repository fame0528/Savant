---
name: savant-coding-system
description: AAA-quality autonomous coding standards for all Savant agents. Universal foundation + language supplements.
version: "0.0.1"
execution_mode: Reference
capabilities:
  network_access: false
  filesystem_access: false
  max_memory_mb: 0
  max_execution_time_ms: 0
---

# Savant Coding System v0.0.1

**Embedded skill. Loaded as agent context. Not executed.**

Every agent working in code loads this skill before starting any task. It defines HOW code should be written, audited, and verified.

---

## How This System Works

```
This file (SKILL.md) = Universal foundation (all languages)
coding-standards/     = Language-specific supplements

When agent starts coding task:
1. Read this SKILL.md (foundation)
2. Read coding-standards/<LANGUAGE>.md (supplement for what you're working in)
3. Follow both exactly
```

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

## Implementation Protocol (12 Steps)

```
Objective Triggered
    ↓
1. Understand the task (scope, acceptance criteria)
2. Pattern Discovery (find 2-3 working examples)
3. Create Structured Todo (atomic tasks, proper order)
4-N. Execute Each Task (atomic, complete, documented)
N+1. Verification (0 errors, all tests pass)
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

---

## Language Supplements

See `coding-standards/` directory for language-specific rules:
- `RUST.md` — cargo check/test/clippy, serde, thiserror, Arc/Mutex
- `TYPESCRIPT.md` — strict mode, branded types, Result pattern, React
- `PYTHON.md` — type hints, dataclass, asyncio, pydantic

**When working in a language:** Read the supplement AFTER reading this foundation.

---

*This skill is loaded as context by every Savant agent before coding. Version 0.0.1.*

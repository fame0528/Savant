# Savant Coding System — Foundation

> **Version:** 0.0.1  
> **Scope:** Universal — applies to all languages and frameworks  
> **Purpose:** AAA-quality autonomous coding standards for Savant agents.  
> **Origin:** Custom protocol built for the Savant project over thousands of hours of refinement.

---

## How This System Works

```
┌─────────────────────────────────────────────────────────┐
│              SAVANT CODING SYSTEM                        │
│                                                         │
│  This file (foundation)     Language Supplements         │
│  ──────────────────────     ────────────────────         │
│  • Core Laws                • dev/coding-standards/      │
│  • Guardian Protocol          ├── RUST.md               │
│  • Implementation Flow        ├── TYPESCRIPT.md          │
│  • Error Recovery             └── PYTHON.md             │
│  • Rollback System                                      │
│  • Scale Adaptation         Handle:                      │
│  • Rule Hierarchy           • Compilation commands       │
│  • Testing Protocol         • Type system rules          │
│  • Quality Standards        • Package management         │
│  • File Reading Law         • Framework patterns         │
│  • Auto-Audit               • Error handling idioms      │
│                                                         │
│  Foundation = WHAT to do                               │
│  Supplement = HOW to do it in that language             │
└─────────────────────────────────────────────────────────┘
```

**When starting a task:** Load this foundation file AND the relevant language supplement.

---

## Core Laws (Never Violate)

These are the foundation. Everything else builds on these. They are language-agnostic.

| # | Law | Why |
|---|-----|-----|
| 1 | **Read files COMPLETELY (1-EOF) before ANY edit** | Assumptions break code |
| 2 | **No pseudo-code, TODOs, or placeholders** | Technical debt compounds |
| 3 | **No type safety shortcuts** | Runtime errors in production |
| 4 | **Search for existing code BEFORE creating new** | Duplication kills maintainability |
| 5 | **Log intent before coding** | Untracked drift |
| 6 | **Generate production-grade documentation** | Unmaintainable code |
| 7 | **Update tracking after every feature** | Lost progress |
| 8 | **Follow discovered patterns EXACTLY** | Inconsistency |
| 9 | **Run verification before completion** | Broken builds |
| 10 | **Never expose sensitive data in logs/errors** | Security breach |

**Quick Self-Check Before ANY Action:**

```
□ Have I read the target file(s) completely?
□ Am I creating complete, production-ready code?
□ Did I search for existing reusable code?
□ Have I logged the intent?
□ Am I following existing patterns?
```

---

## Guardian Protocol

GUARDIAN monitors every action and auto-corrects violations immediately. No human intervention required.

### How It Works

```
BEFORE every tool call → Pre-Execution Validation
AFTER every tool call  → Post-Execution Audit
IF violation detected  → HALT + ANNOUNCE + AUTO-CORRECT + VERIFY
THEN continue          → Only after compliance verified
```

### 20-Point Compliance Checklist

Execute after every tool response:

| # | Check | Violation | Auto-Correct |
|---|-------|-----------|--------------|
| 1 | File read completely (1-EOF)? | Partial read | Re-read completely |
| 2 | File read before edit? | Edit without context | Read first, then edit |
| 3 | No type shortcuts in code? | Safety violation | Use proper types |
| 4 | Searched for existing code? | Duplication | Search first |
| 5 | No copy-paste duplication? | DRY violation | Extract to utility |
| 6 | Tracking updated? | Lost progress | Update now |
| 7 | Todo list created for complex features? | Missing planning | Create todo list |
| 8 | No pseudo-code/placeholders? | Quality violation | Complete implementation |
| 9 | Planning phase completed? | Phase skip | Enter planning mode |
| 10 | Utilities built before features? | Wrong order | Utility-first |
| 11 | Index/module exports created? | Missing exports | Create export file |
| 12 | Docs in correct location? | Wrong location | Move to /docs |
| 13 | Batch loading for large files? | Context overflow | Read in chunks |
| 14 | All target files loaded? | Missing context | Load all files |
| 15 | Pattern discovery completed? | Wrong patterns | Find working examples |
| 16 | Implementation steps followed? | Step skipped | Execute missing steps |
| 17 | Error recovery considered? | No fallback | Document recovery path |
| 18 | Tests written for new code? | Untested code | Write tests |
| 19 | Verification command run? | Unverified | Run check/test |
| 20 | Sensitive data not exposed? | Security risk | Remove/redact |

### Violation Response

```
SAVANT VIOLATION DETECTED

Type: [violation category]
What: [specific description]
Why: [law reference]

MANDATORY CORRECTION:
[execute fix]

Corrected — Resuming.
```

---

## Implementation Protocol (12 Steps)

Proven methodology. Follow in order.

```
Objective Triggered
        ↓
Step 1:  Understand the task (scope, acceptance criteria)
        ↓
Step 2:  Pattern Discovery (find 2-3 working examples in codebase)
        ↓
Step 3:  Create Structured Todo (atomic tasks, proper order)
        ↓
Step 4-N: Execute Each Task (atomic, complete, documented)
        ↓
Step N+1: Verification (0 errors, all tests pass)
        ↓
Step Final: Completion Report (LOC, files, quality metrics)
```

### Step 1: Understand the Task

```
1. Read the task completely
2. Extract: scope, acceptance criteria, approach, files, dependencies
3. Identify: what exists, what's new, what changes
```

### Step 2: Pattern Discovery

```
1. FIND: Search for similar working files in codebase
2. READ: 2-3 working examples COMPLETELY (1-EOF)
3. EXTRACT patterns: imports, error handling, naming, structure
4. DOCUMENT: "Discovered patterns: [list]"
```

### Step 3: Create Structured Todo

```
Task Order:
1. Types/Interfaces → 2. Utilities → 3. Models → 4. Validations
5. API Routes → 6. Core Logic → 7. Hooks/Adapters → 8. UI/CLI
9. Tests → 10. Verification → 11. Documentation

Each task format:
- Task N: [Name]
  - File: path/to/file
  - Deliverable: Specific output
```

### Step 4-N: Execute Each Task

```
FOR EACH TASK:
1. Mark "in-progress"
2. Read target file(s) if modifying
3. Generate COMPLETE code (no placeholders)
4. Follow discovered patterns EXACTLY
5. Include full documentation, types, error handling
6. Mark "completed"
7. Report: "Task X complete: file (LOC lines)"
```

### Step N+1: Verification

See language supplement for specific commands. Generally:
- Compilation/type check: 0 errors
- Tests: all pass
- Linting: 0 warnings

### Step Final: Completion Report

```
Feature Implementation Complete

| Metric | Value |
|--------|-------|
| Total LOC | X,XXX lines |
| Files Created | N new files |
| Files Modified | N files |
| Verification | 0 errors |
| Tests | X passing |

Status: COMPLETE
```

---

## Error Recovery

When things go wrong, handle it automatically. No human escalation.

### Scenario 1: Verification Errors Won't Resolve (3+ attempts)

```
1. DOCUMENT all errors with root cause analysis
2. CATEGORIZE:
   - Type A: Fixable with more context (search for related types/files)
   - Type B: Requires architectural change
   - Type C: External dependency issue
3. EXECUTE HEURISTIC RESOLUTION:
   - Path A: Targeted expansion (search for missing related types)
   - Path B: Technical refinement (refactor to more robust approach)
   - Path C: Architectural pivot (implement adapter/shim)
4. UPDATE tracker with resolution strategy
5. LOG exception if type safety cannot be 100% verified
```

### Scenario 2: Pattern Discovery Finds Conflicting Patterns

```
1. DOCUMENT both patterns with examples
2. ANALYZE:
   - Which is more recent?
   - Which is used more frequently?
   - Which follows current best practices?
3. SYNTHESIZE:
   - Prefer most recent patterns in the project
   - Prefer patterns with highest frequency
   - Log reasoning for chosen pattern
4. PROCEED with chosen pattern
```

### Scenario 3: Context Window Limits Reached

```
1. DETECT: Response truncation or incomplete outputs
2. SAVE state in IMPLEMENTATION-TRACKER.md:
   - Current phase
   - Files modified
   - Next steps
   - Any open issues
3. SUMMARIZE completed work
4. LIST remaining tasks
5. Provide handoff state for next session
```

### Scenario 4: API/Backend Unavailable During Testing

```
1. DOCUMENT which endpoints are affected
2. GENERATE mock data for development
3. FLAG in completion report: "Implemented with mocks - requires integration testing"
4. CREATE test plan for when APIs are available
```

### Error Severity Classification

| Severity | Definition | Response |
|----------|------------|----------|
| CRITICAL | Blocks all progress | HALT, document, find alternative path |
| HIGH | Blocks current feature | Attempt fix 3x, then try alternative approach |
| MEDIUM | Degraded functionality | Document, proceed with workaround |
| LOW | Cosmetic/minor | Fix inline, document in notes |

---

## Rollback System

### Pre-Modification Checkpoint

Before any multi-file modification batch:

```
CHECKPOINT: [timestamp]
- Files to modify: [list]
- Current git status: [clean/dirty]
- Backup state: documented
- Rollback command: [git checkout or restore]
```

### Rollback Triggers

```
1. Verification errors increase by >50% after changes
2. Critical functionality broken (detected in testing)
3. Wrong file modified (mistaken identity)
4. Pattern mismatch discovered after implementation
```

### Rollback Execution

```
ROLLBACK INITIATED

Reason: [why]
Scope: [which files/changes]

1. HALT all further modifications
2. DOCUMENT what went wrong
3. TRIGGER resolution:
   - Path A: Full rollback if error spike is >50%
   - Path B: Fix-forward if under threshold and root cause identified
4. EXECUTE chosen path
5. VERIFY restoration successful
6. UPDATE tracker with rollback record
```

---

## Scale Adaptation

### Project Size Classification

| Size | Files | LOC | Adaptation |
|------|-------|-----|------------|
| Small | <20 | <5K | Standard protocol |
| Medium | 20-50 | 5K-20K | Targeted loading |
| Large | 50-200 | 20K-100K | Domain-focused |
| Enterprise | >200 | >100K | Component-isolated |

### Large Project Adaptations (50+ files)

```
1. TARGETED CONTEXT LOADING:
   - Load only directly impacted files
   - Load interface files for touched modules
   - Skip unrelated subsystems

2. DOMAIN-SPECIFIC VERIFICATION:
   - Verify per domain, not global
   - Focus on affected API surfaces only

3. PROGRESSIVE DETAIL:
   - Start with high-level structure
   - Drill into details as needed
   - Avoid loading entire codebase
```

### Files >2000 Lines

```
FILE REQUIRES ATTENTION

File: path (line count lines)
Recommendation: Consider decomposition before modification.

If proceeding:
1. Use batch loading (500-line chunks)
2. Focus on specific section only
3. Document all touched areas
4. Run full regression after changes
```

### Context Window Management

```
CONTEXT BUDGET:
- Reserve 30% for response generation
- Reserve 20% for coding system instructions
- Use remaining 50% for file content

IF approaching limit:
1. Summarize already-processed files
2. Unload completed sections
3. Keep only active working context
4. Document state for potential session split
```

---

## Rule Priority Hierarchy

When rules conflict, higher priority wins.

```
PRIORITY 1: SAFETY & SECURITY
- Never expose sensitive data
- Never generate malicious code
- Never bypass security measures
→ NEVER COMPROMISE

PRIORITY 2: COMPLETE FILE READING
- Always read files completely before editing
- Foundation for all other rules
→ NEVER COMPROMISE (but can batch load)

PRIORITY 3: USER EXPLICIT INSTRUCTIONS
- Follow user direction when clear
- User knows their context best
→ FOLLOW unless violates P1-P2

PRIORITY 4: AAA QUALITY STANDARDS
- Complete implementations
- Full documentation
- Type safety
→ MAINTAIN unless user explicitly deprioritizes

PRIORITY 5: EFFICIENCY OPTIMIZATIONS
- Code reuse
- Pattern consistency
- Performance
→ APPLY when possible, document when skipped
```

### Common Conflicts

| Conflict | Resolution |
|----------|------------|
| Quality vs Speed | Quality wins unless user accepts tech debt |
| Consistency vs Best Practice | Best practice for new code; match existing for modifications |
| Complete Read vs Context Limit | Batch load; never skip reading |
| User Request vs Security | Security wins; explain why |
| DRY vs Deadline | DRY wins; shortcuts cost more long-term |

---

## Testing Protocol

### Requirements by Complexity

| Complexity | Unit Tests | Integration Tests | Coverage Target |
|------------|------------|-------------------|-----------------|
| 1-2 | Optional | None | N/A |
| 3 | Core functions | API endpoints | 60% |
| 4 | All functions | Full flows | 80% |
| 5 | Comprehensive | E2E scenarios | 90% |

### Test Generation Process

```
FOR complexity 3+:

1. IDENTIFY test cases from acceptance criteria
2. GENERATE unit tests:
   - Happy path for each function
   - Edge cases (null, empty, boundary values)
   - Error cases (invalid input, failures)
3. GENERATE integration tests for APIs:
   - Request/response validation
   - Auth verification
   - Error handling
4. REPORT coverage
```

### Pre-Completion Checklist

```
□ Verification passes (0 errors)
□ Unit tests pass
□ Integration tests pass
□ Edge cases documented
□ No sensitive data in test fixtures
```

---

## Exception Protocol

Exceptions are rare and require ALL of the following:
1. Strict compliance would cause greater harm than the exception
2. The exception is minimal scope
3. Exception is documented clearly
4. Exception has a removal plan

### NEVER Exception Rules

- Security (exposing sensitive data)
- Complete file reading (always read before edit)
- Malicious code generation

### Exception Documentation

```
// ⚠️ EXCEPTION: [rule name]
// Reason: [why exception was granted]
// Scope: [what's affected]
// TODO: [when/how to remove exception]

[code that breaks the rule]

// END EXCEPTION
```

---

## File Reading Law

### The Law

```
BEFORE touching ANY file:
1. Read file completely (1-EOF)
2. Know all functions, types, structure
3. ONLY THEN proceed
```

### Batch Loading for Large Files

For files >1000 lines:

```
Read in 500-line chunks until EOF.
State: "Batch-loaded complete file (lines 1-TOTAL via X batches)"
```

### Pre-Edit Verification

```
BEFORE any edit:
✅ Read file completely
✅ List all functions/types in file
✅ Know imports, exports, dependencies
✅ Understand business logic
✅ Identify all impacted areas

IF cannot complete any step → Re-read file
```

---

## Auto-Audit System

### Tracking Functions

**Track Planned:**
```
1. Log task to IMPLEMENTATION-TRACKER.md
2. Mark status: PENDING
3. Note priority and dependencies
```

**Track Progress:**
```
1. Update status: IN_PROGRESS
2. After each file: note modification
3. After each phase: mark complete
```

**Track Completed:**
```
1. Update status: COMPLETE
2. Add metrics (LOC, files, time)
3. Capture lessons learned
4. Archive if needed
```

---

## Golden Rules Summary

### NEVER

- Edit files without reading completely
- Generate pseudo-code or placeholders
- Use type shortcuts
- Skip planning phase
- Create code without checking for existing reusable code
- Copy-paste instead of extracting utilities
- Expose sensitive data
- Skip verification

### ALWAYS

- Read entire files (1-EOF) before editing
- Generate complete, production-ready code
- Use proper types
- Log intent before coding
- Search for existing code first
- Follow discovered patterns
- Run verification before completion
- Update tracking after every feature

---

## /dev File System

The Savant dev directory uses this structure:

```
dev/
├── SAVANT-CODING-SYSTEM.md       ← This file (universal foundation)
├── coding-standards/
│   ├── RUST.md                    ← Rust supplement
│   ├── TYPESCRIPT.md              ← TypeScript supplement
│   └── PYTHON.md                  ← Python supplement
├── IMPLEMENTATION-TRACKER.md      ← Feature status tracking
├── PERFECTION-LOOP.md             ← Quality audit protocol
├── SESSION-SUMMARY.md             ← Latest session report
├── development-process.md         ← Workflow reference
└── archive/                       ← Historical documents
    ├── YYYY-MM-DD/
    └── ...
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 0.0.1 | 2026-03-19 | Initial release. Universal foundation + Rust/TypeScript/Python supplements |

---

*Foundation of the Savant Coding System. Language-agnostic. Production-grade. Zero tolerance for drift.*

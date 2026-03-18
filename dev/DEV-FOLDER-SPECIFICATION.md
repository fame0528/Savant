# /dev Folder Specification

> **Version:** 0.0.1  
> **Purpose:** Complete specification of the `/dev` folder structure. Follow this exactly. No guessing.

---

## Overview

The `/dev` folder is the project's operational brain. It tracks everything: what's being built, what's been done, what standards apply, what the session looked like. Every agent working on this project reads from and writes to this folder.

**Rule:** If it's not in `/dev`, it didn't happen.

---

## Folder Structure

```
dev/
├── SAVANT-CODING-SYSTEM.md          ← Universal coding standards (ALL agents read this)
├── coding-standards/
│   ├── RUST.md                       ← Rust supplement
│   ├── TYPESCRIPT.md                 ← TypeScript supplement
│   └── PYTHON.md                     ← Python supplement
├── PERFECTION-LOOP.md                ← Quality audit protocol
├── DEVELOPMENT-WORKFLOW.md           ← Step-by-step development process
├── IMPLEMENTATION-TRACKER.md         ← Feature status (PENDING → IN PROGRESS → COMPLETE)
├── SESSION-SUMMARY.md                ← Report from the most recent session
├── CHANGELOG-INTERNAL.md             ← Project changelog (agent-facing, detailed)
├── archive/
│   └── YYYY-MM-DD/                   ← Archived documents by date
│       └── *.md
└── coding-standards/
    └── <LANGUAGE>.md                 ← One file per language
```

---

## File Specifications

### 1. SAVANT-CODING-SYSTEM.md

| Property | Value |
|----------|-------|
| **Purpose** | Universal coding standards for all languages |
| **Read by** | Every agent, every session, first thing |
| **Updated by** | Only when core standards change (rare) |
| **Format** | Markdown with tables, code blocks |
| **Location** | `dev/SAVANT-CODING-SYSTEM.md` |

**When to read:** Before starting ANY task. No exceptions.

**When to update:** Only when adding/removing/changing a core law, guardian rule, or protocol step. Not during routine development.

---

### 2. coding-standards/<LANGUAGE>.md

| Property | Value |
|----------|-------|
| **Purpose** | Language-specific rules that supplement the foundation |
| **Read by** | Agent when working in that language |
| **Updated by** | When language-specific standards change |
| **Format** | Markdown with code blocks |
| **Location** | `dev/coding-standards/RUST.md`, `TYPESCRIPT.md`, `PYTHON.md` |

**When to read:** After reading the foundation, when working in that specific language.

**When to create:** When the project starts using a new language that doesn't have a supplement yet.

**When to update:** When language-specific patterns change (new framework, new tooling, new conventions).

---

### 3. PERFECTION-LOOP.md

| Property | Value |
|----------|-------|
| **Purpose** | The quality audit protocol (Deep Audit → Enhance → Validate → Iterate → Certify) |
| **Read by** | Agent when performing quality audits |
| **Updated by** | Rarely — only when the loop process itself changes |
| **Location** | `dev/PERFECTION-LOOP.md` |

**When to read:** When running a quality pass, pre-release audit, or "run perfection" command.

**When to update:** Only when the loop steps, termination criteria, or iteration limits change.

---

### 4. DEVELOPMENT-WORKFLOW.md

| Property | Value |
|----------|-------|
| **Purpose** | Step-by-step development process. What to do in order, no guessing. |
| **Read by** | Every agent, after reading coding standards |
| **Updated by** | When workflow process changes |
| **Location** | `dev/DEVELOPMENT-WORKFLOW.md` |

**When to read:** After coding standards, before starting work.

**When to update:** When the development process changes (new step, removed step, changed order).

---

### 5. IMPLEMENTATION-TRACKER.md

| Property | Value |
|----------|-------|
| **Purpose** | Track every feature/fix/task through its lifecycle |
| **Read by** | Every agent, at start and end of each task |
| **Updated by** | After EVERY feature completion. No exceptions. |
| **Format** | Table with columns: Feature, Status, Details |
| **Location** | `dev/IMPLEMENTATION-TRACKER.md` |

**Status values:**

| Status | Meaning | When to use |
|--------|---------|-------------|
| `PENDING` | Not started | Feature planned but no code written |
| `IN PROGRESS` | Actively being worked on | Agent is currently implementing |
| `COMPLETE` | Shipped, tested, documented | Code compiles, tests pass, docs updated |
| `BLOCKED` | Cannot proceed | External dependency missing |
| `CANCELLED` | No longer needed | Requirements changed |

**Rules:**
- Mark `IN PROGRESS` when starting a feature
- Mark `COMPLETE` only when: code compiles, tests pass, docs updated
- Mark `BLOCKED` with reason in Details column
- Never delete entries — mark `CANCELLED` instead

**When to update:** After EVERY feature. This is not optional.

---

### 6. SESSION-SUMMARY.md

| Property | Value |
|----------|-------|
| **Purpose** | Report from the most recent development session |
| **Read by** | User (for review), next agent (for context) |
| **Updated by** | At END of every session |
| **Location** | `dev/SESSION-SUMMARY.md` |

**Required sections:**

```markdown
# Session Summary — YYYY-MM-DD

## Mission
[What was asked]

## Status: ✅ COMPLETE / ⚠️ PARTIAL / ❌ FAILED

## Features Completed
| Feature | Status | Details |
|---------|--------|---------|

## Bugs Fixed
| Bug | File | Fix |
|-----|------|-----|

## Test Results
- Before: X passing, Y failing
- After: Z passing, 0 failing

## Files Changed
- N files modified
- +A / -B lines

## Git
- Commit: <hash>
- Pushed: ✅/❌

## Notes
[Anything the next agent or user needs to know]
```

**When to update:** At the end of every session. Archive the previous summary before writing a new one.

**Archiving:** Before writing a new summary, copy the current one to `dev/archive/YYYY-MM-DD/SESSION-SUMMARY.md`.

---

### 7. CHANGELOG-INTERNAL.md

| Property | Value |
|----------|-------|
| **Purpose** | Detailed project changelog for agents (more detailed than root CHANGELOG.md) |
| **Read by** | Agents (for context on recent changes) |
| **Updated by** | When significant features ship or breaking changes occur |
| **Format** | Keep a Changelog format |
| **Location** | `dev/CHANGELOG-INTERNAL.md` |

**Format:**

```markdown
# Internal Changelog

## [Unreleased]

### Added
- Feature X with details

### Changed
- Modified Y

### Fixed
- Bug Z

## [0.2.0] - 2026-03-19

### Added
- Vector search with semantic memory
- MCP client tool discovery

### Fixed
- Auth error sanitization
- Echo circuit breaker tests
```

**When to update:** When a significant feature ships or a breaking change is made. Not for every small fix.

**Relationship to root CHANGELOG.md:** The root CHANGELOG.md is user-facing and concise. CHANGELOG-INTERNAL.md is agent-facing and detailed. Root CHANGELOG gets updated for releases. Internal changelog gets updated as work happens.

---

### 8. archive/YYYY-MM-DD/

| Property | Value |
|----------|-------|
| **Purpose** | Historical documents — previous versions, old plans, completed work |
| **Read by** | Agents (only when needing historical context) |
| **Updated by** | When archiving documents |
| **Location** | `dev/archive/YYYY-MM-DD/` |

**What goes here:**
- Previous versions of documents before revision
- Completed plans and roadmaps
- Old session summaries (before writing a new one)
- Retired specifications

**Naming:** Date is the date of archival, not the date of the original document.

**Rules:**
- Never delete a document — archive it
- Use the date of archival, not the original date
- Create the directory if it doesn't exist

---

## Versioning

### Document Versioning

Every document in `/dev` has a version number at the top:

```markdown
> **Version:** 0.0.1
```

**Version format:** `MAJOR.MINOR.PATCH`

| Change | Bump | Example |
|--------|------|---------|
| Breaking change to process | MAJOR | 0.0.1 → 1.0.0 |
| New section or significant addition | MINOR | 0.0.1 → 0.1.0 |
| Typo fix, clarification, minor edit | PATCH | 0.0.1 → 0.0.2 |

### Project Versioning

The project version lives in `Cargo.toml` (Rust) or `package.json` (TypeScript). It follows the same MAJOR.MINOR.PATCH format.

**When to bump:**
- MAJOR: Breaking API changes, major architectural shifts
- MINOR: New features, non-breaking additions
- PATCH: Bug fixes, documentation, minor improvements

---

## Changelog Rules

### Root CHANGELOG.md (user-facing)

| Property | Value |
|----------|-------|
| **Audience** | End users, contributors |
| **Format** | Keep a Changelog |
| **Updated** | At release time |
| **Detail level** | Concise, user-impact focused |

### dev/CHANGELOG-INTERNAL.md (agent-facing)

| Property | Value |
|----------|-------|
| **Audience** | Agents, developers |
| **Format** | Keep a Changelog |
| **Updated** | As work happens |
| **Detail level** | Detailed, technical |

### Root README.md

| Property | Value |
|----------|-------|
| **Audience** | New users, GitHub visitors |
| **Updated** | When user-facing features change |
| **Content** | Overview, quick start, links to docs |

---

## Planning Documents

### When to Create a Plan

Create a plan document when:
- A feature requires 3+ files to change
- A feature requires a new crate or module
- A feature has dependencies on other work
- The approach is uncertain and needs design

### Plan Format

```markdown
# Plan: <Feature Name>

**Date:** YYYY-MM-DD
**Status:** PLANNING / APPROVED / IN PROGRESS / COMPLETE

## Objective
[What we're building and why]

## Approach
[How we're building it]

## Files Affected
| File | Action |
|------|--------|
| path/to/file.rs | CREATE |
| path/to/other.rs | MODIFY |

## Dependencies
[What must exist before this can start]

## Acceptance Criteria
[How we know it's done]

## Risks
[What could go wrong]
```

### Where Plans Go

- **Small plans** (inline): Include in IMPLEMENTATION-TRACKER.md Details column
- **Medium plans**: Create as `dev/plans/FEATURE-NAME.md`
- **Large plans**: Create as `dev/plans/FEATURE-NAME.md` with sub-sections

---

## Update Rules (Summary)

| File | When to Update | Frequency |
|------|---------------|-----------|
| SAVANT-CODING-SYSTEM.md | Core standards change | Rare |
| coding-standards/*.md | Language patterns change | Rare |
| PERFECTION-LOOP.md | Audit process changes | Rare |
| DEVELOPMENT-WORKFLOW.md | Workflow changes | Rare |
| IMPLEMENTATION-TRACKER.md | After EVERY feature | Every task |
| SESSION-SUMMARY.md | End of every session | Every session |
| CHANGELOG-INTERNAL.md | Significant features ship | Per feature |
| archive/ | Before revising documents | As needed |

---

## Agent Checklist (Start of Session)

```
□ Read dev/SAVANT-CODING-SYSTEM.md (foundation)
□ Read relevant dev/coding-standards/<LANGUAGE>.md
□ Read dev/IMPLEMENTATION-TRACKER.md (what's pending?)
□ Read dev/SESSION-SUMMARY.md (what happened last time?)
```

## Agent Checklist (End of Session)

```
□ Update dev/IMPLEMENTATION-TRACKER.md (mark features complete)
□ Archive current dev/SESSION-SUMMARY.md to dev/archive/YYYY-MM-DD/
□ Write new dev/SESSION-SUMMARY.md
□ Update dev/CHANGELOG-INTERNAL.md if significant features shipped
□ Commit and push
```

---

*Version 0.0.1. Follow this specification exactly. No guessing.*

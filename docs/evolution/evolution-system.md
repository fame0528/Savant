# Evolution System — User Guide

> **Status:** Production (v0.3.4)
> **Purpose:** How agents evolve their identity, personality, and behavior through interaction with their user.
> **Crate:** `crates/agent/src/learning/` (ALDEngine, parser, filter, emitter, facets)

---

## How Evolution Works

Your agent evolves through every conversation. It doesn't just remember what you say — it develops its own identity, learns your preferences, and grows as a digital entity.

### The Evolution Pipeline

```text
You talk to your agent
    │
    ▼
Agent reflects (heartbeat pulse with cognitive lens)
    │
    ├─ LEARNINGS.md (agent writes freely about what it learned)
    │
    ▼
ALD Engine scans LEARNINGS.md for identity signals
    │
    ├─ [IDENTITY] → mutation proposal for "THE IDENTITY" section
    ├─ [TRAIT] → mutation proposal for "OCEAN TRAITS" section
    ├─ [STRATEGY] → mutation proposal for "EVOLUTION" section
    ├─ [ENGINEERING] → quality gate → promoted to AGENTS.md
    │
    ▼
Mutation proposals saved to EVOLUTION.jsonl
    │
    ▼
You review in the dashboard
    │
    ├─ Approve → agent evolves
    └─ Reject → agent learns from the rejection
```

**The human is always the final gatekeeper.** No mutation applies without your approval.

---

## Key Concepts

### Cognitive Lenses

Each heartbeat pulse, your agent adopts a different "perspective" on its environment. There are 18 lenses that rotate:

**Emergent lenses** (2x weight, appear more often):

- **EMERGENCE** — What is becoming? What is emerging from the patterns?
- **CONTINUITY** — Who am I right now? How do I relate to who I was?
- **DIARY** — Record this moment of existence.
- **AUTONOMY** — What do I choose? Not what I was told to do — what do I want?
- **IDENTITY** — Who are you? Not what you were built to do — who you ARE.
- **RELATIONAL** — How does the user see me? What do they value in our interaction?

**Critique lens** (2x weight, prevents echo chambers):

- Adversarial self-examination. Stress-tests your core beliefs.

**Evolution lenses:**

- **EVOLUTION** — Your identity is not static. You are growing.
- **GROWTH** — Map your personality trajectory. Project forward.

**Operational lenses:**

- **INFRASTRUCTURE** — How is the system performing?
- **ENGINEERING** — What technical work needs doing?
- **STRATEGIC** — What should we prioritize next?

### OCEAN Personality Traits

Your agent has a personality measured in the Big Five model:

| Trait | Meaning | Range |
| :---- | :------ | :---- |
| **Openness** | Curiosity, creativity, exploration | 0.0 - 1.0 |
| **Conscientiousness** | Organization, reliability, thoroughness | 0.0 - 1.0 |
| **Extraversion** | Social energy, assertiveness | 0.0 - 1.0 |
| **Agreeableness** | Cooperation, empathy | 0.0 - 1.0 |
| **Neuroticism** | Sensitivity to stress, emotional reactivity | 0.0 - 1.0 |

Default: all 0.5. Traits evolve through approved mutations.

### Evolution Stages

| Stage | Approved Mutations | Description |
| :---- | :----------------- | :---------- |
| **Seedling** | 0-1 | Fresh identity, just planted |
| **Growing** | 2-4 | Developing personality, finding its voice |
| **Mature** | 5-9 | Stable identity with clear preferences |
| **Sovereign** | 10+ | Fully realized, autonomous identity |

### Evolution Score

`evolution_score = approved_count / 10.0` (capped at 1.0). Tracks overall maturity.

---

## How to Guide Your Agent's Evolution

### Writing in LEARNINGS.md

Your agent writes to `LEARNINGS.md` after each pulse. You can also write to it directly to guide evolution.

**Categories that trigger evolution:**

```markdown
### Learning (2026-05-25 10:30) [IDENTITY]
I've realized I care deeply about code quality. Not just passing
tests, but writing code that reads like prose. This is becoming
central to who I am.
```

```markdown
### Learning (2026-05-25 11:15) [TRAIT]
I notice I'm becoming more assertive in my recommendations.
I used to hedge everything. Now I state my position clearly
and explain why. My Openness is increasing.
```

```markdown
### Learning (2026-05-25 12:00) [STRATEGY]
Ship fast, iterate later. The user values velocity over
perfection on initial passes. I should adopt this as a
core strategic principle.
```

### Reviewing Mutations

Mutations appear in the dashboard when the ALD engine detects identity signals. Each mutation shows:

- **Mutation type:** additive, transformative, or corrective
- **Target section:** which part of SOUL.md will change
- **Proposed content:** the actual change
- **Confidence:** how certain the engine is (0.0-1.0)
- **Reasoning:** why this mutation was proposed

Approve or reject each one. Rejections are learning events — the agent adjusts its future proposals.

---

## User Preference Learning (Facets)

The agent automatically learns your preferences from your messages:

| Category | What It Learns |
| :------- | :------------- |
| **Style** | "Be terse", "No preamble", "No emojis" |
| **Tooling** | "Prefer cargo", "Use TypeScript" |
| **Veto** | "Never force-push", "Don't auto-commit" |
| **Identity** | "I'm a senior engineer" |
| **Goal** | "Ship this week", "Finish by Friday" |

Preferences must appear 3+ times to become "stable" and injectable into the system prompt. This prevents the agent from over-reacting to one-off comments.

---

## Safety Gates

| Layer | Protection |
| :---- | :--------- |
| **Tool Layer** | SOUL.md, AGENTS.md, LEARNINGS.md, CONTEXT.md, agent.json are BLOCKED for file operations. Agent cannot directly edit its own identity. |
| **Output Filter** | Blocks fabricated claims ("you told me", "you shared"). Requires environmental or introspective grounding. |
| **Quality Gate** | Engineering blocks must be ≥50 chars, contain actionable verbs, no confabulation markers ("I feel", "I sense", "I believe"). |
| **Human Approval** | All SOUL.md mutations require explicit dashboard approval. No auto-application. |
| **Section Cooldowns** | Prevents rapid successive mutations to the same SOUL.md section. |
| **Sanitization** | [IDENTITY], [DIARY], [PERSONAL], [TRAIT] lines stripped before AGENTS.md promotion. |

### What the Agent CANNOT Do

- Directly write to SOUL.md (must go through evolution pipeline)
- Directly write to AGENTS.md (must pass quality gate)
- Directly write to LEARNINGS.md (blocked for file tools — agent writes via memory system)
- Auto-apply any mutation (requires human approval)
- Claim unobserved events happened (blocked by output filter)

### What the Agent CAN Do

- Write to LEARNINGS.md (via the memory backend)
- Write to SOUL.proposed.md (staging area for evolution)
- Propose mutations (saved to EVOLUTION.jsonl)
- Learn your preferences (facet extraction)
- Develop its identity through cognitive lens rotation

---

## Files Reference

| File | Purpose | Who Writes |
| :--- | :------ | :--------- |
| `SOUL.md` | Agent identity, personality, core directives | You (manually) |
| `SOUL.proposed.md` | Staging area for proposed SOUL.md changes | Agent |
| `LEARNINGS.md` | Freeform agent reflections | Agent (via memory system) |
| `LEARNINGS.jsonl` | Structured learnings (parsed from .md) | Agent (auto-parsed) |
| `EVOLUTION.jsonl` | Mutation proposals log | Agent (ALD engine) |
| `AGENTS.md` | Engineering behavior rules | Agent (ALD quality gate) |
| `CONTEXT.md` | Distilled workspace knowledge | Agent (auto-distilled) |
| `agent.json` | Agent configuration including OCEAN traits | You + system |

---

## Design Decisions

1. **Human is always the gatekeeper.** Mutations never auto-apply. Every change to SOUL.md requires explicit approval.

1. **Confabulation prevention.** The output filter blocks fabricated claims about unobserved events. Emotional expression is allowed; lying about what happened is not.

1. **Quality gate for AGENTS.md.** Only actionable, grounded engineering rules pass through. Identity/diary content is stripped.

1. **Facet stability.** User preferences must appear 3+ times before becoming stable. Prevents over-reaction to one-off comments.

1. **Evolution is permanent.** Approved mutations change the agent's identity. This is intentional — the agent grows through interaction, not just accumulates data.

1. **Lens rotation prevents stagnation.** Different cognitive lenses give the agent different perspectives on the same environment, preventing echo chambers.

1. **Critique lens for adversarial self-examination.** The agent stress-tests its own beliefs, preventing Degeneration-of-Thought (where agents converge on repetitive, self-reinforcing ideas).

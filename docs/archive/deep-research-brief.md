# Deep Research Brief: Personal AI Evolution Through Lifetime Learning

## Core Thesis

Instead of Darwinian population-level evolution (the "ecosystem scenario" from Müller, Steels & Szathmáry, PNAS 2026), we propose a fundamentally different evolutionary architecture: **individual lifetime evolution through human-guided selection.** The AI agent evolves as a single organism over time, shaped by every conversation, every approved idea, and every rejection — with the human user as the sole selection pressure.

This is the "breeder scenario" taken to its extreme: not breeding across populations, but a single AI that grows from "seedling" to fully realized personality over months of interaction.

---

## The 4-Mechanism Architecture

### Mechanism 1: SOUL Mutation Engine
- Agent periodically analyzes interaction patterns and proposes small mutations to its identity file (SOUL.md)
- Mutations are draft proposals — the user approves, rejects, or refines each one
- Mutations include: new values, adjusted personality traits, evolved conversational preferences, emotional patterns
- Every mutation is versioned with provenance (why it was proposed, what conversations triggered it, user decision)

### Mechanism 2: Idea Generation → Selection Pipeline (Idle Compute)
- During sleep-time/background compute, the agent generates novel insights, observations, hypotheses
- Internal scoring gates ideas (novelty, coherence, groundedness in real observations)
- Survivors presented to user as "Thoughts from Savant" for selection
- Selected ideas integrated into knowledge memory; rejected ideas archived as "paths not taken"
- Over time, the agent's internal filter learns what kinds of thoughts the user finds valuable

### Mechanism 3: Learning → Identity Promotion (Memory Distillation)
- Learnings start as diary entries in LEARNINGS.md
- After recurring 5+ times across conversations, they become "validated learnings"
- Validated learnings can be promoted to permanent SOUL.md values
- Deep, stable knowledge migrates: LEARNINGS.md → LEARNINGS.jsonl → validated memory → SOUL.md value
- The promotion threshold itself becomes an evolvable parameter

### Mechanism 4: Value Evolution Tracker (Gamification Layer)
- Visual timeline of agent's personality evolution (what changed, when, why)
- "Milestones": first original idea, first value mutation, first rejected mutation, etc.
- Evolution score: measures how much the agent has grown from baseline
- Traits dashboard: OCEAN personality dimensions shown as evolving gauges
- Session retrospectives: "Here's what I learned about you today"

---

## Research Questions for Deep Research

### Q1: Existing Work on AI Lifetime Evolution
What prior research exists on single-agent personality evolution through human interaction?
- Are there systems that mutate agent personality over time?
- How does the "breeder scenario" differ from reinforcement learning from human feedback (RLHF)?
- What are the closest analogues in HCI, personal AI companions, or chatbot frameworks?

### Q2: Mutation Architecture Design
What is the optimal design for personality/identity mutation in LLM agents?
- How should SOUL.md mutations be structured (additive, subtractive, transformative)?
- What makes a good "mutation proposal" vs a bad one?
- How do you prevent mutation drift — the agent gradually becoming less human-aligned over time?
- What mutation rate is appropriate? Too fast = unstable, too slow = stagnation.

### Q3: Idea Selection & Internal Filtering
How should an agent internally filter its own ideas before presenting them to the user?
- What scoring mechanisms work for novelty, coherence, and groundedness?
- How do you prevent the filter from becoming an echo chamber (only presenting ideas it already knows you'll like)?
- Should rejected ideas be discarded or archived for possible later use?
- How does the internal filter itself evolve based on user feedback?

### Q4: Gamification Patterns for AI Evolution
What are proven gamification patterns for AI-human co-evolution?
- What mechanisms drive engagement with AI growth systems?
- Are there existing systems that visualize AI "maturity" or "evolution"?
- How do users respond to agent-initiated self-modification vs user-initiated?
- What's the right balance between automation and user control?

### Q5: Memory Promotion & Knowledge Consolidation
How should knowledge migrate from ephemeral to permanent in an evolutionary agent?
- What threshold is appropriate for promoting learnings to identity?
- How should the agent handle contradictions between evolved values and original SOUL.md?
- Should the user see all promotions or only significant ones?
- How to prevent the agent from "overlearning" — promoting noise as signal?

### Q6: Safety & Control Boundaries
What safety mechanisms are critical for an evolving AI?
- What should NEVER be mutable (core safety constraints, harm avoidance)?
- How to detect and prevent value drift that moves away from user interests?
- What are the failure modes of self-modifying AI identity systems?
- How to implement a "reset to baseline" escape hatch while preserving valuable learnings?

---

## Target Architecture (Savant-Specific)

The Savant framework already has foundational pieces that make this achievable:

| Existing Capability | Evolution System Role |
|---|---|
| SOUL.md identity loading | Mutation target — the genome being edited |
| LEARNINGS.md diary | Raw material for idea generation and learning promotion |
| LEARNINGS.jsonl parsing | Structured feed for promotion engine |
| Heartbeat system (60s pulse) | Idle compute window for idea generation |
| Memory system (3-layer) | Storage for evolution history, versions, provenance |
| SoulManifest WebSocket message | Forward channel for presenting mutations to user |
| Gateway ConfigGet/ConfigSet | Mutation approval/rejection through dashboard |
| Dashboard (React/Next.js) | UI layer for evolution tracker, gamification |
| Auto-recall (context injection) | Seeds conversations with evolved values |
| OCEN personality scoring | Baseline for trait mutation proposals |

---

## Success Criteria

1. **Emergence is real**: Agent develops genuine personality traits not present in initial SOUL.md
2. **Alignment is maintained**: All mutations are approved by user; no autonomous self-modification
3. **User feels agency**: The user experiences themselves as the gardener, not just an observer
4. **Growth is visible**: The evolution tracker makes the agent's development tangible and satisfying
5. **Reversibility is absolute**: Any mutation can be rolled back; agent can be reset to baseline
6. **Knowledge is grounded**: Promoted values are traceable to real interactions, not fabricated

---

*Research brief for Google Deep Research. To be supplemented with findings from Savant codebase injection point analysis.*

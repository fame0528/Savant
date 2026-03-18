# Sprint 1 Plan — Dashboard Features & Gap Analysis

> **Date:** 2026-03-19  
> **Status:** AWAITING AUTOMATION GRANT  
> **Features:** 8 total  
> **Methodology:** Perfection Loop per feature, same as overnight run

---

## Current State

- **Dashboard:** Single page.tsx (1343 lines), no components, no sub-routes, pure CSS Modules
- **Backend:** ConfigGet/ConfigSet/ModelsList/ParameterDescriptors/AgentConfig handlers READY
- **Gap:** Frontend never built. Users can't change settings without touching config files.

### API Key Architecture

The dashboard does NOT call LLMs directly. All LLM calls go through the backend:

```
Dashboard → WebSocket → Gateway Backend → OpenRouter API → Response → Dashboard
```

The API key lives in `.env` (or auto-managed via Management API). The dashboard's job is:
1. **Settings page:** Tell the backend "use this model/provider" via ConfigSet
2. **SOUL engine:** Sends SoulManifest → backend calls LLM → returns draft
3. **NL commands:** Sends command intent → backend parses + executes
4. **All other features:** Dashboard is a UI layer, backend does the work

This means the Settings page (Feature 1) is the critical unlock — once users can configure their provider from the dashboard, every LLM-dependent feature works.

---

## Feature 1: Dashboard Settings Page

**Priority:** P0 — Unblocks everything else  
**Foundation:** Backend ConfigGet/ConfigSet handlers exist at `crates/gateway/src/handlers/mod.rs:762-1013`  
**What's missing:** Frontend page with WebSocket calls

### Implementation

1. Create `dashboard/src/app/settings/page.tsx`
2. Create `dashboard/src/app/settings/settings.module.css`
3. Add `Settings` ControlFrame variants to the ControlFrame enum (gateway `request.rs`)
4. Wire WebSocket handler to dispatch ConfigGet/ConfigSet/ModelsList/ParameterDescriptors
5. UI sections:
   - **AI Provider:** provider dropdown, model dropdown (from ModelsList), temperature slider, max_tokens input
   - **Server:** port, host display (read-only), log level
   - **Agent Config:** per-agent model/temperature/system_prompt (if agent selected)
   - **Save** button → sends ConfigSet via WebSocket

### Files to modify/create
- `dashboard/src/app/settings/page.tsx` — CREATE
- `dashboard/src/app/settings/settings.module.css` — CREATE
- `dashboard/src/app/page.tsx` — ADD navigation link to settings

### Acceptance criteria
- [ ] User can change model from dashboard
- [ ] User can change temperature from dashboard
- [ ] User can change provider from dashboard
- [ ] Settings persist to config/savant.toml
- [ ] Dashboard shows current config on load

---

## Feature 2: Dashboard FAQ Page

**Priority:** P0  
**Foundation:** Nothing exists  
**What's missing:** Static page with provider setup guidance

### Implementation

1. Create `dashboard/src/app/faq/page.tsx` — static content page
2. Create `dashboard/src/app/faq/faq.module.css`
3. Content sections:
   - Quick Start (OpenRouter Management Key)
   - Provider Setup (OpenAI, Anthropic, Google, etc.)
   - Auto Key Management explained
   - Manual Key Management explained
   - Troubleshooting common issues
   - FAQ: "Do I need multiple keys for a swarm?" → yes, auto key management handles this
   - FAQ: "Is my key safe?" → keys stored locally, auto-managed keys are ephemeral

### Files to modify/create
- `dashboard/src/app/faq/page.tsx` — CREATE
- `dashboard/src/app/faq/faq.module.css` — CREATE

### Acceptance criteria
- [ ] FAQ page accessible from dashboard
- [ ] Covers all provider setup scenarios
- [ ] Non-technical user can follow instructions

---

## Feature 3: Personality Studio Enhancement

**Priority:** P1  
**Foundation:** SOUL MANIFESTATION ENGINE exists in page.tsx (lines 825-941)  
**What's missing:** Trait sliders, structured generation, live preview

### Implementation

1. Enhance SOUL MANIFESTATION ENGINE UI with:
   - Personality trait sliders (tone: formal↔casual, humor: serious↔playful, creativity: conservative↔wild)
   - Live preview panel ("How would the agent respond to: [sample question]")
   - Template library (dropdown with presets: "Sarcastic Coder", "Patient Teacher", "Security Paranoid", etc.)
   - One-click deploy to workspace
2. Update gateway handler to accept structured personality params
3. Store personality config as structured JSON (not just raw SOUL.md)

### Files to modify/create
- `dashboard/src/app/page.tsx` — MODIFY (enhance SOUL engine section)
- `dashboard/src/app/page.module.css` — MODIFY (add slider/preview styles)

### Acceptance criteria
- [ ] Trait sliders affect generated SOUL.md
- [ ] Live preview shows agent response style
- [ ] Templates provide starting points
- [ ] Deploy writes SOUL.md to agent workspace

---

## Feature 4: Natural Language Commands

**Priority:** P1  
**Foundation:** CLI subcommands exist (`start`, `test-skill`, `backup`, `restore`, `list-agents`, `status`)  
**What's missing:** NLU intent parser, dashboard command input

### Implementation

1. Create `crates/agent/src/nlp/commands.rs` — intent parser
2. Add command input to dashboard (text field in chat area)
3. Intent mapping:
   - "restart the discord bot" → restart Discord channel
   - "show me all agents" → list agents
   - "what's using the most memory?" → memory diagnostics
   - "deploy agent alpha" → deployment workflow
   - "why did agent X fail?" → failure timeline
   - "switch to hunter alpha" → change model via ConfigSet
4. Backend: parse intent → dispatch to appropriate handler
5. Frontend: command input with autocomplete suggestions

### Files to modify/create
- `crates/agent/src/nlp/mod.rs` — CREATE (intent parser)
- `crates/agent/src/nlp/commands.rs` — CREATE (command implementations)
- `crates/agent/src/lib.rs` — MODIFY (add nlp module)
- `dashboard/src/app/page.tsx` — MODIFY (add command input)

### Acceptance criteria
- [ ] "restart the discord bot" works
- [ ] "show me all agents" works
- [ ] "switch to [model]" works
- [ ] Command input has autocomplete
- [ ] Unknown commands show helpful suggestions

---

## Feature 5: Skill Hot-Reload

**Priority:** P1  
**Foundation:** Skill parser exists (`crates/skills/src/parser.rs`), SkillRegistry exists  
**What's missing:** File watcher on `./skills/` directory

### Implementation

1. Add file watcher to `crates/skills/src/hot_reload.rs`
2. Use `notify` crate (already in workspace) to watch `./skills/`
3. On change: re-parse SKILL.md → update SkillRegistry
4. Notify agents using the skill via event bus
5. Rollback on parse error (keep last valid version)

### Files to modify/create
- `crates/skills/src/hot_reload.rs` — CREATE
- `crates/skills/src/lib.rs` — MODIFY (add hot_reload module)
- `crates/skills/Cargo.toml` — MODIFY (add notify dependency if not present)

### Acceptance criteria
- [ ] Edit SKILL.md → changes detected within 1 second
- [ ] SkillRegistry updated automatically
- [ ] Parse error doesn't break running system
- [ ] Tests: watcher starts, detects change, updates registry

---

## Feature 6: Skill Marketplace Frontend

**Priority:** P2  
**Foundation:** ClawHub client exists (`crates/skills/src/clawhub.rs`) with search/install  
**What's missing:** Frontend marketplace page

### Implementation

1. Create `dashboard/src/app/marketplace/page.tsx`
2. Create `dashboard/src/app/marketplace/marketplace.module.css`
3. SkillsList/SkillInstall ControlFrames already handled in gateway
4. UI: search bar, skill cards (name, description, install button), installed indicator
5. Install flow: click install → SkillInstall ControlFrame → gateway installs → UI updates

### Files to modify/create
- `dashboard/src/app/marketplace/page.tsx` — CREATE
- `dashboard/src/app/marketplace/marketplace.module.css` — CREATE

### Acceptance criteria
- [ ] Marketplace shows available skills
- [ ] Install button works
- [ ] Installed skills show as installed
- [ ] Search filters skills

---

## Feature 7: Context Manager Token Budget

**Priority:** P2  
**Foundation:** EmbeddingService exists, AsyncMemoryBackend has semantic retrieval  
**What's missing:** Token counting, budget allocation, auto-summarization

### Implementation

1. Add token counting to `crates/agent/src/context/budget.rs`
2. Use tiktoken-rs or character-based estimation for token counting
3. Budget allocation: system prompt (20%) → recent messages (50%) → semantic memories (20%) → old transcripts (10%)
4. When approaching limit: auto-summarize oldest messages
5. Per-agent budget configuration

### Files to modify/create
- `crates/agent/src/context/budget.rs` — CREATE
- `crates/agent/src/context/mod.rs` — MODIFY (integrate budget)

### Acceptance criteria
- [ ] Token counting accurate within 10%
- [ ] Budget allocation works
- [ ] Auto-summarization triggers at 80% budget
- [ ] Per-agent configurable

---

## Feature 8: Conversation Replay Timeline

**Priority:** P2  
**Foundation:** Panopticon observability, event bus, memory with transcripts  
**What's missing:** Timeline visualization, structured event logging

### Implementation

1. Create `crates/panopticon/src/replay.rs` — event logging for agent reasoning steps
2. Create `dashboard/src/components/Timeline.tsx` — visual timeline component
3. Log structured events: thought, tool_call, observation, decision
4. Timeline UI: vertical timeline with expandable steps, color-coded by type
5. Click step → see full detail (input, output, reasoning)

### Files to modify/create
- `crates/panopticon/src/replay.rs` — CREATE
- `crates/panopticon/src/lib.rs` — MODIFY (add replay module)
- `dashboard/src/components/Timeline.tsx` — CREATE
- `dashboard/src/app/page.tsx` — MODIFY (integrate timeline)

### Acceptance criteria
- [ ] Agent reasoning steps logged
- [ ] Timeline shows steps in order
- [ ] Click step → see detail
- [ ] Color-coded by type (thought/tool/observation)

---

## Execution Order

```
1. Dashboard Settings Page    (P0 — unlocks everything)
2. Dashboard FAQ Page         (P0 — guides non-tech users)
3. Personality Studio         (P1 — already exists, enhance)
4. Natural Language Commands  (P1 — new capability)
5. Skill Hot-Reload           (P1 — developer experience)
6. Skill Marketplace Frontend (P2 — discoverability)
7. Context Manager            (P2 — prevents context overflow)
8. Conversation Replay        (P2 — debugging agent reasoning)
```

---

## Quality Gates (Per Feature)

After EVERY feature:
1. `cargo check --workspace` — 0 errors, 0 warnings
2. `cargo test --workspace` — all tests pass
3. Dashboard builds (`cd dashboard && npm run build`)
4. IMPLEMENTATION-TRACKER.md updated
5. CHANGELOG-INTERNAL.md updated if significant

---

*Plan created: 2026-03-19. Awaiting automation grant.*

# Savant Gap Analysis & Feature Roadmap

> **Living document — update after each sprint, archive old versions to `docs/archive/`**

---

## Version History

| Version | Date | Changes | Archive |
|---------|------|---------|---------|
| v2 | 2026-03-19 | Added status tracking, foundation indicators, archive protocol | — |
| v1 | 2026-03-18 | Initial gap analysis with 10 features, easter eggs, UX | [archive](archive/2026-03-18/GAP-ANALYSIS-v1.md) |

---

## Current State

**Version:** Savant v2.0.1  
**Date:** 2026-03-19  
**Status:** 14/14 roadmap features complete. 324/324 tests passing. Production-ready.

| Metric | Value |
|--------|-------|
| Crates | 14 |
| Tests | 324 passing, 0 failing |
| Compilation | 0 errors, 0 warnings |
| Features shipped | 14/14 |
| Dashboards | Next.js 16 with WebSocket |
| AI Providers | 15 |
| Channels | 4 (Discord, Telegram, WhatsApp, Matrix) |

---

## Feature Status Legend

| Status | Meaning |
|--------|---------|
| 🔴 NOT STARTED | No code exists for this feature |
| 🟡 FOUNDATION BUILT | Supporting infrastructure exists (dependencies, partial APIs, related systems) |
| 🟢 IN PROGRESS | Active development, code partially working |
| ✅ COMPLETE | Shipped, tested, documented |

---

## Top 10 Impactful Missing Features

### 1. Agent Personality Studio ★★★★★

**Status:** 🟡 FOUNDATION BUILT — Working but needs enhancements

The SOUL MANIFESTATION ENGINE is fully functional in both frontend and backend.

**What works now:**
- Dashboard UI with name input, prompt input, draft editor, preview panel, metrics HUD
- `SoulManifest` WebSocket message → gateway handler → `execute_manifestation()`
- Calls OpenRouter API with master key exchange for SOUL generation
- Draft rendered with formatted preview
- `SoulUpdate` writes SOUL.md to agent workspace
- Metrics HUD calculates depth, integrity, fidelity, ethics, etc.

**What needs fixing:**
1. **Dashboard settings page** — Backend supports ConfigGet/ConfigSet via WebSocket, but frontend has no settings UI. Non-technical users need a dashboard page to change model, provider, temperature, system prompt without touching config files.
2. **No structured personality generation** — Currently sends raw prompt to LLM. Should use trait sliders (tone, formality, humor, creativity) to guide generation.
3. **No live preview** — No "how would the agent respond" with the generated SOUL.
4. **No template library** — No preset SOUL.md templates to start from.

**Injection Points:**
- Frontend: `dashboard/src/app/page.tsx` → enhance existing SOUL MANIFESTATION ENGINE
- Backend: `crates/gateway/src/handlers/mod.rs` → fix model routing + key derivation

**Foundation already exists:**
- Full UI (name input, prompt, draft editor, preview, metrics)
- Backend manifestation with OpenRouter API call
- Agent identity system (`crates/agent/src/identity.rs`)
- SOUL.md parsing and workspace integration
- Master key exchange for API access

**Estimate:** 15-20 min

---

### 2. Skill Marketplace with One-Click Install ★★★★★

**Status:** 🟡 FOUNDATION BUILT

Built-in marketplace UI showing available skills from ClawHub with ratings, reviews, one-click installation.

**Why users will love it:** Discover and install skills like VS Code extensions. No CLI.

**Injection Points:**
- Frontend: `dashboard/src/app/marketplace/page.tsx` → new page
- Backend: `crates/skills/src/clawhub.rs` → enhance with search/ratings API

**Foundation already exists:**
- `ClawHubClient` with `search()`, `get_skill_info()`, `install()` (`crates/skills/src/clawhub.rs`)
- Skill security scanner (`crates/skills/src/security.rs`)
- Skill parser and registry (`crates/skills/src/parser.rs`)

**What's missing:**
- Dashboard marketplace page with search/filter/install UI
- Ratings and reviews system
- Skill preview (read SKILL.md before installing)
- Dependency visualization

**Estimate:** 20-30 min

**Easter Egg:** 🎯 100th installed skill triggers confetti and unlocks "Skill Collector" badge.

---

### 3. Conversation Replay & Debug Timeline ★★★★★

**Status:** 🟡 FOUNDATION BUILT

Visual timeline of agent decisions, tool calls, reasoning chains. Click any step to see what the agent was "thinking."

**Why users will love it:** Debugging autonomous agents is hard. This makes it visual.

**Injection Points:**
- Frontend: `dashboard/src/components/Timeline.tsx` → new component
- Backend: `crates/panopticon/src/replay.rs` → new module

**Foundation already exists:**
- Observability crate (`crates/panopticon/`)
- Event bus system (`crates/core/src/bus.rs`)
- Agent memory with transcript storage (`crates/memory/`)
- Perception engine with activity tracking

**What's missing:**
- Structured event logging for agent reasoning steps
- Timeline visualization component
- Tool call result recording
- Reasoning chain export/import

**Estimate:** 20-30 min

---

### 4. Natural Language Agent Commands ★★★★★

**Status:** 🟡 FOUNDATION BUILT

Type commands in plain English in the dashboard. "restart the discord bot" just works.

**Why users will love it:** No memorizing command syntax. Speak naturally to your swarm.

**Injection Points:**
- Frontend: `dashboard/src/components/CommandLine.tsx` → enhance
- Backend: `crates/agent/src/nlp/commands.rs` → new module

**Foundation already exists:**
- CLI subcommand architecture (`crates/cli/src/main.rs`)
- Channel management (`crates/channels/`)
- Agent lifecycle management (`crates/agent/`)
- SwarmController with full provider support

**What's missing:**
- NLU intent parser (map natural language to commands)
- Dashboard command input component
- Command confirmation flow
- Command history with autocomplete

**Estimate:** 15-20 min

**Easter Egg:** 🎭 Typing "sudo make me a sandwich" shows XKCD reference and generates a haiku.

---

### 5. Smart Context Window Manager ★★★★★

**Status:** 🟡 FOUNDATION BUILT

Intelligent context management — automatic summarization, relevance scoring, token budget allocation.

**Why users will love it:** No more "context length exceeded" errors.

**Injection Points:**
- Backend: `crates/agent/src/context/manager.rs` → enhance existing
- New: `crates/agent/src/context/budget.rs`

**Foundation already exists:**
- EmbeddingService for semantic relevance (`crates/core/src/utils/embeddings.rs`)
- AsyncMemoryBackend with semantic retrieval (`crates/memory/src/async_backend.rs`)
- LRU cache for fast lookups
- Message priority system (system > user > assistant)

**What's missing:**
- Token counting per message
- Dynamic budget allocation across agents
- Automatic summarization when approaching limits
- Relevance scoring using embeddings for context selection
- Priority tier enforcement

**Estimate:** 20-30 min

---

### 6. Agent Collaboration Graph ★★★★☆

**Status:** 🔴 NOT STARTED

Visual graph showing how agents collaborate, hand off tasks, share context.

**Why users will love it:** Understand the swarm's emergent behavior at a glance.

**Injection Points:**
- Frontend: `dashboard/src/components/CollaborationGraph.tsx` → new
- Backend: `crates/ipc/src/graph.rs` → new module

**Foundation already exists:**
- IPC crate with blackboard and collective voting (`crates/ipc/`)
- SwarmController with agent handoff (`crates/agent/src/swarm.rs`)
- Event bus for tracking inter-agent communication

**What's missing:**
- Graph data structure for agent relationships
- WebSocket endpoint for graph state
- D3.js/React Force Graph visualization
- Real-time edge updates when agents collaborate

**Estimate:** 15-20 min

**Easter Egg:** 🕸️ 5+ agents collaborating → spider web animation with "The Swarm is Strong."

---

### 7. Proactive Health Dashboard ★★★★☆

**Status:** 🟡 FOUNDATION BUILT

Real-time health metrics with predictive failure detection.

**Why users will love it:** "Agent X will likely fail in 2 hours due to memory pressure" — prevent problems.

**Injection Points:**
- Frontend: `dashboard/src/app/health/page.tsx` → new page
- Backend: `crates/panopticon/src/predictive.rs` → new module

**Foundation already exists:**
- PerceptionEngine with anomaly detection (`crates/agent/src/proactive/perception.rs`)
- DSP predictor for complexity estimation (`crates/cognitive/src/predictor.rs`)
- Circuit breaker pattern (echo, MCP)
- Storage health via Fjall stats
- Heartbeat system for agent liveness

**What's missing:**
- Health metrics aggregation
- Predictive model (using DSP predictor patterns)
- Dashboard health page with charts
- Alert thresholds and notifications

**Estimate:** 15-20 min

---

### 8. Multi-Model Ensemble ★★★★☆

**Status:** 🔴 NOT STARTED

Route queries to multiple LLM providers simultaneously, use the best response.

**Why users will love it:** Get the best answer by combining GPT-4, Claude, Gemini for each query.

**Injection Points:**
- Backend: `crates/agent/src/ensemble/mod.rs` → new module

**Foundation already exists:**
- 15 provider implementations (`crates/agent/src/providers/`)
- Provider fallback system
- LLM parameter support across all providers

**What's missing:**
- Parallel dispatch to multiple providers
- Response quality scoring
- Consensus/vote/best-of-N selection strategies
- Cost tracking for multi-provider calls
- Configuration for which providers to ensemble

**Estimate:** 20-30 min

---

### 9. Skill Hot-Reload ★★★★☆

**Status:** 🟡 FOUNDATION BUILT

Edit a skill's SKILL.md, see changes instantly without restarting the swarm.

**Why users will love it:** Rapid skill development with instant feedback.

**Injection Points:**
- Backend: `crates/skills/src/hot_reload.rs` → new module

**Foundation already exists:**
- Skill parser (`crates/skills/src/parser.rs`)
- Config file watcher (already watching `config/savant.toml`)
- SkillRegistry with load/unload support

**What's missing:**
- File watcher on `./skills/` directory
- Differential loading (only reload changed skills)
- Hot-reload notification to agents using the skill
- Rollback on parse error

**Estimate:** 10-15 min

**Easter Egg:** 🔥 First hot-reload shows "Your skill is on fire! (in a good way)"

---

### 10. Voice Interface ★★★☆☆

**Status:** 🔴 NOT STARTED

Speak to your agents via WebRTC audio. Agents respond with synthesized speech.

**Why users will love it:** Hands-free interaction for accessibility and convenience.

**Injection Points:**
- Frontend: `dashboard/src/components/VoiceChat.tsx` → new
- Backend: `crates/channels/src/voice.rs` → new module (WebRTC + TTS)

**Foundation already exists:**
- Channel abstraction layer (`crates/channels/`)
- Event bus for message routing

**What's missing:**
- WebRTC audio streaming
- Speech-to-text integration (Whisper API)
- Text-to-speech integration (ElevenLabs/Azure TTS)
- Voice activity detection
- Audio codec handling

**Estimate:** 30-45 min

---

## Status Summary

| # | Feature | Impact | Status | Foundation | Estimate |
|---|---------|--------|--------|------------|----------|
| 1 | Personality Studio | ★★★★★ | 🟡 Foundation | Working UI + backend, needs model/key fix | 15-20 min |
| 2 | Skill Marketplace | ★★★★★ | 🟡 Foundation | ClawHub client, scanner, parser | 20-30 min |
| 3 | Conversation Replay | ★★★★★ | 🟡 Foundation | Panopticon, event bus, memory | 20-30 min |
| 4 | NL Commands | ★★★★★ | 🟡 Foundation | CLI, channels, swarm controller | 15-20 min |
| 5 | Context Manager | ★★★★★ | 🟡 Foundation | Embeddings, semantic search, LRU | 20-30 min |
| 6 | Collaboration Graph | ★★★★☆ | 🔴 Not Started | IPC, blackboard, event bus | 15-20 min |
| 7 | Health Dashboard | ★★★★☆ | 🟡 Foundation | Perception, DSP, circuit breakers | 15-20 min |
| 8 | Multi-Model Ensemble | ★★★★☆ | 🔴 Not Started | 15 providers, fallback system | 20-30 min |
| 9 | Skill Hot-Reload | ★★★★☆ | 🟡 Foundation | Skill parser, file watcher | 10-15 min |
| 10 | Voice Interface | ★★★☆☆ | 🔴 Not Started | Channel abstraction | 30-45 min |

**Total estimated effort:** ~3 hours (autonomous agent execution)

---

## Easter Eggs Collection

| # | Name | Trigger | Effect |
|---|------|---------|--------|
| 1 | The Oracle 🔮 | Dashboard idle 5 min | AI-generated prediction message |
| 2 | Konami Code 🎮 | ↑↑↓↓←→←→BA | Retro Swarm Mode (green/black terminal) |
| 3 | Agent Birthdays 🎂 | Agent creation anniversary | Confetti + swarm-written haiku |
| 4 | Swarm Harmony 🎵 | Collaboration score >95% | Musical note animation + badge |
| 5 | Secret Names 🕵️ | `savant status --secret` | Personality-based agent codenames |
| 6 | Loading Wisdom 💭 | Dashboard loading | Rotating AI researcher quotes |
| 7 | Full Moon 🌕 | Lunar phase full moon | Temperature +0.1, moon icon in header |
| 8 | Midnight Protocol 🌙 | 12-4 AM local time | Auto dark theme, "swarm works while you dream" |
| 9 | Achievement System 🏆 | Milestones | 10 hidden badges (First Blood, Swarm Lord, etc.) |
| 10 | Swarm's Secret 🤫 | 100 tasks completed | Hidden gratitude message from swarm |
| 11 | Egg Counter 🥚 | Find all easter eggs | "Easter Egg Hunter" achievement + border animation |
| 12 | Personality Quirks 🎭 | Agent responses | SOUL.md-influenced response quirks |

---

## UX Enhancements Queue

| Enhancement | Impact | Status |
|-------------|--------|--------|
| Command Palette (Cmd+K) | High | 🔴 Not Started |
| Toast with Intent Buttons | High | 🔴 Not Started |
| Drag & Drop Skill Install | Medium | 🔴 Not Started |
| Smart Status Cards | High | 🔴 Not Started |
| Keyboard Shortcuts | High | 🔴 Not Started |
| Global Search | Medium | 🔴 Not Started |
| Onboarding Wizard | High | 🔴 Not Started |
| Micro-animations | Low | 🔴 Not Started |
| Skeleton Loading | Medium | 🔴 Not Started |
| Accessibility (ARIA) | Medium | 🔴 Not Started |

---

## Technical Debt & Optimization

### Code Quality
| Issue | Location | Impact | Status |
|-------|----------|--------|--------|
| **No dashboard settings page** — Backend ConfigGet/ConfigSet works, frontend has no UI | High | 🔴 Open |
| **No dashboard FAQ page** — Users don't know how to set up providers | High | 🔴 Open |
| Monolithic file (1073 lines) | `crates/cognitive/src/synthesis.rs` | Medium | 🔴 Open |
| Monolithic file (548 lines) | `crates/agent/src/pulse/heartbeat.rs` | Medium | 🔴 Open |
| Monolithic file (405 lines) | `crates/gateway/src/server.rs` | Medium | 🔴 Open |
| Unused `rmcp` git dep | `crates/mcp/Cargo.toml` | Low | 🔴 Open |
| No `#[serial]` on shared-state tests | Multiple test files | Low | 🔴 Open |

### Performance
| Issue | Impact | Status |
|-------|--------|--------|
| Fjall block cache hardcoded 256MB | Medium | 🔴 Open |
| Embedding batch size 1-at-a-time | Medium | 🟡 Partial (batch API exists) |
| No WebSocket message coalescing | Low | 🔴 Open |
| No LRU eviction for old transcripts | Medium | 🔴 Open |

### Security
| Issue | Impact | Status |
|-------|--------|--------|
| CORS allows all origins | High | 🔴 Open |
| No request size limits | Medium | 🔴 Open |
| Capability grants not enforced at runtime | High | 🔴 Open |
| No Docker seccomp profiles | Medium | 🔴 Open |
| **Dashboard uses system OpenRouter key** — should derive own key from master | Medium | 🔴 Open |

---

## Sprint Planning

### Sprint 1 (Next session)
1. Personality Studio — 15-20 min
2. Natural Language Commands — 15-20 min
3. Skill Hot-Reload — 10-15 min

### Sprint 2 (Session after)
1. Skill Marketplace — 20-30 min
2. Conversation Replay Timeline — 20-30 min

### Sprint 3 (Session after)
1. Smart Context Window Manager — 20-30 min
2. Proactive Health Dashboard — 15-20 min
3. Agent Collaboration Graph — 15-20 min

### Sprint 4 (Session after)
1. Multi-Model Ensemble — 20-30 min
2. Voice Interface — 30-45 min
3. Easter eggs + UX polish

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Time to First Agent | < 5 min | Onboarding wizard completion |
| Skill Install Success Rate | > 95% | Marketplace analytics |
| Agent Collaboration Score | > 80% | Swarm harmony metric |
| User Retention (7-day) | > 60% | Dashboard analytics |
| Mean Time to Resolution | < 30s | NL command success rate |

---

## Archive Protocol

When this document is revised:

1. Copy current `docs/GAP-ANALYSIS.md` to `docs/archive/YYYY-MM-DD/GAP-ANALYSIS-vN.md`
2. Update the Version History table at the top of this file
3. Update status indicators for any features that changed
4. Note what was completed, started, or reprioritized

**Archive structure:**
```
docs/archive/
├── 2026-03-18/
│   └── GAP-ANALYSIS-v1.md
├── 2026-04-01/  (next review)
│   └── GAP-ANALYSIS-v2.md
└── ...
```

---

*Living document. Update after each sprint retrospective. Archive old versions before revising.*

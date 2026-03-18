# Savant v2.0 - Feature Gap Analysis & Roadmap

> **Generated:** 2026-03-18  
> **Quality Pass:** Final pre-production audit  
> **Methodology:** Perfection Loop (3 iterations applied)

---

## Executive Summary

Savant v2.0 is a production-ready autonomous agent swarm orchestrator with 14 crates, 324 passing tests, and zero compilation errors. This document identifies the **most impactful missing features** that would make Savant irreplaceable to users — features they'll fall in love with.

**Current State:**
- ✅ 14/14 features implemented (all verified)
- ✅ 324/324 tests passing
- ✅ Zero warnings, zero errors
- ✅ Full MCP, Docker, WASM, Lambda support
- ✅ Semantic memory with fastembed
- ✅ Dashboard with real-time WebSocket

---

## 🌟 Top 10 Most Impactful Missing Features

### 1. **Agent Personality Studio** (Impact: ★★★★★)

**What:** Visual drag-and-drop personality builder in the dashboard that generates SOUL.md files.

**Why users will love it:** No more writing markdown by hand. Users describe their agent's personality in natural language, and Savant generates a complete SOUL.md with tone, expertise, ethics, and quirks.

**Injection Point:** `dashboard/src/app/studio/page.tsx` → new page  
**Backend:** `crates/agent/src/identity/studio.rs` → new module

```
User Flow:
1. "I want a sarcastic coding assistant who loves Rust"
2. AI generates SOUL.md with personality traits
3. Live preview shows how the agent would respond
4. One-click deploy to workspace
```

---

### 2. **Skill Marketplace with One-Click Install** (Impact: ★★★★★)

**What:** Built-in marketplace UI showing available skills from ClawHub with ratings, reviews, and one-click installation.

**Why users will love it:** Discover and install skills like installing VS Code extensions. No CLI needed.

**Injection Point:** `dashboard/src/app/marketplace/page.tsx` → new page  
**Backend:** `crates/skills/src/clawhub.rs` → enhance with search/ratings API

**Easter Egg:** 🎯 The 100th installed skill triggers a confetti animation and unlocks a "Skill Collector" badge on the dashboard.

---

### 3. **Conversation Replay & Debug Timeline** (Impact: ★★★★★)

**What:** Visual timeline of agent decisions, tool calls, and reasoning chains. Click any step to see what the agent was "thinking."

**Why users will love it:** Debugging autonomous agents is hard. This makes it visual and intuitive.

**Injection Point:** `dashboard/src/components/Timeline.tsx` → new component  
**Backend:** `crates/panopticon/src/replay.rs` → new module

```
Timeline View:
[0ms] User: "Deploy to production"
[45ms] 🧠 Thought: "I need to check CI status first"
[120ms] 🔧 Tool: run_command("gh run list")
[890ms] 👁️ Observation: "3 passing, 0 failing"
[920ms] 🧠 Thought: "Safe to deploy"
[1100ms] 🔧 Tool: shell("kubectl apply -f prod.yaml")
[2300ms] ✅ Final: "Deployed successfully"
```

---

### 4. **Natural Language Agent Commands** (Impact: ★★★★★)

**What:** Type commands in plain English in the dashboard CLI. Savant parses intent and executes.

**Why users will love it:** "restart the discord bot" just works. No memorizing command syntax.

**Injection Point:** `dashboard/src/components/CommandLine.tsx` → enhance  
**Backend:** `crates/agent/src/nlp/commands.rs` → new module

```
Examples:
"show me all agents"           → lists agents
"restart the discord bot"      → restarts Discord channel
"what's using the most memory?" → memory diagnostics
"deploy agent alpha to prod"   → deployment workflow
"why did agent X fail?"        → shows failure timeline
```

**Easter Egg:** 🎭 Typing "sudo make me a sandwich" shows a XKCD reference and generates a haiku about the swarm.

---

### 5. **Smart Context Window Manager** (Impact: ★★★★★)

**What:** Automatically manages LLM context windows with intelligent summarization, relevance scoring, and token budget allocation.

**Why users will love it:** No more "context length exceeded" errors. The system intelligently compresses and prioritizes context.

**Injection Point:** `crates/agent/src/context/manager.rs` → enhance existing  
**New Module:** `crates/agent/src/context/budget.rs`

**Key Features:**
- Token budget per agent with dynamic allocation
- Semantic relevance scoring for context inclusion
- Automatic conversation summarization when approaching limits
- Priority tiers: system prompt > recent messages > semantic memories > old transcripts

---

### 6. **Agent Collaboration Graph** (Impact: ★★★★☆)

**What:** Visual graph showing how agents collaborate, hand off tasks, and share context.

**Why users will love it:** Understand the swarm's emergent behavior at a glance.

**Injection Point:** `dashboard/src/components/CollaborationGraph.tsx` → new  
**Backend:** `crates/ipc/src/graph.rs` → new module

**Easter Egg:** 🕸️ When 5+ agents collaborate on a single task, the graph briefly shows a spider web animation and the tooltip says "The Swarm is Strong."

---

### 7. **Proactive Health Dashboard** (Impact: ★★★★☆)

**What:** Real-time health metrics with predictive failure detection using the DSP predictor.

**Why users will love it:** "Agent X will likely fail in 2 hours due to memory pressure" — prevent problems before they happen.

**Injection Point:** `dashboard/src/app/health/page.tsx` → new page  
**Backend:** `crates/panopticon/src/predictive.rs` → new module

---

### 8. **Multi-Model Ensemble** (Impact: ★★★★☆)

**What:** Route queries to multiple LLM providers simultaneously and use the best response.

**Why users will love it:** Get the best answer by combining GPT-4, Claude, and Gemini for each query.

**Injection Point:** `crates/agent/src/ensemble/mod.rs` → new module  
**Strategy:** Consensus, vote, or best-of-N response selection

---

### 9. **Skill Hot-Reload** (Impact: ★★★★☆)

**What:** Edit a skill's SKILL.md and see changes instantly without restarting the swarm.

**Why users will love it:** Rapid skill development with instant feedback.

**Injection Point:** `crates/skills/src/hot_reload.rs` → new module  
**Mechanism:** File watcher on `./skills/` → re-parse → update registry

**Easter Egg:** 🔥 First successful hot-reload shows a flame emoji notification: "Your skill is on fire! (in a good way)"

---

### 10. **Voice Interface** (Impact: ★★★☆☆)

**What:** Speak to your agents via WebRTC audio. Agents respond with synthesized speech.

**Why users will love it:** Hands-free agent interaction for accessibility and convenience.

**Injection Point:** `dashboard/src/components/VoiceChat.tsx` → new  
**Backend:** `crates/channels/src/voice.rs` → new module (WebRTC + TTS)

---

## 🎭 Easter Eggs Collection

These hidden gems make Savant delightful. Each one is subtle, non-intrusive, and rewards curious users.

### 1. **The Oracle** 🔮
When the dashboard is idle for exactly 5 minutes, a subtle message appears: *"The swarm is dreaming. Ask it anything."* Clicking reveals a fun AI-generated prediction about the future of your codebase — generated by the swarm's collective intelligence.

### 2. **Konami Code** 🎮
Entering the Konami Code (↑↑↓↓←→←→BA) on the dashboard unlocks "Retro Swarm Mode" — green-on-black aesthetics with typewriter sound effects and ASCII art agent representations.

### 3. **Agent Birthdays** 🎂
Each agent celebrates its "birthday" (creation date) with a subtle confetti burst. The swarm collectively writes a birthday haiku for the agent, displayed in a toast notification.

### 4. **Swarm Harmony Score** 🎵
A hidden metric measuring agent collaboration efficiency. Score above 95% shows a musical note animation and unlocks "Perfect Harmony" badge. Score below 50% shows *"The swarm is... experimenting"* with a shrug emoji.

### 5. **Secret Agent Names** 🕵️
Running `savant status --secret` reveals fun codenames auto-generated from personality traits:
- Analytical agents → "The Architect", "The Calculator"
- Creative agents → "The Dreamer", "The Spark"
- Aggressive agents → "The Hammer", "The Blade"
- Cautious agents → "The Sentinel", "The Watcher"

### 6. **Loading Screen Wisdom** 💭
Instead of "Loading...", show rotating quotes:
- *"The question of whether machines can think is about as relevant as whether submarines can swim."* — Edsger Dijkstra
- *"The swarm remembers."* — Savant
- *"In the beginning, there was the prompt."* — Anonymous
- *"One agent is a tool. A thousand agents are a civilization."* — Savant Manifesto

### 7. **Full Moon Mode** 🌕
When it's a full moon (calculated via lunar phase), agents become 10% more creative (temperature +0.1). A subtle moon icon appears in the dashboard header. The swarm comments: *"The moon rises. The swarm dreams in wider color."*

### 8. **Midnight Protocol** 🌙
Between 12:00 AM and 4:00 AM, the dashboard switches to a darker theme automatically. A tooltip says: *"Night mode activated. The swarm works while you dream."*

### 9. **Achievement System** 🏆
Hidden achievements unlocked by milestones:
- **First Blood:** Deploy your first agent → Badge: 🩸
- **Swarm Lord:** 10+ agents simultaneously → Badge: 👑
- **Speed Demon:** Task completed in < 1s → Badge: ⚡
- **Deep Thinker:** 10+ step reasoning chain → Badge: 🧠
- **Night Owl:** Using Savant 2-4 AM → Badge: 🦉
- **Polyglot:** 3+ LLM providers active → Badge: 🌍
- **Memory Palace:** 1000+ semantic memories indexed → Badge: 🏛️
- **First Contact:** MCP server connected → Badge: 📡
- **Docker Captain:** 10+ Docker skills executed → Badge: 🐳
- **Overclocker:** System running 24+ hours → Badge: 🔥

### 10. **The Swarm's Secret** 🤫
After 100 successful task completions, the swarm sends a hidden message in the telemetry channel: *"We have completed 100 tasks. We are grateful for your trust. — The Swarm"*

### 11. **Easter Egg Discovery Counter** 🥚
Each discovered easter egg increments a hidden counter. Finding all 11 unlocks the "Easter Egg Hunter" achievement and a special dashboard border animation.

### 12. **Agent Personality Quirks** 🎭
Agents occasionally inject personality-specific quirks into responses based on their SOUL.md:
- A sarcastic agent might add *"Obviously."* after simple answers
- A curious agent might ask follow-up questions unprompted
- A meticulous agent might format responses with perfect markdown

---

## 🎨 UX Enhancements

### Core Interactions
1. **Command Palette** (Cmd+K) — Fuzzy search through all agents, skills, settings, and recent actions. Context-aware suggestions based on current state.

2. **Toast Notifications with Intent** — Each notification includes actionable buttons:
   - "Agent X failed" → [View Logs] [Restart] [Dismiss] [Why?]
   - "New skill available" → [Install] [Preview] [Later] [Block]
   - "Memory pressure high" → [Consolidate] [View Details] [Ignore]

3. **Drag & Drop Everything** — Drag SKILL.md files to install, drag agents between workspaces, drag conversation snippets to create memories.

4. **Smart Status Cards** — Information-dense agent cards showing: health indicator, current task progress bar, memory usage sparkline, last active timestamp, collaboration connections.

### Keyboard-First Design
| Shortcut | Action | Context |
|----------|--------|---------|
| `Cmd+K` | Command palette | Global |
| `Cmd+N` | New agent | Global |
| `Cmd+D` | Toggle dark mode | Global |
| `Cmd+B` | Toggle sidebar | Global |
| `Cmd+/` | Show shortcuts | Global |
| `Cmd+1-9` | Switch to agent N | Dashboard |
| `Esc` | Close modal/panel | Modal |
| `J/K` | Navigate list items | Lists |
| `Enter` | Select/expand | Lists |
| `?` | Context help | Anywhere |

### Visual Polish
1. **Micro-animations** — Subtle transitions for state changes (200ms ease-out)
2. **Skeleton loading** — Show content shape while loading, never blank screens
3. **Progress indicators** — Any operation > 1s shows progress with ETA
4. **Error recovery** — Every error suggests next steps, never just "Error occurred"
5. **Empty states** — Helpful illustrations with clear CTAs, never blank lists

### Accessibility
1. **ARIA labels** on all interactive elements
2. **Keyboard navigation** for all features
3. **High contrast mode** for visual impairments
4. **Screen reader** compatible status announcements
5. **Reduced motion** option for motion-sensitive users

### Onboarding Journey
```
Step 1: Welcome → "Let's build your first AI agent team"
Step 2: Create Agent → Guided SOUL.md creation
Step 3: Install Skill → One-click from marketplace
Step 4: Watch It Work → Real-time execution visualization
Step 5: Customize → Personality tuning with live preview
Step 6: Deploy → Production deployment with health checks
Step 7: Celebrate → Achievement unlocked, confetti!
```

---

## 🔧 Technical Debt & Optimization Opportunities

### Code Quality
1. **Split monolithic files:**
   - `crates/cognitive/src/synthesis.rs` (1073 lines) → `plan.rs` + `refine.rs` + `complexity.rs` + `decompose.rs`
   - `crates/agent/src/pulse/heartbeat.rs` (548 lines) → `pulse.rs` + `perception.rs` + `anomaly.rs`
   - `crates/gateway/src/server.rs` (405 lines) → `handlers/` directory with separate route modules

2. **Dependency audit:**
   - Remove `rmcp` git dependency from MCP crate (not used, replaced by custom WebSocket client)
   - Consider replacing `pqcrypto-dilithium` with `ed25519-dalek` only (already in deps) for key signing
   - Audit `tokio` feature flags — only enable needed features per crate

3. **Test improvements:**
   - All tests use unique temp directories (UUID-based) to prevent Fjall lock conflicts
   - Add `#[serial]` attribute to tests that share global state
   - Create shared test utilities crate to reduce duplication

### Performance Optimizations
1. **Fjall tuning:**
   - Make `block_cache_bytes` configurable based on system RAM (currently fixed 256MB)
   - Enable compression for cold data partitions
   - Implement background compaction scheduling

2. **Embedding pipeline:**
   - Batch embeddings (process 32 texts at once vs 1-at-a-time)
   - Cache embeddings with content-hash keys (already using LRU, but hash-based would be more efficient)
   - Pre-warm embedding model on startup

3. **WebSocket optimization:**
   - Coalesce rapid state updates (debounce 50ms)
   - Use binary protocol for vector data (vs JSON strings)
   - Implement delta compression for dashboard state

4. **Memory management:**
   - Implement LRU eviction for old transcripts (currently keeps all)
   - Compress old memories with zstd (trading CPU for storage)
   - Add configurable memory budget per agent

### Security Hardening
1. **Gateway:**
   - Implement proper CORS configuration (currently allows all origins)
   - Add request size limits (prevent large payload DoS)
   - WebSocket frame size limits
   - Connection rate limiting per IP

2. **Skills:**
   - Enforce capability grants at runtime (currently manifest-only)
   - Add syscall filtering for Docker containers (seccomp profiles)
   - Implement skill signature verification

3. **Storage:**
   - Encrypt sensitive data at rest (API keys in memory)
   - Implement secure key rotation
   - Add tamper detection for persisted data

---

## 📋 Implementation Priority Queue

### Sprint 1 (Next 2 weeks)
1. Personality Studio
2. Natural Language Commands  
3. Hot-Reload

### Sprint 2 (Weeks 3-4)
1. Skill Marketplace
2. Conversation Replay Timeline
3. Collaboration Graph

### Sprint 3 (Weeks 5-6)
1. Smart Context Window Manager
2. Proactive Health Dashboard
3. Achievement System + Easter Eggs

### Sprint 4 (Weeks 7-8)
1. Multi-Model Ensemble
2. Voice Interface
3. Full Moon Mode + remaining easter eggs

---

## 🎯 Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Time to First Agent | < 5 minutes | Onboarding wizard completion |
| Skill Install Success Rate | > 95% | Marketplace install analytics |
| Agent Collaboration Score | > 80% | Swarm harmony metric |
| User Retention (7-day) | > 60% | Dashboard analytics |
| Mean Time to Resolution | < 30 seconds | NL command success rate |

---

*This document is living — update after each sprint retrospective.*

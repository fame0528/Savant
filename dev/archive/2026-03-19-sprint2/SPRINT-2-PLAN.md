# Sprint 2 Plan — Dashboard Integration & Ensemble

> **Date:** 2026-03-19  
> **Status:** AWAITING EXECUTION  
> **Features:** 4 remaining from gap analysis  
> **Methodology:** Perfection Loop per feature

---

## What Sprint 1 Completed (8 features)

All 8 backend/frontend features done. Tests passing. CI fixed.

## What's Still Missing

Sprint 1 built the **backend** for several features but didn't wire the **frontend**:

| Feature | Backend | Frontend | Gap |
|---------|---------|----------|-----|
| NL Commands | ✅ NLU parser + executor | ❌ Command input UI missing | Wire to chat |
| Conversation Replay | ✅ ReplayRecorder | ❌ Timeline visualization | Dashboard component |
| Proactive Health | ✅ PerceptionEngine | ❌ No health page | Dashboard page |
| Multi-Model Ensemble | ❌ Not started | N/A | Backend module |

---

## Feature 1: Dashboard Command Input

**Priority:** P0 — Wires existing backend to existing UI

### Implementation
1. Add command input field to `dashboard/src/app/page.tsx` chat area
2. When user types `/` prefix, treat as NL command
3. Parse intent → send to gateway via WebSocket
4. Gateway dispatches to NLU parser → returns response
5. Show response in chat lane

### Files
- `dashboard/src/app/page.tsx` — MODIFY (add command input)
- Gateway handler — ADD (NLCommand control frame)

---

## Feature 2: Conversation Replay Timeline

**Priority:** P1

### Implementation
1. Create `dashboard/src/components/Timeline.tsx` — vertical timeline
2. Event types: thought (blue), tool_call (cyan), observation (green), decision (yellow), error (red)
3. Click step → expand detail panel
4. Wire to ReplayRecorder via WebSocket

### Files
- `dashboard/src/components/Timeline.tsx` — CREATE
- `dashboard/src/components/timeline.module.css` — CREATE

---

## Feature 3: Proactive Health Dashboard

**Priority:** P1

### Implementation
1. Create `dashboard/src/app/health/page.tsx` — system health page
2. Show: agent status, memory usage, connection health, circuit breaker states
3. Wire to PerceptionEngine data via WebSocket

### Files
- `dashboard/src/app/health/page.tsx` — CREATE
- `dashboard/src/app/health/health.module.css` — CREATE

---

## Feature 4: Multi-Model Ensemble

**Priority:** P2

### Implementation
1. Create `crates/agent/src/ensemble/mod.rs`
2. Parallel dispatch to N providers
3. Best-of-N selection: pick response with highest quality score
4. Configurable: which providers, how many, selection strategy

### Files
- `crates/agent/src/ensemble/mod.rs` — CREATE
- `crates/agent/src/lib.rs` — MODIFY (add ensemble module)

---

## Execution Order

```
1. Dashboard Command Input (P0 — wires existing backend)
2. Conversation Replay Timeline (P1 — visual debugging)
3. Proactive Health Dashboard (P1 — system visibility)
4. Multi-Model Ensemble (P2 — parallel provider dispatch)
```

---

*Created: 2026-03-19. Ready for execution.*

# FID-20260528-DASHBOARD-UI-FIXES

| Field            | Value                              |
|------------------|------------------------------------|
| **Document ID**  | FID-20260528-DASHBOARD-UI-FIXES    |
| **Date Created** | 2026-05-28                         |
| **Status**       | PARTIAL — 2/6 fixed, 4 pending rebuild + runtime test |
| **Priority**     | CRITICAL                           |
| **Phase**        | 5 — Documentation                  |

## Context

After v0.3.5 build + launch, the Savant desktop app shows the agent is running (consciousness daemon streaming, heartbeat active, gateway on port 8080), but the dashboard has multiple UI regressions that make it unusable.

## Issue: Dashboard UI Regressions — 5 Problems

### Symptoms
1. **Sidebar completely empty** — no agents listed despite `agents.discovered` firing and gateway reporting 1 agent
2. **Input box too narrow** — doesn't fill the full row width
3. **Messages doubling** — user message appears twice in the chat when sent
4. **No user feedback** — no typing indicator, pending state, or agent status while waiting for response
5. **Reflections tab empty** — consciousness daemon is generating narratives (confirmed in logs) but dashboard shows nothing
6. **Copy All fails in Agent Logs** — Tauri clipboard plugin not accessible from packaged app's logs.html window

### Root Cause Analysis
- Backend is healthy: consciousness daemon streaming, heartbeat active, orchestrator processing messages
- All issues are frontend/dashboard UI layer
- Sidebar: component state or CSS issue preventing agent list rendering
- Input: CSS layout bug — input element doesn't expand to fill container
- Messages: likely both optimistic add AND server echo adding the same message
- Reflections: consciousness narrative published to Nexus but dashboard not subscribing/displaying
- Copy All: previous fix may not reach packaged Tauri window

### Fix Plan (with impact matrix)

| # | File | Change | Blast Radius | Risk |
|---|------|--------|-------------|------|
| 1 | `dashboard/src/components/DashboardShell.tsx` | Fix sidebar agent list rendering | Sidebar only | LOW |
| 2 | `dashboard/src/components/DashboardShell.tsx` | Fix input box CSS width | Input area only | LOW |
| 3 | `dashboard/src/context/DashboardContext.tsx` | Fix message doubling — dedup or guard against double-add | Chat messages | MED |
| 4 | `dashboard/src/app/page.tsx` or context | Add typing/pending indicator | Chat UI | LOW |
| 5 | `dashboard/src/components/DashboardShell.tsx` or new component | Wire consciousness narrative to Reflections tab | Reflections panel | LOW |
| 6 | `dashboard/public/logs.html` | Fix Copy All for packaged Tauri window | Logs window only | LOW |

### Verification Checklist
- [ ] Sidebar shows .savant agent with display name "SAVANT" — pending rebuild + runtime test
- [x] Input box fills full row width — CSS fix pushed (01f79e1)
- [x] Messages don't double when sent — dedup guard pushed (01f79e1)
- [ ] Typing/pending indicator shows while agent processes — pending
- [ ] Reflections tab shows consciousness narrative — pending rebuild with filter fix
- [ ] Copy All works in Agent Logs window — pending Tauri plugin verification
- [x] `npx tsc --noEmit` — 0 errors

## Notes
- All 6 issues are in dashboard frontend layer
- Backend confirmed healthy from agent logs
- Consciousness daemon generating narratives (confirmed: "Narrative updated (1232 chars)")

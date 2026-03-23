# FID-20260321-SUPREME-AUDIT-SUBTASK-NANOCLAW — FULL SCAN

**Competitor:** NanoClaw (TypeScript/Node.js >=20)
**Repo:** `research/competitors/nanoclaw`
**Source Files Scanned:** All .ts files under src/ + container/ + setup/
**Production LOC:** ~5,446
**Features Found:** 15+ features Savant doesn't have

---

## Architecture

Single-process host + per-group Docker containers. File-based IPC (no network between host and container). Channels self-register via factory pattern. SQLite message bus.

---

## 1. Container Isolation Model

Each group gets an isolated Docker container with:
- Own filesystem mounts (group folder, .claude/, IPC)
- Own IPC namespace (per-group directories)
- Own session/transcript directory
- Own agent-runner source copy
- Global concurrency limit (default 5)
- Per-group queue with exponential backoff retry (5s base, max 5)

**Main group privileges:** No trigger required, sees all groups/tasks, can register groups, can send to any group, elevated mount permissions.

**Savant has:** Docker executor in skills crate. No per-group isolation, no concurrency queue.
**Gap:** MAJOR — Need container-per-group architecture for multi-tenant agents.

---

## 2. Credential Proxy (`src/credential-proxy.ts:125`)

HTTP reverse proxy between containers and Anthropic API. Two modes:
- API key: `ANTHROPIC_API_KEY=placeholder`, proxy injects real key
- OAuth: `CLAUDE_CODE_OAUTH_TOKEN=placeholder`, proxy injects real OAuth token

**Key properties:** Real credentials never in container env/filesystem/proc. Hop-by-hop headers stripped. Platform-specific bind (localhost on macOS/WSL, bridge IP on Linux).

**Savant has:** No credential proxy. Just implemented URL validation.
**Gap:** CLOSED (in v2 tool system plan).

---

## 3. File-Based IPC System (`src/ipc.ts:461`)

**7 IPC message types:** send_message, schedule_task, pause_task, resume_task, cancel_task, update_task, register_group.

**Architecture:** Host polls `data/ipc/{group}/messages/*.json` and `data/ipc/{group}/tasks/*.json` every 1s. Atomic writes (temp + rename). Per-group identity from directory path. Authorization: `isMain || (targetGroup.folder === sourceGroup)`.

**Savant has:** No IPC system. Agent communicates via WebSocket only.
**Gap:** LOW — Savant's architecture is different (single-process, not container-per-group).

---

## 4. Group Queue (`src/group-queue.ts:365`)

Per-group concurrency with global limit. Features:
- FIFO drain with task priority
- Idle preemption (task can preempt idle container)
- Follow-up message sending to active container via IPC
- Close sentinel (`_close`) for container wind-down
- Exponential backoff retry (5s base, max 5)
- Graceful shutdown (detach, don't kill)

**Savant has:** No group queue. Agents run independently.
**Gap:** LOW — Architecture-dependent.

---

## 5. Mount Security (`src/mount-security.ts:419`)

External allowlist at `~/.config/nanoclaw/mount-allowlist.json`. 16 blocked patterns (.ssh, .gnupg, .aws, .env, id_rsa, etc.). Symlink resolution before validation. Non-main groups forced read-only.

**Savant has:** No mount security for Docker containers.
**Gap:** MODERATE — Need mount validation for container tool execution.

---

## 6. Sender Allowlist (`src/sender-allowlist.ts:128`)

Two modes per chat:
- `trigger`: disallowed senders don't trigger agent
- `drop`: disallowed senders' messages never stored

**Savant has:** No sender filtering.
**Gap:** LOW — Channel-specific feature.

---

## 7. Task Scheduler (`src/task-scheduler.ts:282`)

Three schedule types: cron, interval, once. Interval tasks anchored to scheduled time (prevents drift). Agent-mediated execution through full container. Tasks use short-lived containers (10s close delay vs 30min idle timeout).

**Savant has:** Heartbeat with cron scheduling. No task management CRUD.
**Gap:** MODERATE — Need task CRUD + run history.

---

## 8. Remote Control (`src/remote-control.ts:224`)

Spawns `claude remote-control` as detached process. Provides web URL for remote Claude Code access. State persisted to disk, restored on startup.

**Savant has:** No remote control.
**Gap:** LOW — Nice-to-have.

---

## 9. Conversation Archiving

PreCompact hook archives full transcript to `conversations/` as Markdown before SDK compaction. Preserves history that would otherwise be lost.

**Savant has:** No conversation archiving on compaction.
**Gap:** MODERATE — Need archiving when compaction is implemented.

---

## 10. Setup System (`setup/`)

- `register.ts:177` — Group registration with validation
- `service.ts:362` — OS service management (launchd, systemd, nohup fallback)
- `platform.ts:132` — Platform detection (macOS, Linux, WSL)
- `environment.ts:94` — Docker/container runtime detection
- `container.ts:144` — Container image building
- `mounts.ts:115` — Mount allowlist setup
- `groups.ts:229` — WhatsApp group sync
- `verify.ts:192` — End-to-end health check

**Savant has:** No setup wizard. Manual configuration.
**Gap:** MODERATE — Need onboarding wizard.

---

## 11. X Integration Skill

Browser automation via Playwright for X (Twitter) interactions: post, like, reply, retweet, quote. Container-side MCP tools + host-side IPC handler.

**Savant has:** No browser automation.
**Gap:** LOW — Platform-specific feature.

---

## 12. SQLite Schema (7 tables)

chats, messages, scheduled_tasks, task_run_logs, router_state, sessions, registered_groups. JSON state migration from files.

**Savant has:** CortexaDB for memory. No structured chat/task management DB.
**Gap:** MODERATE — Need structured DB for chat/task management.

---

## ALL FEATURES CATALOGUED

| # | Feature | LOC | Priority | Savant Status |
|---|---------|-----|----------|---------------|
| 1 | Container-per-group isolation | ~700 | P2 | Missing |
| 2 | Credential proxy | ~125 | P1 | Closed |
| 3 | File-based IPC (7 message types) | ~461 | P3 | Missing |
| 4 | Group queue with concurrency | ~365 | P3 | Missing |
| 5 | Mount security (allowlist + blocked patterns) | ~419 | P2 | Missing |
| 6 | Sender allowlist (trigger/drop modes) | ~128 | P3 | Missing |
| 7 | Task scheduler (cron/interval/once) | ~282 | P2 | Missing |
| 8 | Remote control | ~224 | P3 | Missing |
| 9 | Conversation archiving on compaction | ~100 | P2 | Missing |
| 10 | Setup wizard (service/platform/container) | ~1,200 | P3 | Missing |
| 11 | SQLite structured DB (7 tables) | ~697 | P2 | Missing |
| 12 | Streaming output protocol | ~100 | P2 | Missing |
| 13 | Main group privilege model | ~200 | P3 | Missing |
| 14 | Channel plugin self-registration | ~28 | P2 | Missing |
| 15 | .env isolation (never in process.env) | ~42 | P1 | Partial |

---

*Exhaustive scan of all .ts files. Every function and architectural decision catalogued.*
*Generated 2026-03-21*

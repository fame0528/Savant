# NanoClaw Comprehensive Analysis Report

**Date:** 2026-03-24  **Project:** NanoClaw v1.2.21  **Language:** TypeScript (Node.js 20+)  **License:** MIT

---

## 1. Project Overview

NanoClaw is a personal AI assistant orchestrator that runs Claude Agent SDK instances inside isolated Docker containers with multi-channel messaging (WhatsApp, Telegram, Discord, Slack, Gmail). Lightweight alternative to OpenClaw emphasizing OS-level container isolation.

### Tech Stack

| Layer | Technology |
|-------|-----------|
| Runtime | Node.js 20+ (ES2022, ESM) |
| Language | TypeScript 5.7+ strict |
| Database | SQLite via better-sqlite3 |
| Agent SDK | claude-agent-sdk 0.2.76+ |
| Container | Docker / Apple Container |
| MCP | @modelcontextprotocol/sdk |
| Logging | pino + pino-pretty |
| Validation | zod v4 |
| Scheduling | cron-parser |
| Testing | Vitest 4.0+ |
| Linting | ESLint 9 + typescript-eslint |
| Formatting | Prettier 3.8+ |
| Hooks | Husky 9 |

### Key Statistics
- ~32 source files (12 test files)
- Host: ~5,500 lines TypeScript
- Container: ~900 lines TypeScript
- 5 prod + 10 dev deps (host); 4 prod deps (container)

---

## 2. Architecture

### 2.1 Single-Process Architecture
Three polling loops: Message (2s), Scheduler (60s), IPC (1s). Converge on GroupQueue (5 max containers). One process, no microservices.

### 2.2 Channel Registry
Self-registration factory. registerChannel() at import. Barrel file triggers. Factory returns null if no credentials. Channel interface: connect/send/isConnected/ownsJid/disconnect. Added via git branches.

### 2.3 File-Based IPC
data/ipc/{group}/messages/ tasks/ input/ plus _close sentinel and current_tasks.json. Atomic writes via temp+rename.

### 2.4 Per-Group Queue
Per-group state. Global 5-container limit. Tasks prioritized. Exponential backoff. 30min idle timeout.

### 2.5 Credential Proxy
HTTP proxy port 3001. Containers get placeholder. Proxy injects real auth. API key + OAuth. Real credentials never in containers.

### 2.6 SQLite
chats, messages (bot filtering), scheduled_tasks, task_run_logs, router_state, sessions, registered_groups. Auto-migration from JSON.

### 2.7 Hierarchical Memory
groups/CLAUDE.md (global), groups/main/CLAUDE.md (admin), groups/{name}/CLAUDE.md (per-group). SDK settingSources: project.

---

## 3. Top Features

### 3.1 Multi-Channel Messaging
WhatsApp/Telegram/Discord/Slack/Gmail. Git branch per channel. JID routing. Per-channel formatting. Decoupled from core.

### 3.2 Container-Isolated Execution
Docker containers. Main: project root (ro) + group (rw). Non-main: group (rw) + global (ro). .env shadowed. Per-group .claude/. Non-root. Sentinel markers. Streaming. Timeout. Mount allowlist at ~/.config/nanoclaw/. Blocked: .ssh, .gnupg, .aws, .env. Symlink resolution.

### 3.3 Scheduled Tasks
Cron/interval/once as full Claude agents. 60s scheduler. GroupQueue. Drift prevention. Group/isolated context. Full CRUD.

### 3.4 Per-Group Isolation
Own folder, memory, IPC, sessions, agent-runner. {channel}_{group} naming. Validation. Main elevated privileges.

### 3.5 Credential Isolation
Real keys never in containers. HTTP proxy. .env shadowed.

### 3.6 Streaming + Idle
Sentinel parsing. Real-time delivery. internal tags stripped. 30min idle. _close sentinel.

### 3.7 Skills
Feature (branch), Utility (code), Operational (instructions), Container (runtime). Git merge. Claude resolves conflicts.

### 3.8 Sender Allowlist
~/.config/nanoclaw/sender-allowlist.json. Trigger/drop modes. is_from_me bypass. Fail-open.

### 3.9 Remote Control
Claude Code remote-control. Detached process. URL polling. State persistence.

### 3.10 Conversation Archiving
PreCompact hook. Transcripts to markdown.

---

## 4. Implementation

### Message Flow
1. Channel->onMessage 2. Allowlist 3. SQLite 4. Poll 5. Dedup 6. Trigger 7. Catch-up 8. Format XML 9. Queue 10. Container 11. Agent+MCP 12. Stream 13. Send 14. Session 15. Idle

### Agent Runner
stdin JSON, drain IPC, query(cwd/resume/systemPrompt/mcpServers), stream, wait IPC or _close, repeat

### MCP Tools
send_message, schedule_task, list_tasks, pause/resume/cancel/update_task, register_group

---

## 5. Strengths
1. Minimal ~6,400 lines. 2. True OS isolation. 3. Credential proxy ~120 lines. 4. File-based IPC atomic writes. 5. Skills as git branches. 6. AI-native no UI. 7. Session isolation. 8. Sentinel parsing. 9. Mount security. 10. Conversation archiving

---

## 6. Weaknesses
1. Polling inefficient. 2. No WebSocket. 3. SQLite queue limitations. 4. 2-5s cold starts + TS rebuild. 5. No horizontal scaling. 6. Error recovery edge cases. 7. No rate limiting. 8. Test gaps. 9. Hardcoded image. 10. Windows limitations. 11. No dedup restarts. 12. Runner too large 717 lines. 13. TS rebuild 5-10s overhead

---

## 7. Ideas for Savant

### HIGH VALUE
7.1 Credential Proxy (~120 lines) - HTTP proxy injects real keys. Compromised containers cannot see them.

7.2 File-Based IPC - Filesystem IPC. Atomic writes. Per-group. _close sentinel. Debuggable.

7.4 Hierarchical CLAUDE.md - groups/CLAUDE.md global + per-group. SDK settingSources: project. Zero-code memory.

7.9 Mount Allowlist Outside Root - ~/.config/{app}/. No tampering. Defense in depth.

### MEDIUM VALUE
7.3 Skills as Branches - Git merge composition. Claude resolves conflicts. Needs CI.

7.5 Container Skills - Separate agent/host skills. Synced at start.

7.10 Conversation Catch-Up - All messages since last interaction.

### LOW-MEDIUM
7.6 Sentinel Parsing. 7.8 Pre-Compaction Archiving ~80 lines.

### LOW
7.7 Sender Allowlist Modes - trigger/drop.

---

## 8. Key Differences vs Savant

| Dimension | NanoClaw | Savant |
|-----------|----------|--------|
| Use case | Personal assistant | Coding assistant |
| Agent exec | Docker containers | Direct process |
| Channels | Multi-channel | IDE integration |
| Size | ~6,400 lines | Larger |
| Config | Change code | Config files |
| Extensions | Git branches | Plugins |
| Memory | CLAUDE.md | Custom system |
| Security | Container (OS) | Process permissions |
| Scheduling | Built-in | Not primary |
| Deploy | Service | Desktop app |
| Tenancy | Per-group | Single user |
| Philosophy | Anti-framework | Feature-rich |
| Credentials | HTTP proxy | Env vars |
| IPC | File-based | Network/stdio |

### Key Differences
1. Isolation: Docker (OS) vs processes. NanoClaw more secure, higher overhead.
2. Extensions: Git branches vs plugins.
3. Deploy: Background service vs desktop IDE.
4. Scope: Narrow (assistant) vs broad (coding).
5. Philosophy: Anti-framework vs feature-rich.

---

*Report from complete source code analysis of all files, docs, container config, and tests.*
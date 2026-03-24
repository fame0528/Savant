# HiClaw - Comprehensive Analysis Report

**Date**: 2026-03-24
**Source**: hiclaw/ (competitor research)
**License**: Apache 2.0
**Latest Version**: 1.0.8

---

## 1. Project Overview

HiClaw is an open-source Collaborative Multi-Agent OS that enables transparent, human-in-the-loop task coordination via Matrix (the decentralized messaging protocol). Built by the Higress Group (Alibaba Cloud), it represents an evolution of the OpenClaw agent framework.

**Core idea**: A Manager Agent (AI chief of staff) coordinates a team of Worker Agents (task executors), all communicating through Matrix rooms where the human admin can observe and intervene at any time.

### Tech Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| AI Gateway | Higress (all-in-one) | LLM proxy, MCP Server hosting, consumer auth |
| Matrix Server | Tuwunel (conduwuit fork) | IM communication between Agents and Human |
| Matrix Client | Element Web | Browser-based IM interface |
| File System | MinIO + mc mirror | Centralized HTTP file storage with local sync |
| Agent Runtime | OpenClaw (Node.js 22 fork) | Agent runtime with Matrix plugin, skills |
| Alt Runtime | CoPaw (Python 3.11) | Lightweight runtime (~150MB vs ~500MB) |
| MCP CLI | mcporter | Worker calls MCP Server tools via CLI |
| Skills | skills.sh (80K+ skills) | On-demand skill installation for Workers |
| Container Mgmt | Docker API proxy (Go) | Secured container creation for Workers |
| Process Mgmt | supervisord | Orchestrates all services in Manager container |
| Build | Make + Docker multi-stage | Multi-arch builds (amd64 + arm64) |
| CI/CD | GitHub Actions | Build, test, release workflows |

### Languages Used
- **Shell (Bash)**: All orchestration, startup, install scripts, skills
- **Python (3.11)**: CoPaw worker runtime, bridge/config translation, Matrix channel (matrix-nio)
- **Go**: Docker API security proxy
- **Node.js (22)**: OpenClaw agent runtime (the base framework)
- **Markdown**: Agent behavior definition (SOUL.md, AGENTS.md, SKILL.md, HEARTBEAT.md)

---

## 2. Architecture and Design Patterns

### 2.1 Manager-Workers Architecture

The Manager is the all-in-one container running Higress (AI gateway), Tuwunel (Matrix server), MinIO (object storage), Element Web (browser client), and the Manager Agent (OpenClaw). Workers are lightweight, stateless containers that connect to the Manager via Matrix for task communication.

All persistent data lives in MinIO. Workers are designed to be destroyed and recreated freely.

### 2.2 All-in-One Manager Container

The Manager container runs 8 services via supervisord:

| Priority | Service | Purpose |
|----------|---------|---------|
| 50 | MinIO | Object storage (port 9000) |
| 100 | Tuwunel | Matrix homeserver (port 6167) |
| 200-500 | Higress (4 components) | AI Gateway (port 8080), Console (port 8001) |
| 650 | Element Web (Nginx) | Browser IM client (port 8088) |
| 700 | mc mirror | MinIO <-> local filesystem sync |
| 800 | Manager Agent | OpenClaw agent runtime |

Dockerfile layer ordering is optimized: stable binaries -> stable config -> shared lib -> scripts -> agent (highest frequency changes invalidate smallest layers).

### 2.3 Communication Model (Matrix Rooms)

All communication happens in Matrix Rooms with Human-in-the-Loop:
- Each Worker has a dedicated Room: Human + Manager + Worker
- Multi-worker projects get Project Rooms: Human + Manager + all participating Workers
- @mentions required in group rooms (anti-spam/noise prevention)
- NO_REPLY is a standalone signal (not a tag to append)
- History context accumulation for non-mentioned messages
- Cross-channel escalation (Manager can DM admin on primary channel)

### 2.4 Security Model

Workers hold only a consumer token (BEARER key). The Higress AI Gateway holds real API keys and GitHub PATs. Workers never see real credentials. Manager controls per-Worker route and MCP Server permissions via Higress Console API.

The Docker API proxy (Go) prevents container escape: allowlisted operations only, no bind mounts, no privileged mode, no host network/PID, dangerous capabilities blocked.

### 2.5 Centralized File System (MinIO)

MinIO bucket hiclaw-storage/ contains:
- agents/<worker-name>/ - Worker config (SOUL.md, openclaw.json, skills/, mcporter-servers.json)
- shared/tasks/ - Task specs, metadata, results
- shared/knowledge/ - Shared reference materials
- workers/ - Worker work products

Workers are stateless - destroy and recreate freely. All config/state in MinIO.

### 2.6 Agent Behavior as Markdown Files

Agent behavior is defined by documentation files, not code:
- **SOUL.md**: Agent identity, personality, security rules
- **AGENTS.md**: Workspace guide, session protocol, memory management, gotchas
- **HEARTBEAT.md**: Periodic check routine (task monitoring, capacity assessment, admin reporting)
- **SKILL.md** files: Self-contained tool/API references (auto-discovered by OpenClaw)
- **TOOLS.md**: Quick-reference cheat sheet for skills

### 2.7 File Sync Architecture

- Local -> Remote (push): Writer pushes immediately via mc cp/mirror (change-triggered, 5s interval)
- Remote -> Local (pull): Reader pulls on-demand via Matrix @mention notification
- Fallback: 5-minute periodic pull as safety net
- Manager workspace (~/hiclaw-manager/) is local-only, never synced to MinIO
- Worker workspace (~/hiclaw-fs/agents/<name>/) is synced bidirectionally

### 2.8 Cloud Deployment Mode

HiClaw supports Alibaba Cloud SAE deployment:
- External Tuwunel, MinIO (OSS), and Higress services
- STS credential management via OIDC tokens
- Workspace sync between local container and OSS
- Welcome message onboarding for cloud users

---

## 3. Top Features (Detailed)

### 3.1 Conversational Worker Creation
Users create Workers by chatting with the Manager via Matrix DM. Manager handles: Matrix account registration, Higress consumer creation, config generation in MinIO, Room creation, container creation, skill assignment.

### 3.2 Human-in-the-Loop Oversight
Every Matrix Room includes human admin. Watch interactions in real-time, intervene mid-task, redirect/cancel tasks, switch notification channels.

### 3.3 Multi-Worker Collaboration
Manager coordinates multiple Workers: Project Rooms, task dependencies, heartbeat monitoring, task handoffs, status reporting.

### 3.4 MCP Server Integration
Workers access external tools via MCP Servers hosted by Higress. MCP Server holds real credentials. Workers call via mcporter CLI. Dynamic per-Worker permissions.

### 3.5 Worker Lifecycle Management
Auto-create via Docker/Podman socket, auto-stop idle (default 12h), auto-start on task, auto-recreate on restart.

### 3.6 Skills Ecosystem
Workers pull from skills.sh (80K+ skills) on demand. Safe - no real credential access. Manager can push. Private registry support.

### 3.7 Dual Worker Runtimes
OpenClaw (Node.js 22, ~500MB) for full features. CoPaw (Python 3.11, ~150MB) for lightweight. CoPaw bridges config format.

### 3.8 Worker Import System
Migrate standalone OpenClaw or import templates. Migration skill analyzes setup, generates package.

### 3.9 Docker API Security Proxy
Go proxy: allowlisted ops, image validation, no bind mounts/privileged/host network, dangerous caps blocked, name prefix enforcement.

### 3.10 One-Command Installation
curl | bash. Cross-platform, smart mirror selection, interactive/non-interactive modes, upgrade support.

### 3.11 Agent Memory System
Daily notes (memory/YYYY-MM-DD.md), long-term (MEMORY.md, DM-only), task history (LRU top 10), progress logs per task.

### 3.12 Multi-Channel Communication
Discord, Feishu, Telegram, Slack, WhatsApp. Primary channel, cross-channel escalation, trusted contacts.

---

## 4. How Features Work

### 4.1 Worker Creation Flow
1. Admin sends message to Manager via Matrix DM
2. Manager parses request (natural language to intent)
3. Manager calls create-worker.sh: register Matrix account, create Higress consumer, generate config in MinIO, create Room, write registry
4. Manager calls container_create_worker via Docker API or outputs docker run command
5. Worker pulls config from MinIO, starts OpenClaw, joins Room

### 4.2 Task Assignment Flow
1. Admin mentions Worker in Room
2. Manager creates task dir in MinIO (shared/tasks/{task-id}/)
3. Manager writes meta.json and spec.md
4. Manager @mentions Worker
5. Worker pulls files, executes, writes progress logs and result.md
6. Manager reads result, updates meta.json, notifies admin

### 4.3 Heartbeat Monitoring
Triggered by OpenClaw heartbeat: checks active tasks, sends follow-ups, monitors projects, manages lifecycle, reports to admin.

### 4.4 File Sync
Push: find detects modified files (last 10s), mc mirror with exclusions, 5s interval. Pull: on-demand via @mention, fallback 5-min periodic.

### 4.5 Config Bridge (OpenClaw to CoPaw)
bridge.py translates openclaw.json to CoPaw config.json + providers.json. Maps Matrix config, LLM providers, context window.

---

## 5. Strengths

5.1 **Human-in-the-Loop by Default** - Everything visible and interruptible. No black-box calls.
5.2 **Security-First Design** - Credential isolation via gateway. Docker API proxy.
5.3 **Markdown as Agent Config** - SOUL.md, AGENTS.md, SKILL.md. Readable, version-controllable, self-documenting.
5.4 **Stateless Workers** - Destroy and recreate freely. All state in MinIO.
5.5 **One-Command Setup** - Polished installation experience.
5.6 **Dual Runtime Support** - OpenClaw (500MB) and CoPaw (150MB).
5.7 **Multi-Channel Support** - Discord, Telegram, Feishu alongside Matrix.
5.8 **Comprehensive Documentation** - Architecture, quickstart, guides, FAQ.
5.9 **Well-Structured Codebase** - Clear separation of concerns.
5.10 **Cloud Deployment Support** - Alibaba Cloud SAE with STS credentials.

---

## 6. Weaknesses

6.1 **Shell Script Complexity** - ~2000+ lines of shell. Hard to test, prone to bugs, difficult to extend.
6.2 **Tight Coupling to Higress** - Deeply coupled for LLM proxy, consumer auth, MCP hosting, route management.
6.3 **MinIO Dependency** - All file sync relies on MinIO + mc CLI. No alternative backends.
6.4 **OpenClaw Fork Dependency** - Specific fork (johnlanni/openclaw, hiclaw-v1). Maintenance burden.
6.5 **Manager Single Point of Failure** - All services in one container. No HA or failover.
6.6 **Limited Scalability** - Single-machine design. Scaling requires manual setup.
6.7 **Session Reset Issues** - Daily resets at 04:00 lose context.
6.8 **No Native Tool Execution** - mcporter CLI adds indirection layer.
6.9 **Heavy Container Footprint** - Even CoPaw needs Python 3.11, Node.js 22, mc, libolm.
6.10 **No Native Code Sandbox** - Relies on MCP Servers for code execution.

---

## 7. Ideas for Savant

7.1 **Markdown-Driven Agent Config** - Natural language config is accessible, version-controllable, self-documenting. Savant could define agent personas in markdown.
7.2 **Human-in-the-Loop Rooms** - Transparency builds trust. Enables intervention.
7.3 **Centralized Credential Management** - Gateway holds real keys; agents get tokens only.
7.4 **Stateless Worker Design** - Centralized storage, destroyable containers. Resilient and scalable.
7.5 **Dual Runtime Support** - Full-featured and lightweight modes for different constraints.
7.6 **Skill Ecosystem Integration** - On-demand skill discovery and installation.
7.7 **Agent Memory System** - Daily + long-term + task history + progress logs. Files provide continuity.
7.8 **Docker API Security Proxy** - Validate container creation requests.
7.9 **One-Command Installation** - Minimize setup friction.
7.10 **Worker Lifecycle Management** - Auto-stop idle, auto-start on task.

---

## 8. Key Differences vs Savant

| Dimension | HiClaw | Savant |
|-----------|--------|--------|
| Architecture | Manager-Workers (supervisor) | TBD |
| Communication | Matrix protocol (IM) | Likely direct API/stream |
| Runtime | OpenClaw (Node.js) / CoPaw (Python) | Native Rust or custom |
| Credentials | Higress gateway | Needs implementation |
| File System | MinIO | Local or custom |
| Config | Markdown files | Likely code-based |
| Installation | One-command Docker | TBD |
| Skills | skills.sh (80K+) | TBD |
| Code Execution | No native sandbox | Could have native |
| Multi-Agent | Built-in | TBD |
| Visibility | Matrix rooms | TBD |
| Cloud | Alibaba SAE | TBD |
| Language | Shell + Python + Node.js + Go | Rust (likely) |
| Market | Chinese | Western (likely) |
| License | Apache 2.0 | TBD |

### Key Takeaways for Savant

1. **Human-in-the-loop is non-negotiable** - transparent, interruptible collaboration builds user trust
2. **Credential isolation is critical** - never let agents hold real API keys
3. **Markdown-driven agent config is powerful** - consider for agent personas
4. **Stateless workers are resilient** - centralize state, make containers disposable
5. **One-command setup reduces friction** - polish the installation experience
6. **Skills ecosystem enables extensibility** - agents should discover tools dynamically
7. **Agent memory should be file-based** - design for session continuity
8. **Security proxy for container ops** - enforce policies if creating containers
9. **Dual runtimes offer flexibility** - lightweight and full-featured modes
10. **Lifecycle management saves resources** - auto-stop idle, auto-start on demand

---

*Report generated from full codebase audit of HiClaw v1.0.8. All source files read.*

<div align="center">
  <img src="img/savant.png" alt="Savant Logo" width="180" />
  <h1>SAVANT</h1>
  <p><strong>One Mind. A Thousand Faces.</strong></p>
  <p>A production-grade, Rust-native framework for building, deploying, and coordinating swarms of autonomous AI agents with OpenClaw skill compatibility, mandatory security scanning, and real-time dashboard observability.</p>

  [![Rust](https://img.shields.io/badge/Rust-2021-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
  [![Next.js](https://img.shields.io/badge/Next.js-14-000000?style=for-the-badge&logo=nextdotjs&logoColor=white)](https://nextjs.org/)
  [![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
</div>

---

## Overview

Savant is an autonomous agent swarm orchestrator with **mandatory security scanning** for all skills:

- **Swarm Orchestration** — Spawn, coordinate, and manage hundreds of concurrent AI agents from a unified control plane
- **OpenClaw Skill Compatibility** — Install skills from ClawHub with automatic OpenClaw `SKILL.md` format parsing
- **Mandatory Security Scanning** — Every skill is scanned before execution; user sovereignty with click-based approval (0-3 clicks based on risk)
- **Real-Time Dashboard** — A Next.js observability dashboard with live WebSocket streaming, message history, cognitive insights, and soul manifestation
- **Multi-Channel Gateway** — Axum-based WebSocket gateway with authentication, message routing, and event-driven architecture
- **Persistent Memory** — Hybrid storage combining SQLite (WAL), Fjall LSM-tree, and rkyv-serialized vector embeddings
- **Cognitive Architecture** — Goal decomposition, strategic synthesis, memory consolidation, and proactive heartbeat loops
- **Threat Intelligence** — Global blocklist sync with configurable threat intelligence feed

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Dashboard (Next.js)                      │
│   WebSocket Client │ React Markdown │ Security UI            │
└───────────────────────────┬─────────────────────────────────┘
                            │ ws://
┌───────────────────────────▼─────────────────────────────────┐
│                   Gateway (Axum)                             │
│   WebSocket Handler │ Auth Middleware │ Skill Control        │
└───────────────────────────┬─────────────────────────────────┘
                            │ NexusBridge
┌───────────────────────────▼─────────────────────────────────┐
│               Message Bus (Nexus Event Bus)                  │
└───────┬───────────┬───────────┬───────────┬────────────────┘
        │           │           │           │
┌───────▼──┐ ┌──────▼────┐ ┌───▼──────┐ ┌──▼───────────────┐
│  Agent   │ │ Cognitive │ │  Memory  │ │    Skills        │
│  Swarm   │ │  Engine   │ │  Engine  │ │  Security Gate   │
│  (ECHO)  │ │ (Synth)   │ │  (VHSS)  │ │  ClawHub/Docker  │
└──────────┘ └───────────┘ └──────────┘ └──────────────────┘
```

---

## Security Model

Savant implements a **mandatory security gate** for all skills. Every skill must pass through the security scanner before execution. The user is always sovereign — no hard blocks, but increasing click friction based on risk:

| Risk Level | Clicks Required | Behavior |
|:-----------|:---------------:|:---------|
| **Clean** | 0 | Auto-proceed, no prompts |
| **Low** | 0 | Proceed with notification |
| **Medium** | 1 | Acknowledge findings |
| **High** | 2 | Double-confirm with full disclosure |
| **Critical** | 3 | Triple-confirm with "I understand the risks" |

### Security Scanner Capabilities

- **Global Blocklist** — Hash-based and name-based blocking, synced with threat intelligence feed
- **Malicious URL Detection** — Shortened URLs, pastebin, executables, direct IP access
- **Credential Theft Detection** — SSH keys, AWS credentials, GPG, keychain, environment variables
- **Fake Prerequisite Detection** — Snyk attack pattern (fake required packages)
- **Data Exfiltration Detection** — Webhooks, base64 encoding of sensitive files
- **Dangerous Command Detection** — sudo, chmod 777, crontab, pipe-to-bash
- **10 Proactive Checks:**
  1. Clipboard hijacking
  2. Persistence injection
  3. Lateral movement
  4. Cryptojacking
  5. Reverse shell
  6. Keylogger
  7. Screen capture
  8. Time-bomb
  9. Typosquatting (Levenshtein distance)
  10. Dependency confusion (async registry verification)

---

## Quick Start

### Prerequisites

- **Rust** 1.75+ (stable)
- **Node.js** 18+ (for the dashboard)
- **Docker** (optional, for containerized skill execution)
- **OpenRouter API Key** (for AI-powered agent generation)

### 1. Start the Backend

```bash
cargo run --bin savant_cli
```

The gateway starts on `ws://127.0.0.1:8080/ws` by default.

### 2. Start the Dashboard

```bash
cd dashboard
npm install
npm run dev
```

The dashboard is available at `http://localhost:3000`.

### 3. Configure Environment

Create a `.env` file in the project root:

```env
# OpenRouter API Key (required for AI-powered soul generation)
OR_MASTER_KEY=sk-or-v1-...

# Gateway Configuration
SAVANT_GATEWAY_HOST=127.0.0.1
SAVANT_GATEWAY_PORT=8080

# Threat Intelligence (optional)
SAVANT_THREAT_INTEL_URL=https://api.savant.ai/v1/threat-intel/blocklist
```

---

## Skill Management

### Installing Skills from ClawHub

```rust
// Skills are automatically scanned before installation
let manager = SkillManager::new(skills_dir);
let result = manager.install_from_clawhub("username/skill-name", None).await?;

match result {
    InstallResult { success: true, .. } => println!("Skill installed"),
    InstallResult { success: false, gate_result: Some(gate), .. } => {
        // Show approval prompt to user
        let prompt = gate.approval_prompt();
        println!("{} - {} clicks required", prompt.warning_message, prompt.clicks_required);
    }
}
```

### Skill Discovery

Skills are discovered from two locations:
- **Swarm-wide:** `<workspace>/skills/`
- **Agent-specific:** `<workspace>/workspaces/workspace-{name}/skills/`

Every discovered skill is mandatory-scanned before loading.

### OpenClaw Compatibility

Skills use the OpenClaw format with `SKILL.md` files:

```markdown
---
name: my-skill
description: A useful skill for doing things
version: 1.0.0
author: username
metadata:
  capabilities: ["file-read", "web-search"]
---

# My Skill

Instructions and implementation details...
```

---

## Crate Map

| Crate | Purpose |
|:------|:--------|
| `savant_core` | Shared types, config, error handling, traits |
| `savant_gateway` | Axum WebSocket server, authentication, skill control |
| `savant_agent` | Agent lifecycle, swarm coordination, LLM providers |
| `savant_cognitive` | Strategic synthesis, goal decomposition, proactive loops |
| `savant_memory` | Hybrid storage (SQLite + Fjall + rkyv vectors) |
| `savant_skills` | OpenClaw skills, security scanner, ClawHub, Docker/Nix/MCP |
| `savant_ipc` | Zero-copy inter-process communication |
| `savant_echo` | ECHO protocol (speculative ReAct) |
| `savant_canvas` | A2UI rendering and diff-based state updates |
| `savant_channels` | Telegram and WhatsApp integrations |
| `savant_cli` | CLI entry point |
| `savant_security` | Crypto-cap token verification |
| `savant_panopticon` | Monitoring and telemetry |

---

## Project Structure

```
Savant/
├── Cargo.toml              # Workspace root (wasmtime 36.0.0)
├── .env                    # Environment configuration
├── CHANGELOG.md            # Release changelog
├── AUDIT.md                # Production audit report
├── crates/
│   ├── core/               # Shared types, config, DB, errors
│   ├── gateway/            # Axum WebSocket server + auth + skills
│   ├── agent/              # Agent lifecycle + LLM providers
│   ├── cognitive/          # Synthesis, decomposition, proactive loops
│   ├── memory/             # Hybrid storage engine + consolidation
│   ├── skills/             # OpenClaw skills + security + ClawHub
│   ├── ipc/                # Zero-copy IPC substrate
│   ├── echo/               # ECHO protocol (speculative ReAct)
│   ├── canvas/             # A2UI rendering
│   ├── channels/           # Telegram, WhatsApp integrations
│   ├── cli/                # CLI entry point
│   ├── security/           # CCT verification
│   └── panopticon/         # Telemetry and monitoring
├── dashboard/              # Next.js observability dashboard
└── docs/                   # Documentation suite
```

---

## Documentation

- [Architecture Overview](docs/architecture/) — System design and data flow
- [API Reference](docs/api/) — Control frame schemas and WebSocket protocol
- [Security Model](docs/security/) — Authentication, sandboxing, and threat detection
- [Changelog](CHANGELOG.md) — Release history and changes
- [Audit Report](AUDIT.md) — Production readiness audit

---

## Building

```bash
# Build all crates
cargo build

# Run all tests
cargo test --workspace

# Check for warnings (should be zero)
cargo check

# Build release
cargo build --release
```

---

<div align="center">
  <p><i>Savant is an Atlas-class autonomous project.</i></p>
  <p><b>Savant</b> &bull; 2026</p>
</div>

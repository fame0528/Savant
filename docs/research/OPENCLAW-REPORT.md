# OpenClaw — Comprehensive Audit Report

**Date:** 2026-03-24  
**Auditor:** Automated Codebase Audit  
**Source:** C:\Users\spenc\dev\Savant\research\competitors\openclaw\  
**Version:** 2026.3.23  

---

## 1. Project Overview

**OpenClaw** is an open-source, personal AI assistant platform that acts as a **multi-channel messaging gateway**. It connects to 20+ messaging platforms (WhatsApp, Telegram, Slack, Discord, Signal, iMessage, IRC, Microsoft Teams, Matrix, etc.) and routes messages through a central **Gateway** server to an AI agent that can execute tools, manage sessions, and interact with the user across any channel.

### Language & Tech Stack

| Layer | Technology |
|-------|-----------|
| **Primary language** | TypeScript (ESM, strict mode) |
| **Runtime** | Node.js 22.16+ / 24 (recommended) |
| **Package manager** | pnpm (monorepo with workspaces) |
| **TypeScript execution** | Bun (preferred for dev/scripts), tsx (fallback) |
| **Build tool** | tsdown (custom build pipeline) |
| **Test framework** | Vitest with V8 coverage |
| **Lint/Format** | Oxlint + Oxfmt |
| **HTTP server** | Hono (gateway HTTP) + Express (legacy) |
| **WebSocket** | ws library |
| **Browser automation** | Playwright-core (Chromium/CDP) |
| **Mobile apps** | Swift (iOS/macOS), Kotlin (Android) |
| **Config schema** | Zod + TypeBox |
| **Database** | SQLite (sessions, memory), sqlite-vec (vector search) |
| **Container support** | Docker, Podman |
| **Package type** | ESM (`"type": "module"`) |

### Architecture Summary

```
WhatsApp / Telegram / Slack / Discord / Signal / iMessage / ... (20+ channels)
                    │
                    ▼
          ┌─────────────────┐
          │     Gateway      │  (WebSocket control plane)
          │  ws://127.0.0.1:18789 │
          └────────┬────────┘
                   │
          ┌────────┼────────────────────┐
          │        │                    │
     Pi Agent    CLI               WebChat UI
     (RPC mode)  (openclaw ...)   (Control UI)
          │        │                    │
          ▼        ▼                    ▼
     Tool exec   Send/Config     Dashboard/Chat
     Browser     Onboard/Doctor  Canvas/A2UI
     Canvas      Plugins         Sessions view
     Cron        Skills
```

---

## 2. Architecture & Design Patterns

### 2.1 Gateway-First Architecture

The Gateway is the **central control plane** — a WebSocket server bound to `127.0.0.1:18789` that:
- Manages all channel connections (inbound + outbound message routing)
- Hosts the agent runtime (embedded Pi agent via RPC)
- Serves the Control UI (web dashboard)
- Runs cron jobs, hooks, and background maintenance
- Handles authentication (tokens, Tailscale identity, loopback)
- Manages session persistence (JSONL transcripts on disk)

Key files:
- `src/gateway/server.impl.ts` — Main gateway orchestration (1357 lines)
- `src/gateway/server-methods.ts` — 27+ WS method handlers
- `src/gateway/server.ts` — Re-exports

### 2.2 Plugin System (Extensions)

OpenClaw uses a **plugin architecture** with two categories:
1. **Channel plugins** — Implement `ChannelPlugin` contract (Telegram, Discord, Slack, etc.)
2. **Provider plugins** — Register model providers (OpenAI, Anthropic, Google, etc.)

Each plugin lives in `extensions/<id>/` as a workspace package with:
- `openclaw.plugin.json` — Manifest (id, channels, providers, config schema)
- `index.ts` — Entry point using `definePluginEntry()` or `defineChannelPluginEntry()`
- `src/` — Implementation code

**Plugin SDK surface** (`openclaw/plugin-sdk/*`) provides ~120+ subpath exports covering:
- Channel contracts (`channel-contract.ts`, `channel-runtime.ts`, `channel-setup.ts`)
- Provider contracts (`provider-entry.ts`, `provider-stream.ts`)
- Plugin lifecycle (`plugin-entry.ts`, `plugin-runtime.ts`)
- Security helpers (`security-runtime.ts`, `ssrf-runtime.ts`)
- Media handling (`media-runtime.ts`, `media-understanding-runtime.ts`)
- Hook system (`hook-runtime.ts`)
- Config/schema utilities

### 2.3 Channel Plugin Contract

Every channel plugin implements the `ChannelPlugin` type with these adapters:

```typescript
type ChannelPlugin = {
  id: ChannelId;
  meta: ChannelMeta;
  capabilities: ChannelCapabilities;
  config: ChannelConfigAdapter;
  setup?: ChannelSetupAdapter;
  pairing?: ChannelPairingAdapter;
  security?: ChannelSecurityAdapter;
  groups?: ChannelGroupAdapter;
  outbound?: ChannelOutboundAdapter;
  status?: ChannelStatusAdapter;
  gateway?: ChannelGatewayAdapter;
  auth?: ChannelAuthAdapter;
  lifecycle?: ChannelLifecycleAdapter;
  threading?: ChannelThreadingAdapter;
  messaging?: ChannelMessagingAdapter;
  agentTools?: ChannelAgentToolFactory | ChannelAgentTool[];
  // ... ~20 more adapters
};
```

### 2.4 Multi-Agent Routing

The routing system (`src/routing/resolve-route.ts`) implements a **7-tier binding resolution**:

1. **binding.peer** — Direct peer match (specific user/group/channel ID)
2. **binding.peer.parent** — Thread parent inheritance
3. **binding.guild+roles** — Guild + Discord role-based routing
4. **binding.guild** — Guild-level routing
5. **binding.team** — Team-level routing
6. **binding.account** — Account-level routing
7. **binding.channel** — Channel-level wildcard routing
8. **default** — Fallback to default agent

Each binding maps to an **agent ID**, which determines:
- Workspace directory (`~/.openclaw/workspace`)
- System prompt injection (AGENTS.md, SOUL.md, TOOLS.md)
- Tool policy (which tools are available)
- Session isolation

### 2.5 Session Model

Sessions are identified by **session keys** built from:
```
agentId:channel:accountId:peerKind:peerId
```

- **Main sessions** — Direct 1:1 chats with the owner
- **Group sessions** — Isolated per-group/channel conversations
- **Subagent sessions** — Child agent runs with depth limits
- **Cron sessions** — Scheduled job runs
- **Hook sessions** — Event-triggered runs

Sessions persist as **JSONL transcript files** on disk with:
- Message history (user/assistant/tool messages)
- Tool call results with media attachments
- Session metadata (model, tokens, cost)

### 2.6 Plugin Hook System

OpenClaw implements an extensive **lifecycle hook system** with 25+ hook types:

```
Agent lifecycle: beforeAgentStart, agentEnd, beforePromptBuild, beforeModelResolve
Session lifecycle: sessionStart, sessionEnd, beforeReset
Message lifecycle: messageReceived, messageSending, messageSent, beforeMessageWrite
Tool lifecycle: beforeToolCall, afterToolCall, toolResultPersist
Compaction: beforeCompaction, afterCompaction
LLM lifecycle: llmInput, llmOutput
Subagent lifecycle: subagentSpawning, subagentSpawned, subagentEnded
Gateway lifecycle: gatewayStart, gatewayStop
Inbound claim: inboundClaim
```

Hooks support priority ordering, async execution, and result modification.

---

## 3. Top Features (Detailed)

### 3.1 Multi-Channel Messaging Gateway

**20+ channels** supported with unified routing, typing indicators, media handling, and DM security:

| Channel | Library/Protocol | Key Features |
|---------|-----------------|-------------|
| Telegram | grammY | Topics, inline buttons, native commands, forum auto-labeling |
| WhatsApp | Baileys (web) | QR link, group routing, media pipeline |
| Discord | discord.js | Slash commands, thread bindings, components |
| Slack | Bolt (Socket Mode) | Auto-threading, blocks, workflow triggers |
| Signal | signal-cli | Linked device, group routing |
| iMessage | imsg / BlueBubbles | macOS native, webhook integration |
| Matrix | matrix-js-sdk | Rooms, E2EE, bot policies |
| IRC | irc library | Nick/channel routing, pairing |
| Microsoft Teams | Bot Framework | Cards, approvals |
| Google Chat | Chat API | HTTP webhook |
| LINE | Messaging API | Webhook bot |
| Feishu | Feishu SDK | Cards, streaming, ACP |
| Mattermost | Mattermost API | Channel routing |
| Twitch | Twitch API | Chat integration |
| Nostr | Nostr protocol | Decentralized messaging |

**DM Security Model:**
- Default `dmPolicy="pairing"` — unknown senders get a pairing code
- `allowFrom` lists — explicit sender allowlists per channel
- `opencl

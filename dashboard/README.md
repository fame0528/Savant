<!-- markdownlint-disable MD033 -->
<div align="center">

<img src="../img/savant.png" alt="Savant Logo" width="120" />

# SAVANT DASHBOARD

**The Brain. Real-Time Swarm Observability.**

A production-grade Next.js 16 dashboard for monitoring, controlling, and interacting with the Savant agent swarm. Live WebSocket streaming, cognitive insights, soul manifestation, and full agent lifecycle management — all in a single pane of glass.

[![Next.js](https://img.shields.io/badge/Next.js-16-%23000000?style=flat-square&logo=nextdotjs&logoColor=%2300fbff)](https://nextjs.org/)[![React](https://img.shields.io/badge/React-19-%23000000?style=flat-square&logo=react&logoColor=%2300fbff)](https://react.dev/)[![Tauri](https://img.shields.io/badge/Tauri-2.0-%23000000?style=flat-square&logo=tauri&logoColor=%2300fbff)](https://tauri.app/)[![TypeScript](https://img.shields.io/badge/TypeScript-5-%23000000?style=flat-square&logo=typescript&logoColor=%2300fbff)](https://www.typescriptlang.org/)[![Vitest](https://img.shields.io/badge/Vitest-4-%23000000?style=flat-square&logo=vitest&logoColor=%2300fbff)](https://vitest.dev/)

</div>

---

## Overview

The Savant Dashboard is the primary observability and control interface for the Savant swarm. It connects to the Axum gateway via WebSocket, streams real-time telemetry, and provides a rich UI for agent interaction.

- **Real-Time Chat** — Send messages to any agent lane, receive streaming responses with thinking/reasoning visibility
- **Agent Management** — Sidebar shows all discovered agents with status, lane switching, and swarm capacity metrics
- **Cognitive Insights** — Live feed of agent reflections, learning insights, and heartbeat telemetry
- **Soul Manifestation** — Generate and refine agent personality manifests with depth/integrity/ethics scoring
- **Personality Evolution** — View proposed mutations, approve/reject changes, track evolution score and trait snapshots
- **Debug Console** — Full gateway log streaming with copy-to-clipboard and pause controls
- **Authenticated API** — All `/api/*` calls use `authFetch` with Bearer token from Tauri ignition
- **Enterprise Image Loading** — Agent avatars loaded via authenticated blob URLs (no broken images)
- **Copy-to-Clipboard** — Unified 3-tier fallback (Tauri plugin, navigator.clipboard, execCommand) across all copy buttons
- **Splash Screen** — Animated boot sequence with auto-dismiss
- **Setup Wizard** — First-launch configuration for API keys and model selection

---

## Pages

| Route | Page | Purpose |
| :--- | :--- | :--- |
| `/` | **Chat** | Main conversation view with agent messages, streaming, and thinking blocks |
| `/health` | **Health** | Agent health monitoring and subsystem status |
| `/evolution` | **Evolution** | Personality mutation proposals, approval workflow, trait history |
| `/evolution/behind-the-curtain` | **Behind the Curtain** | Deep evolution internals viewer |
| `/tune` | **Fine-Tuning** | Agent parameter tuning and configuration |
| `/settings` | **Settings** | Dashboard and gateway configuration |
| `/marketplace` | **Marketplace** | Skill marketplace browser and installer |
| `/mcp` | **MCP** | Model Context Protocol server management |
| `/changelog` | **Changelog** | Release history and version notes |
| `/faq` | **FAQ** | Frequently asked questions |
| `/browser` | **Browser** | Embedded web browser panel |
| `/telemetry` | **Telemetry** | Raw telemetry data viewer |

---

## Architecture

```text
dashboard/
├── src/
│   ├── app/                    # Next.js App Router pages
│   │   ├── page.tsx            # Main chat view (manifest, evolution, chat modes)
│   │   ├── layout.tsx          # Root layout with DashboardProvider
│   │   ├── health/             # Agent health monitoring
│   │   ├── evolution/          # Personality evolution viewer
│   │   ├── tune/               # Fine-tuning controls
│   │   ├── settings/           # Configuration UI
│   │   ├── marketplace/        # Skill marketplace
│   │   ├── mcp/                # MCP server management
│   │   ├── changelog/          # Release notes
│   │   ├── faq/                # Help content
│   │   ├── browser/            # Embedded browser
│   │   └── telemetry/          # Raw telemetry viewer
│   ├── components/
│   │   ├── DashboardShell.tsx  # Main shell: sidebar, header, right panel, debug console
│   │   ├── FormattedContent.tsx # Markdown renderer with code blocks and copy buttons
│   │   ├── BrowserPanel.tsx    # Embedded browser component
│   │   ├── SplashScreen.tsx    # Animated boot sequence
│   │   └── SetupWizard.tsx     # First-launch configuration
│   ├── context/
│   │   └── DashboardContext.tsx # Global state: WebSocket, agents, messages, insights
│   └── lib/
│       ├── tauri.ts            # Tauri bridge: ignition, auth, clipboard, URL helpers
│       ├── logger.ts           # Structured logging utility
│       ├── browser.ts          # Browser automation helpers
│       └── theme.ts            # Theme configuration
├── public/
│   └── img/                    # Static assets (logo, icons)
├── package.json
├── tsconfig.json
├── next.config.ts
└── vitest.config.ts
```

---

## Data Flow

```text
┌─────────────┐    WebSocket     ┌─────────────────┐
│   Dashboard  │ ◄──────────────► │  Axum Gateway    │
│  (Next.js)   │   EVENT: frames  │  (port 8080)     │
└──────┬───────┘                  └────────┬─────────┘
       │                                   │
       ▼                                   ▼
┌──────────────┐                  ┌─────────────────┐
│ DashboardCtx │                  │  Agent Swarm     │
│  - agents    │                  │  - .savant       │
│  - messages  │                  │  - consciousness │
│  - insights  │                  │  - memory engine │
│  - streaming │                  │  - skill forge   │
└──────────────┘                  └─────────────────┘
```

**Event Types (case-insensitive):**

| Event | Direction | Purpose |
| :--- | :--- | :--- |
| `session.assigned` | Server → Client | Session handshake with session ID |
| `agents.discovered` | Server → Client | Agent list update (sidebar population) |
| `history` | Server → Client | Lane message history response |
| `chat.message` | Server → Client | New chat message (persisted) |
| `chat.chunk` | Server → Client | Streaming text chunk (with reasoning) |
| `swarm_insight_history` | Server → Client | Cognitive insight history |
| `learning.insight` | Server → Client | Emergent learning event |
| `debug.log` | Server → Client | Gateway debug log entry |
| `heartbeat` | Server → Client | Keepalive (ignored by UI) |
| `HistoryRequest` | Client → Server | Request lane message history |
| `InitialSync` | Client → Server | Request full sidebar hydration |
| `SwarmInsightHistoryRequest` | Client → Server | Request insight history |

---

## Quick Start

### Development

```bash
cd dashboard
npm install        # Install dependencies
npm run dev        # Start dev server on http://localhost:3000
```

### Production Build

```bash
npm run build      # Next.js production build
npm run start      # Start production server
```

### Tauri Desktop App

The dashboard is bundled as the frontend for the Tauri desktop app. The Tauri shell handles:

- **Swarm ignition** — Calls `ignite_swarm` on startup, receives API key and gateway port
- **WebSocket connection** — Connects to `ws://127.0.0.1:{port}/ws` with authenticated session
- **System log forwarding** — Tauri `system-log-event` events displayed in debug console
- **Clipboard integration** — Uses `@tauri-apps/plugin-clipboard-manager` with fallback chain

### Testing

```bash
npm run test           # Run tests once
npm run test:watch     # Watch mode
npm run test:coverage  # Coverage report
```

### Linting

```bash
npm run lint           # ESLint
npx tsc --noEmit       # TypeScript type checking
```

---

## Key Components

### DashboardContext (`src/context/DashboardContext.tsx`)

The global state provider. Manages:

- **WebSocket lifecycle** — Connect, authenticate, reconnect with exponential backoff
- **Agent state** — Discovery, selection, lane switching
- **Message routing** — Lane-based message storage, streaming content accumulation
- **Cognitive insights** — Learning events, heartbeat telemetry, debug logs
- **Evolution state** — Mutation proposals, trait snapshots, evolution score

All event types are normalized to lowercase at both entry points (`processEvent` and `onmessage` handler) for gateway compatibility.

### DashboardShell (`src/components/DashboardShell.tsx`)

The main layout shell containing:

- **Left sidebar** — Agent list, navigation links, swarm capacity
- **Header** — Connection status, agent lane selector
- **Right panel** — Cognitive insights (REFLECTIONS) with collapse/expand
- **Debug console** — Log streaming with copy and pause controls

### FormattedContent (`src/components/FormattedContent.tsx`)

Markdown renderer using `react-markdown` with `remark-gfm`. Features:

- Code block syntax highlighting with copy button (`CodeCopyButton` component)
- Collapsible thinking/reasoning blocks
- Tag stripping for OpenRouter processing markers and tool calls

### Tauri Bridge (`src/lib/tauri.ts`)

Centralized utilities for Tauri integration:

- `copyToClipboard()` — 3-tier fallback (Tauri → navigator → execCommand)
- `getGatewayHost()` / `getGatewayPort()` / `getHttpUrl()` / `getWsUrl()` — URL helpers
- `authFetch()` — Authenticated fetch with Bearer token
- `setDashboardApiKey()` / `setGatewayPort()` — Runtime configuration
- `isTauri()` / `getAppVersion()` / `getDashboardConfig()` — Platform detection

---

## Configuration

The dashboard reads configuration from the Tauri ignition response:

| Key | Source | Purpose |
| :--- | :--- | :--- |
| `dashboard_api_key` | `ignite_swarm` IPC | API key for authenticated REST calls |
| `gateway_port` | `ignite_swarm` IPC | Gateway HTTP/WS port (default: 8080) |

Environment variables (for non-Tauri development):

```env
NEXT_PUBLIC_DASHBOARD_API_KEY=your-api-key
NEXT_PUBLIC_GATEWAY_PORT=8080
```

---

## Styling

The dashboard uses CSS Modules with a dark theme:

- **Primary accent** — Cyan (`#00d4ff`)
- **Background** — Near-black (`#0a0a0a`)
- **Glass morphism** — Translucent panels with backdrop blur
- **Monospace** — JetBrains Mono for code and telemetry
- **Responsive** — Sidebar collapse, right panel toggle

All styles are in `page.module.css` and component-specific `.module.css` files. Global styles in `globals.css`.

---

<div align="center>

**Savant Dashboard** &bull; Part of the [Savant](../README.md) ecosystem

**Savant** &bull; 2026

</div>

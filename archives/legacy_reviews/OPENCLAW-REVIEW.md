# OpenClaw Framework Review

> **Repository:** github.com/openclaw/openclaw — 327k+ stars
> **Language:** TypeScript (ESM), Node 22.16+
> **Position:** The standard. All other frameworks are compared to it.

---

## What Makes It The Standard

1. **25+ channel integrations** — WhatsApp, Telegram, Slack, Discord, Signal, iMessage, Google Chat, MS Teams, Matrix, IRC, Feishu, LINE, Mattermost, Nostr, Twitch, Zalo, and more
2. **Production hardened** — 18 months in production, formal security audit, DM pairing, sandboxing
3. **60+ plugin extensions** — npm package distribution, plugin SDK with 100+ subpath exports
4. **Native companion apps** — macOS (SwiftUI menu bar), iOS (SwiftUI), Android (Kotlin) with voice wake, camera, screen recording
5. **Live Canvas (A2UI)** — Agent pushes HTML/CSS/JS to a live visual workspace
6. **Device nodes** — Remote execution on paired macOS/iOS/Android devices
7. **ClawHub skills registry** — External skill discovery and installation
8. **Config hot-reload** — File watcher with hybrid restart mode
9. **Voice Wake + Talk Mode** — Wake words on macOS/iOS + continuous voice on Android
10. **Sponsorship** — OpenAI, Vercel, Blacksmith, Convex

---

## Architecture

- Single-process gateway owns all state
- WebSocket-first protocol with TypeBox schemas
- Agent runtime DELEGATED to `pi-agent-core` (external library) — OpenClaw does NOT own the agent inference loop
- Hub-and-spoke: Gateway → Pi Agent, CLI, macOS App, iOS/Android, Browser
- Flat agent model — explicitly rejects hierarchical multi-agent orchestration

---

## What Savant Does Better

| Area | Savant Advantage |
|------|-----------------|
| **Agent runtime ownership** | OpenClaw delegates to external `pi-agent-core`. Savant owns the full stack. |
| **Memory architecture** | OpenClaw has multiple memory plugins, "planning to converge." Savant has a purpose-built system. |
| **Hierarchical multi-agent** | OpenClaw explicitly rejects it. Savant supports swarm orchestration. |
| **First-class MCP** | OpenClaw uses `mcporter` bridge. Savant has native MCP crate. |
| **Rust performance** | TypeScript vs Rust — order of magnitude difference in throughput. |
| **Cognitive layer** | OpenClaw has no cognitive synthesis/prediction. Savant does. |
| **Speculative execution** | Not present in OpenClaw. Savant has it. |
| **Security architecture** | OpenClaw has sandboxing. Savant has enclaves, WASM, attestation, proofs. |

---

## What Savant Is Missing

1. 25+ channel integrations
2. DM pairing system with cryptographic codes
3. Voice Wake + Talk Mode
4. Live Canvas (A2UI) — agent-driven visual workspace
5. Device nodes — remote execution on phones/tablets
6. ClawHub skills registry
7. Config hot-reload
8. Formal security audit CLI
9. Native companion apps (macOS, iOS, Android)
10. Tailscale integration
11. Multi-agent routing with channel bindings
12. Auth profile rotation with cooldown
13. Nix packaging
14. mDNS/Bonjour discovery
15. Secret management (env/file/exec refs)
16. Session maintenance (pruning, rotation, disk budgets)

---

## Key Takeaway

OpenClaw wins on **breadth** (25 channels, native apps, voice, canvas). Savant wins on **depth** (agent intelligence, memory, security, performance). The opportunity: match OpenClaw's channel breadth while maintaining Savant's depth advantage. Focus on channels and native apps as the next frontier.

# FID-20260322-CHANNEL-EXPANSION-ALL-29

**Date:** 2026-03-22
**Status:** PLANNING
**Protocol:** Development Workflow + Perfection Loop
**Source:** Exhaustive scan of 6 competitor frameworks — full channel/provider inventory

---

## Overview

Surpass the entire competitive landscape by implementing all 29 channels that any competitor supports. Current Savant has 4 channels (Discord, Telegram, WhatsApp, CLI). Competitor leaders: ZeroClaw (38), OpenClaw (21), PicoClaw (15).

---

## Full Channel List (29)

| # | Channel | Priority | Competitors | Transport | Dependencies |
|---|---------|----------|-------------|-----------|-------------|
| 1 | Discord | DONE | 6/6 | Gateway WebSocket | serenity |
| 2 | Telegram | DONE | 6/6 | Long polling | teloxide |
| 3 | WhatsApp | DONE | 6/6 | WebSocket bridge | existing |
| 4 | CLI | DONE | 6/6 | stdin/stdout | existing |
| 5 | Slack | CRITICAL | 6/6 | Socket Mode | slack-morphism or slack-rs-api |
| 6 | Matrix | CRITICAL | 6/6 | Sync API | matrix-sdk |
| 7 | IRC | CRITICAL | 5/6 | TCP | irc crate |
| 8 | Email | CRITICAL | 5/6 | IMAP+SMTP | imap + lettre |
| 9 | Signal | HIGH | 5/6 | signal-cli JSON-RPC | subprocess |
| 10 | LINE | HIGH | 4/6 | Messaging API webhook | reqwest |
| 11 | Google Chat | HIGH | 3/6 | Chat API webhook | reqwest |
| 12 | Microsoft Teams | HIGH | 3/6 | Bot Framework | reqwest |
| 13 | iMessage | HIGH | 3/6 | AppleScript/imessage | subprocess |
| 14 | Feishu/Lark | HIGH | 5/6 | WebSocket | reqwest |
| 15 | DingTalk | MEDIUM | 4/6 | Stream Mode | reqwest |
| 16 | QQ | MEDIUM | 4/6 | WebSocket | botpy or websocket |
| 17 | WeCom | MEDIUM | 3/6 | WebSocket | reqwest |
| 18 | Mattermost | MEDIUM | 2/6 | WebSocket | reqwest |
| 19 | Nextcloud Talk | LOW | 2/6 | API polling | reqwest |
| 20 | Twitch | LOW | 2/6 | IRC | irc crate |
| 21 | Nostr | LOW | 2/6 | Relay WebSocket | nostr-sdk |
| 22 | Bluesky | LOW | 1/6 | AT Protocol | atrium-api |
| 23 | Twitter/X | LOW | 1/6 | API v2 | reqwest |
| 24 | Reddit | LOW | 1/6 | API polling | reqwest |
| 25 | Notion | LOW | 1/6 | API | reqwest |
| 26 | WhatsApp Business | LOW | 1/6 | API | reqwest |
| 27 | Webhook | LOW | 1/6 | HTTP | axum (existing) |
| 28 | Voice/TTS | LOW | 3/6 | Various | reqwest |
| 29 | MaixCam | SKIP | 1/6 | Hardware | Skip |

---

## Architecture

### Existing Infrastructure
- `ChannelAdapter` trait in `core/src/traits/mod.rs` — 3 methods: name(), send_event(), handle_event()
- `InboxPool` in `channels/src/pool.rs` — channel registry with broadcast + route
- `EventFrame` type in `core/src/types/mod.rs` — standard message format

### Channel Pattern (from existing Discord/Telegram/WhatsApp)
Each channel is a separate module in `channels/src/`:
1. `pub struct XAdapter` — implements `ChannelAdapter`
2. Connection setup (WebSocket/polling/subprocess)
3. Message parsing → `EventFrame`
4. Outbound `EventFrame` → platform-specific API call
5. Health check + reconnection logic

### Competitor Code References (for working examples)
| Competitor | Best channel implementations |
|-----------|---------------------------|
| ZeroClaw | telegram.rs (175K), slack.rs (133K), discord.rs (55K), email.rs (34K) |
| PicoClaw | All channels in pkg/channels/ — clean Go implementations |
| NanoBot | feishu.py (50K), telegram.py (34K), matrix.py (31K), dingtalk.py (24K) |
| OpenClaw | Extension plugins — TypeScript, plugin architecture |
| IronClaw | WASM channels + signal.rs (64K), webhook |

---

## Implementation Plan

### Batch 1: Critical (Slack, Matrix, IRC, Email, Signal)
- ~1,500 LOC
- Covers enterprise + developer + privacy use cases

### Batch 2: High Priority (LINE, Google Chat, Teams, iMessage, Feishu)
- ~1,200 LOC
- Covers Asian market + enterprise + Apple ecosystem

### Batch 3: Medium (DingTalk, QQ, WeCom, Mattermost, Nextcloud)
- ~1,000 LOC
- Covers Chinese enterprise + self-hosted

### Batch 4: Low (Twitch, Nostr, Bluesky, Twitter, Reddit, Notion, Webhook, WhatsApp Business, Voice)
- ~1,200 LOC
- Covers social + decentralized + misc

---

## Perfection Loop

Each channel follows the Perfection Loop:
1. Deep Audit — read competitor implementation for that channel
2. Enhance — identify best patterns from competitor
3. Validate — cargo check
4. Iterate — if improvements found
5. Certify — move to next channel

---

*FID created 2026-03-22. Awaiting Perfection Loop on Batch 1.*

# Provider & Key Management Architecture

> **Decision:** 2026-03-19  
> **Status:** Accepted — no code changes yet, design only

---

## Problem

Savant's swarm architecture requires multiple API keys to avoid rate limiting (one key = instant bottleneck in a swarm). But:

1. **Not all users use OpenRouter** — many have paid plans with OpenAI, Anthropic, Google, etc.
2. **Storing N keys on disk is a security risk** — if the system is compromised, all keys leak
3. **Users shouldn't need to manage keys manually** — non-technical users need this to "just work"
4. **Technical users need full control** — they want to use their own keys, providers, and limits

---

## Design: Two Modes

### Mode 1: Auto Key Management (Default)

**Who it's for:** Savant default users, OpenRouter users, anyone who doesn't want to think about keys.

**How it works:**
1. User creates one **OpenRouter Management API Key** (free, from openrouter.ai/settings/management-keys)
2. User puts it in `.env` as `OR_MANAGEMENT_KEY=sk-or-v1-...`
3. On startup, Savant creates N API keys programmatically (one per agent)
4. Each agent gets its own key → no rate limiting bottleneck
5. On shutdown, runtime keys are deleted
6. **Only one secret on disk. Ever.**

**Why it's the default:**
- Simplest setup — one key, one `.env` file
- Most secure — keys are ephemeral, created at runtime, destroyed on shutdown
- Free tier available — OpenRouter has free models
- Works with the free model router (`openrouter/free`) as ultra-fallback

**Dashboard toggle:** `Settings → Provider → Auto Key Management: ON/OFF`

### Mode 2: Manual Key Management

**Who it's for:** Users with paid plans elsewhere, enterprise users, anyone who wants direct provider control.

**How it works:**
1. User places their API key(s) in agent-specific `.env` files:
   ```
   workspaces/agents/agent-alpha/.env    → OPENAI_API_KEY=sk-...
   workspaces/agents/agent-beta/.env     → ANTHROPIC_API_KEY=sk-ant-...
   workspaces/agents/agent-gamma/.env    → GOOGLE_API_KEY=...
   ```
2. OR user puts a single provider key in the root `.env`:
   ```
   OPENAI_API_KEY=sk-...
   ```
3. Savant respects the user's provider choice from `config/savant.toml`
4. Single-key rate limiting is managed with backoff, not rotation

**Security model:**
- Per-agent keys are isolated (compromised agent can't access other agents' keys)
- Root key is shared across all agents (simpler, but single point of compromise)
- Dashboard shows key status per agent with usage stats

**Dashboard toggle:** `Settings → Provider → Auto Key Management: OFF`

When OFF, dashboard shows:
- Current provider and model
- Key status (active, rate-limited, expired)
- Per-agent key assignment
- "Add Key" button per agent

---

## Dashboard FAQ Page

A new page at `dashboard/src/app/faq/page.tsx` explaining:

### For Non-Technical Users

**Q: What's the fastest way to get started?**
A: Create a free OpenRouter account, get a Management API key, paste it in Settings. Savant handles the rest.

**Q: I have a ChatGPT Plus/Pro subscription. Can I use that?**
A: Yes. Go to Settings → Provider → Auto Key Management: OFF, then add your OpenAI API key.

**Q: I have an Anthropic (Claude) subscription. Can I use that?**
A: Yes. Same process — disable auto management, add your Anthropic key.

**Q: Do I need multiple keys for a swarm?**
A: If using Auto Key Management (OpenRouter), no — Savant creates them automatically. If using another provider, you may hit rate limits with a single key. Consider per-agent keys.

**Q: Where do I put my API key?**
A: Settings page → Provider section → paste your key. Or manually create `workspaces/agents/{agent-name}/.env`.

**Q: Is my key safe?**
A: Keys are stored locally on your machine. Auto-managed keys are ephemeral (created at runtime, deleted on shutdown). Manual keys are in your `.env` files — protect them like passwords.

### For Technical Users

**Q: How does auto key management work?**
A: Uses OpenRouter's Management API (`/api/v1/keys`) to create per-agent API keys at startup. One Management key → N runtime keys → zero on-disk secrets (besides the Management key itself).

**Q: Can I use multiple providers simultaneously?**
A: Not directly — each agent uses one provider at a time. But different agents can use different providers. Configure per-agent in their workspace.

**Q: What if I want OpenRouter but my own model selection?**
A: Keep Auto Key Management ON, then change the model in Settings → AI → Model. The Management API keys work with any model on OpenRouter.

**Q: How do I set spending limits?**
A: OpenRouter Management keys support `limit` and `limit_reset` (daily/weekly/monthly). Configure these when creating the Management key on openrouter.ai.

---

## Settings Page Design

The dashboard settings page should show:

```
┌─────────────────────────────────────────────────┐
│  Provider Settings                              │
├─────────────────────────────────────────────────┤
│                                                 │
│  Auto Key Management    [ON ▼]                  │
│  ─────────────────────────────────              │
│  ✓ Recommended for most users                   │
│  Creates per-agent keys at runtime.             │
│  Only one key stored on disk.                   │
│                                                 │
│  Management API Key:                            │
│  [sk-or-v1-••••••••••••••••] [Change]           │
│                                                 │
│  Status: ✅ Active (3 keys created)             │
│  Agent Alpha: key-abc123 (active)               │
│  Agent Beta:  key-def456 (active)               │
│  Agent Gamma: key-ghi789 (active)               │
│                                                 │
├─────────────────────────────────────────────────┤
│  OR (when Auto OFF)                             │
├─────────────────────────────────────────────────┤
│                                                 │
│  Provider: [OpenAI ▼]                           │
│  Model:    [gpt-4.1 ▼]                          │
│                                                 │
│  API Key:  [sk-••••••••••••] [Save]             │
│                                                 │
│  Per-Agent Keys:                                │
│  Agent Alpha: [sk-••••] [Change] [Remove]       │
│  Agent Beta:  [sk-••••] [Change] [Remove]       │
│  Agent Gamma: (using root key)                  │
│                                                 │
│  [ℹ️ Need help?] → FAQ Page                    │
│                                                 │
└─────────────────────────────────────────────────┘
```

---

## Implementation Notes (Future)

### When implemented:
- **Core crate:** Add `KeyManager` struct with `create_keys()`, `get_key(agent_id)`, `delete_keys()`
- **Gateway:** Add `KeyManagementSet`, `KeyManagementStatus` control frames
- **Dashboard:** Settings page with toggle, key input, FAQ link
- **Security:** Never log keys, never store runtime keys to disk, encrypt at rest if persisted

### Not implemented yet:
- This is a design doc only
- Current system continues to work with `OR_MASTER_KEY` in `.env`
- Migration path: `OR_MASTER_KEY` → `OR_MANAGEMENT_KEY` (rename, same concept, better API)

---

*This is a planning document. No code changes were made.*

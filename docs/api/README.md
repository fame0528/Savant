# API Reference

> **Last Updated:** 2026-05-25 (v0.3.4)

---

## HTTP REST Endpoints

### Health & Status

| Endpoint | Method | Auth | Description |
|:---------|:-------|:-----|:------------|
| `/live` | GET | No | Returns "OK" if gateway is running |
| `/ready` | GET | No | Returns "OK" if gateway is ready |
| `/health` | GET | No | System health with memory status, uptime |
| `/api/status` | GET | No | Detailed system status |

### Agents

| Endpoint | Method | Auth | Description |
|:---------|:-------|:-----|:------------|
| `/api/agents` | GET | Yes | List all discovered agents |
| `/api/agents/:name/image` | GET | Yes | Agent avatar image |

### Settings & Config

| Endpoint | Method | Auth | Description |
|:---------|:-------|:-----|:------------|
| `/api/settings` | GET | Yes | Current gateway configuration |
| `/api/settings` | POST | Yes | Update configuration |
| `/api/settings/reset` | GET/POST | Yes | Reset configuration to defaults |
| `/api/models` | GET | Yes | Available AI providers and models |

### MCP (Model Context Protocol)

| Endpoint | Method | Auth | Description |
|:---------|:-------|:-----|:------------|
| `/api/mcp/servers` | GET | Yes | List MCP servers |
| `/api/mcp/servers/install` | POST | Yes | Install MCP server |
| `/api/mcp/servers/add` | POST | Yes | Add MCP server config |
| `/api/mcp/servers/remove` | POST | Yes | Remove MCP server |
| `/api/mcp/servers/uninstall` | POST | Yes | Uninstall MCP server |
| `/api/mcp/servers/info` | GET | Yes | MCP server info |

### Trajectories

| Endpoint | Method | Auth | Description |
|:---------|:-------|:-----|:------------|
| `/api/trajectories` | GET | Yes | List trajectory recordings |
| `/api/trajectories/stats` | GET | Yes | Trajectory statistics |

### Snapshot & Restore

| Endpoint | Method | Auth | Description |
|:---------|:-------|:-----|:------------|
| `/api/snapshot` | POST | Yes | Create system snapshot |
| `/api/restore` | POST | Yes | Restore from snapshot |

### Changelog

| Endpoint | Method | Auth | Description |
|:---------|:-------|:-----|:------------|
| `/api/changelog` | GET | Yes | Public changelog |

### Dashboard Feature APIs (v0.3.4)

| Endpoint | Method | Auth | Description |
|:---------|:-------|:-----|:------------|
| `/api/memory/search` | GET | Yes | Search memory. Params: `q` (query), `limit` (max results) |
| `/api/governor/status` | GET | Yes | Resource governor status: pressure level, CPU%, memory%, permits |
| `/api/consciousness/status` | GET | Yes | Consciousness daemon state: Thinking/Idle/Dormant/Wondering, entropy |

### Config Mutation (v0.3.4 — Immutable Fields)

The `ConfigSet` WebSocket frame and `POST /api/settings` endpoint block changes to security-critical fields at runtime:

**Immutable fields:** `server.dashboard_api_key`, `server.host`, `server.port`, `server.signing_key`, `security.enable_blocklist_sync`

Attempting to modify these returns a 403 Forbidden with a message explaining that the config file must be edited and the service restarted.

---

## WebSocket Protocol

All communication between the dashboard and gateway occurs over a single WebSocket connection at `ws://localhost:3000/ws`.

Canvas A2UI visualization at `ws://localhost:3000/ws/canvas` (requires API key authentication).

### Authentication (v0.3.4)

REST and WebSocket endpoints (except `/live`, `/ready`, `/health`, `/ws`) require authentication when `dashboard_api_key` is configured:

- **Header:** `Authorization: Bearer <key>` or `X-API-Key: <key>`
- **Constant-time comparison** prevents timing attacks
- **Empty key** = development mode (no auth required)

### Frame Format

Every message follows the AEC (Agent Event Control) protocol:

```json
{
  "session_id": "dashboard-session",
  "payload": {
    "type": "<RequestType>",
    "data": { ... }
  }
}
```

### Event Format

Events from the gateway are prefixed with `EVENT:`:

```
EVENT:{"event_type":"<EventType>","payload":{...}}
```

The dashboard client splits on the first `:` to extract the prefix and JSON payload.

---

## Client → Gateway (Request Frames)

### ChatMessage

Send a message to an agent or broadcast to the swarm.

```json
{
  "session_id": "dashboard-session",
  "payload": {
    "role": "user",
    "content": "Hello swarm",
    "recipient": "prometheus"
  }
}
```

| Field | Type | Required | Description |
|:------|:-----|:---------|:------------|
| `role` | string | Yes | Always `"user"` for client messages |
| `content` | string | Yes | Message text |
| `recipient` | string | No | Target agent ID, omit for swarm broadcast |

### SoulManifest

Request AI-powered soul generation from a natural language prompt.

```json
{
  "session_id": "dashboard-session",
  "payload": {
    "type": "SoulManifest",
    "data": {
      "prompt": "A business strategist who operates with zero cost",
      "name": "Prometheus"
    }
  }
}
```

### SoulUpdate (v0.3.4 — 100KB limit)

Update an agent's SOUL.md file on disk. **Max 100KB per field** (content and reasoning).

```json
{
  "session_id": "dashboard-session",
  "payload": {
    "type": "SoulUpdate",
    "data": {
      "agent_id": "prometheus",
      "content": "# SOUL.md\n\n## 1. Identity Core..."
    }
  }
}
```

### BulkManifest (v0.3.4 — 10 agent limit)

Deploy multiple agents from an expansion plan. **Max 10 agents per request.**

```json
{
  "session_id": "dashboard-session",
  "payload": {
    "type": "BulkManifest",
    "data": {
      "agents": [
        { "name": "Agent A", "soul": "# SOUL.md..." },
        { "name": "Agent B", "soul": "# SOUL.md..." }
      ]
    }
  }
}
```

### NLCommand (v0.3.4 — 10KB limit)

Natural language command. **Max 10,000 characters.**

### ConfigGet

Retrieve the current gateway configuration from `savant.toml`.

### ConfigSet (v0.3.4 — Immutable field protection)

Update a configuration value. Security-critical fields are blocked (see Immutable Fields above).

### ModelsList

Get available AI providers and their parameter descriptors.

### HistoryRequest

Retrieve persisted message history for a communication lane.

### SwarmInsightHistoryRequest

Retrieve the swarm's cognitive insight history.

### InitialSync

Sent on WebSocket open to request full state synchronization.

### SoulMutationPropose (v0.3.4 — 100KB limit)

Propose a SOUL.md mutation. Max 100KB per field.

### SoulMutationApprove / SoulMutationReject

Approve or reject a pending mutation proposal.

---

## Gateway → Client (Event Types)

### `agents.discovered`

Sent when the agent registry updates.

### `history`

Sent in response to `HistoryRequest`.

### `chat.message`

An agent's complete response message.

### `chat.chunk`

A streaming chunk of an agent's response (true streaming — chunks arrive as they're generated).

### `manifest_draft`

Soul manifestation generation result.

### `update_success`

Sent after a `SoulUpdate` is written to disk.

### `bulk_success`

Sent after a `BulkManifest` completes.

### `swarm_insight_history`

Sent in response to `SwarmInsightHistoryRequest`.

### `learning.insight`

Proactive cognitive insight pushed in real-time.

### `heartbeat`

Agent heartbeat signal. Contains `agent_id`, `status`, `delta_score`.

### `system.evolution.*`

Evolution system events:
- `system.evolution.mutation_proposed` — New mutation proposed
- `system.evolution.mutation_applied` — Mutation approved and applied
- `system.evolution.mutation_rejected` — Mutation rejected

### `system.config.updated`

Configuration value changed via ConfigSet.

### `system.config.reset`

Configuration reset to defaults.

### `system.vault.*`

Glass House vault events:
- `system.vault.sync_complete` — Vault sync finished
- `system.vault.file_changed` — Vault file modified externally

### `EVOLUTION_SCORE`

Agent evolution score updated.

### `EVOLUTION_HISTORY`

Full evolution history for an agent.

### `agent.ocen.traits`

OCEAN personality traits updated.

### `debug.log`

Debug log entry from the gateway.

---

## Error Responses

### 401 Unauthorized (v0.3.4)

Returned when API key is missing or invalid:

```json
{
  "error": "Unauthorized",
  "message": "Valid API key required. Provide via 'Authorization: Bearer <key>' or 'X-API-Key: <key>' header."
}
```

### 403 Forbidden (v0.3.4)

Returned when attempting to modify immutable config fields:

```json
{
  "status": "error",
  "message": "Field 'server.dashboard_api_key' is immutable at runtime. Update the config file and restart."
}
```

### 400 Bad Request

Returned for invalid input (malformed agent IDs, invalid config section/key names).

---

## Authentication

### Token Format

Session tokens are Ed25519-signed JWTs containing:

- `session_id` — Unique session identifier
- `agent_id` — Associated agent (if applicable)
- `nonce` — Replay prevention nonce
- `expires_at` — Token expiration timestamp

### Key Exchange Flow (OpenRouter)

The soul manifestation engine uses a master key exchange flow:

1. **Read** `OR_MASTER_KEY` from environment
2. **Exchange** via `POST https://openrouter.ai/api/v1/auth/key` with `Authorization: Bearer <master_key>`
3. **Receive** a regular API key in the response envelope
4. **Cache** the regular key process-wide via `OnceCell`
5. **Use** the regular key for completions

The master key is never used directly for completions.

---

*Documentation updated: 2026-05-25. Reflects v0.3.4 codebase.*

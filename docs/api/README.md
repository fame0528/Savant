# API Reference

## WebSocket Protocol

All communication between the dashboard and gateway occurs over a single WebSocket connection at `ws://localhost:3000/ws`.

### Health Endpoints

| Endpoint | Method | Description |
|:---------|:-------|:------------|
| `/live` | GET | Returns "OK" if gateway is running |
| `/ready` | GET | Returns "OK" if gateway is ready |
| `/ws` | WebSocket | Main communication endpoint |
| `/api/agents/:name/image` | GET | Agent avatar image |

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
| :--- | :--- | :--- | :--- |
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

| Field | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `prompt` | string | Yes | Natural language description of the desired soul |
| `name` | string | No | Preferred agent name |

### SoulUpdate

Update an agent's SOUL.md file on disk.

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

| Field | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `agent_id` | string | Yes | Target agent identifier |
| `content` | string | Yes | Complete SOUL.md markdown content |

### BulkManifest

Deploy multiple agents from an expansion plan.

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

### ConfigGet

Retrieve the current gateway configuration from `savant.toml`.

```json
{
  "session_id": "dashboard-session",
  "payload": { "type": "ConfigGet" }
}
```

### ConfigSet

Update a configuration value (saved to `savant.toml`, auto-reloads).

```json
{
  "session_id": "dashboard-session",
  "payload": {
    "type": "ConfigSet",
    "data": {
      "key": "ai.temperature",
      "value": 0.7
    }
  }
}
```

| Field | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `key` | string | Yes | Dotted config path (e.g., `ai.temperature`, `server.port`) |
| `value` | any | Yes | New value (number, string, or boolean) |

### ModelsList

Get available AI providers and their parameter descriptors.

```json
{
  "session_id": "dashboard-session",
  "payload": { "type": "ModelsList" }
}
```

### HistoryRequest

Retrieve persisted message history for a communication lane.

```json
{
  "session_id": "dashboard-session",
  "payload": {
    "type": "HistoryRequest",
    "data": {
      "lane_id": "global",
      "limit": 100
    }
  }
}
```

| Field | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `lane_id` | string | Yes | Lane identifier (agent ID or `"global"`) |
| `limit` | number | No | Maximum messages to retrieve (default: 100) |

### SwarmInsightHistoryRequest

Retrieve the swarm's cognitive insight history.

```json
{
  "session_id": "dashboard-session",
  "payload": {
    "type": "SwarmInsightHistoryRequest",
    "data": { "limit": 50 }
  }
}
```

### InitialSync

Sent on WebSocket open to request full state synchronization.

```json
{
  "session_id": "dashboard-session",
  "payload": { "type": "InitialSync" }
}
```

---

## Gateway → Client (Event Types)

### `agents.discovered`

Sent when the agent registry updates.

```json
{
  "event_type": "agents.discovered",
  "payload": {
    "agents": [
      { "id": "agent-1", "name": "Prometheus", "status": "active", "role": "strategist" }
    ]
  }
}
```

### `history`

Sent in response to `HistoryRequest`.

```json
{
  "event_type": "history",
  "payload": {
    "lane_id": "global",
    "history": [
      { "role": "user", "content": "Hello", "sender": "dashboard" },
      { "role": "assistant", "content": "Greetings!", "agent_id": "prometheus" }
    ]
  }
}
```

### `chat.message`

An agent's complete response message.

```json
{
  "event_type": "chat.message",
  "payload": {
    "role": "assistant",
    "content": "Response text...",
    "agent_id": "prometheus",
    "recipient": "global",
    "is_telemetry": false
  }
}
```

### `chat.chunk`

A streaming chunk of an agent's response (for real-time display).

```json
{
  "event_type": "chat.chunk",
  "payload": {
    "agent_id": "prometheus",
    "content": "chunk text",
    "is_telemetry": false
  }
}
```

### `manifest_draft`

Soul manifestation generation result. Sent after a `SoulManifest` request completes.

```json
{
  "event_type": "manifest_draft",
  "payload": {
    "prompt": "A business strategist...",
    "name": "Prometheus",
    "content": "# SOUL.md\n\n## 1. Identity Core...",
    "status": "complete",
    "metrics": {
      "lines": 320,
      "sections": 18,
      "depth_score": 0.92
    }
  }
}
```

On error:

```json
{
  "event_type": "manifest_draft",
  "payload": {
    "status": "error",
    "error": "OpenRouter API error: 429"
  }
}
```

### `update_success`

Sent after a `SoulUpdate` is written to disk.

```json
{
  "event_type": "update_success",
  "payload": {}
}
```

### `bulk_success`

Sent after a `BulkManifest` completes.

```json
{
  "event_type": "bulk_success",
  "payload": { "count": 3 }
}
```

### `swarm_insight_history`

Sent in response to `SwarmInsightHistoryRequest`.

```json
{
  "event_type": "swarm_insight_history",
  "payload": {
    "history": [
      { "agent_id": "prometheus", "content": "...", "category": "insight", "timestamp": "..." }
    ]
  }
}
```

### `learning.insight`

Proactive cognitive insight pushed in real-time.

```json
{
  "event_type": "learning.insight",
  "payload": {
    "agent_id": "savant",
    "content": "Strategic observation...",
    "category": "synthesis",
    "timestamp": "2026-03-16T12:00:00Z"
  }
}
```

### `heartbeat`

Agent heartbeat signal. Processed silently by the dashboard.

```json
{
  "event_type": "heartbeat",
  "payload": {
    "agent_id": "prometheus",
    "status": "alive"
  }
}
```

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
3. **Receive** a regular API key in the response envelope: `{ "key": { "key": "sk-or-v1-..." } }`
4. **Cache** the regular key process-wide via `OnceCell`
5. **Use** the regular key for `POST https://openrouter.ai/api/v1/chat/completions`

The master key is never used directly for completions. This ensures:
- Master keys cannot be leaked through API responses
- Regular keys can be revoked independently
- Rate limits are properly tracked per-use

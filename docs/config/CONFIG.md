# Savant Configuration Guide

> **Last Updated:** 2026-05-25 (v0.3.2)
> **Reference:** `config/savant.toml`

Configuration uses [Figment](https://docs.rs/figment) for layered loading:
1. **Defaults** (from `Default` impl)
2. **TOML file** (`config/savant.toml` or `~/.savant/savant.toml`)
3. **Environment variables** with `SAVANT_` prefix (e.g., `SAVANT_SERVER_HOST`)

All fields use `#[serde(default)]` — partial config files are valid. Missing sections use defaults.

---

## [ai] — AI Engine

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `provider` | `string` | Model provider (`openrouter`, `ollama`, `openai`, `anthropic`, `google`, `mistral`, `groq`, `deepseek`, `cohere`, `together`, `azure`, `xai`, `fireworks`, `novita`, `lmstudio`, `local`, `perplexity`) | `openrouter` |
| `model` | `string` | Model identifier | `stepfun/step-3.5-flash:free` |
| `temperature` | `float` | Creativity setting (0.0–1.0) | `0.4` |
| `max_tokens` | `integer` | Completion token limit | `256000` |
| `system_prompt_path` | `path` | Path to modular markdown prompt | `config/prompts/default.md` |
| `manifestation_model` | `string` | Model for SOUL.md generation | `stepfun/step-3.5-flash:free` |

---

## [server] — Gateway & Security

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `port` | `integer` | Gateway listen port | `3000` |
| `host` | `string` | Listen address | `127.0.0.1` |
| `dashboard_api_key` | `string` | API key for REST + WS auth. Empty = no auth (dev mode). | `""` |
| `signing_key` | `string` | Ed25519 signing key for session tokens | auto-generated |
| `allowed_origins` | `string[]` | CORS allowed origins | `["*"]` |

**Note:** Security-critical fields (`dashboard_api_key`, `host`, `port`, `signing_key`) are immutable at runtime. Changing them requires editing `savant.toml` and restarting.

---

## [swarm] — Hivemind Settings

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `heartbeat_interval` | `integer` | Agent health check interval in seconds | `60` |
| `max_agents` | `integer` | Maximum concurrent agents | `128` |

---

## [resource_governor] — CPU/Memory-Aware Agent Spawning

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `enabled` | `boolean` | Enable resource governor | `true` |
| `monitor_interval_secs` | `integer` | How often to poll system resources | `5` |
| `memory_medium_pct` | `float` | Memory pressure: medium threshold (%) | `60.0` |
| `memory_high_pct` | `float` | Memory pressure: high threshold (%) | `80.0` |
| `memory_critical_pct` | `float` | Memory pressure: critical threshold (%) | `92.0` |
| `cpu_medium_pct` | `float` | CPU pressure: medium threshold (%) | `70.0` |
| `cpu_high_pct` | `float` | CPU pressure: high threshold (%) | `85.0` |
| `cpu_critical_pct` | `float` | CPU pressure: critical threshold (%) | `95.0` |
| `max_agents_low` | `integer` | Max concurrent agents at Low pressure | `16` |
| `max_agents_medium` | `integer` | Max concurrent agents at Medium pressure | `8` |
| `max_agents_high` | `integer` | Max concurrent agents at High pressure | `4` |
| `max_agents_critical` | `integer` | Max concurrent agents at Critical pressure | `1` |
| `max_deferral_retries` | `integer` | Max retries before dropping deferred agent (60 × 5s = 5 min) | `60` |

---

## [wasm] — Security Sandboxing

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `fuel_limit` | `integer` | Instructions per execution | `50000000` |
| `memory_limit_mb` | `integer` | MB per instance | `512` |
| `max_instances` | `integer` | Max parallel skills | `120` |

---

## [memory] — Memory Engine

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `consolidation_interval_secs` | `integer` | How often to run memory consolidation | `300` |
| `max_entries` | `integer` | Maximum memory entries before pruning | `100000` |

---

## [security] — Security Configuration

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `enable_blocklist_sync` | `boolean` | Sync global threat blocklist | `true` |
| `blocklist_url` | `string` | Threat intelligence feed URL | `""` |

---

## [evolution] — Personality Evolution

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `enabled` | `boolean` | Enable personality evolution | `true` |
| `cooldown_secs` | `integer` | Cooldown between mutations to same section | `300` |
| `max_proposals_per_hour` | `integer` | Maximum mutation proposals per hour | `5` |
| `quiet_hours_start` | `integer` | Quiet hours start (UTC hour, 0-23). Consciousness daemon pauses. | `3` (3AM UTC = 11PM EDT) |
| `quiet_hours_end` | `integer` | Quiet hours end (UTC hour, 0-23). Consciousness daemon resumes. | `11` (11AM UTC = 7AM EDT) |

---

## [obsidian] — Glass House (Obsidian Sync)

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `enabled` | `boolean` | Enable Obsidian vault sync | `true` |
| `vault_path` | `string` | Path to Obsidian vault | `""` |
| `sync_interval_secs` | `integer` | Outbox poll interval | `300` |
| `max_files` | `integer` | File ceiling before forced cold storage | `15000` |
| `cold_storage_days` | `integer` | Episodic age threshold for archival | `90` |
| `project_procedures` | `boolean` | Project procedural memory to vault | `true` |
| `project_lessons` | `boolean` | Project lessons + insights to vault | `true` |
| `project_graphs` | `boolean` | Project MAGMA graphs to vault | `true` |
| `project_audit_trail` | `boolean` | Project audit log (noisy, off by default) | `false` |

---

## [browser] — Browser Automation

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `vision_model` | `string` | Model for image understanding | `gemma4` |
| `embedding_model` | `string` | Model for text embeddings | `gemma4` |

---

## [telemetry] — Observability

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `log_level` | `string` | Log level (`trace`, `debug`, `info`, `warn`, `error`) | `info` |
| `log_color` | `boolean` | ANSI color support | `true` |
| `enable_tracing` | `boolean` | OTLP/gRPC tracing | `false` |

---

## [mcp] — Model Context Protocol

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `servers` | `string[]` | MCP server names to connect | `[]` |

---

## [channels] — Channel Adapters

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `discord.token` | `string` | Discord bot token | `""` |
| `telegram.token` | `string` | Telegram bot token | `""` |
| `slack.token` | `string` | Slack bot token | `""` |

---

## [privacy] — Privacy Router

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `local_only` | `boolean` | Force all requests to local providers | `false` |
| `allowed_cloud_providers` | `string[]` | Cloud providers allowed for non-sensitive content | `["openrouter"]` |

---

## [proactive] — Proactive Behavior

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `enabled` | `boolean` | Enable proactive agent behavior | `true` |
| `context_gatherer_enabled` | `boolean` | Enable proactive context gathering | `true` |

---

## [trajectory] — Trajectory Recording

| Key | Type | Description | Default |
|:----|:-----|:------------|:--------|
| `enabled` | `boolean` | Enable trajectory recording | `true` |
| `output_dir` | `string` | Directory for trajectory JSONL files | `./data/trajectories` |
| `compress_tool_results` | `boolean` | Apply TOON compression to tool results | `true` |
| `max_file_size_mb` | `integer` | Max file size before rotation | `100` |

---

## Example Configuration

```toml
[ai]
provider = "openrouter"
model = "mimo-v2.5-pro"
temperature = 0.4
max_tokens = 1000000

[server]
port = 3000
host = "127.0.0.1"
dashboard_api_key = ""

[resource_governor]
enabled = true
monitor_interval_secs = 5
memory_high_pct = 80.0
max_agents_high = 4

[memory]
consolidation_interval_secs = 300

[evolution]
enabled = true
cooldown_secs = 300

[obsidian]
enabled = true
vault_path = "./workspaces/substrate/vault"

[telemetry]
log_level = "info"
```

---

## Environment Variables

All config fields can be overridden via environment variables with `SAVANT_` prefix:

```bash
SAVANT_AI_PROVIDER=ollama
SAVANT_AI_MODEL=gemma4:e4b
SAVANT_SERVER_HOST=127.0.0.1
SAVANT_SERVER_PORT=3000
```

**Secrets** (API keys, tokens) belong in `.env`, never in `savant.toml`:

```env
OR_MASTER_KEY=sk-or-v1-...
SAVANT_DEV_MODE=1
```

---

*Documentation updated: 2026-05-25. Reflects v0.3.2 codebase.*

# Savant Configuration Guide (AAA)

This document provides a comprehensive overview of all configuration options available in `config/savant.toml`.

## [ai] - AI Engine
| Key | Type | Description | Default |
| :--- | :--- | :--- | :--- |
| `provider` | `string` | Model provider (openrouter, ollama, openai) | `openrouter` |
| `model` | `string` | Model identifier | `stepfun/step-3.5-flash:free` |
| `temperature` | `float` | Creativity setting (0.0 - 1.0) | `0.4` |
| `max_tokens` | `integer` | Completion token limit | `256000` |
| `system_prompt_path` | `path` | Path to modular markdown prompt | `config/prompts/default.md` |

## [swarm] - Hive Mind Settings
| Key | Type | Description | Default |
| :--- | :--- | :--- | :--- |
| `heartbeat_interval` | `integer` | Agent health check interval in seconds | `60` |

## [server] - Gateway & Security
| Key | Type | Description | Default |
| :--- | :--- | :--- | :--- |
| `port` | `integer` | Gateway listen port | `3000` |
| `host` | `string` | Listen address | `0.0.0.0` |
| `dashboard_api_key` | `string` | Secret key for dashboard auth | `savant-dev-key` |
| `allowed_origins` | `string[]` | CORS allowed origins | `["*"]` |

## [wasm] - Security Sandboxing
| Key | Type | Description | Default |
| :--- | :--- | :--- | :--- |
| `fuel_limit` | `integer` | Instructions per execution | `50000000` |
| `memory_limit_mb` | `integer` | MB per instance | `512` |
| `max_instances` | `integer` | Max parallel skills | `120` |

## [telemetry] - Observability
| Key | Type | Description | Default |
| :--- | :--- | :--- | :--- |
| `log_level` | `string` | trace/debug/info/warn/error | `info` |
| `log_color` | `boolean` | ANSI color support | `true` |
| `enable_tracing` | `boolean` | OTLP/gRPC tracing | `false` |

---
*Note: Secrets like API keys belong in the root `.env` file.*

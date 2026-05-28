# Savant Security Model

> **Last Updated:** 2026-05-25 (v0.3.4)

## Overview

Savant implements security across five layers:

1. **API Authentication** — REST middleware with constant-time comparison
2. **Transport Security** — WebSocket authentication with Ed25519 signatures
3. **Skill Security** — Mandatory scanning with 10 proactive checks
4. **Execution Security** — Sandboxed skill execution with resource limits
5. **API Key Security** — Master key exchange flow prevents direct key exposure

**Core Principle:** The user is sovereign. No hard blocks — just increasing click friction based on risk level.

---

## API Authentication (v0.3.4)

### REST API Middleware

All non-public REST endpoints require authentication via Tower middleware:

- **Methods:** `Authorization: Bearer <key>` or `X-API-Key: <key>`
- **Constant-time comparison** prevents timing attacks
- **Public endpoints** (no auth): `/health`, `/live`, `/ready`, `/ws`, `/ws/canvas`
- **Development mode:** Empty `dashboard_api_key` = no auth required

### Canvas WebSocket Auth

`/ws/canvas` requires API key validation on WebSocket upgrade. Same constant-time comparison as REST.

### Immutable Security Fields

These fields cannot be changed at runtime via ConfigSet or REST API. Returns 403 Forbidden:

- `server.dashboard_api_key`
- `server.host`
- `server.port`
- `server.signing_key`
- `security.enable_blocklist_sync`

Requires editing `savant.toml` and restarting.

### Input Size Limits

| Input | Limit | Enforcement |
|:------|:------|:------------|
| SoulUpdate content | 100KB | Gateway handler |
| SoulUpdate reasoning | 100KB | Gateway handler |
| BulkManifest agents | 10 max | Gateway handler |
| NLCommand text | 10,000 chars | Gateway handler |
| WebSocket message | 1MB | Axum layer |
| Tool output | 50,000 chars | Stream handler |

### Environment Variable Filtering

Shell commands use `env_clear()` + only pass safe variables:
- `PATH` — required for command resolution
- `HOME` — required for user context
- `LANG` — required for locale
- `TERM` — required for terminal

Prevents credential leakage to child processes.

---

## Security Gate Behavior

| Risk Level | Clicks | Behavior |
|:-----------|:------:|:---------|
| **Clean** | 0 | Auto-proceed, no prompts |
| **Low** | 0 | Proceed with notification |
| **Medium** | 1 | Acknowledge findings |
| **High** | 2 | Double-confirm with full disclosure |
| **Critical** | 3 | Triple-confirm with "I understand risks" |

---

## Mandatory Skill Scanning

### Security Scanner Layers

#### Layer 1: Global Blocklist

Content-hash based blocking synced with threat intelligence feed.

#### Layer 2: Typosquatting Detection

Uses Levenshtein distance to detect skill names that mimic popular skills. Distance threshold: ≤2 characters.

#### Layer 3: Dependency Confusion

Async verification against package registries (npm, PyPI, crates.io). Conservative on network error.

#### Layer 4: Content Pattern Analysis

Regex-based detection of malicious URLs, credential theft, data exfiltration, and dangerous commands.

#### Layer 5: Proactive Behavioral Checks

| Check | Detects | Severity |
|:------|:--------|:---------|
| Clipboard hijacking | `pbpaste`, `pbcopy`, `xclip` | High |
| Persistence injection | `crontab`, `launchctl`, `systemctl enable` | High |
| Lateral movement | Workspace access, soul file manipulation | High |
| Cryptojacking | Mining pools, wasm mining | Critical |
| Reverse shell | `/dev/tcp/`, `nc -e`, `socat` | Critical |
| Keylogger | `GetAsyncKeyState`, `pynput` | Critical |
| Screen capture | `screencapture`, `scrot` | High |
| Time-bomb | Long sleeps (>3000s), date-based conditionals | Medium |
| Typosquatting | Levenshtein distance to known skills | High |
| Dependency confusion | Package install without verification | High |

---

## Secrets Redaction

Log output is automatically scanned and redacted for:
- `sk-...` patterns (API keys)
- `key=...` patterns (key-value secrets)
- `token=...` patterns (auth tokens)
- `bearer ...` patterns (JWT tokens)

---

## Path Traversal Prevention

All file operations use `secure_resolve_path()`:
- Validates absolute paths are under workspace root
- Blocks `..` traversal above workspace root
- Re-roots absolute paths to workspace
- Null byte injection blocked

Config mutations validated via `validate_config_path()`:
- Blocks `..` in config paths
- Blocks null bytes

---

## Authentication

### Ed25519 Session Tokens

All WebSocket connections require a signed session token containing:
- `session_id` — UUIDv4 unique session identifier
- `agent_id` — Optional agent association
- `nonce` — Random nonce for replay prevention
- `expires_at` — ISO 8601 expiration timestamp

### Nonce Replay Prevention

LRU cache of recently seen nonces (10K entries). Each nonce checked against cache before accepting.

---

## Execution Sandboxes

| Sandbox | Isolation | Use Case |
|:--------|:----------|:---------|
| **Docker** | Full container isolation | Untrusted code |
| **Nix** | Deterministic build isolation | Reproducible environments |
| **Native** | Dangerous-character filtering | Trusted local |
| **WASM** | WebAssembly sandbox | Portable, resource-limited |
| **MCP** | Protocol-level isolation | External tool servers |

---

## API Key Security

### Master Key Exchange

OpenRouter master keys are never used directly for completions:
1. Master key authenticates to OpenRouter key exchange endpoint
2. Scoped regular API key returned
3. Regular key cached process-wide via `OnceCell`
4. All completions use the regular key

### Ephemeral Credentials

`CredentialBroker` issues per-task ephemeral tokens with configurable TTL. Tokens auto-expire. No static keys stored.

---

## Threat Model

| Threat | Mitigation |
|:-------|:-----------|
| Token replay | Nonce-based replay prevention with LRU cache |
| API key exposure | Constant-time comparison, master key exchange |
| Code injection | Docker/Nix/native/WASM sandbox isolation |
| Path traversal | `secure_resolve_path()`, null byte blocking |
| Credential leak | Env var filtering, secrets redaction in logs |
| Resource exhaustion | Docker resource limits, execution timeouts |
| Config tampering | Immutable security fields, input validation |
| Tool output flooding | 50K char cap on tool output |
| Memory recall dedup | System prompt only, not conversation history |
| Tool panic | tokio::spawn isolation, panic caught via JoinHandle |
| Unbounded context | Pre-send token estimation vs context_window |

---

*Documentation updated: 2026-05-25. Reflects v0.3.4 codebase.*

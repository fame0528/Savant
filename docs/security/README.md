# Savant Security Model

## Overview

Savant implements a **mandatory security gate** for all skills. Every skill must pass through the security scanner before execution. There are no bypasses, no "trusted" shortcuts, and no exceptions.

**Core Principle:** The user is sovereign. No hard blocks — just increasing click friction based on risk level.

---

## Defense in Depth

Savant implements security across four layers:

1. **Transport Security** — WebSocket authentication with Ed25519 signatures
2. **Skill Security** — Mandatory scanning with 10 proactive checks
3. **Execution Security** — Sandboxed skill execution with resource limits
4. **API Key Security** — Master key exchange flow prevents direct key exposure

---

## Security Gate Behavior

| Risk Level | Clicks | Icon | Color | Behavior |
|:-----------|:------:|:----:|:------|:---------|
| **Clean** | 0 | ✅ | Green | Auto-proceed, no prompts |
| **Low** | 0 | ℹ️ | Blue | Proceed with notification |
| **Medium** | 1 | ⚠️ | Yellow | Acknowledge findings |
| **High** | 2 | 🔶 | Orange | Double-confirm with full disclosure |
| **Critical** | 3 | 🔴 | Red | Triple-confirm with "I understand risks" |

---

## Mandatory Skill Scanning

### Security Scanner Layers

#### Layer 1: Global Blocklist

Content-hash based blocking synced with threat intelligence feed.

```rust
use savant_skills::security::{is_blocked_hash, is_blocked_name};

// Check if a hash is blocked
if is_blocked_hash(&content_hash) {
    return RiskLevel::Critical;
}

// Check if a skill name is blocked
if is_blocked_name(&skill_name) {
    return RiskLevel::Critical;
}
```

#### Layer 2: Typosquatting Detection

Uses Levenshtein distance to detect skill names that mimic popular skills:

- Known skills: google, gmail, calendar, drive, notion, slack, github, jira, linear, figma, aws, docker
- Distance threshold: ≤2 characters

#### Layer 3: Dependency Confusion

Async verification against package registries:

- **Suspicious names:** core, helper, runtime, sdk, utils, common, lib, toolkit, config, base, foundation, shared, internal, private
- **Registry checks:** npm (registry.npmjs.org), PyPI (pypi.org), crates.io
- **Conservative on network error:** Assumes package exists to prevent false positives

#### Layer 4: Content Pattern Analysis

Regex-based detection of:

| Category | Patterns |
|:---------|:---------|
| Malicious URLs | Shortened URLs, pastebin, executables, direct IP |
| Credential theft | SSH keys, AWS credentials, GPG, keychain |
| Data exfiltration | Webhooks (Discord, Slack), base64 of sensitive files |
| Dangerous commands | sudo, chmod 777, crontab, pipe-to-bash, rm -rf / |

#### Layer 5: Proactive Behavioral Checks

| Check | Detects | Severity |
|:------|:--------|:---------|
| Clipboard hijacking | `pbpaste`, `pbcopy`, `xclip`, electron clipboard | High |
| Persistence injection | `crontab`, `launchctl`, `systemctl enable` | High |
| Lateral movement | Workspace access, soul file manipulation | High |
| Cryptojacking | Mining pools, wasm mining, hashrate monitoring | Critical |
| Reverse shell | `/dev/tcp/`, `nc -e`, `socat`, bind shells | Critical |
| Keylogger | `GetAsyncKeyState`, `pynput`, keyboard hooks | Critical |
| Screen capture | `screencapture`, `scrot`, selenium screenshots | High |
| Time-bomb | Long sleeps (>3000s), date-based conditionals | Medium |
| Typosquatting | Levenshtein distance to known skills | High |
| Dependency confusion | Package install instructions without verification | High |

---

## Threat Intelligence

### Blocklist Sync

```rust
use savant_skills::security::sync_threat_intelligence;

let result = sync_threat_intelligence().await;
if result.success {
    println!("Synced: {} hashes, {} names, {} domains",
        result.hashes_synced, result.names_synced, result.domains_synced);
}
```

### Monitoring

```rust
use savant_skills::security::get_blocklist_stats;

let (hashes, names, domains) = get_blocklist_stats();
```

---

## Authentication

### Ed25519 Session Tokens

All WebSocket connections require a signed session token. Tokens contain:

- `session_id` — UUIDv4 unique session identifier
- `agent_id` — Optional agent association
- `nonce` — Random nonce for replay prevention
- `expires_at` — ISO 8601 expiration timestamp

**Verification flow:**

1. Client sends token in the `Authorization` header during WebSocket upgrade
2. Gateway verifies the Ed25519 signature against the configured public key
3. Nonce is checked against the replay cache (LRU with 10K entries)
4. Expiration timestamp is validated
5. Session is established with the decoded claims

### Nonce Replay Prevention

The gateway maintains an LRU cache of recently seen nonces. Each session token includes a unique nonce that:

- Must not have been seen before in the current or adjacent time window
- Is stored in an `lru::LruCache` with capacity 10,000
- Is evicted automatically when the cache is full

---

## Execution Sandboxes

### Docker Sandbox

Isolates skill execution in Docker containers with:

- **Network isolation** — `--network none` by default
- **Resource limits** — CPU and memory caps
- **Read-only rootfs** — Writable only in designated temp directories
- **User namespace** — Non-root execution inside the container
- **Timeout enforcement** — 30-second hard limit with `SIGKILL` on expiry

### Native Sandbox

For trusted local execution, the native sandbox filters dangerous patterns:

**Blocked characters:**
```
| & ; > < ` ( ) { } [ ] $ \ ! ' " \n \r
```

**Execution model:**
- Direct process execution (no shell wrapper)
- Arguments passed directly to the process
- PowerShell execution uses `-ExecutionPolicy Restricted`

### Nix Sandbox

Deterministic skill execution via Nix flakes:

- **Flake reference validation** — Checks for path traversal and dangerous characters
- **Path existence verification** — Ensures local paths exist before evaluation
- **Size limits** — Payload capped at 10KB
- **Allowlisted prefixes** — Only `flake:`, `path:`, `github:`, `gitlab:`, `sourcehut:`, and `.` are accepted

### WASM Sandbox

WebAssembly execution with fuel limits:

- **Fuel consumption** — Instruction counting for execution limiting
- **Epoch interruption** — Timeout enforcement
- **Memory limits** — 64MB maximum
- **Output limits** — 1MB stdout/stderr capture

---

## API Key Security

### Master Key Exchange

OpenRouter master keys (`OR_MASTER_KEY`) are never used directly for API completions. The exchange flow:

1. Master key authenticates to `POST https://openrouter.ai/api/v1/auth/key`
2. A scoped regular API key is returned in the response
3. The regular key is cached process-wide via `tokio::sync::OnceCell`
4. All subsequent API calls use the regular key

**Benefits:**
- Master key never appears in API request logs
- Regular keys can be revoked without rotating the master key
- Rate limits are tracked per-regular-key, not per-master-key

---

## Threat Model

| Threat | Mitigation |
|:-------|:-----------|
| Token replay | Nonce-based replay prevention with LRU cache |
| Code injection | Docker/Nix/native/WASM sandbox isolation |
| Path traversal | Nix flake reference validation, dangerous char filtering |
| Master key exposure | Key exchange flow, never used in completions |
| Resource exhaustion | Docker resource limits, execution timeouts |
| Container escape | User namespace, read-only rootfs, network isolation |
| Unbounded output | Response size validation, chunk limits |
| Malicious skills | Mandatory security scanning with 10 proactive checks |
| Skill typosquatting | Levenshtein distance detection |
| Dependency confusion | Async registry verification |
| Data exfiltration | Webhook detection, base64 pattern analysis |
| Persistence attacks | Proactive persistence injection detection |

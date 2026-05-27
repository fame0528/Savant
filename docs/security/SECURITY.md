# Savant Security Model: CCT & Sandboxing

> **Last Updated:** 2026-05-25 (v0.3.2)

Savant introduces a bulletproof security model designed to prevent data exfiltration and malicious tool execution in high-scale swarms.

## 1. Crypto-Cap Tokens (CCT)

Every action performed by an agent is authorized via an **Ed25519-signed Capability Token**. Unlike legacy frameworks that rely on environment variables, Savant tokens are:

- **Assignee-Locked**: Only the specific agent hash it was issued to can use it.
- **Resource-Scoped**: Restricted to specific directory paths or network CIDRs.
- **Time-Bound**: Expiration is calculated in milliseconds for high-frequency rotation.
- **Atomic**: Multiple capabilities can be bundled into a single zero-copy payload.

### Token Flow

1. **Gateway** mints a token for an agent during task delegation.
2. **Agent** passes the token to the **Wassette Sandbox** via IPC.
3. **Sandbox** verifies the signature and scope *mathematically* before granting tool access.

## 2. Wassette Sandbox (OCI WASM)

Agents do not run as native processes. They execute as **OCI-compliant WebAssembly Components**.

- **Memory Isolation**: 4GB linear memory per agent, zero shared heap.
- **Punctured Capability Model**: No FS/Net access unless explicitly punctured via CCT.
- **Deterministic Execution**: Prevents side-channel attacks by normalizing instruction timing.

## 3. REST API Security (v0.3.2)

### Authentication Middleware

All non-public REST endpoints require API key authentication:

- **Methods:** `Authorization: Bearer <key>` or `X-API-Key: <key>`
- **Constant-time comparison** prevents timing attacks
- **Public endpoints:** `/health`, `/live`, `/ready`, `/ws`, `/ws/canvas`

### Immutable Config Fields

Security-critical fields blocked from runtime mutation (returns 403):
- `server.dashboard_api_key`, `server.host`, `server.port`, `server.signing_key`, `security.enable_blocklist_sync`

### Input Validation

| Input | Limit |
|:------|:------|
| SoulUpdate content/reasoning | 100KB |
| BulkManifest agents | 10 max |
| NLCommand text | 10,000 chars |
| WebSocket message | 1MB |
| Tool output | 50,000 chars |

## 4. Tool Execution Security (v0.3.2)

### Panic Isolation

Side-effect tool execution wrapped in `tokio::spawn` to isolate panics. Panics caught via `JoinHandle`, error returned instead of crashing agent.

### Security Scanner

Mandatory on every tool execution (no optional bypass). Scans for:
- Command injection patterns
- Dangerous file operations
- Network exfiltration attempts
- Credential theft patterns

### Environment Filtering

Shell commands use `env_clear()` + only pass `PATH`, `HOME`, `LANG`, `TERM`. Prevents credential leakage to child processes.

## 5. Threat Mitigation Matrix

| Threat | Mitigation |
|:-------|:-----------|
| Token Theft | Identity hash verification (assignee_hash mismatch) |
| API Key Exposure | Constant-time comparison, master key exchange |
| Infinite Loops | TTL/Depth-limit enforcement in IPC header |
| Credential Leak | In-memory key vault (never written to disk), env var filtering |
| Resource Exhaustion | WebAssembly memory/gas limits, tool timeouts |
| Tool Panic | tokio::spawn isolation, JoinHandle error recovery |
| Config Tampering | Immutable security fields, input validation |
| Path Traversal | secure_resolve_path(), null byte blocking |
| Log Leakage | Secrets redaction (sk-, key, token, bearer patterns) |

## 6. Security Audit

All cryptographic operations use `ed25519-dalek` with formal verification. Zero-copy IPC paths are audited for pointer-overflow and UAF via `kani`.

---

*Documentation updated: 2026-05-25. Reflects v0.3.2 codebase.*

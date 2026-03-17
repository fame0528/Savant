# Tomorrow's Session Plan

**Date:** 2026-03-18  
**Purpose:** Areas requiring deeper investigation and work  
**Priority Order:** Top to bottom

**Current State:** 107/121 audit issues fixed. All 157 tests pass. Clean compilation.

---

## 1. Docker Sandbox - Deep Verification

**Status:** Untested since wasmtime upgrade  
**Risk:** High — primary untrusted code execution path

### What to investigate:

- [ ] Verify Docker API works with current bollard version (0.16)
- [ ] Test container creation, execution, and cleanup lifecycle
- [ ] Check resource limits (CPU, memory) are actually enforced
- [ ] Verify network isolation (`--network none`) works
- [ ] Test timeout enforcement (30s hard limit with SIGKILL)
- [ ] Review read-only rootfs configuration
- [ ] Test Windows named pipes for Docker Desktop integration
- [ ] Verify the skill executor properly streams stdout/stderr

### Files to review:
- `crates/skills/src/docker.rs` — Main Docker executor
- `crates/skills/src/sandbox.rs` — Sandbox trait/dispatch

### Potential issues:
- Docker socket permissions on different platforms
- Container cleanup if process is killed mid-execution
- Image pull timeouts for large images
- Volume mount security (path traversal from container)

---

## 2. Nix Sandbox - Windows Compatibility

**Status:** Tests disabled with `#[cfg(not(windows))]`, path canonicalization added  
**Risk:** Medium — Nix primarily targets Linux

### What to investigate:

- [ ] Determine if Nix integration is viable on Windows (WSL2?)
- [ ] Review flake reference validation logic
- [ ] Test the allowlisted prefixes: `flake:`, `path:`, `github:`, `gitlab:`, `sourcehut:`
- [ ] Verify canonicalized path resolution works
- [ ] Check 10KB payload limit enforcement
- [ ] Document Windows limitations if Nix can't run natively

### Files to review:
- `crates/skills/src/nix.rs` — Nix executor and validation (canonicalize added)

### Decision needed:
- Should we add a `#[cfg(windows)]` stub that returns a clear "not supported" error?
- Or should we support Nix via WSL2 detection?

---

## 3. MCP Integration - End-to-End Testing

**Status:** Auth + rate limiting + circuit breaker implemented  
**Risk:** Medium — external tool server integration

### What to investigate:

- [ ] Test auth token flow: initialize → authenticate → tools/list → tools/call
- [ ] Test rate limiting under load (100 req/min per connection)
- [ ] Verify circuit breaker state transitions
- [ ] Test tool discovery and registration
- [ ] Check error handling for malformed tool responses
- [ ] Verify `auth_tokens` HashMap from config

### Files to review:
- `crates/mcp/src/server.rs` — Auth + rate limiting
- `crates/mcp/src/client.rs` — MCP client
- `crates/mcp/src/circuit.rs` — Full circuit breaker (5 unit tests)

---

## 4. Memory System - Integration & Stress Testing

**Status:** 20+ tests pass, atomic writes + auto-persist + error propagation fixed  
**Risk:** Medium — data corruption under load

### What to investigate:

- [ ] Test hybrid storage (Fjall + vectors) end-to-end
- [ ] Verify context consolidation correctly summarizes messages
- [ ] Test vector persistence with atomic write (temp + rename)
- [ ] Check `load_from_path` properly restores from disk
- [ ] Test concurrent access patterns (100 writers same session)
- [ ] Verify RAII cleanup on vector engine drop
- [ ] Stress test: 10,000 message write + consolidation cycle

### Files to review:
- `crates/memory/src/engine.rs` — MemoryEngine (delete cascade fixed)
- `crates/memory/src/async_backend.rs` — Query filtering implemented
- `crates/memory/src/vector_engine.rs` — Atomic writes, Drop impl
- `crates/memory/src/lsm_engine.rs` — atomic_compact now deletes old

---

## 5. Gateway Security - Penetration Testing

**Status:** Path traversal fixed, signing key fixed, auth errors sanitized  
**Risk:** High — entry point for all external connections

### What to investigate:

- [ ] Test Ed25519 token verification with malformed tokens
- [ ] Verify nonce replay prevention with LRU cache
- [ ] Test expired token rejection
- [ ] Check WebSocket origin validation
- [ ] Review CORS configuration
- [ ] Test rate limiting
- [ ] Test directive injection protection

### Attack scenarios:
- Token replay with captured valid token
- Cross-site WebSocket hijacking
- Path traversal in skill installation (blocked by `validate_skill_name`)
- DoS via large skill packages
- SSRF via threat intel feed (redirects disabled)

### Files to review:
- `crates/gateway/src/handlers/skills.rs` — Path validation added
- `crates/gateway/src/handlers/pairing.rs` — OsRng signing key
- `crates/gateway/src/lanes.rs` — Directive sanitization

---

## 6. ECHO Protocol - Speculative ReAct Verification

**Status:** Circuit breaker fixed (Mutex + CAS), env filtering fixed  
**Risk:** Medium — incorrect behavior could cause agent errors

### What to investigate:

- [ ] Test circuit breaker CAS transitions under concurrent load
- [ ] Test overlapping tool execution with cognitive planning
- [ ] Verify sub-millisecond context swapping works
- [ ] Test speculative execution rollback on failure
- [ ] Check for race conditions in parallel tool dispatch

### Files to review:
- `crates/echo/src/circuit_breaker.rs` — Mutex + CAS transitions
- `crates/echo/src/compiler.rs` — Env filtering (AWS key fix)

---

## 7. Dashboard - UI/UX Review

**Status:** WebSocket URL fixed (port 3000)  
**Risk:** Low — frontend only

### What to investigate:

- [ ] Test the multi-click approval UI for skill installation
- [ ] Verify security scan results display correctly
- [ ] Check WebSocket reconnection handling
- [ ] Test the skill management interface
- [ ] Verify the insight panel shows cognitive data
- [ ] Check responsive design on different screen sizes

### Files to review:
- `dashboard/src/app/page.tsx` — WebSocket URL fixed to port 3000

---

## 8. Threat Intelligence Feed

**Status:** SSRF protection added (redirects disabled)  
**Risk:** Medium — security scanning effectiveness depends on fresh data

### What to investigate:

- [ ] Determine the actual endpoint URL for threat intel
- [ ] Test `sync_threat_intelligence()` with a mock server
- [ ] Add periodic sync (cron/scheduler)
- [ ] Implement webhook push for real-time updates
- [ ] Decide: Who hosts the feed? What's the update frequency?

---

## 9. Cross-Platform Testing

**Status:** Developed primarily on Windows  
**Risk:** Medium — deployment targets may differ

### What to test:

- [ ] Run full test suite on Linux (Ubuntu 22.04 LTS)
- [ ] Run full test suite on macOS (Apple Silicon)
- [ ] Verify Docker integration on Linux (native vs Desktop)
- [ ] Test WASM execution on all platforms
- [ ] Check file path handling across platforms
- [ ] Verify signal handling (SIGKILL, SIGTERM)

---

## 10. Performance Profiling

**Status:** No profiling done yet  
**Risk:** Low — important for production scale

### What to profile:

- [ ] Gateway WebSocket throughput
- [ ] Skill execution latency (Docker vs Native vs WASM)
- [ ] Memory engine with 10K+ entries
- [ ] Vector search latency with 100K embeddings
- [ ] CPU usage during cognitive synthesis
- [ ] Memory usage patterns under load

### Tools:
- `cargo flamegraph` for CPU profiling
- `cargo bench` for microbenchmarks
- Custom tracing spans for distributed profiling

---

## 11. Documentation Gaps

**Status:** Core docs updated, examples still needed  
**Risk:** Low — affects adoption

### What to create:

- [ ] Example skill package (complete working skill)
- [ ] Tutorial: Creating and publishing a skill to ClawHub
- [ ] Tutorial: Setting up Docker sandbox
- [ ] Troubleshooting guide
- [ ] Contributing guidelines

---

## 12. CI/CD Pipeline

**Status:** Not implemented  
**Risk:** Medium — no automated quality gates

### What to set up:

- [ ] GitHub Actions: `cargo check`, `cargo test`, `cargo clippy`, `cargo fmt`
- [ ] Automated dependency updates (Dependabot)
- [ ] Security scanning (cargo audit)
- [ ] Formal proofs (cargo kani)

---

## 13. Remaining Audit Issues (14 from 121)

**Status:** 107/121 fixed, 14 remaining (all medium/low priority)

| # | Phase | ID | Issue | Status |
|---|-------|-----|-------|--------|
| 1 | 3 | L-026 | Docker dead code cleanup | PENDING |
| 2 | 4 | H-023 | `semantic_search` stub returns empty | PENDING |
| 3 | 4 | M-012 | Blocking I/O in async `fs/mod.rs` | PENDING |
| 4 | 4 | M-013 | SQLite connection per file | PENDING |
| 5 | 6 | M-030 | Input filtering for cognitive events | N/A (no such function) |
| 6 | 9 | M-024 | Discord channel cancellation handles | PENDING |
| 7 | 9 | M-025 | WhatsApp child process Drop impl | PENDING |
| 8 | 11 | L-022 | Embedding service blocks async | PENDING |
| 9 | 13 | A-001 | Three Fjall instances architecture | PENDING |
| 10 | 13 | A-003 | Error type proliferation | PENDING |
| 11 | 13 | A-004 | Global mutable state for API keys | PENDING |
| 12 | 14 | L-001 | Blanket clippy suppress | PENDING |
| 13 | 3 | L-006 | `is_blocked` docs misleading | N/A (field removed) |
| 14 | 3 | L-004/L-009 | Custom URL encoder / SSRF method | Already fixed |

---

## Quick Wins (30 min each)

1. **Add `cargo clippy` to dev workflow** — catches subtle bugs
2. **Add `cargo audit` check** — catches vulnerable dependencies
3. **Create a `.env.example`** — documents all required env vars
4. **Add `#[cfg(windows)]` stub to nix.rs** — clear error instead of confusing failure
5. **Add `tracing::instrument` to WAL write paths** — debugging concurrent issues

---

## Platform Notes

### Windows-Specific
- `start.bat` handles smart builds, dashboard install, and health check polling
- WebSocket default: `ws://127.0.0.1:3000/ws`
- Dashboard default: `http://localhost:3000`

### Configuration Reference
```
.env                    → API keys and secrets (OR_MASTER_KEY, SAVANT_DEV_MODE)
config/savant.toml      → All settings (auto-reloads on change)
  [ai]                  → provider, model, temperature, max_tokens
  [server]              → port (3000), host (0.0.0.0), dashboard_api_key
  [system]              → db_path, memory_db_path, substrate_path, agents_path
```

### Database Architecture
```
./data/savant/          → Sovereign substrate storage (chat history, WAL)
./data/memory/          → Agent memory engine (messages, vectors, metadata)
                        ↑ SEPARATE Fjall instances — cannot share same path
```

### Health Endpoints
```
GET /live               → Returns "OK" if gateway is running
GET /ready              → Returns "OK" if gateway is ready
WS  /ws                 → Main dashboard WebSocket
GET /api/agents/:name/image → Agent avatar image
```

### CLI Flags
```
--config <path>         → Load config from custom path
--keygen                → Generate master key pair and print
--skip                  → Skip build in start.bat
--force                 → Force rebuild in start.bat
```

---

## Questions to Answer

1. Is the threat intel feed endpoint live, or do we need to build it?
2. Should Nix support be Linux-only, or do we pursue WSL2 on Windows?
3. What's the deployment target? Docker, bare metal, or Kubernetes?
4. Should we add OpenTelemetry traces for production debugging?
5. Do we need a migration guide from SQLite (old) to Fjall (new)?

---

## End of Day State

```
✅ 107/121 audit issues fixed
✅ 0 compilation warnings, 0 clippy errors
✅ 157 tests passing across all crates
✅ 15 AI providers wired in swarm (full provider support)
✅ MCP server with auth + circuit breaker
✅ Security scanner: recursive walkdir, SHA-256, full directory hash
✅ Path traversal protection on all user inputs
✅ Gateway: persistent OsRng signing key, sanitized errors
✅ Memory: atomic writes, auto-persist on Drop, error propagation
✅ Channels: resource leak prevention, safe UTF-8 handling
✅ CLI: --keygen, --config flags working
✅ Config: auto-reload via file watcher
✅ Documentation: archived, updated, and synced
✅ Pushed to GitHub: origin/main
```

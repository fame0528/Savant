# Tomorrow's Session Plan

**Date:** 2026-03-18  
**Purpose:** Areas requiring deeper investigation and work  
**Priority Order:** Top to bottom

---

## 1. Docker Sandbox - Deep Verification

**Status:** Untested since wasmtime upgrade  
**Risk:** High - this is the primary untrusted code execution path

### What to investigate:

- [ ] Verify Docker API still works with current bollard version (0.16)
- [ ] Test container creation, execution, and cleanup lifecycle
- [ ] Check resource limits (CPU, memory) are actually enforced
- [ ] Verify network isolation (`--network none`) works
- [ ] Test timeout enforcement (30s hard limit with SIGKILL)
- [ ] Review read-only rootfs configuration
- [ ] Test Windows named pipes for Docker Desktop integration
- [ ] Verify the skill executor properly streams stdout/stderr

### Files to review:
- `crates/skills/src/docker.rs` - Main Docker executor
- `crates/skills/src/sandbox.rs` - Sandbox trait/dispatch

### Potential issues:
- Docker socket permissions on different platforms
- Container cleanup if process is killed mid-execution
- Image pull timeouts for large images
- Volume mount security (path traversal from container)

---

## 2. Nix Sandbox - Windows Compatibility

**Status:** Tests disabled with `#[cfg(not(windows))]`  
**Risk:** Medium - Nix primarily targets Linux

### What to investigate:

- [ ] Determine if Nix integration is viable on Windows (WSL2?)
- [ ] Review flake reference validation logic
- [ ] Test the allowlisted prefixes: `flake:`, `path:`, `github:`, `gitlab:`, `sourcehut:`
- [ ] Verify path existence verification works (canonicalize added)
- [ ] Check 10KB payload limit enforcement
- [ ] Document Windows limitations if Nix can't run natively

### Files to review:
- `crates/skills/src/nix.rs` - Nix executor and validation (path canonicalization added)

### Decision needed:
- Should we add a `#[cfg(windows)]` stub that returns a clear "not supported" error?
- Or should we support Nix via WSL2 detection?

---

## 3. MCP Integration - Production Readiness

**Status:** Auth implemented, circuit breaker implemented  
**Risk:** Medium - external tool server integration

### What to investigate:

- [ ] Test the stdio-based server communication
- [ ] Verify tool discovery and registration works
- [ ] Check error handling for malformed tool responses
- [ ] Test the circuit breaker integration
- [ ] Test auth token flow end-to-end
- [ ] Verify rate limiting works correctly

### Files to review:
- `crates/mcp/src/server.rs` - MCP server (auth + rate limit added)
- `crates/mcp/src/client.rs` - MCP client
- `crates/mcp/src/circuit.rs` - Full circuit breaker implemented

---

## 4. Memory System - Integration Testing

**Status:** 20+ tests pass, integration between components needs testing  
**Risk:** Medium - data loss if persistence fails

### What to investigate:

- [ ] Test hybrid storage (Fjall + vectors) end-to-end
- [ ] Verify context consolidation correctly summarizes messages
- [ ] Test vector persistence with atomic write (temp + rename)
- [ ] Check that `load_from_path` properly restores from disk
- [ ] Test concurrent access patterns
- [ ] Verify RAII cleanup on vector engine drop

### Files to review:
- `crates/memory/src/engine.rs` - MemoryEngine orchestrator (error handling fixed)
- `crates/memory/src/async_backend.rs` - Query filtering implemented
- `crates/memory/src/vector_engine.rs` - Atomic writes, Drop impl
- `crates/memory/src/models.rs` - Tool role mapping fixed

---

## 5. ECHO Protocol - Speculative ReAct Verification

**Status:** Implementation exists, circuit breaker fixed  
**Risk:** Medium - incorrect behavior could cause agent errors

### What to investigate:

- [ ] Review the circuit breaker CAS transitions under load
- [ ] Test overlapping tool execution with cognitive planning
- [ ] Verify sub-millisecond context swapping works
- [ ] Test the speculative execution rollback on failure
- [ ] Check for race conditions in parallel tool dispatch

### Files to review:
- `crates/echo/src/circuit_breaker.rs` - Mutex + CAS transitions
- `crates/echo/src/compiler.rs` - Env filtering (AWS key fix)

---

## 6. Gateway Security - Penetration Testing

**Status:** Path traversal fixed, auth error leak fixed, signing key fixed  
**Risk:** High - entry point for all external connections

### What to investigate:

- [ ] Test Ed25519 token verification with malformed tokens
- [ ] Verify nonce replay prevention with LRU cache
- [ ] Test expired token rejection
- [ ] Check WebSocket origin validation
- [ ] Review CORS configuration for strictness
- [ ] Test rate limiting if implemented
- [ ] Verify no information leakage in error messages (fixed)

### Files to review:
- `crates/gateway/src/handlers/skills.rs` - Path validation added
- `crates/gateway/src/handlers/pairing.rs` - Persistent signing key
- `crates/gateway/src/lanes.rs` - Directive injection protection

### Attack scenarios to test:
- Token replay with captured valid token
- Cross-site WebSocket hijacking
- Path traversal in skill installation (blocked by validation)
- DoS via large skill packages

---

## 7. Dashboard - UI/UX Review

**Status:** WebSocket URL fixed (port 3000)  
**Risk:** Low - frontend only, no security impact

### What to investigate:

- [ ] Test the multi-click approval UI for skill installation
- [ ] Verify security scan results display correctly
- [ ] Check WebSocket reconnection handling
- [ ] Test the skill management interface
- [ ] Verify the insight panel shows cognitive data
- [ ] Check responsive design on different screen sizes

### Files to review:
- `dashboard/src/app/page.tsx` - WebSocket URL fixed to port 3000
- `dashboard/src/app/globals.css` - Global styles

---

## 8. Threat Intelligence Feed

**Status:** SSRF protection added (redirects disabled)  
**Risk:** Medium - security scanning effectiveness depends on fresh data

### What to investigate:

- [ ] Determine the actual endpoint URL for threat intel
- [ ] Test `sync_threat_intelligence()` with a mock server
- [ ] Verify the JSON response parsing
- [ ] Check error handling for network failures
- [ ] Add periodic sync (cron/scheduler)
- [ ] Implement webhook push for real-time updates

### Files to review:
- `crates/skills/src/security.rs` - Lines 87-200 (threat intel sync, SSRF fixed)

---

## 9. Cross-Platform Testing

**Status:** Developed primarily on Windows  
**Risk:** Medium - deployment targets may differ

### What to investigate:

- [ ] Run full test suite on Linux (CI/CD?)
- [ ] Run full test suite on macOS
- [ ] Verify Docker integration on Linux (native vs Desktop)
- [ ] Test WASM execution on all platforms
- [ ] Check file path handling across platforms
- [ ] Verify signal handling (SIGKILL, SIGTERM) works correctly

---

## 10. Performance Profiling

**Status:** No profiling done yet  
**Risk:** Low - but important for production scale

### What to investigate:

- [ ] Profile gateway WebSocket throughput
- [ ] Measure skill execution latency (Docker vs Native vs WASM)
- [ ] Benchmark memory engine with 10K+ entries
- [ ] Test vector search latency with 100K embeddings
- [ ] Profile CPU usage during cognitive synthesis
- [ ] Check memory usage patterns under load

---

## 11. Documentation Gaps

**Status:** Core docs updated, examples still needed  
**Risk:** Low - but affects adoption

### What to create:

- [ ] Example skill package (complete working skill)
- [ ] Tutorial: Creating and publishing a skill to ClawHub
- [ ] Tutorial: Setting up Docker sandbox for skill execution
- [ ] Troubleshooting guide for common issues
- [ ] Contributing guidelines

---

## 12. CI/CD Pipeline

**Status:** Not implemented  
**Risk:** Medium - no automated quality gates

### What to set up:

- [ ] GitHub Actions workflow for:
  - `cargo check` on every PR
  - `cargo test --workspace` on every PR
  - `cargo clippy` for lint warnings
  - `cargo fmt --check` for formatting
  - `cargo kani --workspace` for formal proofs (if available)
- [ ] Automated dependency updates (Dependabot)
- [ ] Security scanning of dependencies (cargo audit)

---

## 13. Remaining Code Quality (14 issues from audit)

**Status:** 107/121 audit issues fixed  
**Remaining items from audit:**

1. [ ] Phase 4: `semantic_search` stub returns empty - needs implementation or error
2. [ ] Phase 4: Blocking I/O in async `fs/mod.rs` - wrap in `spawn_blocking`
3. [ ] Phase 4: SQLite connection per file - add connection pooling
4. [ ] Phase 9: Channel cancellation handles (M-024 Discord, M-025 WhatsApp struct)
5. [ ] Phase 11: L-022 - Embedding service blocks async
6. [ ] Phase 13: Three Fjall instances architecture consolidation
7. [ ] Phase 13: Error type proliferation (lossy conversions)
8. [ ] Phase 13: Global mutable state for API keys
9. [ ] Phase 14: L-001 - Clippy disallowed_methods blanket suppress
10. [ ] Phase 14: L-026 - Docker dead code cleanup
11. [ ] Phase 3: L-006 - `is_blocked` docs misleading

---

## Quick Wins (30 min each)

1. **Add `cargo clippy` to CI** - catches subtle bugs
2. **Add `cargo audit` check** - catches vulnerable dependencies
3. **Write a `.env.example`** - documents all required env vars
4. **Add `#[cfg(windows)]` stub to nix.rs** - clear error instead of confusing failure
5. **Add `tracing::instrument` to WAL write paths** - debugging concurrent issues

---

## End of Day State

```
✅ 107/121 audit issues fixed
✅ All crates compile with zero warnings
✅ 57 tests pass across all crates
✅ All 15 AI providers wired in swarm
✅ MCP server with auth + circuit breaker
✅ Security scanner: recursive, SHA-256, full directory hash
✅ Path traversal protection on all skill handlers
✅ Gateway: persistent signing key, auth error sanitization
✅ Memory: atomic writes, auto-persist, error propagation
✅ Channels: resource leak prevention, safe UTF-8 handling
✅ CLI: --keygen, --config flags working
✅ Documentation updated and archived
```

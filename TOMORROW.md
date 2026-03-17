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
- [ ] Verify path existence verification works
- [ ] Check 10KB payload limit enforcement
- [ ] Document Windows limitations if Nix can't run natively

### Files to review:
- `crates/skills/src/nix.rs` - Nix executor and validation

### Decision needed:
- Should we add a `#[cfg(windows)]` stub that returns a clear "not supported" error?
- Or should we support Nix via WSL2 detection?

---

## 3. MCP Integration - Production Readiness

**Status:** Code exists, minimal testing  
**Risk:** Medium - external tool server integration

### What to investigate:

- [ ] Review `crates/mcp/src/server.rs` - MCP server implementation
- [ ] Review `crates/mcp/src/client.rs` - MCP client implementation
- [ ] Test the stdio-based server communication
- [ ] Verify tool discovery and registration works
- [ ] Check error handling for malformed tool responses
- [ ] Test the circuit breaker integration
- [ ] Review authentication if MCP servers require it

### Files to review:
- `crates/mcp/src/server.rs`
- `crates/mcp/src/client.rs`
- `crates/mcp/src/circuit.rs`

---

## 4. Memory System - Integration Testing

**Status:** 13 tests pass, but integration between components untested  
**Risk:** Medium - data loss if persistence fails

### What to investigate:

- [ ] Test hybrid storage (SQLite + Fjall + vectors) end-to-end
- [ ] Verify context consolidation actually summarizes messages correctly
- [ ] Test vector persistence with `persist()` method
- [ ] Check that `load_from_path` properly restores from disk
- [ ] Verify memory entry cleanup/compaction works
- [ ] Test concurrent access patterns
- [ ] Review the async backend wrapper for race conditions

### Files to review:
- `crates/memory/src/engine.rs` - MemoryEngine orchestrator
- `crates/memory/src/async_backend.rs` - Async wrapper with consolidation
- `crates/memory/src/vector_engine.rs` - Vector persistence
- `crates/memory/src/models.rs` - Data models

### Specific tests to add:
- Concurrent read/write to same session
- Consolidation of 1000+ messages
- Vector index rebuild after corruption
- Fjall LSM-tree range query performance

---

## 4a. WAL Stress Testing - "WAL is Law" (Core Law #3)

**Status:** No concurrent write tests exist  
**Risk:** CRITICAL - data corruption under load violates Core Law #3

**"WAL is Law" means:** Write-Ahead Logging is the source of truth. If WAL fails, everything fails. This needs adversarial testing.

### What to investigate:

#### Basic Concurrency
- [ ] 100 concurrent writers to the same session
- [ ] 100 concurrent writers to 100 different sessions
- [ ] Mixed read/write: 50 writers + 50 readers simultaneously
- [ ] Write during WAL checkpoint/compaction
- [ ] Write during database vacuum

#### Edge Cases
- [ ] Write interruption mid-transaction (kill process during write)
- [ ] Write to WAL when disk is nearly full
- [ ] Write after improper shutdown (no WAL checkpoint)
- [ ] Write with extremely large message (>1MB payload)
- [ ] Write with Unicode/emoji heavy content
- [ ] Write with null bytes in content

#### Recovery
- [ ] Verify WAL replay after crash recovers ALL committed transactions
- [ ] Verify WAL replay after crash does NOT replay uncommitted transactions
- [ ] Verify WAL replay produces identical database state as正常 shutdown
- [ ] Test WAL truncation doesn't lose data
- [ ] Test WAL file rotation under sustained writes

#### Performance
- [ ] Measure write throughput at 100, 1000, 10000 messages/second
- [ ] Measure latency percentile (p50, p95, p99, p99.9) under load
- [ ] Verify no write starvation during heavy reads
- [ ] Verify no reader starvation during heavy writes
- [ ] Measure WAL file size growth rate
- [ ] Measure checkpoint duration under load

#### Multi-Process
- [ ] Two processes writing to same database (if supported)
- [ ] WAL sharing between reader and writer processes
- [ ] Lock contention measurement

### Test structure:

```rust
// Pseudocode for stress test
#[tokio::test]
async fn wal_stress_concurrent_writes() {
    let engine = MemoryEngine::new(test_dir);
    let session = "stress-test";
    
    // Spawn 100 concurrent writers
    let mut handles = Vec::new();
    for i in 0..100 {
        let eng = engine.clone();
        handles.push(tokio::spawn(async move {
            for j in 0..100 {
                eng.store_message(session, create_message(i, j)).await?;
            }
            Ok::<(), Error>(())
        }));
    }
    
    // Wait for all writers
    for h in handles {
        h.await.unwrap().unwrap();
    }
    
    // Verify all 10,000 messages present and ordered
    let messages = eng.fetch_session_tail(session, 20000);
    assert_eq!(messages.len(), 10000);
    // Verify ordering
}
```

### Files to review:
- `crates/core/src/db.rs` - WAL configuration and connection
- `crates/memory/src/engine.rs` - Store/fetch operations

### Pass criteria:
- Zero data loss under all test conditions
- Zero corruption after simulated crashes
- Write throughput > 1000 msg/sec sustained
- p99 latency < 10ms under normal load
- WAL recovery produces bit-identical database state

---

## 5. ECHO Protocol - Speculative ReAct Verification

**Status:** Implementation exists, needs stress testing  
**Risk:** Medium - incorrect behavior could cause agent errors

### What to investigate:

- [ ] Review the `DelegationBloomFilter` for cycle detection accuracy
- [ ] Test overlapping tool execution with cognitive planning
- [ ] Verify sub-millisecond context swapping works
- [ ] Test the speculative execution rollback on failure
- [ ] Check for race conditions in parallel tool dispatch

### Files to review:
- `crates/echo/src/` - All ECHO protocol files

---

## 6. Gateway Security - Penetration Testing

**Status:** Auth implemented, needs adversarial testing  
**Risk:** High - entry point for all external connections

### What to investigate:

- [ ] Test Ed25519 token verification with malformed tokens
- [ ] Verify nonce replay prevention with LRU cache
- [ ] Test expired token rejection
- [ ] Check WebSocket origin validation
- [ ] Review CORS configuration for strictness
- [ ] Test rate limiting if implemented
- [ ] Verify no information leakage in error messages

### Files to review:
- `crates/gateway/src/auth/mod.rs` - Authentication
- `crates/gateway/src/handlers/mod.rs` - Request handling
- `crates/gateway/src/handlers/skills.rs` - Skill management handlers

### Attack scenarios to test:
- Token replay with captured valid token
- Cross-site WebSocket hijacking
- SQL injection via skill name/path parameters
- Path traversal in skill installation
- DoS via large skill packages

---

## 7. Dashboard - UI/UX Review

**Status:** Basic functionality works  
**Risk:** Low - frontend only, no security impact

### What to investigate:

- [ ] Test the multi-click approval UI for skill installation
- [ ] Verify security scan results display correctly
- [ ] Check WebSocket reconnection handling
- [ ] Test the skill management interface
- [ ] Verify the insight panel shows cognitive data
- [ ] Check responsive design on different screen sizes

### Files to review:
- `dashboard/src/app/page.tsx` - Main dashboard component
- `dashboard/src/app/globals.css` - Global styles

---

## 8. Threat Intelligence Feed

**Status:** API client exists, no real endpoint  
**Risk:** Medium - security scanning effectiveness depends on fresh data

### What to investigate:

- [ ] Determine the actual endpoint URL for threat intel
- [ ] Test `sync_threat_intelligence()` with a mock server
- [ ] Verify the JSON response parsing
- [ ] Check error handling for network failures
- [ ] Add periodic sync (cron/scheduler)
- [ ] Implement webhook push for real-time updates

### Files to review:
- `crates/skills/src/security.rs` - Lines 87-200 (threat intel sync)

### Decision needed:
- Who hosts the threat intel feed?
- What's the update frequency?
- How do we handle private/threat intelligence sharing?

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

### Platforms to test:
- Windows 10/11 (current dev environment)
- Ubuntu 22.04 LTS
- macOS 14+ (Apple Silicon)

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

### Tools to use:
- `cargo flamegraph` for CPU profiling
- `cargo bench` for microbenchmarks
- Custom tracing spans for distributed profiling

---

## 11. Documentation Gaps

**Status:** Core docs exist, examples missing  
**Risk:** Low - but affects adoption

### What to create:

- [ ] Example skill package (complete working skill)
- [ ] Tutorial: Creating and publishing a skill to ClawHub
- [ ] Tutorial: Setting up Docker sandbox for skill execution
- [ ] API reference for WebSocket protocol
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
  - `cargo kani --workspace` for formal proofs (if available in runner)
- [ ] Automated dependency updates (Dependabot)
- [ ] Release automation (cargo publish or GitHub releases)
- [ ] Security scanning of dependencies (cargo audit)

---

## Quick Wins (30 min each)

These are high-value, low-effort items:

1. **Add `cargo clippy` to dev workflow** - catches subtle bugs
2. **Add `cargo audit` check** - catches vulnerable dependencies
3. **Create a `.env.example`** - documents all required env vars
4. **Add `#[cfg(windows)]` stub to nix.rs** - clear error instead of confusing failure
5. **Write a CLAUDE.md** - documents conventions for AI assistants
6. **Add `tracing::instrument` to WAL write paths** - debugging concurrent issues
7. **Add `cargo deny` for license compliance** - catches licensing issues early

---

## 13. Kani Proofs - CI Integration

**Status:** Proofs exist but only run locally with `--cfg kani`  
**Risk:** Low-Medium - proofs may drift if not enforced

### What exists:

- `crates/security/src/proofs.rs` - Token verification boundary proof
- `crates/memory/src/safety.rs` - Memory safety verification
- `crates/agent/src/orchestration/synthesis.rs` - Synthesis properties

### What to investigate:

- [ ] Verify proofs still pass with current codebase
- [ ] Add Kani to CI/CD pipeline
- [ ] Check proof coverage - are all critical invariants covered?
- [ ] Document which properties are formally verified

### Files to review:
- `crates/security/src/proofs.rs`
- `crates/memory/src/safety.rs`
- `crates/memory/src/lib.rs` (kani feature gate)

---

## 14. CCT Token - Exhaustion & Edge Cases

**Status:** Token minting and verification implemented  
**Risk:** Medium - token handling under stress untested

### What exists:

- `crates/security/src/token.rs` - Token structure
- `crates/security/src/enclave.rs` - SecurityAuthority verification
- `crates/agent/src/swarm.rs:307-333` - Token minting during agent spawn
- `crates/agent/src/react/reactor.rs` - Token verification during tool execution

### What to investigate:

- [ ] Test token expiry during long-running tool execution
- [ ] Test token revocation mid-session
- [ ] Verify no partial state commits when token becomes invalid
- [ ] Test concurrent token verification (no race conditions)
- [ ] Verify error propagation to agent on token failure

### Files to review:
- `crates/security/src/token.rs`
- `crates/security/src/enclave.rs`
- `crates/agent/src/react/reactor.rs`

---

## Questions to Answer Tomorrow

1. Is the threat intel feed endpoint ready, or do we need to build it?
2. Should Nix support be Linux-only, or do we pursue WSL2 on Windows?
3. Do we need a migration guide from the old wasmtime API?
4. What's the deployment target? Docker, bare metal, or Kubernetes?
5. Should we add OpenTelemetry traces for production debugging?

---

## End of Day State

```
✅ All crates compile with zero warnings
✅ 14 savant_skills tests pass
✅ Documentation updated (README, AUDIT, CHANGELOG, security, architecture)
✅ Wasmtime upgraded to 36.0 (matching wassette)
✅ OpenClaw skill system fully wired
✅ Security scanner with 10 proactive checks
✅ Threat intelligence sync implemented
✅ Context consolidation implemented
✅ Vector persistence implemented
```

---

*Sleep well. Tomorrow we go deeper.*

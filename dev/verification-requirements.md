# Verification Task Requirements

**Purpose:** Document exactly what information, infrastructure, and resources are needed to complete the 69 remaining verification tasks from the roadmap.  
**Created:** 2026-03-18  
**Source:** `dev/roadmap/roadmap-fix.md` phases 1-13

All 107 code-level fixes are complete. Remaining tasks are runtime verification, testing, infrastructure setup, and documentation.

---

## Phase 1: Docker Sandbox Verification (12 tasks)

**What's needed:**

| Requirement | Details | Who Provides |
|-------------|---------|--------------|
| Docker Desktop or Docker Engine | Windows: Docker Desktop with WSL2 backend, or Linux: native Docker | Spencer |
| Test container image | A minimal image to test with (e.g., `alpine:latest` or a custom Savant skill image) | Spencer |
| Network access for image pull | Docker Hub or private registry access | Environment |
| 30 minutes of uninterrupted testing | Container lifecycle testing requires wait times | Spencer |

**Tasks and what they require:**
- DS-001: `cargo test` + Docker daemon running to verify bollard connection
- DS-002: Run a container end-to-end: create → start → wait → logs → cleanup
- DS-003: Run container with memory limit, verify enforcement via `docker stats`
- DS-004: Run container with CPU limit, verify via `docker stats`
- DS-005: Run container with `--network none`, verify no outbound connectivity
- DS-006: Run long-running container, verify SIGKILL after 30s timeout
- DS-007: Windows-specific: verify named pipe connection to Docker Desktop
- DS-008: Run container that outputs to stdout/stderr, verify streaming works
- DS-009: Kill container process, verify cleanup removes it
- DS-010: Add `read_only_rootfs: Some(true)` to container config (code change)
- DS-011: Add volume mount security (code change)
- DS-012: Add health check on startup (code change)

**Expected time:** 2-3 hours

---

## Phase 2: Nix Sandbox (5 tasks)

**What's needed:**

| Requirement | Details | Who Provides |
|-------------|---------|--------------|
| Decision: Windows support | WSL2 approach or "Linux only" with clear error | Spencer |
| Linux machine (for testing) | Ubuntu 22.04+ with Nix installed | Spencer or CI |
| Nix flakes enabled | `nix.settings.experimental-features = "flakes"` | Environment |

**Tasks:**
- NX-001: Add `#[cfg(windows)]` stub (code change, 5 minutes)
- NX-002: Test on Linux with Nix installed
- NX-003: Test all 5 flake reference formats
- NX-004: Verify 10KB payload limit
- NX-005: Document decision

**Expected time:** 30 minutes (NX-001) + Linux testing

---

## Phase 3: MCP Integration Testing (8 tasks)

**What's needed:**

| Requirement | Details | Who Provides |
|-------------|---------|--------------|
| Integration test infrastructure | tokio::test with WebSocket client | Code (already available) |
| Mock MCP client | A simple WebSocket client for testing | Needs to be written |

**Tasks:**
- MCP-001 through MCP-006: Write integration tests using tokio::test and a WebSocket client
- MCP-007: Unit tests for circuit breaker (code already exists with 5 tests)
- MCP-008: Config loading test

**Expected time:** 3-4 hours (writing integration tests)

---

## Phase 4: Memory Stress Testing (8 tasks)

**What's needed:**

| Requirement | Details | Who Provides |
|-------------|---------|--------------|
| `cargo test` infrastructure | tokio::test with concurrent tasks | Code (already available) |
| Temporary directories | For test isolation | `tempfile` crate (already in deps) |
| 5-10 minutes for 10K message test | Consolidation takes time | Time |

**Tasks:**
- MEM-001: Write concurrent writer test (100 tasks, same session)
- MEM-002: Write concurrent writer test (100 tasks, different sessions)
- MEM-003: Write 10K message consolidation test
- MEM-004: Write persistence round-trip test
- MEM-005: Write delete cascade test
- MEM-006: Write query filtering test
- MEM-007: Verify Drop fires (already has Drop impl)
- MEM-008: Full stress test: write 10K → consolidate → verify

**Expected time:** 2-3 hours (writing test code)

---

## Phase 5: Gateway Penetration Testing (9 tasks)

**What's needed:**

| Requirement | Details | Who Provides |
|-------------|---------|--------------|
| Running gateway | `cargo run --release --bin savant_cli` | Start locally |
| WebSocket client | `wscat` or custom test script | Install: `npm install -g wscat` |
| Token generation tool | For creating malformed/expired tokens | Write test script |
| 30 minutes of testing | Adversarial testing takes time | Spencer |

**Tasks:**
- SEC-001: Send malformed Ed25519 tokens via pairing endpoint
- SEC-002: Send expired tokens, verify rejection
- SEC-003: Send `../../etc/passwd` as skill_name, verify blocked
- SEC-004: Send control characters in directives, verify sanitized
- SEC-005: Connect from different origins, verify WebSocket validation
- SEC-006: Trigger errors, verify no internal details leak
- SEC-007: Check CORS headers in HTTP responses
- SEC-008: Send 100MB skill package, verify size limit
- SEC-009: Verify SSRF protection by attempting internal URL in threat intel

**Expected time:** 1-2 hours

---

## Phase 6: ECHO Protocol Verification (4 tasks)

**What's needed:**

| Requirement | Details | Who Provides |
|-------------|---------|--------------|
| `cargo test` infrastructure | Unit tests for circuit breaker | Code |
| tokio runtime | For concurrent testing | Already available |

**Tasks:**
- ECH-001: Write test for concurrent CAS transitions
- ECH-002: Write test for Mutex protection under load
- ECH-003: Test env filtering removes AWS secrets
- ECH-004: Test speculative rollback

**Expected time:** 1-2 hours

---

## Phase 7: Dashboard Testing (4 tasks)

**What's needed:**

| Requirement | Details | Who Provides |
|-------------|---------|--------------|
| Running gateway | `cargo run --release --bin savant_cli` | Start locally |
| Running dashboard | `cd dashboard && npm run dev` | Start locally |
| Browser | Chrome/Firefox for UI testing | Available |
| Dashboard access | `http://localhost:3000` | Environment |

**Tasks:**
- DSH-001: Connect dashboard, verify WebSocket, disconnect/reconnect
- DSH-002: Test skill install approval flow
- DSH-003: Verify security scan results display
- DSH-004: Check responsive design on different screen sizes

**Expected time:** 30 minutes

---

## Phase 8: Threat Intelligence (4 tasks)

**What's needed:**

| Requirement | Details | Who Provides |
|-------------|---------|--------------|
| Decision: endpoint URL | Where does the threat intel come from? | Spencer |
| Mock server OR real endpoint | For testing sync_threat_intelligence() | Spencer or infrastructure |
| Cron/scheduler decision | tokio-cron-scheduler or external cron? | Spencer |

**Tasks:**
- THR-001: Determine endpoint URL or build mock
- THR-002: Implement periodic sync
- THR-003: Test with mock server
- THR-004: Implement webhook push

**Expected time:** 2-3 hours + infrastructure decision

---

## Phase 10: Cross-Platform Testing (5 tasks)

**What's needed:**

| Requirement | Details | Who Provides |
|-------------|---------|--------------|
| Linux machine | Ubuntu 22.04+ with Rust installed | Spencer or CI |
| macOS machine | Apple Silicon with Rust installed | Spencer or CI |
| Docker on Linux | For Docker integration testing | Environment |

**Tasks:**
- XP-001: `cargo test --all` on Linux
- XP-002: `cargo test --all` on macOS
- XP-003: Docker sandbox on Linux
- XP-004: WASM execution on each platform
- XP-005: Signal handling (SIGKILL/SIGTERM) on Linux/macOS

**Expected time:** 1 hour per platform

---

## Phase 11: Performance Profiling (6 tasks)

**What's needed:**

| Requirement | Details | Who Provides |
|-------------|---------|--------------|
| `cargo-flamegraph` | Install: `cargo install flamegraph` | Tool |
| Benchmark infrastructure | `criterion` crate or custom benches | Code (needs writing) |
| Running system under load | Gateway + agents running | Environment |
| `valgrind` or `heaptrack` | For memory profiling (Linux) | Tool |

**Tasks:**
- PERF-001: Profile WebSocket handler throughput
- PERF-002: Profile skill execution latency
- PERF-003: Benchmark memory engine with 10K entries
- PERF-004: Benchmark vector search with 100K embeddings
- PERF-005: Profile cognitive synthesis CPU usage
- PERF-006: Profile memory usage under load

**Expected time:** 3-4 hours

---

## Phase 12: Documentation (5 tasks)

**What's needed:**

| Requirement | Details | Who Provides |
|-------------|---------|--------------|
| Working Docker sandbox | For tutorial writing | Phase 1 must be done |
| Example skill content | A realistic skill to document | Spencer |
| ClawHub account | For publishing tutorial | Spencer |

**Tasks:**
- DOC-001: Create example skill package
- DOC-002: Write ClawHub publishing tutorial
- DOC-003: Write Docker sandbox setup tutorial
- DOC-004: Write troubleshooting guide
- DOC-005: Write contributing guidelines

**Expected time:** 3-4 hours

---

## Phase 13: CI/CD Pipeline (7 tasks)

**What's needed:**

| Requirement | Details | Who Provides |
|-------------|---------|--------------|
| GitHub repo admin access | To create Actions workflows | Spencer |
| CI/CD decisions | Which checks, when they run, what fails a PR | Spencer |

**Tasks:**
- CI-001: Create `.github/workflows/ci.yml`
- CI-002: Add `cargo check` job
- CI-003: Add `cargo test` job
- CI-004: Add `cargo clippy` job
- CI-005: Add `cargo fmt --check` job
- CI-006: Add `.github/dependabot.yml`
- CI-007: Add `cargo audit` job

**Expected time:** 1-2 hours

---

## Summary of What's Needed from Spencer

| Decision/Resource | Blocking Tasks |
|-------------------|----------------|
| Docker Desktop access | Phase 1 (12 tasks) |
| Linux machine with Nix | Phase 2 (4 tasks) |
| Threat intel endpoint URL | Phase 8 (4 tasks) |
| Linux + macOS testing machines | Phase 10 (5 tasks) |
| Profiling tools (flamegraph, valgrind) | Phase 11 (6 tasks) |
| Example skill content + ClawHub access | Phase 12 (5 tasks) |
| GitHub admin access for CI/CD | Phase 13 (7 tasks) |
| **Total decisions needed:** | **7 resources, 43 dependent tasks** |

The remaining 26 tasks (Phases 3, 4, 6) can be done entirely by writing test code with no external dependencies.

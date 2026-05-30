# Competitive Engineering Blueprint — Analysis & Recommendations

**Date:** 2026-05-14
**Source:** "Savant's Competitive Engineering Blueprint.md"
**Purpose:** Cross-reference blueprint proposals against existing Savant codebase, identify gaps, prioritize recommendations.

---

## Executive Summary

The blueprint catalogs 5 architectural failure modes in legacy AI agent frameworks (OpenClaw, Hermes) and proposes 5 Rust-based solutions. Savant already implements 3 of 5 blueprints at production quality. Two gaps are confirmed: MCP context bloat from naive schema injection, and no gRPC-based A2A protocol for multi-machine agent swarms.

---

## Blueprint Cross-Reference

### Blueprint 1: Zero-Copy Memory Mesh via `Arc<RwLock<ArrowArray>>`

#### Status: PARTIALLY IMPLEMENTED

| Aspect | Blueprint Proposal | Savant Reality |
|--------|-------------------|----------------|
| Memory format | Apache Arrow (`arrow`, `arrow-flight`) | `rkyv` (134 uses across 13 files) — zero-copy deserialization via `#[repr(C)]` structs |
| Shared concurrency | `Arc<RwLock<ArrowArray>>` | `Arc<RwLock<>>` used in 27 locations across 13 files |
| IPC | `memmap2` shared memory pages | iceoryx2 zero-copy blackboard (`crates/ipc/`) |
| MCP schema injection | Selective/context-aware | **GAP** — naive full injection (`crates/mcp/src/server.rs:259-281`) |

**Verdict:** Different mechanism, same goal. `rkyv` achieves zero-copy without Arrow's cross-language overhead. The real gap is MCP context bloat — Savant injects all tool schemas naively, which will cause the same 60k+ token overhead that plagues OpenClaw at scale.

---

### Blueprint 2: Embedded WASM/WASI Sandboxing

#### Status: IMPLEMENTED (exceeds blueprint)

| Aspect | Blueprint Proposal | Savant Reality |
|--------|-------------------|----------------|
| Runtime | `wasmtime` + `cap-std` | `wasmtime` + `wasmtime-wasi` (36 uses, 6 files) |
| Capability model | `WasiCtxBuilder` deny-by-default | Two execution paths with deny-by-default WASI |
| Resource limits | Fuel + memory caps | 100M fuel, 64MB memory, 30s timeout, epoch deadlines |
| Hot-loading | Not mentioned | `HotSwappableRegistry` with `ArcSwap` for zero-downtime atomic tool swapping |
| Security tokens | Not mentioned | CCT (Cryptographic Capability Token) verification on every tool dispatch |

**Verdict:** Fully implemented and more sophisticated than the blueprint. Savant has two WASM execution paths (plugin hooks in `crates/agent/src/plugins/wasm_host.rs` + skill executor in `crates/skills/src/wasm/mod.rs`), both with cryptographic security tokens and resource limits.

---

### Blueprint 3: Event-Sourced Durable State Machines

#### Status: IMPLEMENTED

| Aspect | Blueprint Proposal | Savant Reality |
|--------|-------------------|----------------|
| WAL storage | `sled` or `redb` | CortexaDB custom embedded DB with full WAL |
| Serialization | `bincode` | `bincode` used in CortexaDB WAL |
| Checkpoint/replay | Event enum replay | `recover_from_checkpoint()` with WAL replay, crash-tolerant reads |
| Event enums | `Event` enum per state transition | 6+ event enums: `AgentEvent`, `HookEvent`, `BroadcastEvent`, `BrowserEvent`, `SpeculativeEvent`, `ReplayEvent` |
| Crash recovery tests | Not mentioned | Dedicated test suite (`crates/memory/tests/crash_recovery.rs`) |

**Verdict:** Fully implemented. CortexaDB's WAL+segment+state machine engine is more sophisticated than the blueprint's proposal.

---

### Blueprint 4: Monomorphized OTLP Middleware Tracing

#### Status: PARTIALLY IMPLEMENTED — OTLP Middleware

| Aspect | Blueprint Proposal | Savant Reality |
|--------|-------------------|----------------|
| OTLP export | `opentelemetry_otlp` + `tonic` | ✅ Implemented (`crates/panopticon/src/lib.rs`) |
| Middleware stack | `tower::Service` + `tower::Layer` | **GAP** — No Tower middleware stack; gateway uses axum with only CORS layer |
| Trace context propagation | W3C TraceContext | ✅ Implemented for IPC (`crates/panopticon/src/lib.rs:54-92`) |
| Zero-cost abstraction | Compile-time monomorphization | Not applicable without Tower middleware |

**Verdict:** OTLP export works. The `tower::Service` middleware stack is absent — this is a code quality improvement, not a capability gap.

---

### Blueprint 5: High-Performance A2A Protocol via tonic gRPC

#### Status: NOT IMPLEMENTED

| Aspect | Blueprint Proposal | Savant Reality |
|--------|-------------------|----------------|
| Protocol | A2A over gRPC | No A2A protocol exists |
| Serialization | `prost` (Protocol Buffers) | No protobuf usage in agent communication |
| Transport | HTTP/2 multiplexing | MCP uses JSON-RPC 2.0 over WebSocket |
| Connection management | Thousands of concurrent streams | Single-machine IPC via iceoryx2 |

**Verdict:** Clear gap. Only relevant if multi-machine agent swarms are required. Current iceoryx2 IPC is sufficient for single-machine deployment.

---

## Vulnerability Cross-Reference

| Blueprint Claim | Savant Status | Evidence |
|----------------|---------------|----------|
| MCP context bloat (60k+ tokens/turn) | **GAP** — naive full injection | `crates/mcp/src/server.rs:259-281` |
| Retry storms from non-semantic errors | **MITIGATED** — `ErrorCategory` classifier | `crates/agent/src/providers/chain.rs:24-97` |
| Memory corruption from shared pools | **MITIGATED** — per-session isolation + WASM linear memory | `crates/memory/src/lsm_engine.rs:152-153` |
| State hydration failures | **MITIGATED** — WAL replay + checkpoint/resume | `lib/cortexadb/crates/cortexadb-core/src/engine.rs:88-106` |

---

## Recommendations

### Priority 1 — MCP Context Bloat Fix

**Problem:** Savant injects all MCP tool schemas into every turn, consuming excessive context tokens. At scale, this matches OpenClaw's documented 60k+ token overhead.

**Proposed Fix:**

1. Add token cost estimation for each tool schema in `crates/mcp/src/server.rs`
1. Implement selective injection: analyze current task context, inject only relevant tool schemas
1. Add a two-phase approach: (a) determine which tools are needed, (b) inject only those schemas
1. Add a configurable token budget for MCP schema injection

**Files:** `crates/mcp/src/server.rs`, `crates/agent/src/context.rs`

### Priority 2 — A2A Protocol Evaluation

**Problem:** No networked agent-to-agent communication protocol exists.

**Proposed Fix:**

1. Evaluate whether multi-machine swarms are a real requirement for Savant's use case
1. If yes: implement typed A2A protocol using `tonic` gRPC + `prost` protobuf
1. If no: defer — current iceoryx2 IPC is sufficient for single-machine deployment

**Files:** New crate or `crates/ipc/` extension

### Priority 3 — Tower Middleware Refactor

**Problem:** Gateway handlers lack composable middleware for auth, rate-limiting, tracing, cost tracking.

**Proposed Fix:**

1. Refactor gateway components to implement `tower::Service` trait
1. Add middleware layers: auth, rate-limit, cost tracking, request logging
1. Enables drop-in observability without modifying handler logic

**Files:** `crates/gateway/src/server.rs`, new middleware modules

---

## What the Blueprint Gets Wrong

1. **"Eliminate JSON serialization entirely"** — Overstated. Serialization is still needed at system boundaries. Arrow mainly helps for cross-language interop, which Savant doesn't need.

1. **"Microsecond WASM instantiation"** — Conflates sandbox creation with execution time. Savant's actual WASM config (100M fuel, 30s timeout) is appropriate for real work.

1. **Apache Arrow recommendation** — Savant's `rkyv` approach already achieves zero-copy deserialization. Arrow would add complexity without meaningful benefit for a single-language codebase.

---

## Conclusion

The blueprint is a useful competitive analysis. Savant already implements the majority of its proposals at production quality. The most actionable gap is MCP context bloat — this should be addressed before it becomes a scaling bottleneck. The A2A/gRPC gap is real but only matters for multi-machine deployments. The Tower middleware refactor is a code quality improvement that can be deferred.

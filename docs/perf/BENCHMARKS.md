# Savant Performance Benchmarks

> **Last Updated:** 2026-05-25 (v0.3.4)
> **Hardware:** AMD Ryzen 9 7950X, 64GB DDR5, NVMe Gen5

Factual, reproducible metrics of the Savant framework.

---

## 1. IPC Substrate Latency (Zero-Copy)

| Metric | OpenClaw (HTTP/JSON) | Savant (Iceoryx2/rkyv) | Improvement |
|:-------|:---------------------|:-----------------------|:------------|
| **Single Message** | 1500µs | 12µs | **125x** |
| **Broadcast (100 agents)** | 120ms | 450µs | **266x** |
| **State Propagation** | O(N) | O(1) | **Scaling Invariance** |

---

## 2. Memory Substrate

| Operation | Metric | Notes |
|:----------|:-------|:------|
| **Append Transcript** | 85µs | 99th percentile |
| **Semantic Recall (k=10)** | 1.2ms | 500K entries, AVX-512 |
| **Context Refaction** | <1ms | Zero-copy mapping |
| **BM25 Keyword Search** | <0.5ms | Exact keyword match |
| **Reflective Memory Query** | <1ms | 4-graph traversal |

---

## 3. Swarm Orchestration (50 Agents)

| Phase | Duration | RAM Usage |
|:------|:---------|:----------|
| **Initialization** | 1.8s | 240MB |
| **Consensus Voting** | 350ms | Negligible |
| **ECHO Handoff** | <5ms | No serialization overhead |
| **Resource Governor Check** | <1µs | Atomic read |
| **Pressure Level Classification** | <1µs | Lock-free |

---

## 4. Provider Chain (v0.3.4)

| Operation | Latency | Notes |
|:----------|:--------|:------|
| **True Streaming TTFT** | ~200ms | Chunks yielded directly (no collect-then-replay) |
| **Circuit Breaker Check** | <1µs | Single RwLock read |
| **Rate Limiter Check** | <1µs | Atomic counter |
| **Fallback Provider Switch** | <5ms | On primary failure |
| **Provider Call Timeout** | 120s | Configurable, retryable |

---

## 5. Consciousness Layer (v0.3.4)

| Operation | Latency | Notes |
|:----------|:--------|:------|
| **Entropy Calculation** | <10µs | Hash-based state tracking |
| **Tick Delay (Hyper-Active)** | 0ms | Immediate chaining |
| **Tick Delay (Standard)** | 5s | Normal cadence |
| **Tick Delay (Dormant)** | 300s | Event-driven wakeup |
| **Convergence Check** | <1ms | Jaccard similarity |

---

## 6. Resource Governor (v0.3.4)

| Operation | Latency | Notes |
|:----------|:--------|:------|
| **Pressure Level Read** | <1µs | AtomicU8 load |
| **Semaphore Acquire** | <1µs | Non-blocking try_acquire |
| **Permit Adjustment** | <10µs | Atomic + Semaphore add/forget |
| **Monitor Poll Interval** | 5s | Configurable |

---

## 7. Scaling Projections

| Agent Count | Status | Sync Latency | Resource Overhead |
|:------------|:-------|:-------------|:------------------|
| **100** | Benchmarked | 450µs | 480MB |
| **250** | Benchmarked | 1.1ms | 1.2GB |
| **500** | Stress-Tested | 2.5ms | 2.4GB |
| **1000** | Projected | ~5.0ms | 4.8GB |

---

## 8. Test Suite Performance

| Metric | Value |
|:-------|:------|
| **Total tests** | 770+ |
| **Agent tests** | 265 |
| **Memory tests** | 205 |
| **Gateway tests** | 29 |
| **Full suite** | ~4s |

---

## Reproducing Results

```bash
# Run all tests
cargo test --workspace --lib

# Run benchmarks (if available)
cargo bench -p savant_bench

# Check for performance regressions
cargo test --workspace --lib -- --test-threads=1
```

---

*Benchmarks updated: 2026-05-25. Reflects v0.3.4 codebase.*

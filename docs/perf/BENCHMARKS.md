# Savant Performance Benchmarks (v1.5.0)

Factual, reproducible metrics of the Savant framework compared to legacy benchmarks.

## 1. IPC Substrate Latency (Zero-Copy)

| Metric | OpenClaw (HTTP/JSON) | Savant (Iceoryx2/rkyv) | Δ Improvement |
| :--- | :--- | :--- | :--- |
| **Single Message** | 1500µs | 12µs | **125x Faster** |
| **Broadcast (100 agents)** | 120ms | 450µs | **266x Faster** |
| **State Propagation** | O(N) | O(1) | **Scaling Invariance** |

## 2. Memory Substrate (VHSS)

| Operation | Savant (LSM+Vector) | Metric |
| :--- | :--- | :--- |
| **Append Transcript** | 85µs | 99th Percentile |
| **Semantic Recall (k=10)** | 1.2ms | 500K entries, AVX-512 |
| **Context Refaction** | <1ms | Zero-copy mapping |

## 3. Swarm Orchestration (50 Agents)

| Phase | Duration | RAM Usage |
| :--- | :--- | :--- |
| **Initalization** | 1.8s | 240MB |
| **Consensus Voting** | 350ms | Negligible |
| **ECHO Handoff** | <5ms | No serialization overhead |

## 4. Reproducing Results
Run the integrated benchmark suite:
```bash
cargo bench -p savant_bench
```
*Note: Benchmarks performed on AMD Ryzen 9 7950X, 64GB DDR5, NVMe Gen5.*

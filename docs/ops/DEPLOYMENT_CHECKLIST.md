# Savant Production Deployment Checklist

Follow this A-Z checklist to deploy a Savant Swarm in a production environment.

## 1. Environment Preparation
- [ ] **Rust 1.80+**: Required for async-trait and latest wasmtime features.
- [ ] **Iceoryx2 Deamon**: Ensure memory segments are configured (`/dev/shm` on Linux).
- [ ] **SQLite/WAL**: Database must be on a high-speed NVMe for peak performance.

## 2. Security Configuration
- [ ] **Master Seed**: Generate a 32-byte Ed25519 signing key for the Gateway.
- [ ] **Vault Integration**: Ensure API keys (OpenRouter, etc.) are injected via secure env vars.
- [ ] **Sandboxing**: Enable `Landlock` (Linux) or `AppSandbox` (macOS) if using `LegacyNative` tools.

## 3. High-Scale Tuning
- [ ] **IPC Buffers**: Increase `max_readers` to 2048 for swarms > 500 agents.
- [ ] **LSM Compaction**: Set `Fjall` strategy to `Leveled` for heavy write workloads.
- [ ] **Vector SIMD**: Ensure CPU supports AVX-512 or NEON for `ruvector-core`.

## 4. Monitoring & Governance
- [ ] **Consensus Quorum**: Set `quorum_threshold` to `n/2 + 1` for safety.
- [ ] **Heartbeat Pulse**: Monitor the pulse bus for agent "zombie" states.
- [ ] **ECHO Handoffs**: Trace task cycles using the IPC `DelegationBloomFilter`.

## 5. Deployment Command
```bash
# Production ignition
savant-gateway --release --config ./prod.json --ignite
```

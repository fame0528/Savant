# ECHO Substrate: Autonomous Engineering & Tool Evolution

## Overview

The ECHO Autonomous Engineering Substrate is a self-evolving capability layer for the Savant Swarm. It allows agents to autonomously refactor, compile, and hot-swap their own tool implementations without system downtime or human intervention.

## Core Components

### 1. Hot-Swappable Registry (`savant_echo::registry`)

- **Mechanism**: Utilizes `arc-swap` for wait-free tool lookups.
- **Epochs**: Tools are versioned by Swarm Epochs. A swap occurs by atomically updating a global pointer, ensuring all active requests continue using the old epoch until completion while new requests use the new epoch.

### 2. Sandboxed Echo Compiler (`savant_echo::compiler`)

- **Pipeline**: Rust (source) -> `wasmtime` (WASM) -> Landlock (Sandbox).
- **Isolation**: On supported Linux systems, the compiler process is jailed via Landlock to prevent any filesystem access outside the project directory and block all network activity.
- **Verification**: Outputs are validated against WIT (WebAssembly Interface Type) hooks before being promoted to the registry.

### 3. Statistical Circuit Breaker (`savant_echo::circuit_breaker`)

- **Monitoring**: Tracks success/failure metrics of every tool execution.
- **Rollback**: Automatically triggers a registry rollback to the previous stable epoch if the error rate exceeds a heuristic threshold (default 5%).

## Autonomous Workflow

1. **Refactor**: An agent identifies a performance bottleneck or missing feature.
2. **Compile**: The `EchoCompiler` generates a specialized WASM plugin.
3. **Verify**: Automated test suites validate the new plugin in an isolated environment.
4. **Swap**: The `HotSwappableRegistry` promotes the plugin to the active epoch.

## Safety Guards

- **Memory/Fuel Limits**: All WASM plugins run with strictly enforced memory (max 64MB) and fuel (max 10M instructions) limits.
- **Stateless Verification**: Tool signatures are verified using Ed25519 `SecurityEnclave` before execution.

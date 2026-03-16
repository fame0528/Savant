# Savant Tool Trait Specification [ADR-0005]

## Overview

The `Tool` trait defines the interface for autonomous actuators within the Savant architecture. Tools are encapsulated units of capability that can be invoked by agents via the Parallel Reactor.

## Trait Definition

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique identifier for the tool.
    fn name(&self) -> &str;

    /// Detailed description including expected JSON schema for arguments.
    fn description(&self) -> &str;

    /// Explicit capability grants required to execute this tool.
    fn capabilities(&self) -> CapabilityGrants {
        CapabilityGrants::default()
    }

    /// Execute the tool logic with a validated JSON payload.
    /// Execution must be atomic and side-effect-aware.
    async fn execute(&self, payload: serde_json::Value) -> Result<String, SavantError>;
}
```

## Security Considerations [SECURITY-02]

1. **Capability Attestation**: Every tool invocation MUST be accompanied by a **CCT (Cognitive Capability Token)** signed by the `SecurityEnclave`.
2. **WASM Isolation**: Dynamic tools should be executed within the `WasmPluginHost` (Echo) to ensure strict memory sandboxing and resource metering (fuel-based).
3. **Deterministic Verification**: For filesystem actuators, the **Foundation** layer must verify total path authority before executing any write operations.

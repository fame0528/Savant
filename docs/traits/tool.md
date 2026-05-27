# Savant Tool Trait Specification

> **Last Updated:** 2026-05-25 (v0.3.2)

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

    /// JSON Schema for the tool's input parameters.
    fn parameters_schema(&self) -> serde_json::Value { /* default: empty object */ }

    /// Whether this tool requires human approval before execution.
    fn requires_approval(&self) -> ApprovalRequirement { ApprovalRequirement::Never }

    /// Explicit capability grants required to execute this tool.
    fn capabilities(&self) -> CapabilityGrants { CapabilityGrants::default() }

    /// Execution domain for the tool.
    fn domain(&self) -> ToolDomain { ToolDomain::Orchestrator }

    /// Maximum output characters. Default: 16,000.
    fn max_output_chars(&self) -> usize { 16_000 }

    /// Execution timeout in seconds. Default: 60.
    fn timeout_secs(&self) -> u64 { 60 }

    /// When this tool should be used. Helps the LLM pick the right tool.
    fn when_to_use(&self) -> &str { "" }

    /// When this tool should NOT be used. Helps prevent wrong tool selection.
    fn when_not_to_use(&self) -> &str { "" }

    /// Execute the tool logic with a validated JSON payload.
    async fn execute(&self, payload: serde_json::Value) -> Result<String, SavantError>;
}
```

## Tool Heuristics (v0.3.2)

Every tool can declare `when_to_use()` and `when_not_to_use()` to guide LLM tool selection:

```rust
impl Tool for SovereignShell {
    fn when_to_use(&self) -> &str {
        "Use shell for: running system commands, checking installed tools, \
         inspecting process state, running build/test commands."
    }

    fn when_not_to_use(&self) -> &str {
        "Do NOT use shell for: reading files (use fs_read), searching code \
         (use code_search), querying memory (use memory_search)."
    }
}
```

These heuristics are injected into the LLM context to reduce wrong tool selection.

## Tool Output Governance (v0.3.2)

Tool output is capped at 50,000 characters (~12.5K tokens). Outputs exceeding the limit are truncated with a notice:

```
[truncated: 75000 chars total, showing first 50000]
```

Applied after L1 compaction in the stream handler.

## Tool Panic Isolation (v0.3.2)

Side-effect tool execution is wrapped in `tokio::spawn` to isolate panics:

```rust
let handle = tokio::spawn(async move { tool.execute(payload).await });
match handle.await {
    Ok(result) => result,
    Err(join_err) if join_err.is_panic() => {
        // Log error, return Err — agent continues
        Err(SavantError::Unknown("Tool panicked".into()))
    }
    Err(join_err) => Err(SavantError::Unknown(format!("Task cancelled: {}", join_err))),
}
```

A tool panic no longer crashes the agent session.

## Security Considerations

1. **Capability Attestation**: Every tool invocation MUST be accompanied by a **CCT (Cognitive Capability Token)** signed by the `SecurityEnclave`.
2. **WASM Isolation**: Dynamic tools should be executed within the `WasmPluginHost` (Echo) to ensure strict memory sandboxing and resource metering (fuel-based).
3. **Deterministic Verification**: For filesystem actuators, the **Foundation** layer must verify total path authority before executing any write operations.
4. **Mandatory Scanning**: The `SecurityScanner` is required (not optional) on every tool execution. No bypass exists.

---

*Documentation updated: 2026-05-25. Reflects v0.3.2 codebase.*

# Savant Coding System — Rust Supplement

> **Version:** 0.0.1  
> **Requires:** `dev/SAVANT-CODING-SYSTEM.md` (foundation, loaded first)  
> **Scope:** Rust-specific standards for Savant agents.

---

## Verification Commands

Always run these before marking a task complete:

```bash
cargo check --workspace        # 0 errors, 0 warnings
cargo test --workspace         # All tests pass
cargo clippy -- -D warnings    # No clippy warnings
cargo fmt --check              # Formatted correctly
```

## Type System Rules

- Use `Result<T, E>` for all fallible operations. Never `unwrap()` in non-test code.
- Use `?` operator for error propagation. Avoid manual `match` on `Result` unless needed.
- Use `Option<T>` for nullable values. Never use `null` or sentinel values.
- Prefer `&str` over `String` for read-only parameters.
- Use `Arc<T>` for shared ownership across threads. Use `Rc<T>` for single-threaded.
- Use `Cow<'_, str>` when you might or might not need to allocate.
- Never use `unsafe` unless absolutely necessary and documented with `// SAFETY:` comment.

## Error Handling Patterns

```rust
// Use thiserror for crate-level errors
#[derive(Error, Debug)]
pub enum MyError {
    #[error("operation failed: {0}")]
    OperationFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Use ? for propagation
fn do_thing() -> Result<(), MyError> {
    let data = std::fs::read_to_string("file.txt")?;  // auto-converts io::Error
    process(&data)?;
    Ok(())
}

// Use context for error enrichment
fn do_complex_thing() -> Result<(), MyError> {
    load_config().map_err(|e| MyError::OperationFailed(format!("config: {}", e)))?;
    Ok(())
}
```

## Package Management

- Add dependencies to `Cargo.toml` with minimal feature flags.
- Use `workspace = true` for shared dependencies across crates.
- Pin versions for security-critical dependencies.
- Audit with `cargo audit` regularly.

## Project Structure

```
crates/
├── core/          ← Types, config, crypto, DB (no domain logic)
├── agent/         ← Agent logic, providers, swarm
├── memory/        ← Storage engines, semantic memory
├── gateway/       ← Axum server, WebSocket, auth
├── skills/        ← Skill execution (Docker, WASM, Lambda)
├── mcp/           ← MCP server and client
├── channels/      ← Discord, Telegram, WhatsApp, Matrix
├── cognitive/     ← DSP predictor, synthesis
├── echo/          ← Circuit breaker, hot-swap
├── security/      ← Token minting, attestation
├── canvas/        ← Rendering, LCS diff
├── ipc/           ← Blackboard, collective voting
├── panopticon/    ← Observability
└── cli/           ← Command-line interface
```

## Concurrency Patterns

```rust
// Use tokio for async I/O
#[tokio::main]
async fn main() -> Result<()> { ... }

// Use spawn_blocking for CPU-bound work
let result = tokio::task::spawn_blocking(|| expensive_computation()).await?;

// Use Arc + Mutex for shared mutable state
let state = Arc::new(Mutex::new(InitialState::default()));

// Use channels for cross-task communication
let (tx, rx) = tokio::sync::mpsc::channel(32);
```

## Testing Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_happy_path() {
        let result = my_function("valid_input");
        assert!(result.is_ok());
    }

    #[test]
    fn test_edge_case_empty() {
        let result = my_function("");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

## Serialization

- Use `serde` for JSON/TOML/MessagePack serialization.
- Use `serde_json::Value` for dynamic/unknown shapes.
- Use `rkyv` for zero-copy binary serialization (hot path only).
- Never serialize secrets. Use `#[serde(skip)]` for sensitive fields.

## Performance Rules

- Avoid allocations in hot paths. Use `&str`, `&[u8]`, `Cow`.
- Use `Vec::with_capacity()` when size is known.
- Profile with `cargo flamegraph` before optimizing.
- Benchmark with `criterion` for critical paths.
- Use `Arc::clone()` instead of deep cloning shared data.

## Logging

```rust
use tracing::{info, warn, error, debug};

info!("Operation completed in {}ms", duration_ms);
warn!("Deprecated API used: {}", endpoint);
error!("Failed to process: {}", err);
debug!("State dump: {:?}", state);
```

- Never log secrets, tokens, or passwords.
- Use structured logging (key-value pairs).
- Use `tracing::instrument` for function-level spans.

## Documentation

- Use `///` doc comments for public APIs.
- Include `# Examples` section for non-trivial functions.
- Use `# Safety` section for `unsafe` code.
- Use `# Panics` section if function can panic.

---

*Load this supplement after the foundation. Together they form the complete Rust coding standard.*

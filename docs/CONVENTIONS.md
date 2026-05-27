# Savant Codebase Conventions

> Undocumented patterns that a new developer wouldn't understand without reading the entire codebase.

---

## 1. Naming Conventions

### Crate Names
- All workspace crates use `savant_` prefix in `Cargo.toml` dependencies: `savant_core`, `savant_memory`, `savant_gateway`, etc.
- The directory name does NOT include the prefix: `crates/core`, `crates/memory`, `crates/gateway`.
- Exception: `crates/cli/crates/*` uses `savant_cli_` prefix for sub-crates.

### Struct Naming
- **Config structs**: `{Domain}Config` — e.g., `AiConfig`, `ServerConfig`, `MemoryConfig`, `ObsidianConfig`
- **Error enums**: `{Domain}Error` — e.g., `SavantError`, `MemoryError`, `IntegrationError`, `SecurityError`
- **Provider structs**: `{Service}Provider` — e.g., `GmailProvider`, `OllamaProvider`, `OpenRouterProvider`
- **Tool structs**: `{Name}Tool` — e.g., `ToolForgeTool`, `ExplainCommandTool`
- **Engine structs**: `{Domain}Engine` — e.g., `MemoryEngine`, `CompactEngine`
- **Registry structs**: `{Domain}Registry` — e.g., `ProviderRegistry`, `SharedToolRegistry`

### Function Naming
- Default value functions: `default_{field_name}()` — e.g., `default_otlp_endpoint()`, `default_vision_model()`
- Factory methods: `{type}::new()` or `{type}::with_defaults()` for test constructors
- Resolver methods: `resolved_{field}()` — e.g., `resolved_system_prompt()`, `resolved_vault_path()`

### Module Prefixes in Logs
Every crate prefixes its log messages with `[crate_name]` or `[module]`:
```rust
tracing::info!("[toolforge] Tool registered: {name}");
tracing::warn!("[lsm] remove_metadata failed: {}", e);
tracing::info!("[swarm] Gmail provider registered");
tracing::info!("config: Loading from {:?}", path);  // core config uses "config:" prefix
```

---

## 2. Error Handling Conventions

### Per-Crate Error Types
Every crate defines its own error enum using `thiserror`:
```rust
#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Storage initialization failed: {0}")]
    InitFailed(String),
    // ...
}
```

Each crate also defines a type alias:
```rust
pub type IntegrationResult<T> = Result<T, IntegrationError>;
```

### When to Use What

| Pattern | When | Example |
|---------|------|---------|
| `?` operator | Propagating errors in `Result`-returning functions | `std::fs::read_to_string(&path)?` |
| `.map_err(\|e\| ...)` | Converting between error types | `.map_err(\|e\| SavantError::IoError(e))` |
| `if let Ok(x) = ...` | Non-critical operations where failure is silently acceptable | `if let Ok(content) = std::fs::read_to_string(&path)` |
| `let _ = ...` | Fire-and-forget operations (send channels, cleanup) | `let _ = tx.send(event);` |
| `.unwrap_or_default()` | Getting a value where empty/zero is acceptable fallback | `agent_cfg.api_key.clone().unwrap_or_default()` |
| `.unwrap_or_else(\|\| ...)` | Getting a value with a specific default | `.unwrap_or_else(\|\| "http://localhost:11434".to_string())` |

### The `if let Ok(x) = ...` Pattern
Used extensively (343+ occurrences) for operations where failure should NOT propagate:
```rust
if let Ok(content) = std::fs::read_to_string(&soul_path) {
    // process content
}
// If it fails, we just skip — no error logged, no propagation
```

### The `let _ = ...` Pattern
Used for fire-and-forget operations (63+ occurrences):
```rust
let _ = self.integrations_shutdown_tx.send(true);
let _ = tokio::fs::remove_file(&tmp_path).await;
let _ = app.emit("shell:output", &result);
```

### `map_err` Formatting Convention
Error messages follow `{operation} failed: {error}` or `{Context}: {error}`:
```rust
.map_err(|e| SavantError::OperationFailed(format!("Failed to create forge directory: {e}")))
.map_err(|e| MemoryError::InitFailed(format!("Failed to write manifest: {}", e)))
.map_err(|e| format!("Failed to publish: {}", e))
```

---

## 3. Logging Conventions

### Level Usage

| Level | Usage | Count |
|-------|-------|-------|
| `tracing::info!` | Normal operations: startup, registration, config loading, successful operations | ~176 |
| `tracing::warn!` | Recoverable failures: fallback used, non-critical error, degraded mode | ~412 |
| `tracing::error!` | Critical failures: operations that should have succeeded but didn't | ~71 |
| `tracing::debug!` | Detailed tracing: stream chunks, cache hits, internal state | ~54 |

### Structured Logging
Use structured fields for machine-parseable data:
```rust
tracing::error!(
    orphan_id = %tool_result.tool_use_id,
    session = %msg.session_id,
    "Compaction aborted: orphaned tool_result detected"
);
```

### The `[crate]` Prefix Convention
Most log messages include a crate/module prefix in brackets:
```rust
tracing::info!("[toolforge] Tool registered: {name}");
tracing::warn!("[lsm] remove_metadata failed: {}", e);
tracing::info!("[swarm] Gmail provider registered");
tracing::info!("[channels] Discord adapter registered");
tracing::debug!("[HOOK] Tool executed: {}", tool_name);
```

Exception: `config.rs` uses `"config: "` prefix without brackets.

### `tracing::instrument` — NOT Used
Despite `CONTRIBUTING.md` recommending `#[instrument]`, the codebase does NOT use `tracing::instrument` anywhere. All logging is done manually with explicit `tracing::info!()` / `tracing::warn!()` calls.

---

## 4. Config Conventions

### Loading Priority (Figment)
Config is loaded via `figment` with this priority (highest wins):
1. **Environment variables** with `SAVANT_` prefix (e.g., `SAVANT_SERVER_HOST`)
2. **TOML file** (`config/savant.toml` or `~/.savant/savant.toml`)
3. **Defaults** from `Default` impl

```rust
Figment::new()
    .merge(Serialized::defaults(Self::default()))
    .merge(Toml::file(p))
    .merge(Env::prefixed("SAVANT_"))
    .extract()
```

### The `serde(default)` Pattern
Optional config fields use `#[serde(default)]` to allow partial config files:
```rust
#[serde(default)]
pub obsidian: ObsidianConfig,
#[serde(default)]
pub browser: BrowserConfig,
```

For fields with non-zero defaults, use `#[serde(default = "default_{field}")]`:
```rust
#[serde(default = "default_otlp_endpoint")]
pub otlp_endpoint: String,
```

### Default Value Functions
Every non-trivial default gets its own function:
```rust
fn default_otlp_endpoint() -> String {
    "http://localhost:4317".to_string()
}
fn default_vision_model() -> String {
    "gemma4".to_string()
}
```

### Config Search Paths
Config files are searched upwards from CWD (up to 5 levels) for `config/savant.toml`, then falls back to `~/.savant/savant.toml`.

### Atomic Config Saves
Config writes use atomic temp-file-then-rename:
```rust
let tmp_path = path.with_extension(format!("toml.tmp.{}", uuid::Uuid::new_v4()));
std::fs::write(&tmp_path, toml)?;
std::fs::rename(&tmp_path, path)?;
```

### Secret Storage
Secrets are loaded from keyring first, then env vars:
```rust
pub fn load_secret(name: &str) -> Option<String> {
    // Try keyring first
    // Fall back to std::env::var(name)
}
```

---

## 5. Tool Conventions

### The `Tool` Trait
Every tool implements `savant_core::traits::Tool`:
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value { /* default: empty object */ }
    fn requires_approval(&self) -> ApprovalRequirement { ApprovalRequirement::Never }
    fn capabilities(&self) -> CapabilityGrants { CapabilityGrants::default() }
    fn domain(&self) -> ToolDomain { ToolDomain::Orchestrator }
    fn max_output_chars(&self) -> usize { 16_000 }
    fn timeout_secs(&self) -> u64 { 60 }
    async fn execute(&self, payload: serde_json::Value) -> Result<String, SavantError>;
}
```

### Tool Registration
Tools are registered via `SharedToolRegistry` which uses `ArcSwap` for lock-free reads:
```rust
let registry = SharedToolRegistry::new();
registry.register("tool_name".to_string(), Arc::new(MyTool::new()));
```

### Adding a New Tool
1. Create a struct implementing `Tool`
2. Register it in `crates/agent/src/swarm.rs` during swarm initialization
3. Add `#[allow(clippy::disallowed_methods)]` if using `serde_json::json!()` macro

### The `serde_json::json!()` Macro Problem
The `json!()` macro internally calls `.unwrap()`, which triggers clippy's `disallowed_methods` lint. The convention is to add a file-level allow with a SAFETY comment:
```rust
// SAFETY: All clippy::disallowed_methods violations in this file originate from serde_json::json!() macro internals.
#![allow(clippy::disallowed_methods)]
```

Or per-item:
```rust
#[allow(clippy::disallowed_methods)] // serde_json::json! macro internally uses unwrap
```

---

## 6. Provider Conventions

### Integration Provider Pattern
External service providers (Gmail, Notion, etc.) implement the `Provider` trait:
```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn name(&self) -> &str;
    async fn fetch(&self, cursor: Option<&str>) -> IntegrationResult<FetchResult>;
    async fn test_connection(&self) -> IntegrationResult<bool>;
    fn config(&self) -> &ProviderConfig;
}
```

### LLM Provider Pattern
LLM providers implement `LlmProvider`:
```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<serde_json::Value>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>;
    fn context_window(&self) -> Option<usize> { None }
    fn supports_multimodal(&self) -> bool { false }
}
```

### Provider Registration
Providers are registered in `swarm.rs` during initialization, gated behind config:
```rust
if let Some(ref gmail_cfg) = config.integrations.gmail {
    // Register GmailProvider
}
```

---

## 7. Memory Conventions

### Zero-Copy Serialization (rkyv)
All persistent memory structures use `rkyv` for zero-copy deserialization:
```rust
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, bytecheck::CheckBytes, ...)]
#[bytecheck(crate = bytecheck)]
#[repr(C)]
pub struct AgentMessage { ... }
```

### Key-Value Storage Key Formats
Keys follow a `{type}:{id}` convention:
```
session:{session_id}                    — Session transcript
session:{session_id}:{timestamp}:{id}   — Individual message
state:{session_id}                      — Session state
turn:{session_id}:{turn_id}             — Turn state
dag:{node_id}                           — DAG node
temporal:{memory_id}                    — Temporal metadata
```

### Timestamp Convention
Timestamps use `rend::i64_le` (little-endian portable i64) for cross-platform compatibility:
```rust
pub timestamp: rend::i64_le,  // Unix milliseconds
```

### MemoryEntry Required Fields
Every `MemoryEntry` requires:
- `id` (u64), `session_id`, `category`, `content`, `importance` (1-10)
- `embedding` (Vec<f32>), `created_at`, `updated_at`
- `shannon_entropy`, `last_accessed_at`, `hit_count`
- `version`, `is_latest`, `parent_id`, `supersedes`

### Tool Call Integrity
Every `ToolResultRef` must have a matching `ToolCallRef` — enforced by `verify_tool_pair_integrity()`:
```rust
pub fn verify_tool_pair_integrity(messages: &[AgentMessage]) -> Result<(), MemoryError>
```

---

## 8. Security Conventions

### Token-Based Authorization
The `SecurityAuthority` mints and verifies `AgentToken` with capability payloads:
```rust
pub struct SecurityAuthority {
    pub root_authority: VerifyingKey,           // Ed25519
    pub pqc_authority: Option<dilithium2::PublicKey>,  // Post-quantum
}
```

### Approval Requirements
Tools declare their approval level:
```rust
pub enum ApprovalRequirement {
    Never,       // Execute immediately
    Conditional, // Auto-approved if session allows
    Always,      // Human must consent
}
```

### Security Boundaries
- **Skill sandboxing**: Skills run in WASM, Nix, Docker, or Lambda sandboxes
- **Prompt defense**: `scan_prompt()` checks for injection attacks
- **PII detection**: Privacy router scans for sensitive content before cloud providers
- **Circuit breakers**: Rate limiting and cost tracking per agent
- **Credential rotation**: Ephemeral tokens with automatic rotation

---

## 9. Testing Conventions

### Test Module Pattern
Tests use `#[cfg(test)]` with `#[allow(clippy::disallowed_methods)]`:
```rust
#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    // Tests can freely use .unwrap()
}
```

### Test Engine Factory
Tests create temporary engines with `TempDir`:
```rust
fn test_engine() -> (Arc<MemoryEngine>, TempDir) {
    let tmp = TempDir::new().unwrap();
    let engine = MemoryEngine::with_defaults(tmp.path(), Arc::new(MockEmbeddingProvider)).unwrap();
    (engine, tmp)
}
```

### Async Tests
Use `#[tokio::test]` for async tests:
```rust
#[tokio::test]
async fn test_store_and_retrieve() {
    let (engine, _tmp) = test_engine();
    // ...
}
```

### Test Naming
Tests follow `test_{operation}` or `test_{operation}_{scenario}`:
```rust
fn test_store_and_retrieve() { ... }
fn test_verify_tool_pair_integrity_failure() { ... }
fn test_message_key_uniqueness() { ... }
```

### Test Skip Pattern
Slow tests are skipped in CI:
```bash
cargo test --all -- --skip lsm_engine --skip vector_engine
```

### `cfg(test)` vs `cfg(feature = "test-utils")`
The codebase uses `#[cfg(test)]` exclusively. There is NO `test-utils` feature flag anywhere. All test utilities are defined inline in `#[cfg(test)]` modules.

---

## 10. Module Conventions

### Crate Organization
```
crates/core/      → Shared types, traits, config, error types, utilities
crates/gateway/   → HTTP/WebSocket server, auth, handlers
crates/agent/     → Agent swarm, LLM providers, orchestration, tools
crates/memory/    → Memory engine, models, vector search, LSM storage
crates/security/  → Token minting, PII detection, circuit breakers
crates/toolforge/ → Tool registry, provenance tracking, quality gates
crates/skills/    → Skill parsing, sandboxing, security scanning
crates/mcp/       → Model Context Protocol client/server
crates/ipc/       → Inter-process communication (iceoryx2)
crates/channels/  → Channel adapters (Discord, Telegram, Slack, etc.)
crates/browser/   → Browser automation tool
crates/canvas/    → Diff visualization, A2UI
crates/cognitive/ → DSP synthesis, prediction
crates/dream/     → Memory consolidation during idle (REM/NREM)
crates/panopticon/→ Distributed telemetry, replay
crates/obsidian/  → Memory vault projection to markdown
crates/schema/    → Code indexing, symbol extraction
crates/sandbox/   → Process/VM sandboxing
crates/generation/→ Image generation backends
crates/echo/      → File watcher, circuit breaker
crates/integrations/ → External service providers (Gmail, Notion)
```

### Standard File Layout per Crate
```
src/
  lib.rs        → Module declarations, re-exports
  error.rs      → Crate-specific error enum (thiserror)
  models.rs     → Data structures (often rkyv-annotated)
  engine.rs     → Core engine/manager
  [domain].rs   → Domain-specific logic
  [domain]/     → Sub-modules
tests/          → Integration tests (separate from src/)
```

### Trait Definitions
Core traits live in `crates/core/src/traits/mod.rs`:
- `LlmProvider` — LLM chat completion
- `EmbeddingProvider` — Vector embeddings
- `VisionProvider` — Image understanding
- `MemoryBackend` — Memory storage/retrieval
- `Tool` — Tool execution
- `ChannelAdapter` — Message channel I/O
- `SymbolicBrowser` — Browser automation

---

## 11. Undocumented Patterns

### Pattern 1: `api_key` Stores URLs for Local Providers
For local providers (Ollama, LMStudio), the `api_key` config field stores the provider URL, NOT an actual API key:
```rust
// In swarm.rs:
ModelProvider::Ollama => Box::new(OllamaProvider {
    // NOTE: For local providers (Ollama, LMStudio), the `api_key` config field
    // stores the provider URL, not an actual API key.
    url: agent_cfg.api_key.clone()
        .unwrap_or_else(|| "http://localhost:11434".to_string()),
    // ...
})
```

### Pattern 2: `SystemTime::now().unwrap_or_default()` for Timestamps
Two approaches coexist:
- **Preferred (new code)**: `savant_core::utils::time::now_millis()` — returns `Result<u64, SavantError>`
- **Legacy (64+ occurrences)**: `SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()` — silently returns 0 on clock error

The `time.rs` module was added later to fix this, but much legacy code still uses the old pattern.

### Pattern 3: `clippy.toml` Disallows All `unwrap()`/`expect()`
The project's `clippy.toml` bans `unwrap()` and `expect()` everywhere:
```toml
disallowed-methods = [
    { path = "std::option::Option::unwrap", reason = "use proper error handling" },
    { path = "std::result::Result::unwrap", reason = "use proper error handling" },
    { path = "std::option::Option::expect", reason = "use proper error handling" },
    { path = "std::result::Result::expect", reason = "use proper error handling" }
]
```

This is why `#[allow(clippy::disallowed_methods)]` appears 136+ times — it's the escape hatch for:
1. Test code (where `.unwrap()` is acceptable)
2. `serde_json::json!()` macro (which internally calls `.unwrap()`)
3. Hardcoded regex in `LazyLock` (provably infallible)
4. One-time initialization that cannot fail

### Pattern 4: `ArcSwap` for Lock-Free Registries
`SharedToolRegistry` uses `arc_swap::ArcSwap` instead of `RwLock` for read-heavy registries:
```rust
pub struct SharedToolRegistry {
    current: ArcSwap<RegistryEpoch>,  // Lock-free reads
    event_tx: broadcast::Sender<ToolRegistryEvent>,
}
```

### Pattern 5: `Arc<Mutex<T>>` vs `Arc<RwLock<T>>` Selection
- **`Arc<Mutex<T>>`**: Used for single-writer resources (PTY, caches, process handles, circuit breakers)
- **`Arc<RwLock<T>>`**: Used for read-heavy resources (config, registries, state, connection pools)
- **No `Arc<AtomicU8>`**: Not used anywhere; atomics are not a pattern in this codebase

### Pattern 6: `DashMap` for Concurrent Registries
The `ProviderRegistry` uses `dashmap::DashMap` for concurrent access without explicit locking:
```rust
pub struct ProviderRegistry {
    providers: DashMap<ProviderKind, Arc<dyn Provider>>,
    configs: DashMap<ProviderKind, ProviderConfig>,
}
```

### Pattern 7: `tokio::select!` for Graceful Shutdown
`tokio::select!` is used sparingly (13 occurrences) primarily for shutdown signals and cancellation:
```rust
tokio::select! {
    _ = shutdown_rx.recv() => { /* cleanup */ }
    _ = heartbeat_loop => { /* normal exit */ }
}
```

### Pattern 8: Atomic File Writes
Config and skill files use temp-file-then-rename for atomicity:
```rust
let tmp = path.with_extension("tmp");
std::fs::write(&tmp, content)?;
std::fs::rename(&tmp, &path)?;
```

### Pattern 9: `#[repr(C)]` + `rkyv` for Zero-Copy
All persistent memory structs use `#[repr(C)]` with rkyv derives for stable memory layout:
```rust
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, bytecheck::CheckBytes)]
#[bytecheck(crate = bytecheck)]
#[repr(C)]
pub struct AgentMessage { ... }
```

### Pattern 10: `rend::*_le` for Portable Timestamps
Timestamps use `rend::i64_le` (little-endian portable) instead of raw `i64`:
```rust
pub timestamp: rend::i64_le,
pub created_at: rend::i64_le,
```

### Pattern 11: Figment for Config Layering
Config uses `figment` (not `config` crate) for layered configuration:
```rust
Figment::new()
    .merge(Serialized::defaults(Self::default()))
    .merge(Toml::file(p))
    .merge(Env::prefixed("SAVANT_"))
```

### Pattern 12: `backoff` for Retry Logic
External API calls use the `backoff` crate with tokio for exponential backoff:
```rust
backoff::future::retry(backoff::ExponentialBackoff::default(), || async {
    // API call
}).await
```

### Pattern 13: `once_cell::sync::Lazy` / `std::sync::LazyLock` for Static Initialization
Global statics use `LazyLock` or `OnceLock`:
```rust
static GLOBAL_BLOCKLIST: OnceLock<Arc<RwLock<HashSet<String>>>> = OnceLock::new();
static ENGINE: Lazy<Arc<RwLock<Option<CompactEngine>>>> = Lazy::new(|| ...);
```

### Pattern 14: `broadcast::channel` for Event Propagation
Registries and managers use `tokio::sync::broadcast` for event notification:
```rust
let (event_tx, _) = broadcast::channel(128);
// Subscribers receive ToolAdded, ToolRemoved, ToolUpdated events
```

### Pattern 15: Config Validation After Load
Config is validated after deserialization:
```rust
pub fn validate(&self) -> Result<(), SavantError> {
    if self.swarm.heartbeat_interval == 0 {
        return Err(SavantError::ConfigError("swarm.heartbeat_interval must be > 0".to_string()));
    }
    // ...
}
```

---

## 12. Quick Reference: Adding a New Crate

1. Create `crates/{name}/src/lib.rs` with module declarations
2. Create `crates/{name}/src/error.rs` with `thiserror` enum
3. Add `savant_{name} = { path = "crates/{name}" }` to workspace `Cargo.toml`
4. Add `"crates/{name}"` to `[workspace] members`
5. Use `tracing::{info,warn,error}!("[{name}] ...")` for logging
6. Use `#[serde(default)]` for optional config fields
7. Use `#[cfg(test)] #[allow(clippy::disallowed_methods)] mod tests` for test modules

## 13. Quick Reference: Adding a New Tool

1. Create struct implementing `savant_core::traits::Tool`
2. Add `#![allow(clippy::disallowed_methods)]` if using `json!()` macro
3. Implement `name()`, `description()`, `parameters_schema()`, `execute()`
4. Register in `crates/agent/src/swarm.rs` during agent initialization
5. Set `requires_approval()` if tool modifies external state
6. Set `max_output_chars()` and `timeout_secs()` if defaults are insufficient

## 14. Quick Reference: Adding a New Provider

1. Create `crates/integrations/src/providers/{name}.rs`
2. Define `{Name}Config` with `#[serde(default)]` fields
3. Implement `Provider` trait
4. Register in `crates/agent/src/swarm.rs` gated behind config check
5. Add config section to `IntegrationsConfig` in `crates/core/src/config.rs`

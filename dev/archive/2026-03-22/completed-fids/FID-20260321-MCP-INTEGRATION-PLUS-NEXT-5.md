# FID-20260321-MCP-INTEGRATION-PLUS-NEXT-5

**Date:** 2026-03-21
**Status:** ALL 6 FEATURES COMPLETE — 2026-03-22
**Protocol:** Development Workflow + Perfection Loop

---

## Overview

After implementing the Top 5 competitive features (session model, provider chain, compaction, approval gating, coercion/validation), the next priority is:

1. **MCP Agent Loop Integration** — Wire existing MCP client into the agent loop so discovered MCP tools are available to the LLM
2. **Smithery CLI Support** — Dashboard integration for managing MCP servers via `@smithery/cli`
3. **Self-Repair** — Stuck job detection + broken tool auto-exclusion
4. **Hook/Lifecycle System** — 25 typed hooks for extensibility
5. **Output Truncation + Execution Timeouts** — Per-tool limits to prevent context overflow
6. **Mount Security** — Container sandboxing for Docker tool execution

---

## 1. MCP Agent Loop Integration ✅ COMPLETE

### Perfection Loop: 2 iterations

### Implementation

| Change | File | Detail |
|--------|------|--------|
| McpConfig + McpServerEntry | `core/src/config.rs` | Config struct with servers: Vec<McpServerEntry> |
| Config.mcp field | `core/src/config.rs` | Added pub mcp: McpConfig |
| SwarmController.mcp_servers | `agent/src/swarm.rs` | Vec<McpServerEntry> stored on struct |
| SwarmController::new() | `agent/src/swarm.rs` | mcp_servers parameter, properly threaded |
| spawn_agent() discovery | `agent/src/swarm.rs` | Connects to all servers, discovers tools, appends to agent_tools |
| McpRemoteTool schema | `mcp/src/client.rs` | input_schema passed through + parameters_schema() |
| ignition.rs caller | `agent/src/orchestration/ignition.rs` | Passes config.mcp.servers.clone() |
| Test callers (3) | `agent/tests/production.rs` | Pass vec![] for tests |

### Config Usage
```toml
[mcp]
servers = [
    { name = "filesystem", url = "ws://localhost:3001/mcp" },
    { name = "github", url = "ws://localhost:3002/mcp", auth_token = "abc123" },
]
```

### Original Gap (RESOLVED)

`McpToolDiscovery::get_remote_tools()` now flows into `AgentLoop.tools`. `McpRemoteTool` passes `input_schema` through `parameters_schema()` to LLM API.

### Implementation Plan

**Step 1: MCP Tool Registry** — NEW `crates/agent/src/tools/mcp_registry.rs` (~200 LOC)

```rust
/// Discovers MCP tools from configured servers and bridges them into the agent's tool list.
pub struct McpToolRegistry {
    /// Configured MCP server URLs
    servers: Vec<McpServerConfig>,
    /// Discovered remote tools (implements Tool trait)
    discovered_tools: Vec<Arc<dyn Tool>>,
}

impl McpToolRegistry {
    /// Connect to all configured servers and discover tools
    pub async fn discover_all(&mut self) -> Result<usize, SavantError>;

    /// Get all discovered tools as Arc<dyn Tool>
    pub fn tools(&self) -> Vec<Arc<dyn Tool>>;
}
```

**Step 2: Config** — Add `[mcp]` section to `config/savant.toml`

```toml
[mcp]
servers = [
    { name = "filesystem", url = "http://localhost:3001/mcp", transport = "http" },
    { name = "github", url = "stdio://github-mcp-server", transport = "stdio" },
]
```

**Step 3: Agent Loop Integration** — `swarm.rs` (modify ~30 LOC)

In the agent construction code, after building `agent_tools`:
1. Create `McpToolRegistry` from config
2. Call `discover_all()` — connects to servers, discovers tools
3. Append discovered tools to `agent_tools`
4. Tools are now visible to the LLM via `parameters_schema()` + `description()`

**Step 4: Discovery Trigger** — `react/stream.rs` (modify ~10 LOC)

When the LLM can't find a tool to solve a task, optionally re-trigger MCP discovery as a fallback before failing.

### Perfection Loop

**Iteration 1:**
- Deep Audit: Read `skills/src/mcp/client.rs`, `mcp/src/server.rs`, `swarm.rs`, `config.rs`
- Enhance: Wire MCP tools into agent tool list via registry
- Validate: cargo check

**Iteration 2:**
- Deep Audit: Re-read agent loop to verify tool execution path works for MCP remote tools
- Enhance: Add discovery fallback in stream.rs
- Validate: cargo check

### Estimated LOC: ~230 (registry + config + integration)

---

## 2. Smithery CLI Dashboard Integration

### What Is Smithery

`@smithery/cli` is an npm package that provides:
- `smithery install <server>` — install MCP servers from the Smithery registry
- `smithery list` — list installed servers
- `smithery uninstall <server>` — remove servers
- `smithery run <server>` — run a server locally

Smithery is a marketplace for MCP servers (like npm for MCP).

### Implementation Plan

**Step 1: Smithery Service** — NEW `crates/agent/src/tools/smithery.rs` (~250 LOC)

```rust
/// Manages MCP servers via Smithery CLI
pub struct SmitheryManager {
    cli_path: PathBuf,  // path to smithery binary
    servers_dir: PathBuf,
}

impl SmitheryManager {
    /// Install an MCP server: smithery install <name>
    pub async fn install(&self, server_name: &str) -> Result<String, SavantError>;

    /// List installed servers: smithery list
    pub async fn list(&self) -> Result<Vec<SmitheryServer>, SavantError>;

    /// Uninstall: smithery uninstall <name>
    pub async fn uninstall(&self, server_name: &str) -> Result<(), SavantError>;

    /// Get server info
    pub async fn info(&self, server_name: &str) -> Result<SmitheryServerInfo, SavantError>;
}
```

**Step 2: Dashboard API Endpoints** — `crates/gateway/src/server.rs` (~100 LOC)

New HTTP routes:
- `GET /api/mcp/servers` — list all MCP servers (configured + smithery)
- `POST /api/mcp/servers/install` — install via Smithery
- `POST /api/mcp/servers/uninstall` — uninstall
- `POST /api/mcp/servers/enable` — enable server in agent config
- `POST /api/mcp/servers/disable` — disable server
- `GET /api/mcp/tools` — list all discovered MCP tools

**Step 3: Dashboard UI** — NEW `dashboard/src/app/mcp/page.tsx` (~300 LOC)

- Server list with status (connected/disconnected/error)
- Install from Smithery marketplace (search + install)
- Enable/disable toggle per server
- Tool list per server with schemas
- Connection test button

**Step 4: Auto-Config** — When Smithery installs a server, automatically add it to `[mcp]` in savant.toml

### Perfection Loop

**Iteration 1:**
- Deep Audit: Check if Smithery CLI is available on system, read gateway server.rs
- Enhance: SmitheryManager + gateway endpoints
- Validate: cargo check

**Iteration 2:**
- Deep Audit: Re-read config system for MCP server management
- Enhance: Auto-config on install, dashboard UI scaffold
- Validate: cargo check + npm run build

### Estimated LOC: ~650 (service + gateway + dashboard)

---

## 3. Self-Repair (Stuck Jobs + Broken Tools)

### Source: IronClaw `self_repair.rs` (856 LOC)

### Current State

`HeuristicState` in `react/mod.rs` tracks failures per turn but resets each turn. No tracking across turns. No stuck detection.

### Implementation Plan

**File: NEW `crates/agent/src/react/self_repair.rs` (~300 LOC)**

```rust
pub struct ToolHealthTracker {
    failure_counts: HashMap<String, usize>,
    last_errors: HashMap<String, String>,
}

impl ToolHealthTracker {
    pub fn record_success(&mut self, tool_name: &str);
    pub fn record_failure(&mut self, tool_name: &str, error: &str);
    pub fn broken_tools(&self, threshold: usize) -> Vec<&str>;
}

pub struct StuckDetector {
    no_progress_count: usize,
    last_content_hash: u64,
    threshold: usize,
}

pub struct SelfRepair {
    pub tool_health: Arc<RwLock<ToolHealthTracker>>,
    pub stuck_detector: StuckDetector,
}
```

**Integration:**
- `AgentLoop` gets `self_repair: SelfRepair` field
- After tool execution: `self_repair.on_tool_result(tool_name, &result)`
- Before tool execution: filter out tools in `get_excluded_tools()`
- Loop detection: `stuck_detector.check(content_hash)` — if stuck, inject recovery message

### Perfection Loop

**Iteration 1:** Core tracking + detection
**Iteration 2:** Integration into agent loop + cargo check

### Estimated LOC: ~300

---

## 4. Hook/Lifecycle System

### Source: OpenClaw (25 typed hooks)

### Current State

`LoopDelegate` trait has 5 methods. WASM plugins have `execute_before_llm_call` hook. No runtime extensibility.

### Implementation Plan

**File: NEW `crates/core/src/hooks/mod.rs` (~400 LOC)**

```rust
pub enum HookEvent {
    BeforeToolCall, AfterToolCall,
    LlmInput, LlmOutput,
    SessionStart, SessionEnd,
    AgentEnd,
}

pub struct HookRegistry {
    handlers: HashMap<HookEvent, Vec<HookRegistration>>,
}

impl HookRegistry {
    pub fn register_void(&mut self, event: HookEvent, handler: impl VoidHookHandler + 'static);
    pub async fn run_void(&self, event: HookEvent, payload: &HookPayload);
}
```

**Integration:**
- `AgentLoop` gets `hooks: Arc<HookRegistry>`
- Call sites in stream.rs: before/after tool execution, before/after LLM call

### Perfection Loop

**Iteration 1:** Core types + registry
**Iteration 2:** Integration into agent loop + cargo check

### Estimated LOC: ~400

---

## 5. Output Truncation + Execution Timeouts

### Source: NanoBot (truncation caps + configurable timeouts)

### Current State

No tool output size limits. No timeout enforcement on tool execution.

### Implementation Plan

**File: NEW `crates/agent/src/tools/limits.rs` (~130 LOC)**

```rust
pub const DEFAULT_MAX_OUTPUT: usize = 16_000;
pub const SHELL_MAX_OUTPUT: usize = 10_000;
pub const FILE_READ_MAX_OUTPUT: usize = 128_000;
pub const WEB_FETCH_MAX_OUTPUT: usize = 50_000;

pub fn truncate_output(output: &str, max_chars: usize) -> String { ... }
```

**Tool trait additions:**
- `max_output_chars() -> usize` (default: 16000)
- `timeout_secs() -> u64` (default: 60)

**Integration in reactor.rs:**
- After `tool.execute()`: `truncate_output(&result, tool.max_output_chars())`
- Before `tool.execute()`: `tokio::time::timeout(Duration::from_secs(tool.timeout_secs()), ...)`

### Perfection Loop

**Iteration 1:** Core limits + trait methods + integration + cargo check

### Estimated LOC: ~130

---

## 6. Mount Security

### Source: NanoClaw `mount-security.ts` (419 LOC)

### Current State

Docker tool executor mounts workspace directly. No blocked patterns, no allowlist.

### Implementation Plan

**File: NEW `crates/sandbox/src/mount_security.rs` (~200 LOC)**

```rust
const BLOCKED_PATTERNS: &[&str] = &[
    ".ssh", ".gnupg", ".aws", ".azure", ".gcloud", ".kube", ".docker",
    "credentials", ".env", ".netrc", ".npmrc", ".pypirc",
    "id_rsa", "id_ed25519", "id_ed448", "private_key", ".secret",
];

pub fn validate_mount_source(host_path: &Path, allowlist: &MountAllowlist) -> Result<(), MountError>;
pub struct MountAllowlist { allowed_paths: Vec<PathBuf> }
```

**Integration:**
- `DockerSkillExecutor` calls `validate_mount_source()` before mounting

### Perfection Loop

**Iteration 1:** Core validation + allowlist + cargo check

### Estimated LOC: ~200

---

## Implementation Order

| # | Feature | LOC | Dependencies | Sprint |
|---|---------|-----|-------------|--------|
| 1 | MCP Agent Loop Integration | ~230 | None | Immediate |
| 2 | Smithery CLI + Dashboard | ~650 | MCP integration | Immediate |
| 3 | Self-Repair | ~300 | Session model (done) | Next |
| 4 | Hook/Lifecycle System | ~400 | None | Next |
| 5 | Output Truncation + Timeouts | ~130 | None | Next |
| 6 | Mount Security | ~200 | None | Next |

**Total: ~1,910 LOC across 6 features**

---

## Dashboard MCP Page Wireframe

```
┌─────────────────────────────────────────────┐
│  MCP Server Management                      │
├─────────────────────────────────────────────┤
│  [Install from Smithery]                    │
│  ┌─────────────────────────────────────┐    │
│  │ Search: [____________________] 🔍   │    │
│  │                                     │    │
│  │ ├── filesystem-server   [Install]   │    │
│  │ ├── github-mcp          [Install]   │    │
│  │ ├── postgres-mcp        [Install]   │    │
│  │ └── slack-mcp           [Install]   │    │
│  └─────────────────────────────────────┘    │
│                                             │
│  Installed Servers                          │
│  ┌─────────────────────────────────────┐    │
│  │ ● filesystem  Connected  5 tools    │    │
│  │   [Disable] [Test] [Uninstall]      │    │
│  │                                      │    │
│  │ ○ github      Disconnected  3 tools │    │
│  │   [Enable] [Test] [Uninstall]       │    │
│  └─────────────────────────────────────┘    │
│                                             │
│  Discovered Tools (8)                       │
│  ┌─────────────────────────────────────┐    │
│  │ filesystem/read_file    ✅ Active    │    │
│  │ filesystem/write_file   ✅ Active    │    │
│  │ filesystem/list_dir     ✅ Active    │    │
│  │ github/create_issue     ❌ Disabled  │    │
│  └─────────────────────────────────────┘    │
└─────────────────────────────────────────────┘
```

---

*FID created 2026-03-21. Ready for Perfection Loop on each feature.*

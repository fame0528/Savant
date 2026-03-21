# ZeroClaw Framework Review

> **Repository:** github.com/zeroclaw-labs/zeroclaw
> **Language:** Rust
> **Position:** OpenClaw competitor with deep security focus

---

## What Makes It Special

1. **7+ tool call format parser** — XML, JSON, MiniMax, Perl, FunctionCall, GLM, attribute-style. Most comprehensive tool parsing of any framework.
2. **70+ built-in tools** — shell, file, web, browser, Jira, Notion, Google Workspace, Microsoft 365, LinkedIn, Composio, cron, delegate, swarm
3. **WASM plugin system** — extism-based plugins for dynamic tool extension
4. **Hands system** — TOML-defined autonomous agent swarms with knowledge accumulation
5. **Skills platform** — Install from git, audit before install, create skills autonomously
6. **Hardware peripherals** — STM32, RPi GPIO, ESP32 integration
7. **Trait-driven architecture** — Every subsystem is a trait with multiple implementations
8. **Observability** — Prometheus, OpenTelemetry, runtime traces

---

## Architecture

- Single Rust binary + `robot-kit` workspace crate
- Trait-driven: Provider, Channel, Tool, Memory, Observer, RuntimeAdapter, Peripheral
- CLI: onboard, agent, gateway, daemon, service, doctor, status, cron, channel, skills, memory, auth, hardware, migrate, update, estop, plugin
- Axum HTTP/WebSocket gateway
- Multiple memory backends: SQLite, Markdown, PostgreSQL, mem0

---

## Key Innovations For Savant

### A. Multi-Format Tool Parser
The most important takeaway. ZeroClaw parses 7+ tool call formats. Savant currently only parses `Action: ToolName[args]`. **This is why our tools fail.**

### B. Tool Name Aliasing
```rust
fn map_tool_name_alias(name: &str) -> &str {
    match name {
        "bash" | "sh" | "exec" | "command" | "cmd" => "shell",
        "fileread" | "readfile" | "read_file" | "file" => "file_read",
        ...
    }
}
```

### C. Credential Scrubbing
Tool outputs are scrubbed to prevent credential leakage before feeding back to LLM.

### D. Tool Deduplication
Canonical signature (tool name + sorted arguments) prevents duplicate execution.

### E. Max Tool Iterations
Configurable limit (default 10) prevents runaway tool loops.

---

## What Savant Does Better

| Area | Savant |
|------|--------|
| **Dashboard** | ZeroClaw has no web UI. |
| **Speculative execution** | Not present in ZeroClaw. |
| **Cognitive layer** | No synthesis/prediction in ZeroClaw. |
| **Canvas system** | Not present in ZeroClaw. |
| **Desktop app** | Tauri desktop vs CLI only. |
| **Memory sophistication** | ZeroClaw has SQLite/Markdown. Savant has LSM + vector + entity extraction. |
| **Security enclaves** | ZeroClaw has policy + pairing. Savant has WASM + attestation. |

---

## What We're Missing From ZeroClaw

1. Multi-format tool call parser (CRITICAL)
2. Tool name aliasing
3. Credential scrubbing on tool output
4. Tool deduplication
5. Max tool iterations config
6. 70+ built-in tools
7. WASM plugin system
8. Hands (autonomous agent swarms)
9. Skills audit before install
10. Hardware peripheral support
11. Prometheus/OpenTelemetry observability

---

## Key Takeaway

ZeroClaw's tool system is the most mature. The multi-format parser is THE feature we need immediately. The rest (Hands, WASM plugins, observability) are medium-term targets.

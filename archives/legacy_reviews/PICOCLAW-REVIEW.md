# PicoClaw Framework Review

> **Repository:** github.com/sipeed/picoclaw
> **Language:** Go
> **Position:** Ultra-lightweight edge/embedded AI agent

---

## What Makes It Special

1. **<10MB RAM, <1s boot** — Runs on $10 hardware (RISC-V, ARM, MIPS)
2. **Hardware I2C/SPI bus tools** — Direct hardware communication
3. **15+ messaging channels** — Including Chinese platforms (QQ, DingTalk, WeCom, Feishu)
4. **TTL-based tool visibility** — Tools auto-hide after N rounds, reappear when contextually relevant
5. **Dual-channel tool results** — `ForLLM` vs `ForUser` separation
6. **Model routing with complexity scoring** — Route simple queries to cheap models
7. **Async tool pattern** — Callback-as-parameter eliminates race conditions
8. **Subagent tool isolation via Clone()** — Prevents recursive spawning
9. **Sorted tool names** for deterministic KV cache stability
10. **Vision pipeline** — Automatic base64 encoding for multimodal LLMs
11. **Cross-architecture binaries** — RISC-V, ARM, MIPS, LoongArch, x86
12. **Skills marketplace (ClawHub)** — Discover and install skills mid-session

---

## Architecture

- Standard Go project layout: `cmd/` + `pkg/`
- 28 packages in `pkg/`
- Interface-driven: Tool, Store, Channel, Provider
- Cobra CLI framework
- JSON config with zero external config library
- Message-bus-driven agent loop

---

## Key Innovations For Savant

### A. TTL-Based Tool Visibility
Hidden tools only visible when TTL > 0. TickTTL decrements per tool-call round. Tools auto-discover and auto-hide. This prevents system prompt bloat.

### B. Dual-Channel Tool Results
```rust
ToolResult {
    ForLLM: "content for LLM context",
    ForUser: "content for user display",
    Silent: true,  // suppresses user output
}
```

### C. Model Routing
Rule-based complexity classifier. Score 0-1. Default threshold 0.35. Simple queries → cheap model. Complex queries → primary model. 50-70% cost reduction.

### D. Force Compression
On context window errors, drops oldest 50% of messages. Graceful degradation instead of failure.

### E. Provider-Native Search
When `tools.web.prefer_native=true`, strips client-side web_search and delegates to provider's built-in search.

---

## What Savant Does Better

| Area | Savant |
|------|--------|
| **Memory** | LSM + vector + entity extraction vs JSONL flat files |
| **Security** | WASM + enclaves vs workspace restriction |
| **Dashboard** | Web UI vs system tray only |
| **Cognitive layer** | Synthesis, prediction, forge vs model routing only |
| **Speculative execution** | Not present in PicoClaw |
| **Canvas system** | Not present in PicoClaw |
| **Desktop app** | Tauri vs web console |
| **Swarm orchestration** | Built-in vs spawn tool |
| **Learning system** | ALD vs none |

---

## What We're Missing From PicoClaw

1. TTL-based tool visibility (prevent system prompt bloat)
2. Dual-channel tool results (ForLLM/ForUser)
3. Model routing with complexity scoring (cost optimization)
4. Force compression on context errors
5. Provider-native search delegation
6. Sorted tool names for KV cache stability
7. Subagent tool isolation via Clone()
8. 15+ channels including Chinese platforms
9. Hardware integration (I2C/SPI)
10. Cross-architecture binary support
11. Vision pipeline (automatic image encoding)

---

## Key Takeaway

PicoClaw's edge is extreme efficiency and hardware integration. For Savant, the most adoptable patterns are TTL-based tool visibility, dual-channel results, and model routing for cost optimization. The hardware integration is a different market but worth tracking.

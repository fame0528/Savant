# IronClaw Framework Review

> **Repository:** github.com/nearai/ironclaw
> **Language:** Rust
> **Position:** Security-first Rust AI agent framework

---

## What Makes It Special

1. **LoopDelegate trait** — Single agentic loop engine with pluggable strategy. Clean separation of loop logic from I/O.
2. **WASM sandbox with leak detection** — Capability-based permissions, endpoint allowlisting, credential injection at host boundary, request/response scanning for secret exfiltration.
3. **Smart model routing** — 13-dimension complexity scorer. Flash/Standard/Pro/Frontier tiers. Cascade mode tries cheap first, escalates on uncertainty.
4. **Dynamic tool building** — Agents describe what they need, framework builds WASM tools on the fly.
5. **Skills trust model** — Trusted (user-placed) vs Installed (registry). Attenuation restricts tool access for lower-trust skills.
6. **Self-repair system** — Stuck job detection, broken tool rebuilding via SoftwareBuilder.
7. **6 lifecycle hooks** — BeforeInbound, BeforeToolCall, BeforeOutbound, OnSessionStart, OnSessionEnd, TransformResponse. Fail-open, priority-ordered.
8. **EMA learning** — Cost/time estimation that improves over time via Exponential Moving Average.
9. **Hybrid search** — BM25 + vector via Reciprocal Rank Fusion.
10. **Circuit breaker** — Closed → Open → HalfOpen state machine for LLM provider resilience.
11. **Psychographic profiling** — Interaction style directives auto-injected into system prompt.
12. **Context compaction** — 3 strategies: MoveToWorkspace, Summarize, Truncate based on token usage.

---

## Architecture

- Rust monorepo: main crate + ironclaw_safety subcrate
- wasmtime 28 with component model for WASM sandbox
- Dual database: PostgreSQL (pgvector) + libSQL/Turso (embedded)
- Tokio async runtime
- bollard for Docker API, aes-gcm for secrets, ed25519-dalek for webhooks

---

## Key Innovations For Savant

### A. LoopDelegate Trait
```rust
trait LoopDelegate {
    async fn check_signals() -> LoopSignal;
    async fn before_llm_call() -> Option<LoopOutcome>;
    async fn call_llm() -> Result<RespondOutput, Error>;
    async fn handle_text_response() -> TextAction;
    async fn execute_tool_calls() -> Result<Option<LoopOutcome>, Error>;
}
```
Single engine, multiple strategies. Chat, Job, Container delegates all share the same loop.

### B. Smart Model Routing (13 Dimensions)
Weights: reasoning_words(14%), token_estimate(12%), code_indicators(10%), multi_step(10%), domain_specific(10%), creativity(7%), safety_sensitivity(4%), ...
Expected: 50-70% cost reduction.

### C. ToolDomain Separation
```rust
enum ToolDomain { Orchestrator, Container }
```
Orchestrator = safe tools (no filesystem/network). Container = filesystem/shell (require approval).

### D. Skills Attenuation
When mixed-trust skills are active, tool access is restricted to the lowest-trust level. Prevents privilege escalation.

### E. Tool Intent Nudge
When LLM says "let me search..." without calling a tool, inject a nudge. Capped at 2 nudges per turn.

### F. Self-Repair
Background detection of stuck jobs + broken tools. Attempts recovery. Rebuilds broken tools via SoftwareBuilder.

---

## What Savant Does Better

| Area | Savant |
|------|--------|
| **Canvas system** | IronClaw lists Canvas as "not present" |
| **Desktop app** | IronClaw excludes desktop/mobile |
| **Panopticon** | More comprehensive observability than IronClaw's basic module |
| **Echo crate** | Dedicated diagnostic server |
| **IPC crate** | Dedicated inter-process communication |
| **Cognitive layer** | IronClaw has smart routing only. Savant has synthesis/prediction/forge. |
| **More modular** | 16 crates vs 2 crates — better separation of concerns |

---

## What We're Missing From IronClaw

1. LoopDelegate trait pattern (clean agent loop architecture)
2. Smart model routing with 13-dimension scorer (CRITICAL for cost)
3. WASM sandbox with leak detection
4. Dynamic tool building (agent describes → framework builds)
5. Skills trust model with attenuation
6. Self-repair for stuck jobs and broken tools
7. 6 lifecycle hooks (fail-open, priority-ordered)
8. EMA learning for cost/time estimation
9. Hybrid search (BM25 + vector RRF)
10. Circuit breaker for LLM providers
11. Context compaction (3 strategies)
12. Psychographic profiling
13. Tool rate limiting per user/tool/window
14. Parameter redaction for sensitive inputs
15. Credential injection at host boundary

---

## Key Takeaway

IronClaw is the most security-mature Rust framework. The smart routing, WASM sandbox, and skills trust model are directly adoptable. The LoopDelegate trait pattern would clean up Savant's agent loop architecture significantly.

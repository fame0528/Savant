# AI Framework Research — Feature Analysis & Adoption Candidates

> **Date:** 2026-03-20
> **Frameworks Analyzed:** ZeroClaw, NanoClaw, PicoClaw, NanoBot, IronClaw
> **Purpose:** Identify production-grade patterns Savant can adopt and make its own.

---

## Executive Summary

After analyzing 5 leading AI agent frameworks, clear patterns emerge across tool systems, memory architectures, security models, and agent loops. Savant's architecture is competitive but has specific gaps — most critically in tool calling, context management, and security boundaries.

---

## 1. Tool Systems

### What We Learned

| Framework | Tool Calling | Key Innovation |
|-----------|-------------|----------------|
| **ZeroClaw** | Dual-mode (native + XML tags) | 7+ format parser: XML, JSON, MiniMax, Perl, GLM, attribute-style |
| **NanoClaw** | MCP-based + Claude Agent SDK | File-based IPC between container and host |
| **PicoClaw** | OpenAI native + TTL-based visibility | ForLLM vs ForUser dual-channel results |
| **NanoBot** | OpenAI function calling + tool hints | Virtual tool-call pattern for structured decisions |
| **IronClaw** | Trait-based with approval gates | ToolDomain separation, sensitive_params redaction |

### Recommended Patterns for Savant

**A. Multi-Format Parser (ZeroClaw)**
- Stop stripping XML tags from output — parse them into actions
- Add native function calling extraction from provider response
- Support: XML tags, native JSON, Action format, attribute-style XML
- Tool name aliasing: normalize "bash" to "shell", "fileread" to "file_read"

**B. Dual-Channel Tool Results (PicoClaw)**
- ForLLM: content sent back to the LLM for context
- ForUser: content sent directly to user (bypasses LLM)
- Silent: suppresses user-facing output

**C. Virtual Tool-Call Pattern (NanoBot)**
- Use tool calls for structured LLM decisions instead of free-text parsing
- Heartbeat decides skip/run via structured tool call, not prompt parsing
- Post-run evaluation gate: LLM evaluates if result warrants notification

**D. TTL-Based Tool Visibility (PicoClaw)**
- Hidden tools only visible when TTL > 0
- TickTTL decrements per tool-call round
- Dynamic tool discovery without bloating system prompt

**E. Tool Deduplication (ZeroClaw)**
- Canonical signature (tool name + sorted arguments)
- Prevents duplicate tool calls from being executed

---

## 2. Memory Systems

### What We Learned

| Framework | Memory | Key Innovation |
|-----------|--------|----------------|
| **ZeroClaw** | ConversationMessage enum | Preserves reasoning_content in history |
| **NanoClaw** | Hierarchical CLAUDE.md | Global/group/file levels, no vector DB |
| **PicoClaw** | Append-only JSONL | Crash-safe, sharded locking, logical truncation |
| **NanoBot** | MEMORY.md + HISTORY.md | Token-based consolidation, not message-count |
| **IronClaw** | Hybrid search (FTS + vector RRF) | 800-word chunks, 15% overlap, RRF scoring |

### Recommended Patterns for Savant

**A. Token-Based Context Management (NanoBot)**
- Replace message-count limits with token estimation
- Consolidation boundary chosen at user-turn boundaries (preserves tool-call chains)
- Force compression on context errors instead of crashing

**B. Append-Only JSONL Session Store (PicoClaw)**
- Never physically delete messages (append-only writes)
- Logical truncation via skip offset in metadata
- Fsync on every append, atomic metadata writes via temp+rename
- Sharded locking: 64 fixed mutexes via FNV hash

**C. Hybrid Search with RRF (IronClaw)**
- Full-text search + vector similarity using Reciprocal Rank Fusion
- score(d) = Sum of 1/(k + rank(d)) for each method
- Proven pattern for combining keyword and semantic search

**D. ConversationMessage Enum (ZeroClaw)**
- Preserve reasoning_content in conversation history
- Different message types: Chat, AssistantToolCalls, ToolResults
- Maintains round-trip fidelity for native function calling

---

## 3. Security Models

### What We Learned

| Framework | Security | Key Innovation |
|-----------|----------|----------------|
| **ZeroClaw** | Container isolation | Mount allowlist, sender allowlist, read-only project root |
| **NanoClaw** | Credential proxy | Real keys NEVER enter containers, HTTP proxy injects auth |
| **PicoClaw** | Workspace restriction | regex-based allow/read/write paths, exec deny patterns |
| **IronClaw** | WASM sandbox | Allowlist validator, credential injection at host boundary |

### Recommended Patterns for Savant

**A. Credential Proxy (NanoClaw)**
- Real API keys never enter agent environment
- HTTP proxy on port 3001 injects auth headers transparently
- Eliminates entire class of credential leakage
- Savant already has a master key system — this would complement it

**B. External Tamper-Proof Security Config (NanoClaw)**
- Security rules stored at ~/.config/savant/ — outside project root
- Never mounted into containers/agent environments
- Agents cannot modify their own security rules

**C. WASM Sandbox Allowlist (IronClaw)**
- Host matching with wildcard support
- Path prefix matching, HTTP method restrictions
- HTTPS-only enforcement
- URL userinfo rejection (prevents credential-based bypass)
- Path traversal blocking

**D. ToolDomain Separation (IronClaw)**
- Orchestrator: safe tools (no filesystem/network)
- Container: filesystem/shell tools (require approval)
- Clear security boundary based on tool risk level

---

## 4. Agent Loops

### What We Learned

| Framework | Loop Pattern | Key Innovation |
|-----------|-------------|----------------|
| **ZeroClaw** | Multi-format parse + dispatch | Credential scrubbing, dedup |
| **NanoClaw** | Polling + container spawn | MessageStream push-based AsyncIterable |
| **PicoClaw** | Message-bus-driven | Parallel tool execution, model routing |
| **NanoBot** | ReAct with progress streaming | Tool hints during execution |
| **IronClaw** | LoopDelegate trait | Single engine, strategy-based customization |

### Recommended Patterns for Savant

**A. LoopDelegate Trait (IronClaw)**
- Single agentic loop engine with pluggable strategy
- check_signals, before_llm_call, call_llm, handle_text_response, execute_tool_calls
- Clean separation of loop logic from I/O

**B. Parallel Tool Execution (PicoClaw)**
- Execute independent tool calls concurrently
- Significant latency reduction for multi-tool responses
- Savant already has DAG-based parallel execution — can be extended

**C. Tool Intent Nudge (IronClaw)**
- When LLM says "let me search..." without calling a tool, inject a nudge
- Capped at 2 nudges per turn
- Improves tool utilization without being aggressive

**D. Progress Streaming (NanoBot)**
- Stream thought content and tool hints during execution
- Better UX than waiting for complete response
- Tool hint: 'web_search("query")' shown to user while executing

---

## 5. Unique Features Worth Adopting

### From NanoClaw
1. **Self-registering plugin factory** — Plugins register at module load
2. **Conversation archive on compaction** — Pre-compact hook saves transcript as Markdown
3. **Skills-over-features** — Core stays minimal, capabilities contributed as mergeable branches
4. **Idle timeout with sentinel shutdown** — _close file signals container wind-down

### From PicoClaw
1. **Model routing with complexity scoring** — Route simple queries to cheaper models
2. **Subagent tool isolation via Clone()** — Prevents recursive spawning
3. **Sorted tool names** for deterministic KV cache stability
4. **Force compression on context errors** — Graceful degradation

### From NanoBot
1. **Post-run evaluation gate** — LLM evaluates if result warrants notification
2. **Legal tool-call boundary detection** — Prevent orphan tool results in history
3. **Error poisoning prevention** — Don't persist error responses to session
4. **Progressive skill loading** — Summary in prompt, full content on-demand

### From IronClaw
1. **Smart model routing** — 13-dimension complexity scorer, 50-70% cost reduction
2. **Dynamic tool building** — Agents describe what they need, framework builds WASM tool
3. **Skills trust model** — Trusted (user-placed) vs Installed (registry), attenuation by trust
4. **Self-repair** — Stuck job detection, broken tool rebuilding
5. **6 lifecycle hooks** — BeforeInbound, BeforeToolCall, BeforeOutbound, OnSessionStart/End, TransformResponse
6. **Estimation with EMA learning** — Cost/time prediction that improves over time

---

## 6. Priority Adoption Roadmap

### Immediate (This Sprint)
1. Fix tool call parser — stop stripping, start parsing
2. Add native function calling support
3. Tool deduplication by signature
4. Token-based context management

### Short-term (Next Sprint)
1. Credential proxy pattern
2. Dual-channel tool results (ForLLM/ForUser)
3. Virtual tool-call pattern for heartbeat
4. Tool intent nudge

### Medium-term
1. Smart model routing with complexity scoring
2. Hybrid search (FTS + vector RRF)
3. WASM sandbox with allowlist
4. Lifecycle hooks system

### Long-term
1. Dynamic tool building
2. Skills trust model with attenuation
3. Self-repair for stuck jobs and broken tools
4. Estimation with EMA learning

---

*Research compiled from source analysis of all five frameworks. Each pattern evaluated for compatibility with Savant's Rust architecture and existing systems.*

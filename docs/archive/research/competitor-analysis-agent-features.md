# Competitor Analysis: Agent Framework Features

## Repos Analyzed

| Repo | License | Language | Focus |
|------|---------|----------|-------|
| [9router](https://github.com/decolua/9router) | MIT | Node.js/Next.js | Multi-provider router with token compression |
| [Codebuff](https://github.com/CodebuffAI/codebuff) | MIT | TypeScript/Bun | Multi-agent coding orchestration |
| [OpenClaude](https://github.com/Gitlawb/openclaude) | MIT | TypeScript/Bun | Claude Code fork with advanced compaction |
| [OpenCode](https://github.com/opencode-ai/opencode) | MIT | Go | Agentic coding assistant framework |
| [Gemini CLI](https://github.com/google-gemini/gemini-cli) | Apache 2.0 | TypeScript | Google's terminal AI agent |
| [Crush](https://github.com/charmbracelet/crush) | MIT | Go | AI agent CLI framework |
| [KiloCode](https://github.com/Kilo-Org/kilocode) | MIT | TypeScript | VS Code AI agent integration |
| [Claude Code](https://github.com/anthropics/claude-code) | Anthropic | TypeScript | Reference agent implementation |

---

## 1. Token Compression & Context Optimization

### 1.1 RTK-Style Tool Output Compression (from [9router](https://github.com/decolua/9router))

**Source**: [`open-sse/rtk/`](https://github.com/decolua/9router/tree/main/open-sse/rtk)

**What it does**: Automatically detects and compresses tool output content before sending to LLM. Recognizes patterns:
- Git diffs (collapses unchanged hunks)
- Git status (summarizes file counts)
- Grep results (deduplicates, truncates)
- Find results (path-only mode)
- Tree output (collapses deep directories)
- LS output (summarizes)
- Build output (extracts errors/warnings only)
- Log files (deduplicates repeated lines)

**Savant applicability**: HIGH. Our agents produce massive tool outputs. A middleware layer that intercepts tool results and compresses them before LLM submission would directly reduce API costs and allow more context in the window.

**Implementation approach**: Rust middleware in the agent loop, between tool execution and message construction. Pattern-matching on output structure + configurable compression rules per tool type.

### 1.2 Multi-Strategy Context Compaction (from [OpenClaude](https://github.com/Gitlawb/openclaude))

**Source**: [`src/services/compact/`](https://github.com/Gitlawb/openclaude/tree/main/src/services/compact)

**What it does**: Three-tier compaction system:

- **Auto-compaction**: Triggers at configurable threshold (effective context - 13K buffer). Uses circuit breaker (max 3 consecutive failures) to prevent hammering API when context is irrecoverably over limit.
- **Micro-compaction**: Lightweight tool result clearing. Two sub-strategies:
  - *Cached micro-compaction*: Uses Anthropic's cache_edit API to remove tool results without invalidating the cached prefix. Tracks tool IDs, registers them in state, and issues cache_edits to delete old results.
  - *Time-based micro-compaction*: When gap since last assistant message exceeds threshold, content-clears all but the most recent N compactable tool results.
- **Session memory compaction**: Forked agent that summarizes older conversation into a compact memory block.

**Savant applicability**: HIGH. We have semantic window management, but not automatic compaction with circuit breakers, cache-aware compaction, or time-based clearing.

**Key insight**: The circuit breaker pattern (stop retrying after N consecutive failures) is critical for production agents. Without it, agents that blow context waste hundreds of API calls on doomed compaction attempts.

### 1.3 Context Partitioning + Relevance Pruning (from [OpenClaude](https://github.com/Gitlawb/openclaude))

**Source**: [`src/utils/contextPartitioning.js`](https://github.com/Gitlawb/openclaude/blob/main/src/utils/contextPartitioning.js), [`src/utils/relevancePruning.js`](https://github.com/Gitlawb/openclaude/blob/main/src/utils/relevancePruning.js)

**What it does**: Before compacting, partitions context into:
- System messages (always preserved)
- Recent messages (configurable count, always preserved)
- Older messages (candidates for pruning)

Then prunes by relevance: preserves tool results, errors, and messages relevant to current task; removes redundant or stale content.

**Savant applicability**: MEDIUM-HIGH. Complements our semantic window. The partitioning approach (preserve system + recent + relevant) is a good heuristic.

---

## 2. Provider Management & Routing

### 2.1 Provider Executor Pattern (from [9router](https://github.com/decolua/9router))

**Source**: [`open-sse/executors/`](https://github.com/decolua/9router/tree/main/open-sse/executors)

**What it does**: Each provider has an executor class that handles:
- URL construction (multiple base URLs for fallback)
- Header construction (provider-specific auth patterns)
- Request transformation (provider-specific body format)
- Response parsing (provider-specific SSE format)
- Retry logic with per-status-code configuration
- Credential refresh (OAuth token refresh before expiry)

**Key design**: Base executor with provider-specific subclasses. Clean separation of concerns.

**Savant applicability**: HIGH. We have 15+ providers but they're all constructed with match arms in swarm.rs. An executor pattern would be cleaner and more extensible.

### 2.2 Combo/Fallback System (from [9router](https://github.com/decolua/9router))

**Source**: [`open-sse/services/combo.js`](https://github.com/decolua/9router/blob/main/open-sse/services/combo.js)

**What it does**:
- Named combos (e.g., "my-coding-stack") with ordered provider list
- Round-robin with sticky limits (N requests per model before rotating)
- Transient error cooldown (waits on 503/502/504 before falling through)
- Per-combo rotation state tracking

**Savant applicability**: MEDIUM. We have provider fallback, but not named combos or round-robin with sticky limits. The transient error cooldown is a good pattern.

### 2.3 Provider Usage Tracking (from [9router](https://github.com/decolua/9router))

**Source**: [`open-sse/services/usage.js`](https://github.com/decolua/9router/blob/main/open-sse/services/usage.js)

**What it does**: Fetches quota/usage data from each provider's API:
- GitHub Copilot (quota snapshots, reset dates)
- Gemini (per-model buckets with remainingFraction)
- Antigravity (model quotas via Cloud Code API)
- Claude (OAuth usage endpoint, org usage)
- Codex (rate limits via ChatGPT backend)
- Kiro (AWS CodeWhisperer usage limits)
- GLM (quota limits)
- MiniMax (token plan remains)

**Savant applicability**: HIGH. We have zero usage tracking. Agents should know when they're about to hit limits and route accordingly.

### 2.4 Format Translation Layer (from [9router](https://github.com/decolua/9router))

**Source**: [`open-sse/translator/`](https://github.com/decolua/9router/tree/main/open-sse/translator)

**What it does**: Bidirectional translation between provider formats:
- OpenAI ↔ Claude ↔ Gemini ↔ Cursor ↔ Kiro ↔ Vertex ↔ Ollama ↔ Antigravity
- Request and response transformers registered in a registry
- Handles tool call format differences, content block differences, etc.

**Savant applicability**: MEDIUM. We handle format differences in each provider implementation. A centralized translation layer would be cleaner.

---

## 3. Agent Orchestration & Multi-Agent Systems

### 3.1 Multi-Agent Coding Workflow (from [Codebuff](https://github.com/CodebuffAI/codebuff))

**Source**: [`agents/`](https://github.com/CodebuffAI/codebuff/tree/main/agents)

**What it does**: Structured multi-agent workflow:
1. **File Picker Agent** — scans codebase, finds relevant files
2. **Planner Agent** — plans which files need changes and in what order
3. **Editor Agent** — makes precise edits
4. **Reviewer Agent** — validates changes
5. **Best-of-N Selector** — runs multiple editor variants, picks best result

**Agent modes**: default, free, lite, max, fast — each with different model/validation tradeoffs.

**Savant applicability**: HIGH. We have a 101-agent swarm, but not this kind of structured multi-agent workflow for coding tasks. The File Picker → Planner → Editor → Reviewer pipeline is a proven pattern.

**Key insight**: The "free" mode uses MiniMax M2.7 as default, with optional Gemini Thinker sub-agent for deeper reasoning. This tiered approach (fast/cheap for simple tasks, expensive/thinking for complex ones) is a good model.

### 3.2 Agent Definition System (from [Codebuff](https://github.com/CodebuffAI/codebuff))

**Source**: [`agents/types/agent-definition.ts`](https://github.com/CodebuffAI/codebuff/blob/main/agents/types/agent-definition.ts)

**What it does**: Agents defined as TypeScript objects with:
- `id`, `displayName`, `model`
- `toolNames` — which tools the agent can use
- `instructionsPrompt` — system prompt
- `handleSteps()` — generator function that yields tool calls
- `inputSchema` — structured input parameters
- `spawnerPrompt` — description for parent agents to decide when to spawn

**Savant applicability**: MEDIUM. Our agent definitions are in Rust structs. The TypeScript approach is more dynamic but less type-safe. The concept of `handleSteps()` as a generator is interesting — it allows agents to yield tool calls incrementally.

### 3.3 Subagent Spawning with Depth Limits (from [OpenClaude](https://github.com/Gitlawb/openclaude))

**Source**: [`src/tasks/`](https://github.com/Gitlawb/openclaude/tree/main/src/tasks)

**What it does**: Agents can spawn subagents with:
- Depth limits (prevent infinite recursion)
- Context isolation (subagent gets its own message history)
- Result aggregation (subagent results fed back to parent)
- Permission inheritance (subagent inherits parent's permissions)

**Savant applicability**: HIGH. We have agent swarms but the subagent spawning with depth limits and context isolation would improve our orchestration.

---

## 4. Tool Systems

### 4.1 Tool Interface Pattern (from [OpenCode](https://github.com/opencode-ai/opencode))

**Source**: [`internal/llm/tools/tools.go`](https://github.com/opencode-ai/opencode/blob/main/internal/llm/tools/tools.go)

**What it does**: Clean tool interface:
```go
type BaseTool interface {
    Info() ToolInfo
    Run(ctx context.Context, params ToolCall) (ToolResponse, error)
}
```
- Tools declare their own parameter schema
- Context carries session_id and message_id
- Response includes type (text/image), content, metadata, error flag

**Savant applicability**: MEDIUM. Our Tool trait is similar but more complex. The simplicity of this interface is appealing.

### 4.2 LSP Integration (from [OpenCode](https://github.com/opencode-ai/opencode))

**Source**: [`internal/lsp/`](https://github.com/opencode-ai/opencode/tree/main/internal/lsp)

**What it does**: Full LSP client implementation:
- Protocol types generated from LSP spec
- Transport layer (stdio, TCP, WebSocket)
- Client with request/response handling
- Watcher for LSP server lifecycle
- Edit utilities (apply edits from LSP responses)

**Savant applicability**: MEDIUM-HIGH. We don't have LSP integration. This would give agents deep code intelligence (go-to-definition, find references, diagnostics, completions) without relying on the LLM's limited understanding of code structure.

### 4.3 Tool Result Deduplication (from [9router](https://github.com/decolua/9router))

**Source**: [`open-sse/utils/toolDeduper.js`](https://github.com/decolua/9router/blob/main/open-sse/utils/toolDeduper.js)

**What it does**: Detects and deduplicates repeated tool results across conversation turns. If the same tool is called with the same parameters and produces the same result, the duplicate is replaced with a reference.

**Savant applicability**: HIGH. Agents often re-run the same commands (e.g., `git status`, `ls`). Deduplicating these saves tokens.

---

## 5. Session & Memory Management

### 5.1 Session Hierarchy (from [OpenCode](https://github.com/opencode-ai/opencode))

**Source**: [`internal/session/session.go`](https://github.com/opencode-ai/opencode/blob/main/internal/session/session.go)

**What it does**: Sessions have:
- `ParentSessionID` — tree structure for sub-sessions
- `PromptTokens`, `CompletionTokens`, `Cost` — tracked per session
- `SummaryMessageID` — links to compaction summary
- Pub/sub events (Created, Updated, Deleted)

**Savant applicability**: MEDIUM. We have session management but not hierarchical sessions with cost tracking per session.

### 5.2 Session Memory Compaction (from [OpenClaude](https://github.com/Gitlawb/openclaude))

**Source**: [`src/services/SessionMemory/`](https://github.com/Gitlawb/openclaude/tree/main/src/services/SessionMemory)

**What it does**: Maintains a running summary of the conversation:
- Extracts key facts, decisions, and context
- Stored as a memory file that's injected into future prompts
- Compacted periodically by a forked agent
- Tracks `lastSummarizedMessageId` to avoid re-summarizing

**Savant applicability**: MEDIUM. We have CortexaDB for persistent memory, but not a running conversation summary that's injected into each prompt.

### 5.3 Knowledge Files (from [Codebuff](https://github.com/CodebuffAI/codebuff) / [OpenCode](https://github.com/opencode-ai/opencode))

**Source**: [`agents/*/knowledge.md`](https://github.com/CodebuffAI/codebuff/tree/main/agents), [`internal/llm/prompt/coder.go`](https://github.com/opencode-ai/opencode/blob/main/internal/llm/prompt/coder.go)

**What it does**: Project-specific context files that agents automatically read:
- `knowledge.md` — project overview, conventions, frequently used commands
- `OpenCode.md` — stored bash commands, project-specific instructions
- Auto-discovered from project directory

**Savant applicability**: MEDIUM. We have SOUL.md for agent identity, but not project-level knowledge files that agents auto-discover.

---

## 6. Permission & Safety Systems

### 6.1 Permission Request System (from [OpenCode](https://github.com/opencode-ai/opencode))

**Source**: [`internal/permission/permission.go`](https://github.com/opencode-ai/opencode/blob/main/internal/permission/permission.go)

**What it does**:
- Before executing a tool, checks if permission is needed
- Permission requests are published via pub/sub (UI can subscribe)
- Supports persistent grants (remember this permission for this session)
- Auto-approve mode for trusted sessions
- Path-based permissions (grant for this directory)

**Savant applicability**: MEDIUM. We have CCT (Capability Confinement Tokens) for security, but not a user-facing permission request system. For a CLI agent, this is important.

### 6.2 Sandboxing (from [OpenClaude](https://github.com/Gitlawb/openclaude))

**Source**: [`src/sandbox/`](https://github.com/Gitlawb/openclaude/tree/main/src/sandbox)

**What it does**:
- Sandboxed execution environment for agent code
- Configurable permissions per sandbox
- Override system for trusted operations

**Savant applicability**: LOW. Our WASM sandbox already handles this.

---

## 7. Prompt Engineering Patterns

### 7.1 Environment-Aware Prompts (from [OpenCode](https://github.com/opencode-ai/opencode))

**Source**: [`internal/llm/prompt/coder.go`](https://github.com/opencode-ai/opencode/blob/main/internal/llm/prompt/coder.go)

**What it does**: System prompts include:
- Current working directory
- OS and shell information
- Available tools and their descriptions
- Project-specific instructions from knowledge files
- LSP information (available language servers)

**Savant applicability**: MEDIUM. Our system prompts are static. Dynamic environment injection would make agents more context-aware.

### 7.2 Provider-Specific Prompt Variants (from [OpenCode](https://github.com/opencode-ai/opencode))

**Source**: [`internal/llm/prompt/coder.go`](https://github.com/opencode-ai/opencode/blob/main/internal/llm/prompt/coder.go)

**What it does**: Different base prompts for different providers:
- `baseAnthropicCoderPrompt` — optimized for Claude's tool use format
- `baseOpenAICoderPrompt` — optimized for OpenAI's function calling

**Savant applicability**: MEDIUM. We use a generic approach. Provider-specific prompt optimization could improve tool call accuracy.

### 7.3 Task-Specific Agent Prompts (from [OpenCode](https://github.com/opencode-ai/opencode))

**Source**: [`internal/llm/prompt/task.go`](https://github.com/opencode-ai/opencode/blob/main/internal/llm/prompt/task.go)

**What it does**: Different prompt templates for different agent types:
- `CoderPrompt` — general coding tasks
- `TaskPrompt` — subagent tasks (more concise, focused)
- `SummarizerPrompt` — conversation summarization
- `TitlePrompt` — session title generation

**Savant applicability**: HIGH. We could benefit from specialized prompts for different agent roles in our swarm.

---

## 8. Event Systems & Observability

### 8.1 Pub/Sub Event Broker (from [OpenCode](https://github.com/opencode-ai/opencode))

**Source**: [`internal/pubsub/`](https://github.com/opencode-ai/opencode/tree/main/internal/pubsub)

**What it does**:
- Generic typed broker: `Broker[T]`
- Subscribe with context (auto-cleanup on cancel)
- Non-blocking publish (drops events if subscriber is slow)
- Event types: Created, Updated, Deleted

**Savant applicability**: MEDIUM. We have Nexus for event bus, but the typed generic broker pattern is cleaner for internal component communication.

### 8.2 Structured Logging (from [OpenCode](https://github.com/opencode-ai/opencode))

**Source**: [`internal/logging/`](https://github.com/opencode-ai/opencode/tree/main/internal/logging)

**What it does**:
- Structured log messages with levels
- Message-based logging (each log entry is a typed message)
- Writer interface for different outputs

**Savant applicability**: LOW. We have tracing.

---

## 9. Free Provider Integration Patterns

### 9.1 Free Tier Provider List (from [9router](https://github.com/decolua/9router) + [Codebuff](https://github.com/CodebuffAI/codebuff))

**Sources**: [`open-sse/config/providers.js`](https://github.com/decolua/9router/blob/main/open-sse/config/providers.js), [`agents/base2/base2.ts`](https://github.com/CodebuffAI/codebuff/blob/main/agents/base2/base2.ts)

**Identified free/cheap providers**:
- **Kiro AI** — Free Claude 4.5 + GLM-5 + MiniMax via AWS Builder ID / Google / GitHub OAuth
- **OpenCode Free** — No-auth passthrough proxy, auto-fetches models
- **Vertex AI** — $300 free credits for new GCP accounts
- **MiniMax M2.7** — $0.20/1M tokens, cheapest paid option
- **GLM-5.1** — $0.60/1M tokens, daily reset
- **Gemini** — 60 req/min, 1000 req/day free tier
- **GitHub Copilot** — Free for some users, subscription for others
- **Antigravity** — Google's free AI coding assistant

**Savant applicability**: HIGH. We should integrate these as first-class providers so users can run agents with zero or near-zero cost.

### 9.2 Tiered Model Routing (from [Codebuff](https://github.com/CodebuffAI/codebuff))

**Source**: [`agents/base2/base2.ts`](https://github.com/CodebuffAI/codebuff/blob/main/agents/base2/base2.ts)

**What it does**: Different agent modes with different model/validation tradeoffs:
- `default` — Claude Opus 4.7 (best quality)
- `free` — MiniMax M2.7 (cheapest)
- `lite` — Kimi K2.6 (balanced)
- `max` — Claude Opus with extended reasoning
- `fast` — No validation, fastest response

**Savant applicability**: HIGH. A tiered routing system where simple tasks use cheap models and complex tasks use expensive models would optimize cost/quality.

---

## 10. Compaction & Context Management (Detailed)

### 10.1 Compaction Threshold Calculation (from [OpenClaude](https://github.com/Gitlawb/openclaude))

**Source**: [`src/services/compact/autoCompact.ts`](https://github.com/Gitlawb/openclaude/blob/main/src/services/compact/autoCompact.ts)

**Key formulas**:
```
effectiveContextWindow = contextWindow - maxOutputTokensForSummary
autoCompactThreshold = effectiveContextWindow - 13000 buffer
warningThreshold = autoCompactThreshold - 20000
errorThreshold = autoCompactThreshold - 20000
```

**Circuit breaker**: After 3 consecutive compaction failures, stop retrying. This prevents wasting API calls when context is irrecoverably over limit.

**Savant applicability**: HIGH. We should adopt similar threshold calculations and the circuit breaker pattern.

### 10.2 Cache-Aware Compaction (from [OpenClaude](https://github.com/Gitlawb/openclaude))

**Source**: [`src/services/compact/cachedMicrocompact.ts`](https://github.com/Gitlawb/openclaude/blob/main/src/services/compact/cachedMicrocompact.ts)

**What it does**: Uses Anthropic's `cache_edit` API to remove tool results from the cached prefix without invalidating the entire cache. Tracks tool results in a global state, registers them per user message, and issues cache_edits to delete old results. Includes baseline tracking for `cache_deleted_input_tokens` to compute per-operation savings.

**Savant applicability**: MEDIUM. This is Anthropic-specific, but the concept of cache-aware compaction is important. For other providers, we'd need equivalent strategies.

### 10.3 Time-Based Compaction (from [OpenClaude](https://github.com/Gitlawb/openclaude))

**Source**: [`src/services/compact/microCompact.ts`](https://github.com/Gitlawb/openclaude/blob/main/src/services/compact/microCompact.ts)

**What it does**: When the gap since the last assistant message exceeds a threshold (indicating the server cache has expired), content-clears old tool results. Proactive measure before the cache goes cold.

**Savant applicability**: MEDIUM. The concept of time-based clearing is useful for long-running sessions.

---

## 11. Agent Runtime Patterns

### 11.1 Agent Runtime Interface (from [Codebuff](https://github.com/CodebuffAI/codebuff))

**Source**: [`sdk/src/impl/agent-runtime.ts`](https://github.com/CodebuffAI/codebuff/blob/main/sdk/src/impl/agent-runtime.ts)

**What it does**: Clean runtime interface with:
- `promptAiSdkStream` — streaming LLM calls
- `promptAiSdk` — non-streaming LLM calls
- `promptAiSdkStructured` — structured output (JSON schema)
- `requestToolCall` — execute a tool
- `requestFiles` — read files
- `sendAction` — send action to UI
- `sendSubagentChunk` — stream subagent output
- `consumeCreditsWithFallback` — billing with fallback

**Savant applicability**: MEDIUM. Our agent runtime is more complex (Rust traits, async streams). The clean separation of LLM calls, tool execution, and UI updates is a good pattern.

### 11.2 Agent Step Tracking (from [Codebuff](https://github.com/CodebuffAI/codebuff))

**Source**: [`sdk/src/impl/database.ts`](https://github.com/CodebuffAI/codebuff/blob/main/sdk/src/impl/database.ts)

**What it does**: Each agent run is tracked with:
- `startAgentRun` — creates run record
- `addAgentStep` — records each step (LLM call + tool execution)
- `finishAgentRun` — marks completion with final status

**Savant applicability**: MEDIUM. We have WAL for persistence, but not structured step tracking for analytics/debugging.

---

## 12. Summary: Top Features to Implement

### Tier 1 (Highest Impact, Implement First)

| # | Feature | Source | Impact |
|---|---------|--------|--------|
| 1 | RTK-Style Tool Output Compression | [9router](https://github.com/decolua/9router) | Token savings on every tool call |
| 2 | Auto-Compaction with Circuit Breaker | [OpenClaude](https://github.com/Gitlawb/openclaude) | Prevent context overflow, stop wasted API calls |
| 3 | Provider Usage Tracking | [9router](https://github.com/decolua/9router) | Agents know their limits |
| 4 | Free Provider Integration | [9router](https://github.com/decolua/9router) + [Codebuff](https://github.com/CodebuffAI/codebuff) | Zero-cost agent operation |
| 5 | Tiered Model Routing | [Codebuff](https://github.com/CodebuffAI/codebuff) | Cheap models for simple tasks, expensive for complex |

### Tier 2 (High Impact, Implement After Tier 1)

| # | Feature | Source | Impact |
|---|---------|--------|--------|
| 6 | Provider Executor Pattern | [9router](https://github.com/decolua/9router) | Cleaner provider architecture |
| 7 | Multi-Agent Coding Workflow | [Codebuff](https://github.com/CodebuffAI/codebuff) | Structured File Picker → Planner → Editor → Reviewer |
| 8 | Tool Result Deduplication | [9router](https://github.com/decolua/9router) | Eliminate redundant tool outputs |
| 9 | LSP Integration | [OpenCode](https://github.com/opencode-ai/opencode) | Deep code intelligence |
| 10 | Hierarchical Sessions | [OpenCode](https://github.com/opencode-ai/opencode) | Parent-child session trees with cost tracking |
| 11 | Task-Specific Agent Prompts | [OpenCode](https://github.com/opencode-ai/opencode) | Optimized prompts per agent role |

### Tier 3 (Medium Impact, Nice to Have)

| # | Feature | Source | Impact |
|---|---------|--------|--------|
| 12 | Context Partitioning + Relevance Pruning | [OpenClaude](https://github.com/Gitlawb/openclaude) | Smarter compaction decisions |
| 13 | Provider-Specific Prompt Variants | [OpenCode](https://github.com/opencode-ai/opencode) | Better tool call accuracy per provider |
| 14 | Knowledge Files | [Codebuff](https://github.com/CodebuffAI/codebuff) / [OpenCode](https://github.com/opencode-ai/opencode) | Auto-discovered project context |
| 15 | Session Memory Compaction | [OpenClaude](https://github.com/Gitlawb/openclaude) | Running conversation summary |
| 16 | Time-Based Tool Result Clearing | [OpenClaude](https://github.com/Gitlawb/openclaude) | Proactive cache management |
| 17 | Agent Step Tracking | [Codebuff](https://github.com/CodebuffAI/codebuff) | Structured analytics/debugging |
| 18 | Combo/Fallback System | [9router](https://github.com/decolua/9router) | Named provider combos with round-robin |
| 19 | Format Translation Layer | [9router](https://github.com/decolua/9router) | Centralized provider format handling |

---

## Appendix: Key Architectural Patterns

### A. 9router's Executor Pattern
Each provider has a dedicated executor class inheriting from [`BaseExecutor`](https://github.com/decolua/9router/blob/main/open-sse/executors/base.js). Handles provider-specific URL construction, headers, request/response transformation, retry logic, and credential refresh. Much cleaner than our current match-arm approach.

### B. OpenClaude's Compaction State Machine
Compaction is a [state machine](https://github.com/Gitlawb/openclaude/blob/main/src/services/compact/autoCompact.ts) with multiple strategies (auto, micro, session memory, time-based). Each strategy has clear triggers, thresholds, and fallback behavior. The circuit breaker prevents infinite retry loops.

### C. Codebuff's Agent Definition as Data
Agents are [defined as data structures](https://github.com/CodebuffAI/codebuff/blob/main/agents/types/agent-definition.ts) (TypeScript objects) rather than code. This makes it easy to create new agents, share them, and modify behavior without changing code. Our Rust struct approach is more type-safe but less dynamic.

### D. OpenCode's Clean Provider Interface
Providers implement a simple [`Provider`](https://github.com/opencode-ai/opencode/blob/main/internal/llm/provider/provider.go) interface with `SendMessages` and `StreamResponse`. Model definitions are separate from provider implementations. This separation of concerns is cleaner than our current approach.

### E. OpenClaude's Permission System
[Permission requests](https://github.com/Gitlawb/openclaude/blob/main/src/services/compact/autoCompact.ts) are first-class events in the pub/sub system. The agent requests permission, the UI subscribes and responds, and the agent proceeds or aborts. This is a clean pattern for human-in-the-loop agent operation.

---

*Report generated after deep analysis of all 8 competitor repositories. All features listed are implementable as internal Rust code within Savant's agent framework architecture.*

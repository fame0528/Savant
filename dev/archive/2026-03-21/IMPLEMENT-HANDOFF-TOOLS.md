# Tool System Revamp — Implementation Handoff Guide

> **For:** Antigravity
> **Source FID:** dev/fids/FID-20260320-TOOLS.md
> **Status:** All 9 phases ready for implementation
> **Compilation check after each phase:** `cargo check -p savant_core -p savant_agent`

---

## Root Cause

The stream parser in `stream.rs` strips tool call XML tags from output BEFORE they reach the action parser in `parsing.rs`. Tools silently fail because the parser never sees the tags.

The fix: push tag content to `full_trace` so `parsing.rs` can parse it.

---

## Phase 1: Stop Stripping Tool Tags

**File:** `crates/agent/src/react/stream.rs`
**Lines:** 176-222 (the HIDDEN_TAGS handling loop)

There are two paths when a hidden tag is found:

**Path A — Tag fully contained (around line 183-186):**
The code finds both the opening tag and closing tag in the buffer and skips the entire tag. Change: before skipping, push the complete tag text (from opening tag start to end of closing tag) into `full_trace`. The tag stays hidden from user output but the action parser can see it.

Current code pattern:
```
if let Some(end_pos) = fragment_buffer[tag_start..].find(&end_tag) {
    // Tag fully contained - skip it entirely
    fragment_buffer = fragment_buffer[tag_start + end_pos + end_tag.len()..].to_string();
```

Change to:
```
if let Some(end_pos) = fragment_buffer[tag_start..].find(&end_tag) {
    let tag_content = &fragment_buffer[tag_start..tag_start + end_pos + end_tag.len()];
    full_trace.push_str(tag_content);
    fragment_buffer = fragment_buffer[tag_start + end_pos + end_tag.len()..].to_string();
```

**Path B — Tag continues past buffer (around line 188-192):**
The code enters hidden mode to consume the rest of the tag. Change: after setting `in_hidden_tag = true`, push the opening tag text to `full_trace`.

Current code pattern:
```
fragment_buffer = fragment_buffer[tag_start..].to_string();
in_hidden_tag = true;
```

Add after entering hidden mode:
```
full_trace.push_str(&fragment_buffer);
```

**Hidden mode content consumption (around line 217-222):**
The code drains content between opening and closing tags but discards it. Change: push consumed content to `full_trace`.

Current code:
```
let _consumed: String = fragment_buffer.drain(..safe_len).collect();
```

Change to:
```
let consumed: String = fragment_buffer.drain(..safe_len).collect();
full_trace.push_str(&consumed);
```

**Closing tag found (around line 213):**
The code skips past the closing tag. Change: push the closing tag text to `full_trace`.

Current code:
```
fragment_buffer = fragment_buffer[pos + end_tag.len()..].to_string();
in_hidden_tag = false;
```

Add before the skip:
```
full_trace.push_str(&fragment_buffer[..pos + end_tag.len()]);
```

---

## Phase 2: Multi-Format Parser

**File:** `crates/core/src/utils/parsing.rs`
**Current:** 92 lines, parses two formats (Action: name[args] and tool_call XML)

Add three new regex patterns and parsing blocks inside `parse_actions()`:

**Format C — Attribute-style XML:**
Pattern: invoke tag with name attribute containing parameter tags with name and value attributes.
New statics needed: INVOKE_RE, INVOKE_PARAM_RE (OnceLock).
Regex for invoke: capture name attribute and inner content.
Regex for parameters: capture name and value attributes from parameter tags inside invoke.
Build params into JSON object, push (name, args_json) to actions.

**Format D — use_mcp_tool XML:**
Pattern: use_mcp_tool tag containing tool_name tag and arguments tag.
New statics: USE_MCP_RE, MCP_TOOL_NAME_RE, MCP_ARGUMENTS_RE (OnceLock).
Extract tool_name from inner tag, extract arguments (may be JSON or text).
Parse arguments as JSON if possible, wrap in raw object if not.

**Format E — function_call style:**
Pattern: function_call tag with name and arguments attributes.
New static: FN_CALL_RE (OnceLock).
Regex: capture name attribute and arguments attribute (single-quoted JSON).
Parse arguments JSON.

**Apply aliasing:** Call `alias_tool_name()` on every parsed name before pushing to actions.

---

## Phase 3: Tool Name Aliasing

**File:** `crates/core/src/utils/parsing.rs` — new function `alias_tool_name()`

Maps common LLM-generated tool names to actual registered tool names:

```rust
pub fn alias_tool_name(name: &str) -> &str {
    match name {
        "bash" | "sh" | "exec" | "command" | "cmd" => "shell",
        "fileread" | "readfile" | "read_file" | "file" | "list_dir" | "glob" | "search_files" => "foundation",
        "filewrite" | "writefile" | "write_file" => "foundation",
        "fileedit" | "editfile" | "edit_file" | "replace_in_file" => "file_atomic_edit",
        "filemove" | "move" | "rename" => "file_move",
        "filedelete" | "delete" | "rm" => "file_delete",
        "filecreate" | "create" | "touch" | "write_to_file" => "file_create",
        "memory" | "memory_search" | "recall" => "memory_search",
        "memory_write" | "store" | "remember" | "memory_append" => "memory_append",
        "shell" => "shell",
        "foundation" => "foundation",
        _ => name,
    }
}
```

These map to actual tool names from `crates/agent/src/tools/`:
- `shell` (SovereignShell)
- `foundation` (FoundationTool)
- `file_atomic_edit` (FileAtomicEditTool)
- `file_move` (FileMoveTool)
- `file_delete` (FileDeleteTool)
- `file_create` (FileCreateTool)
- `memory_search` (MemorySearchTool)
- `memory_append` (MemoryAppendTool)

---

## Phase 4: Native Function Calling

**File:** `crates/core/src/types/mod.rs`

Add ProviderToolCall struct:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}
```

Add `tool_calls` field to ChatChunk struct:
```rust
#[serde(default)]
pub tool_calls: Option<Vec<ProviderToolCall>>,
```

**File:** `crates/agent/src/providers/mod.rs`

**OpenRouter (openai_stream_to_chunks function, around line 70):**
After extracting `choice["delta"]["content"]`, add:
1. Check for `choice["delta"]["tool_calls"]` array
2. For each element: extract `index`, `id`, `function.name`, `function.arguments`
3. Arguments arrive incrementally across chunks — accumulate by index using a HashMap
4. When `finish_reason` is `"tool_calls"`, flush accumulated tool calls and yield a ChatChunk with `tool_calls` populated

**Anthropic (anthropic_stream_to_chunks):**
Check for content blocks with `type: "tool_use"` — extract `id`, `name`, `input`.

**Ollama (ollama_stream_to_chunks):**
Check for `message.tool_calls` array — extract `function.name` and `function.arguments`.

**File:** `crates/agent/src/react/stream.rs`

In the stream processing loop, after handling reasoning (around line 149-155):
Check if `chunk.tool_calls` is Some. If so, for each tool call yield `AgentEvent::Action { name, args }`.

---

## Phase 5: Tool Dedup and Max Iterations

**File:** `crates/agent/src/react/stream.rs`

Add to stream state variables (around line 125):
```rust
let mut seen_actions: HashSet<String> = HashSet::new();
let mut tool_iteration_count: usize = 0;
```

Before executing an action (around line 306), after yielding `AgentEvent::Action`:
1. Parse arguments as JSON if possible
2. Sort JSON keys for canonical form
3. Compute hash: `format!("{}:{}", name, canonical_args)`
4. If hash already in seen_actions, skip with warning
5. Otherwise insert and proceed

After each tool execution, increment `tool_iteration_count`. If exceeds `self.max_tool_iterations`, stop and yield FinalAnswer warning.

**File:** `crates/agent/src/react/mod.rs`

Add field to AgentLoop struct:
```rust
pub(crate) max_tool_iterations: usize,
```
Default value: 10 (set in `new()` at line 95).

---

## Phase 6: Credential Scrubbing

**File:** `crates/core/src/utils/parsing.rs` — new function `scrub_secrets()`

Replace known secret patterns with [REDACTED] using static OnceLock regex patterns:
- Anthropic keys: sk-ant-...
- OpenAI keys: sk-...
- GitHub tokens: ghp_..., gho_...
- GitLab tokens: glpat-...
- Slack tokens: xox...
- Bearer tokens: Bearer ...
- JWT tokens: eyJ...eyJ...

Apply in `stream.rs` after tool execution, before injecting result into `full_trace` or returning to LLM.

---

## Phase 7: Virtual Tool-Call for Heartbeat

**File:** `crates/agent/src/pulse/heartbeat.rs`

Current: Heartbeat asks LLM to respond with text, parser looks for "HEARTBEAT_OK".

New: Register a temporary `heartbeat` tool with the provider for this pulse only. Schema:
- action: string enum ["skip", "run"] (required)
- reason: string (optional)

Force tool_choice for "heartbeat" tool in the provider call.

If action == "skip": heartbeat is silent.
If action == "run": execute tasks using normal agent loop.

Post-run: Force another tool call `evaluate_notification(should_notify: bool, reason: string)` to decide if results surface to user.

---

## Phase 8: ToolDomain Separation

**File:** `crates/core/src/traits/mod.rs` (NOT traits.rs — it's a directory with mod.rs inside)

Add to the file:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolDomain {
    Orchestrator,  // Safe: memory, search, config
    Container,     // Requires approval: filesystem, shell, network
}
```

Add to Tool trait (around line 94):
```rust
fn domain(&self) -> crate::types::ToolDomain {
    crate::types::ToolDomain::Orchestrator
}
```

**File:** `crates/agent/src/tools/mod.rs`

Add ToolDomain to the re-exports. Update each tool impl:
- MemorySearchTool, MemoryAppendTool, LibrarianTool, SettingsTool: Orchestrator (default)
- FoundationTool, FileMoveTool, FileDeleteTool, FileAtomicEditTool, FileCreateTool, SovereignShell: override domain() to return Container

---

## Phase 9: LoopDelegate Trait

**File:** `crates/agent/src/react/mod.rs`

Define LoopDelegate trait using async_trait (NOT Pin<Box<dyn Future>>):

```rust
#[async_trait]
pub trait LoopDelegate<M: MemoryBackend>: Send + Sync {
    async fn check_signals(&self, loop_state: &LoopState) -> LoopSignal;
    async fn before_llm_call(&self, ctx: &mut LoopContext<M>) -> Option<LoopOutcome>;
    async fn call_llm(&self, ctx: &mut LoopContext<M>) -> Result<ChatChunkStream, SavantError>;
    async fn handle_text_response(&self, text: &str, ctx: &mut LoopContext<M>) -> TextAction;
    async fn execute_tool_calls(&self, calls: Vec<(String, String)>, ctx: &mut LoopContext<M>) -> Result<Option<LoopOutcome>, SavantError>;
}
```

Refactor the agent loop in `stream.rs` to accept a delegate. The loop engine becomes:
1. Check signals via delegate
2. Before LLM call hook via delegate
3. Call LLM via delegate
4. If text: handle via delegate
5. If tools: execute via delegate
6. Repeat until outcome or max iterations

This is LAST phase — depends on all others.

---

## Verification After Each Phase

```
cargo check -p savant_core -p savant_agent
```

## Full Test After All Phases

```
cargo check -p savant_core -p savant_agent -p savant_gateway -p savant_cli
```

## Manual Test Matrix

- Format A: Action: shell[ls]
- Format B: tool_call XML with function= and parameter= tags
- Format C: invoke tag with name attribute and parameter tags
- Format D: use_mcp_tool with tool_name and arguments tags
- Format E: function_call with name and arguments attributes
- Native: Provider tool_calls from OpenRouter
- Dedup: Duplicate tool calls skipped
- Max iterations: Stops at 10
- Scrubbing: API keys show as [REDACTED]
- Heartbeat: Uses structured tool call not free-text

---

*Created: 2026-03-21. Source: FID-20260320-TOOLS.md after Perfection Loop (3 iterations).*

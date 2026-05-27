# Deep Research Prompt: Compact — Tool Output Compression Engine for Savant

## Objective
Conduct a deep analysis of the Compact tool output compression system as implemented in OpenHuman, then design an enhanced version that significantly outperforms it for Savant's enterprise-grade Rust architecture. The output will be used to update Savant's FID and guide implementation.

## Source Material

### OpenHuman Compact Implementation (Reference)
The complete source is at: https://github.com/tinyhumansai/openhuman/tree/b9f1c0856cffe924382e69c022aaed60554a0e60/src/openhuman/compact

Key files to analyze in detail:
- `mod.rs` — Module overview and architecture
- `types.rs` — Core type definitions (JsonRule, CompiledRule, ToolExecutionInput, CompactResult, etc.)
- `classify.rs` — Rule classification engine (matching + scoring)
- `reduce.rs` — Main reduction pipeline (apply_rule, format_inline, select_inline_text)
- `tool_integration.rs` — Agent loop integration (compact_tool_output, CompactionStats)
- `rules/builtin.rs` — 96 embedded JSON rule definitions
- `rules/compiler.rs` — Rule compilation (regex pre-building)
- `rules/loader.rs` — Three-layer rule loading (builtin/user/project)
- `text/` — Text processing utilities (strip_ansi, normalize_lines, head_tail, clamp_text, dedupe_adjacent, etc.)
- `vendor/rules/` — 96 JSON rule files (the actual compression rules)

### Savant's Current Tool Output Pipeline
Savant currently has basic truncation in `crates/agent/src/react/reactor.rs`:
```rust
fn truncate_output(output: &str, max_chars: usize) -> String {
    // Simple head+tail: 60% head, 40% tail, always applied uniformly
}
```
Called from `execute_tool()` at line 103. No rule-based compression exists. The `Tool` trait in `crates/core/src/traits/mod.rs` has `max_output_chars()` with a default of 16,000 chars. Per-tool overrides: FoundationTool=128K, SovereignShell=10K, WebSovereign=50K.

Also relevant: `crates/agent/src/react/compaction.rs` (context compaction at 80/85/95% thresholds), `crates/agent/src/context_compressor.rs` (LLM-based summarization).

## Research Questions

### 1. Architecture Analysis
- How does OpenHuman's Compact pipeline work end-to-end? (classify → match rule → apply filters → transforms → summarize → format_inline → select_inline → clamp)
- What are the strengths and weaknesses of the JsonRule schema design? (match criteria, filters, transforms, summarize, counters, failure mode)
- How effective is the three-layer overlay (builtin/user/project)? What are the limitations?
- What happens when no rule matches? How does generic/fallback work?
- How does `select_inline_text` decide between raw, compacted, and passthrough output?

### 2. Gap Analysis: What OpenHuman Compact Lacks
These are areas where the current implementation has known or discoverable weaknesses:

**Performance:**
- Regex compilation happens at load time but rules are stored as `Vec<CompiledRule>` — is linear scan the best approach for 96+ rules?
- No caching of classification results for repeated similar tool calls
- The `reduce_execution_with_rules` function is synchronous — how does this interact with async agent loops?

**Intelligence:**
- Classification is purely syntactic (tool name + argv matching) — it doesn't consider the actual output content for classification
- No semantic understanding of what the tool output *means* — e.g., it can't distinguish "cargo test with 3 failures" from "cargo test with 200 failures" at the classification stage
- The `output_match` feature (matching on full output text) is limited to regex — no fuzzy or semantic matching
- No context-awareness: doesn't know what the agent was trying to do, what question it's answering, or what it already knows

**Extensibility:**
- Rules are JSON files compiled at build time (builtin) or loaded from disk (user/project) — no runtime rule modification
- No rule composition or inheritance (each rule is standalone)
- No rule testing framework beyond basic fixtures
- The `family` field is a flat string — no hierarchy or taxonomy

**Observability:**
- No structured telemetry about which rules fire, how often, and how much they compress
- No feedback loop — can't learn from agent behavior whether a rule was helpful

**Edge Cases:**
- File-content inspection commands (cat, head, tail) are handled as a special case — is the list complete?
- What about tools that produce binary output, images, or structured data that isn't text?
- How does it handle multi-line JSON output that spans tool output boundaries?
- What about tool outputs that are themselves the result of a chain of commands (piped)?

### 3. Enhancement Design for Savant
Design a Compact system for Savant that addresses all the gaps above. Specifically:

**A. Classification Engine Improvements**
- Should Savant use a trie or index for rule matching instead of linear scan?
- Can we add output-content-aware classification (examining the first N lines of output to predict the best rule)?
- Should rules support weighted scoring based on historical effectiveness?
- Could we add a "semantic classifier" that uses lightweight heuristics (line count, presence of specific patterns like "PASS"/"FAIL"/"error:") to pre-select rules?

**B. Rule Schema Extensions**
- What new fields should Savant's rule schema add beyond OpenHuman's JsonRule?
- Consider: rule composition (extends/inherits from another rule), conditional chaining (if this rule doesn't reduce enough, try the next), output-type hints (text/json/binary), cost-awareness (token count estimation)
- Should rules support LLM-assisted compression for cases where pattern-matching is insufficient? (e.g., "summarize this test output, preserving only failures and the summary line")

**C. Integration with Savant's Existing Architecture**
- Savant uses WASM/Docker/Nix sandboxes — how should Compact interact with sandboxed tool output?
- Savant has a `SemanticVectorEngine` (HNSW) — can we use semantic similarity to detect redundant tool output?
- Savant has a `PromotionEngine` with OCEAN personality — should compression aggressiveness vary by personality? (e.g., high Openness = less compression, preserve more exploration output)
- How should Compact integrate with Savant's context compaction system? Should it run before or after compaction?
- Should Compact emit events to Savant's Nexus bus for real-time monitoring?

**D. Telemetry and Feedback**
- What metrics should Compact emit per compression event? (rule_id, original_bytes, compressed_bytes, ratio, tool_name, classification_family, latency)
- Should Savant track which rules are most/least effective and auto-tune?
- Could the agent itself provide feedback on compression quality? (e.g., if the agent repeatedly re-runs the same tool because it didn't get enough info, the rule may be too aggressive)

**E. Production Hardening**
- What's the worst-case latency for the full classification + reduction pipeline?
- How should the system handle malformed unicode in tool output?
- What's the memory overhead of 96+ compiled regex rules?
- Should rules be hot-reloadable without restarting the agent?

## Output Format

### Section 1: OpenHuman Compact — Complete Architecture Summary
Diagram + description of every component, data flow, and design decision.

### Section 2: Weakness Analysis
Categorized list of every weakness found, with severity (CRITICAL/HIGH/MEDIUM/LOW) and specific evidence from the source code.

### Section 3: Enhanced Design for Savant
A complete proposed architecture for "Savant Compact" that surpasses OpenHuman's implementation. Include:
- Module structure (files, crates, modules)
- Enhanced rule schema (full struct definitions with serde)
- Classification engine design (with indexing/optimization)
- Integration points with Savant's existing systems (reactor.rs, Tool trait, Nexus, OCEAN, compaction)
- Telemetry design
- Memory and performance budget

### Section 4: Comparison Matrix
Side-by-side feature comparison: OpenHuman Compact vs Proposed Savant Compact

### Section 5: Rule Catalog
List of all built-in rules Savant should ship with (the 96 from OpenHuman plus any new ones Savant needs). For each rule, specify: id, family, what it matches, what it does, compression ratio estimate.

### Section 6: Implementation Priority
Phased implementation plan:
- Phase A: Core engine (classify + reduce + integrate with reactor.rs) — delivers 80% of value
- Phase B: Telemetry + feedback loop
- Phase C: Semantic classification + OCEAN-aware compression
- Phase D: LLM-assisted compression for edge cases

## Important Constraints
- Savant is Rust/Tokio — all designs must be idiomatic Rust
- Zero external runtime dependencies for the core engine (rules should be embeddable)
- Must work with Savant's existing Tool trait API without breaking changes
- Classification + reduction must complete in <5ms for outputs up to 100KB
- Must be async-compatible (called from Tokio task)
- Rules must be hot-reloadable for user/project layers

## Repository Context
- Savant: https://github.com/fame0528/Savant (private, but this prompt contains all needed context)
- OpenHuman Compact: https://github.com/tinyhumansai/openhuman/tree/b9f1c0856cffe924382e69c022aaed60554a0e60/src/openhuman/compact
- Upstream origin: https://github.com/vincentkoc/compact (TypeScript, MIT)

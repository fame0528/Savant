# **Deep Research Report: TokenJuice Architecture and Advanced Compaction Engineering for Savant**

## **Section 1: OpenHuman TokenJuice — Complete Architecture Summary**

The explosive growth of Large Language Model (LLM) agent architectures has exposed a critical bottleneck in context window management: the unrestricted ingestion of verbose, low-signal tool outputs. When agents execute terminal-heavy workflows—such as analyzing cargo build logs, parsing 600-message email threads, or reading multi-megabyte docker ps dumps against live clusters—the resulting context ballooning dramatically increases financial costs, degrades model reasoning, and introduces severe latency.1 The original TokenJuice engine, initially conceptualized as a TypeScript utility for workflows and subsequently ported into the OpenHuman Rust ecosystem, was designed specifically to mitigate this phenomenon.1 By interposing a deterministic, rule-driven reduction engine between the raw tool execution and the LLM context window, TokenJuice compacts output payloads without altering the underlying command semantics or exit codes.2

The end-to-end architecture of the OpenHuman TokenJuice pipeline operates as a sequence of discrete, synchronous transformations that intercept, classify, and reduce tool telemetry before it touches the LLM context. The operational flow begins the moment an agent triggers a command. Rather than piping standard output (stdout) and standard error (stderr) directly back into the agent's memory, the host integration intercepts these streams, wrapping the payload into a strictly typed ToolExecutionInput struct. This struct serves as the foundational data carrier, encapsulating the name of the tool, the argument vector (argv), the exit code, the working directory, and the raw combined text output.2

Once the ToolExecutionInput is instantiated, it is passed to the classification engine (classify.rs). This module is responsible for matching the raw execution against a comprehensive registry of rules. The rule loading mechanism (rules/loader.rs) employs a three-layer cascade to resolve these schemas, prioritizing overriding definitions based on file paths.1 The foundational layer consists of the "Builtin" rules, which are 96 embedded JSON schemas shipped directly with the compiled binary, providing sensible defaults for ubiquitous developer utilities such as git, npm, cargo, and kubectl.1 The second layer is the "User" overlay, reading custom configurations from \~/.config/tokenjuice/rules/ to apply global personal preferences across all projects.1 The final layer is the "Project" overlay, derived from .tokenjuice/rules/ within the local repository, enabling team-shared, version-controlled overrides.1 When these rules are ingested by the compiler (rules/compiler.rs), the system parses the JSON configurations into CompiledRule structs. During this phase, regular expressions defined in the rules are pre-built to minimize runtime compilation penalties during the reduction phase.

The classification process relies almost entirely on syntactic matching. The engine linearly scans the Vec\<CompiledRule\>, analyzing the argv and tool names to find the highest-scoring match.2 The JsonRule schema dictates these match criteria, requiring exact string alignments or regex pattern matches against the command signature. The schema's structural design is deliberately rigid, prioritizing inspectable JSON determinism over unpredictable LLM-based filtering.2 A JsonRule typically contains an identifier, a flat string-based family categorization, match conditions, and a specific reduction strategy involving filtering instructions (dropping or keeping specific regex patterns), transformations (line deduplication, whitespace folding), and summation counters.

Upon successful classification, the pipeline hands the ToolExecutionInput and the matched CompiledRule to the reduction module (reduce.rs). The core apply\_rule function executes a pipeline of text processing utilities located in the text/ directory. The first transformation involves sanitization, utilizing functions like strip\_ansi to remove terminal escape sequences and color codes that offer zero semantic value to an LLM. Subsequently, normalize\_lines ensures cross-platform carriage return consistency. If the rule specifies deduplication, the dedupe\_adjacent utility collapses repetitive log spam—such as identical warning lines generated during a recursive compilation—into a single entry annotated with a repetition counter. The engine then applies the rule's specific regex filters, dropping lines matching noise patterns or exclusively retaining lines matching signal patterns. Finally, the text is subjected to clamp\_text, enforcing hard character or token limits by aggressively truncating the output.1

The terminal stage of the OpenHuman pipeline is output formatting, governed by format\_inline and select\_inline\_text. This logic decides how the compacted data is presented to the LLM. Depending on the rule's definition and the execution context, the system may emit the raw text (if bypassed via \--raw flags), a standard compacted string, or a summarized payload. The summarized payload typically includes the preserved critical lines appended with metadata counters, explicitly informing the agent of the omitted volume (e.g., \[tokenjuice: 450 lines dropped, 3 errors retained\]).4 If no rule matches the execution input, the pipeline relies on a generic fallback mechanism. Safe-inventory policies ensure that file-content inspection tools (like cat or head) are strictly bypassed to prevent the corruption of raw file reads.2 For unrecognized commands, the generic fallback applies a basic head/tail truncation strategy, preserving the initial execution context and the final exit status while indiscriminately discarding the middle payload.2

## **Section 2: Weakness Analysis**

While the OpenHuman TokenJuice implementation provides an effective baseline for context compaction, deploying this architecture within Savant's high-throughput, enterprise-grade Rust ecosystem reveals substantial systemic flaws. These weaknesses span performance bottlenecks, intelligence deficits, extensibility constraints, observability gaps, and fragile edge-case handling. The analysis below categorizes these vulnerabilities by severity, providing specific evidence from the source architecture.

### **CRITICAL Severity Weaknesses**

The most severe architectural flaw in the OpenHuman implementation is the synchronous nature of the reduction pipeline. The reduce\_execution\_with\_rules function executes as a blocking operation. Savant's core agent orchestration relies on the Tokio asynchronous runtime to manage highly concurrent data streams, network I/O, and multi-agent coordination. Executing heavy, synchronous text processing—specifically complex regular expression evaluations on payloads potentially exceeding 100KB—directly on a Tokio worker thread will inevitably cause executor starvation. This blocking behavior stalls the entire async reactor, degrading total system throughput and introducing unacceptable latency spikes across parallel agent tasks.

Equally critical is the classification engine's reliance on O(N) linear scanning. The rule compiler loads all 96 builtin rules, plus any user or project rules, into a standard Vec\<CompiledRule\>. During the classify phase, the engine sequentially iterates through this vector, evaluating string matches and regex conditions for every incoming tool execution. In a high-velocity environment where agents operate in tight loops (e.g., repeatedly polling system states or aggressively searching filesystems), linear scanning becomes computationally prohibitive. If the matching rule resides at the end of the vector, or if no rule matches and the engine must exhaust the entire list before falling back, the overhead violates Savant's strict requirement for sub-5 millisecond execution latencies.

### **HIGH Severity Weaknesses**

A primary intelligence deficit of the current system is its content-blind, purely syntactic classification mechanism. The engine determines which rule to apply based exclusively on the tool name and the argv parameters (e.g., matching the strings cargo and test). It possesses zero semantic awareness of the actual output text during the classification phase. Consequently, it cannot differentiate between a successful cargo test execution that generates a clean summary and a catastrophic failure that generates 200 distinct memory allocation panic traces. Because the classification ignores the underlying "crushability" of the data, it applies rigid reduction strategies blindly, risking the destruction of critical anomalous signals that the agent desperately needs to diagnose failures.5

Furthermore, the engine exhibits a severe lack of stateful awareness and context memory. It processes every ToolExecutionInput in a vacuum. It does not possess context regarding what the agent was attempting to achieve, the specific question the LLM is trying to answer, or what data the agent has already ingested in previous turns. Without this temporal and contextual awareness, the compactor cannot dynamically adjust its aggressiveness, potentially stripping out nuanced details that are highly relevant to the agent's current logical pursuit. Compounding this inefficiency is the total absence of caching mechanisms for classification results. Agents frequently repeat similar tool calls, yet TokenJuice re-evaluates the entire rule chain and re-compiles dynamic inputs without leveraging any Historical Least Recently Used (LRU) or Least Frequently Used (LFU) caching, wasting CPU cycles on redundant operations.

The extensibility of the JsonRule schema also presents a high-severity limitation. The rules are entirely standalone, lacking any capability for composition or inheritance. The family field is implemented as a flat string rather than a structured taxonomy. If Savant requires a standard "JSON minification and error extraction" pipeline applied across fifty different cloud infrastructure CLI tools, the OpenHuman architecture forces the developer to duplicate the configuration fifty times. This violation of the DRY (Don't Repeat Yourself) principle makes the rule catalog brittle, difficult to audit, and highly resistant to systemic updates. Additionally, builtin rules are compiled statically at build time; updating them requires recompiling the entire agent binary, preventing hot-reloading in live production environments.

### **MEDIUM Severity Weaknesses**

The complete absence of structured telemetry and observability prevents the system from learning or tuning itself. The OpenHuman implementation relies entirely on basic debug logging enabled via RUST\_LOG=openhuman\_core::openhuman::tokenjuice=debug.1 There is no structured emission of metrics detailing which rules fire, their execution latency, the original byte count, the compressed byte count, or the achieved compression ratio.4 Without a feedback loop, Savant cannot mathematically determine which rules are underperforming (yielding \<5% compression) or overperforming (yielding \>99% compression, which may indicate destructive data loss). This prevents any autonomic tuning or agent-driven feedback regarding compression quality.

The output\_match feature, which allows rules to filter based on the full output text, is artificially limited to standard regular expressions. The engine lacks fuzzy matching, BM25 scoring for relevance 5, or semantic similarity evaluation. When parsing dense outputs like database queries or expansive search results where ranking signals are absent, purely regex-based compression frequently results in the loss of critical entities because it cannot evaluate the semantic importance of the text.

### **LOW Severity Weaknesses**

Edge case handling for non-standard outputs is rigidly hardcoded. While file-content inspection commands like cat, head, and tail are explicitly bypassed to prevent data corruption 2, the engine makes dangerous assumptions about the nature of the standard output stream. It implicitly assumes standard UTF-8 text formatting. When an agent inadvertently executes a tool against a compiled binary, downloads an image, or triggers a massive base64 blob, the OpenHuman text utilities attempt to parse and regex-match the stream. This can lead to malformed unicode panics or trigger severe "Regex DoS" (Denial of Service) conditions, where the regex engine consumes exponential CPU time attempting to evaluate complex patterns against unbroken binary streams. Furthermore, the engine struggles with multi-line structured data (like nested JSON) that spans across aggressive truncation boundaries, frequently returning syntactically invalid JSON chunks to the LLM.

## **Section 3: Enhanced Design for Savant**

To transcend the limitations of the OpenHuman implementation and seamlessly integrate with Savant's existing Rust architecture, the compaction layer must be entirely re-engineered. The proposed "Savant TokenJuice" system abandons synchronous linear scanning in favor of asynchronous, trie-indexed, semantically aware compression. This design natively interfaces with Savant's Tokio reactor, the SemanticVectorEngine (HNSW) \[User Prompt\], and the OCEAN PromotionEngine \[User Prompt\], delivering a production-hardened, zero-copy architecture guaranteed to execute within a strict 5-millisecond budget.

### **Module Structure and Architecture**

The savant\_tokenjuice crate is designed with strict separation of concerns, isolating text processing from telemetry and integration logic.

| Module Path | Primary Responsibility | Key Components |
| :---- | :---- | :---- |
| src/engine/classify.rs | High-speed rule matching | AhoCorasick Trie Index, Heuristic Byte Prober |
| src/engine/reduce.rs | Asynchronous text manipulation | SIMD-accelerated newline/ANSI stripping, Zero-copy Cow\<'a, str\> buffers |
| src/engine/cache.rs | Redundancy mitigation | LRU Cache for classification routing |
| src/rules/schema.rs | Declarative rule definitions | SavantRule, CompressionStrategy structs (Serde implementations) |
| src/rules/registry.rs | Hot-reloadable state management | Arc\<RwLock\<RuleRegistry\>\>, notify filesystem watcher |
| src/semantic/hnsw.rs | Output deduplication | MinHash generation, Savant SemanticVectorEngine interface |
| src/semantic/ocean.rs | Personality-driven tuning | Threshold scalars based on Openness and Conscientiousness |
| src/telemetry/nexus.rs | Observability | Event emission to Savant's central Nexus bus |
| src/integration/reactor.rs | Tool trait API boundaries | Async hook replacing basic 60/40 truncation in execute\_tool |

### **Enhanced Rule Schema (SavantRule)**

The schema must evolve from a flat JSON object into a deeply typed, composable configuration. The SavantRule struct introduces inheritance (extends), computational budget enforcement, output type hinting, and conditional fallback chains.

The family field is upgraded from a flat string to a strongly typed Enum (e.g., RuleFamily::VersionControl, RuleFamily::Infrastructure, RuleFamily::Testing), enabling bulk operations and metrics aggregation by category. The extends field enables DRY composition; a rule can inherit the CompressionStrategy of a parent rule (e.g., a generic JsonMinifier) while overriding specific match criteria.

To prevent Regex DoS, every rule includes a budget\_ms field. If the regex evaluation exceeds this microsecond threshold, the engine aborts the complex reduction and falls back to a guaranteed fast-path truncation. The output\_hint field informs the reduction engine whether it should expect PlainText, Json, Yaml, or Binary, allowing the engine to parse and truncate structured data without violating syntax boundaries (e.g., ensuring a truncated JSON object always closes with }). Finally, the fallback\_chain array specifies alternate rule IDs to attempt if the primary rule fails to achieve a minimum compression ratio.

### **Classification Engine Re-Design**

To eliminate the O(N) bottleneck and achieve sub-5ms latencies, Savant TokenJuice implements a dual-stage classification engine.

**Stage 1: Aho-Corasick Trie Indexing**

Upon initialization and during any hot-reload event, the RuleRegistry compiles all match\_criteria strings (tool names and argv structures) into an Aho-Corasick automaton. When execute\_tool yields an output, the routing to the appropriate rule is determined in O(L) time, where L is the length of the command invocation string. This algorithmic complexity is completely independent of the total number of registered rules, instantly resolving the linear scaling penalty.

**Stage 2: Heuristic Content Probing (Semantic Classifier)**

If the Trie index yields no specific match (or flags the command as generic), the engine routes the output to the Heuristic Byte Prober. Instead of blindly applying a 60/40 truncation, this classifier reads the first 1024 bytes of the payload. It calculates byte entropy to detect compressed or binary data, immediately bypassing reduction if binary magic numbers are found. It scans for structural indicators (e.g., matching { and } for JSON, or \--- for YAML) and specific semantic keywords (PASS, FAIL, error:). If the prober detects a high density of JSON formatting, it automatically dynamically routes the payload to the generic fallback/json\_minify rule, achieving content-aware classification without explicit argv configuration.

### **Integration with Savant's Existing Architecture**

**Tokio Async Integration (reactor.rs)**

The integration point at crates/agent/src/react/reactor.rs line 103 is completely rewritten. The synchronous truncate\_output is replaced with an asynchronous .await call to savant\_tokenjuice::compact(). To prevent Tokio thread exhaustion, the classification index lookup runs inline, but any intensive text manipulation (regex application, line deduplication) is dispatched to the blocking thread pool via tokio::task::spawn\_blocking. The engine exclusively utilizes zero-copy deserialization; outputs are manipulated using Cow\<'a, str\> (Clone-on-Write). If a payload requires no modification, the engine simply passes the original reference, avoiding massive string allocations.

**Sandboxed Tool Output Integration**

Savant frequently executes tools inside isolated WASM, Docker, or Nix sandboxes \[User Prompt\]. These sandboxes generate streaming outputs rather than monolithic strings. Savant TokenJuice integrates directly with Tokio AsyncRead streams. Utilizing a sliding window buffer, the engine applies line-based deduplication and regex filtering on the fly. This streaming architecture prevents 100MB Docker logs from ever materializing fully in memory, drastically reducing the peak RAM overhead of the agent.

**OCEAN Personality Matrix Integration**

Savant's PromotionEngine models agent personalities using the OCEAN framework \[User Prompt\]. TokenJuice dynamically scales its compression aggressiveness based on the active agent's profile traits.

* **High Openness**: Agents scoring high in Openness favor exploration, creativity, and divergent thinking. TokenJuice mathematically lowers its compression thresholds (e.g., truncating at 90% instead of 60%) and disables aggressive regex dropping, preserving maximum context, anomalies, and edge-cases for the agent to explore.  
* **High Conscientiousness**: Agents scoring high in Conscientiousness require structured, highly specific, and disciplined data. TokenJuice increases its aggressiveness, relentlessly stripping whitespace, formatting, boilerplate text, and non-essential warnings, delivering highly dense, noise-free payloads tailored for precise execution.

**SemanticVectorEngine (HNSW) Integration**

Before applying any regex reduction, TokenJuice calculates a rapid localized hash (such as MinHash) of the output payload. It queries Savant's SemanticVectorEngine to determine if identical or highly similar (e.g., \>0.95 cosine similarity) tool output already resides in the agent's recent context window. If redundant data is detected, TokenJuice aborts text processing and replaces the entire payload with a semantic pointer: \`\`. This dramatically prevents context rot caused by agents polling the same endpoints repeatedly.

**Context Compaction Synergy** TokenJuice is positioned sequentially *before* Savant's LLM-based context compaction system (crates/agent/src/context\_compressor.rs). TokenJuice acts as a deterministic, ultra-fast pre-filter. By stripping 80% of the noise at the boundary layer, the subsequent LLM-based summarizer operates on a vastly reduced token footprint, saving massive inference compute costs and lowering total end-to-end latency. If TokenJuice determines a payload is uncrushable via regex but too large for context, it can flag the output for immediate processing by the context\_compressor.rs LLM (e.g., using Claude Haiku to intelligently extract the summary).4

### **Telemetry and Feedback Loop**

Every compaction event generates a highly structured metric payload emitted directly to Savant's Nexus bus. The telemetry schema captures the rule\_id, the classification\_family, the tool\_name, the execution latency\_us (microsecond precision), the original\_bytes, the compressed\_bytes, and the calculated compression\_ratio.

Savant utilizes this continuous data stream to implement an Autonomic Tuning Loop. If the Nexus bus detects that a specific rule consistently yields a compression ratio below 5%, the system flags the rule as computationally wasteful. Conversely, if a rule yields a 99% compression ratio, but the agent's behavioral telemetry indicates it repeatedly re-runs the exact same command immediately afterward, the system infers that the rule is destructively over-aggressive. The tuning loop automatically adjusts the truncation parameters for that rule ID to restore signal balance.

### **Production Hardening and Memory Budget**

To guarantee stability in enterprise deployments, the architecture enforces strict boundaries. Lossy UTF-8 conversion (String::from\_utf8\_lossy) is applied at the ingestion boundary; any malformed byte sequences are replaced with the standard U+FFFD character before regex evaluation, entirely eliminating the risk of unicode decoding panics. Regex DoS is neutralized by the budget\_ms timeout parameter; if a regex operation exceeds its allotted microseconds, the Tokio task cancels the evaluation and defaults to safe head/tail truncation.

Memory overhead for the 96+ compiled rules is negligible. The Aho-Corasick automaton and pre-compiled regex objects reside in a single globally shared Arc\<RwLock\<RuleRegistry\>\>. A background Tokio task utilizes the notify crate to continuously monitor the \~/.config/savant/rules/ directory for filesystem changes. When a user updates a JSON rule, the registry hot-swaps the index and recompiles the specific regex dynamically without requiring a restart of the agent runtime.

## **Section 4: Comparison Matrix**

To clearly articulate the architectural leap from OpenHuman's reference implementation to the proposed Savant TokenJuice system, the following matrix contrasts their capabilities across key engineering domains.

| Architectural Domain | OpenHuman TokenJuice (Reference) | Proposed Savant TokenJuice | Impact for Savant Enterprise |
| :---- | :---- | :---- | :---- |
| **Execution Model** | Synchronous, thread-blocking execution | Asynchronous, tokio::spawn\_blocking for CPU-bound tasks | Eliminates Tokio thread starvation; maintains high throughput for parallel multi-agent workflows. |
| **Memory Management** | Heavy string allocations during processing | Zero-copy Cow\<'a, str\>, streaming AsyncRead buffers | Prevents RAM exhaustion when parsing massive Docker logs or gigabyte database dumps. |
| **Classification Algorithm** | Linear O(N) array scanning | O(L) Trie indexing (Aho-Corasick) | Guarantees sub-5ms classification latency regardless of rule catalog size. |
| **Intelligence** | Purely syntactic (argv \+ tool name) | Syntactic \+ Heuristic content probing (entropy, structural bytes) | Enables content-aware routing (e.g., identifying JSON payloads even if the tool name is unknown). |
| **Rule Composition** | Flat taxonomy, completely standalone JSON | Hierarchical extends attribute | Adheres to DRY principles; allows rapid scaling and modification of tool families. |
| **Redundancy Detection** | Basic adjacent-line deduplication | Semantic Vector (HNSW) lookup | Eradicates context rot by replacing identical polling outputs with pointer references. |
| **Agent State Awareness** | Stateless and blind | OCEAN-aware dynamic thresholding | Customizes data density based on the agent's specific personality (Openness vs. Conscientiousness). |
| **Telemetry & Metrics** | Standard debug tracing (RUST\_LOG=debug) | Structured Nexus bus emission, ratio tracking | Enables the Autonomic Tuning Loop to detect and correct underperforming or destructive rules. |
| **Safety Mechanisms** | Hardcoded file bypasses (cat, head) | Timeout-enforced Regex DoS protection, lossy UTF-8 sanitization | Prevents engine panics and CPU locking when exposed to binary streams or malicious outputs. |
| **Fallback Mechanism** | Simple, uniform 60/40 head/tail truncation | Context-aware fallback chaining and LLM-assisted verification | Intelligently delegates uncrushable complex outputs to lightweight LLMs for semantic extraction. |
| **Hot Reloading** | Project rules only; builtin requires recompile | Full Arc\<RwLock\> registry with notify filesystem watcher | Allows live injection of new rules across the enterprise without tearing down active agent sessions. |

## **Section 5: Rule Catalog**

The rule catalog is the operational core of the TokenJuice engine. Savant TokenJuice will ship with an expansive library of rules, incorporating the 96 definitions derived from OpenHuman while heavily expanding coverage for cloud infrastructure, data science workflows, and complex build systems. The catalog is organized by the strongly typed RuleFamily enum. The table below details a representative sample of critical built-in rules, their match criteria, their reduction strategies, and their estimated compression efficiencies.

| Rule ID | Family | Match Criteria (argv / Heuristic) | Reduction Strategy & Behavior | Est. Compression Ratio |
| :---- | :---- | :---- | :---- | :---- |
| git/status | VersionControl | \["git", "status"\] | Drops empty whitespace lines. Aggregates long lists of untracked files into a single metric \[N untracked files omitted\]. Preserves exact branch state and modified file names. | 70-85% |
| git/diff | VersionControl | \["git", "diff"\] | Drops non-essential context lines (@@). Preserves exact additions (+) and deletions (-). Truncates diffs exceeding 500 lines, appending a summary of omitted changes. | 60-80% |
| rust/cargo\_test | Testing | \["cargo", "test"\] | Drops all individual \[ok\] test lines. Aggressively preserves \`\` test blocks, panic payloads, and stack traces. Retains the final summary count of passed/failed tests. | 85-95% |
| rust/cargo\_check | Build | \["cargo", "check", "\*"\] | Discards all package download, compilation progress bars, and timing metrics. Extracts and preserves only blocks beginning with error: and warning:. | 90-98% |
| node/npm\_install | PackageMgr | \["npm", "install"\], \["npm", "i"\] | Completely discards verbose dependency resolution trees and fetch logs. Preserves critical vulnerability warnings, deprecated package notices, and the final audit count. | 95-99% |
| node/jest | Testing | \["npx", "jest"\], \["jest"\] | Drops passing test suites. Retains specific stack traces for failed assertions. Uses line deduplication to fold identical repetitive error lines within loops. | 80-90% |
| docker/ps | Infrastructure | \["docker", "ps"\] | Converts visually padded terminal tables into dense CSV or minified JSON arrays. Drops stopped containers from the output unless explicitly requested by the agent's prompt. | 50-70% |
| docker/build | Infrastructure | \["docker", "build"\] | Strips step-by-step layer cryptographic hashes and intermediate container IDs. Preserves the actual COPY and RUN commands alongside the final success or failure state. | 80-90% |
| k8s/kubectl\_get | Infrastructure | \["kubectl", "get"\] | Minifies heavy column whitespace. If the agent's intent is diagnosing failures, filters out all Running pods, isolating only CrashLoopBackOff or Pending resources. | 60-75% |
| aws/cli\_json | Cloud | \["aws", "\*"\] *(if JSON)* | Semantically parses and minifies JSON (removing all indents/newlines). Truncates massive arrays exceeding 10 items, inserting a syntax-safe "\[...N more items omitted\]" marker. | 40-60% |
| unix/grep | Search | \["grep", "\*"\], \["rg", "\*"\] | Limits raw output to the top 50 matches. If matches exceed 50, truncates the remainder and summarizes the total count of matches per file, preserving directory structure context. | Variable |
| unix/ls | Filesystem | \["ls", "-l\*"\], \["ll"\] | Strips group/owner columns (unless the agent is executing as root). Preserves file size, permissions, and names. Folds hidden files into a summary count. | 30-50% |
| python/pip\_install | PackageMgr | \["pip", "install"\] | Removes requirement already satisfied logs and wheel download progress bars. Keeps dependency conflict resolution errors and success installation confirmations. | 85-95% |
| generic/json | Fallback | *Heuristic: First byte is {* | Recursive whitespace removal, duplicate key identification, and array truncation without breaking JSON syntactic validity. | 20-50% |
| generic/binary | Fallback | *Heuristic: Entropy \> 7.5* | Aborts all text processing. Replaces entire payload with \`\` | 99.9% |

## **Section 6: Implementation Priority**

Integrating a highly complex compaction engine into a live, concurrent orchestration system like Savant carries significant architectural risk. Replacing the baseline truncation logic in reactor.rs without degrading existing agent functionality requires a disciplined, phased rollout strategy. The implementation plan prioritizes establishing a secure, non-blocking foundation before introducing advanced semantic capabilities.

### **Phase A: Core Engine Rebuild (Delivers 80% of Value)**

The primary objective of Phase A is to establish a deterministic, high-speed asynchronous engine that safely replaces the current rigid 60/40 truncation algorithm.

* **Engineering Tasks**:  
  * Define the SavantRule schema and the runtime CompiledRule structure utilizing the Rust regex and aho-corasick crates.  
  * Implement the Stage 1 Classification Engine (Trie-based indexing) to completely eliminate linear O(N) scanning.  
  * Rewrite the core text reduction pipeline (strip\_ansi, dedupe\_lines, clamp\_text) to strictly utilize zero-copy Cow\<'a, str\> semantics, minimizing heap allocations.  
  * Modify Savant's crates/agent/src/react/reactor.rs. At line 103, replace the synchronous truncate\_output call with an asynchronous invocation of the TokenJuice classification router, utilizing tokio::task::spawn\_blocking for regex workloads.  
* **Exit Criteria**: The pipeline must operate fully asynchronously, achieving sub-5ms latency on 100KB textual payloads without causing any thread starvation in the Tokio executor pool.

### **Phase B: Telemetry, Safety, and Feedback Loop**

Phase B focuses on observability, production hardening, and dynamic configurability.

* **Engineering Tasks**:  
  * Integrate the reduction engine with the Savant Nexus event bus. Configure the emission of structured metrics (including rule\_id, latency\_us, and compression\_ratio) for every tool execution event.  
  * Implement robust safety guards: enforce Regex DoS protection via tokio::time::timeout and ensure safe lossy UTF-8 conversion at the ingestion boundary.  
  * Develop the notify filesystem watcher task to monitor rule directories, enabling the hot-reloading of the Arc\<RwLock\<RuleRegistry\>\> without restarting the agent runtime.  
  * Establish safe-inventory bypass logic to ensure tools like cat and binary downloads are safely skipped.  
* **Exit Criteria**: Developers must have full dashboard visibility of compression efficiency across the agent fleet, and the engine must demonstrate zero panics when intentionally fed malformed or infinite binary streams.

### **Phase C: Semantic Classification & OCEAN Tuning**

This phase elevates the engine from a syntactic filter into an intelligent, state-aware compaction system.

* **Engineering Tasks**:  
  * Develop the Stage 2 Heuristic Byte-Prober to detect JSON, YAML, and unstructured stack traces independent of the argv command signature.  
  * Connect the compaction engine to the PromotionEngine. Establish the mathematical formulas that tie TokenJuice compression threshold limits dynamically to the agent's OCEAN Extroversion and Conscientiousness axes.  
  * Implement the MinHash and SemanticVectorEngine bridge. Before applying text reduction, the engine must query the HNSW index for payload redundancy and generate pointer-references for duplicate outputs.  
* **Exit Criteria**: The system must autonomously route generic HTTP response payloads to appropriate JSON minifiers and successfully deduplicate repeated polling commands using semantic pointers.

### **Phase D: LLM-Assisted Edge Case Compaction**

The final phase addresses outputs that are critically important but highly unstructured and resistant to regex compaction.

* **Engineering Tasks**:  
  * Implement the asynchronous fallback chain for massive payloads that fail to compress below the context threshold using standard rules.  
  * Introduce the "Conditional Verifier" pattern. For uncrushable data, TokenJuice asynchronously triggers a lightweight, high-speed LLM call (e.g., using a smaller, highly quantized model via Savant's existing infrastructure). The prompt enforces strict reduction: *"Summarize this output, preserving only critical failures and summary lines."*  
  * Implement "Exact Line-Match Sanitization" to prevent LLM hallucination. The engine programmatically verifies that any text returned by the summarizer model exists verbatim in the original payload before allowing it to pass to the primary agent's context.  
* **Exit Criteria**: Total context bloat is mathematically capped; the primary orchestration agent is guaranteed never to receive a tool output exceeding a fixed percentage of its total available context window, ensuring sustained reasoning capabilities over long task durations.

#### **Works cited**

1. Smart Token Compression | OpenHuman \- GitBook, accessed May 12, 2026, [https://tinyhumans.gitbook.io/openhuman/features/token-compression](https://tinyhumans.gitbook.io/openhuman/features/token-compression)  
2. GitHub \- vincentkoc/tokenjuice: Token weight loss. Lean output compaction for terminal-heavy agent workflows. Works as a native CLI tool or as an extension to popular coding and agent frameworks., accessed May 12, 2026, [https://github.com/vincentkoc/tokenjuice](https://github.com/vincentkoc/tokenjuice)  
3. Tokenjuice \- OpenClaw Docs, accessed May 12, 2026, [https://docs.openclaw.ai/tools/tokenjuice](https://docs.openclaw.ai/tools/tokenjuice)  
4. gstack/docs/designs/GCOMPACTION.md at main \- GitHub, accessed May 12, 2026, [https://github.com/garrytan/gstack/blob/main/docs/designs/GCOMPACTION.md](https://github.com/garrytan/gstack/blob/main/docs/designs/GCOMPACTION.md)  
5. Tool output compression for agents \- 60-70% token reduction on tool-heavy workloads (open source, works with local models) : r/LocalLLaMA \- Reddit, accessed May 12, 2026, [https://www.reddit.com/r/LocalLLaMA/comments/1qbei13/tool\_output\_compression\_for\_agents\_6070\_token/](https://www.reddit.com/r/LocalLLaMA/comments/1qbei13/tool_output_compression_for_agents_6070_token/)
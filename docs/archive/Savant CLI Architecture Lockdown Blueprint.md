# **Architecture Blueprint: savant\_cli Companion**

The deployment of a terminal-native interface for an autonomous AI coding ecosystem necessitates an architecture that transcends traditional command-line utilities. The savant\_cli is not merely a wrapper for API calls; it is a highly concurrent, stateful, and emotionally engaging fourth interface into the broader Savant ecosystem. This architecture must rival premium tools such as Crush, Claude Code, OpenCode, and Aider, while uniquely integrating Savant's evolutionary learning mechanisms, multi-agent orchestration, and gamified telemetry. The following technical blueprint exhaustively defines the system architecture across seven critical domains, providing specific implementation directives, data flow paradigms, and foundational design decisions required for a robust production deployment.

## **Domain 1: Rust TUI Framework Selection & Architecture**

The terminal user interface represents the primary interaction boundary. It must manage a complex state machine encompassing real-time character-by-character token streaming, syntax-highlighted code viewports, asynchronous background tasks, and a persistently animated pet panel, all while maintaining a rigid 60 frames-per-second (FPS) rendering budget.

### **Framework Evaluation: Immediate vs. Retained Mode Abstractions**

The Rust ecosystem offers several paradigms for terminal interfaces, primarily divided between immediate-mode renderers like ratatui (formerly tui-rs) and retained-mode, Elm-inspired architectures like bubbletea-rs.

The ratatui library functions as a highly optimized, immediate-mode rendering toolkit.1 It operates by recalculating the entire UI grid on every tick or state mutation.2 While it provides unparalleled flexibility and is the foundation for mature tools like gitui and bottom, it lacks an opinionated application state architecture.1 Conversely, bubbletea-rs acts as a direct port of the Go-based Bubble Tea framework, enforcing the Model-View-Update (MVU) pattern natively and providing built-in support for asynchronous command handling.3 The Rust Bubble Tea ecosystem attempts to mirror its Go counterpart, featuring bubbletea-widgets for pre-built components and lipgloss-extras for advanced styling.3

However, deep architectural evaluation reveals that while bubbletea-rs supports Lipgloss styling and the core MVU loop, its ecosystem maturity significantly trails ratatui.3 Implementing a complex, production-grade tool on an experimental framework introduces unacceptable upstream dependency risks. To resolve this, the architecture mandates the use of ratatui paired with tui-realm.5 The tui-realm framework introduces a hybrid React and Elm architecture on top of ratatui, providing reusable, stateful components that communicate via an event-driven message bus.5 This design achieves the strict determinism of the MVU pattern without sacrificing the stability and extensive widget ecosystem of ratatui.1 A raw crossterm implementation with custom rendering is rejected as it requires rebuilding primitive layout and text wrapping engines from scratch, squandering valuable engineering cycles.

### **Viewport Management, Scrollback Buffering, and Input Handling**

Production Rust TUI applications like gitui, bottom, and bandwhich reveal critical patterns for terminal stability.7 A persistent challenge in full-screen TUIs utilizing standard alternate screens (\\x1b\[?1049h) is the destruction of the native terminal scrollback buffer.9 When users attempt to scroll up using their mouse wheel or terminal emulator shortcuts, they are often met with standard command history rather than the application's output.9 Frameworks that rely on internal widget scrolling often suffer from visual flickering during high-frequency diff renders.10

To solve this, savant\_cli will adopt an inline rendering architecture, analogous to the approach utilized by eye-declare and standard CLI output streams.11 The primary chat interface will grow downward natively, allowing the terminal emulator to handle scrollback memory efficiently.10 The split-pane layout (comprising the code diff viewport, the pet panel, and the token telemetry footer) will be anchored as a sticky, fixed-constraint layout at the bottom of the viewport.12

Input handling will leverage crossterm hardware interrupts fed into a multi-producer, single-consumer (MPSC) asynchronous channel, an architectural pattern successfully deployed in gitui for non-blocking key polling.11

### **TUI State Machine and Focus Management**

The TUI state machine will be structured as a directed acyclic graph (DAG) of application modes, managed by a global AppState registry. The framework must handle multiple concurrent user sessions, requiring rigid focus management.

| Modal State | Trigger Mechanism | Interface Focus | System Behavior |
| :---- | :---- | :---- | :---- |
| **Normal Mode** | Esc key | Global Application | Vim-like navigation; enables scrolling through diffs, switching active sessions via tab keystrokes, and inspecting the focus chain. |
| **Insert Mode** | i key or typing | Text Area Viewport | Captures raw alphanumeric input for chat prompts; disables global hotkeys to prevent command collision. |
| **Command Mode** | : key | Command Palette | Invokes the routing system; allows for systemic modifications, gateway reconnections, and tool configurations. |

Focus management is handled internally by tui-realm, ensuring that only one component at a time receives the crossterm input stream.5

### **Optimal 60 FPS Rendering Architecture**

Achieving a 60 FPS rendering cycle without monopolizing CPU resources—a failure mode recently observed in Anthropic's React-based Claude Code CLI—requires a zero-allocation render pipeline.14 Claude Code treats the terminal as a GPU viewport, consuming 11ms of a 16.6ms budget merely to construct a scene graph.14

The savant\_cli will avoid this Golden Hammer anti-pattern by utilizing selective buffer invalidation.14 The application event loop will be decoupled into three independent channels: Tick (triggering at 16.6ms intervals for pet animations), Render (triggered exclusively by actual state mutations like arriving LLM tokens), and Input (crossterm events).17 The rendering pipeline will write to pre-allocated String buffers using String::with\_capacity and render\_to methodologies, minimizing heap allocations during high-frequency character streaming.16 For the animated ASCII pet, the state function will calculate the visual delta based on elapsed time rather than forcing a full application redraw.18

## **Domain 2: Gamification Psychology & Hook Model Integration**

The integration of a Tamagotchi-style pet into a highly technical developer tool serves as a psychological retention mechanism. Coding, particularly in solo or highly autonomous agentic environments, can induce cognitive fatigue. By anthropomorphizing the system's underlying state through a purely presentational pet, savant\_cli transforms utilitarian telemetry into an emotionally resonant experience.19

### **Hook Model Execution in Developer Workflows**

Nir Eyal's Hook Model establishes a four-phase cycle: Trigger, Action, Variable Reward, and Investment. Developer tools like Duolingo, GitHub (via the contribution graph), and Stack Overflow successfully utilize this model by creating visual representations of sustained effort.22

Within savant\_cli:

* **Trigger:** Internal triggers consist of standard developer friction (e.g., encountering a bug, requiring boilerplate). External triggers include CI/CD pipeline failures or proactive alerts from the Savant gateway daemon.20  
* **Action:** The developer invokes the assistant using a /chat command or executes an autonomous /edit.21  
* **Variable Reward:** The system returns functional code, but it also provides unpredictable gamified elements. These include a rare pet animation (e.g., a 1% chance legendary drop accessory) or a sudden evolution milestone.20  
* **Investment:** The user approves the mutation, committing the code. This investment permanently alters the Savant swarm's intelligence matrix and increases the developer's daily streak, heightening the psychological cost of abandoning the tool.

### **Variable Reward Schedules and Achievement Systems**

Habit formation is most deeply ingrained through Variable Ratio schedules, where rewards are delivered after an unpredictable number of responses. While the core functionality (code generation) must remain deterministic, the presentational layer will rely on Variable Ratio mechanics for pet accessories and mood behaviors.20 Fixed Interval schedules will be utilized for daily streaks to ensure sustained daily engagement.19

The achievement system scales dynamically with Savant's evolutionary stages, reflecting both individual developer usage and the swarm's collective intelligence:

| Achievement Tier | Evolution Stage Mapping | Specific Unlock Condition | Reward Output |
| :---- | :---- | :---- | :---- |
| **Genesis** | Seedling | Initiate the first /chat session. | Unlocks base pet visibility; sets energy to 100%. |
| **Symbiosis** | Growing | Maintain a 7-day usage streak. | Grants a "streak flame" accessory to the pet. |
| **Architect** | Mature | First circuit breaker triggered and resolved. | Increases pet's displayed "Debugging" stat.20 |
| **Sovereign** | Sovereign | Over 100 successful mutations approved. | Ethereal visual effects; pet transitions into an autonomous overseer state.19 |

### **Visual State Mapping and Telemetry Reflection**

The pet's visual state serves as a heuristic dashboard for complex swarm telemetry. Rather than reading raw JSON logs, developers can infer system health at a glance.19

* **Evolution Stage → Pet Appearance:** Seedlings appear amorphous and energetic. As the swarm matures, the pet's ASCII geometry becomes more structured and complex.  
* **OCEAN Traits → Behavior/Mood:** A swarm with high Openness triggers curiosity animations when reading new file types. High Conscientiousness results in strict, focused idle animations during test suite execution.  
* **Mutation Count → Accessories:** Accumulated mutations unlock distinct visual flair, acting as a historical ledger of the user's impact on the swarm.19  
* **Streak → Energy Level:** Continuous usage keeps the pet's animations fluid and responsive.21

### **Daily Check-in Mechanics and Ethical Guardrails**

The "daily check-in" mechanism utilizes a behavioral decay model.

* **1 Day Inactivity:** The pet exhibits neutral maintenance requirements, gently requesting interaction via subtle visual cues.  
* **3 Days Inactivity:** The pet enters a "depleted energy" state. The UI mutes its colors. Executing a standard command restores equilibrium.  
* **7 Days Inactivity:** The pet begins to visibly "sleep," requiring a direct /pet-feed or interaction command to awaken.21  
* **30 Days Inactivity:** To prevent abandonment guilt (a common failure point in streak-based applications), a 30-day absence triggers a "re-discovery" sequence. The pet displays immense joy upon the user's return, offering a temporary multiplier to learning extractions to quickly bridge the context gap.

To ensure the CLI remains an ethical productivity tool and avoids dark patterns, the system implements a strict saturation point. If a developer completes an unusually high number of prompts within a short time frame, the pet enters a "satiated" state. During this period, gamified rewards cease, preventing compulsive loop checking and encouraging the developer to step away from the terminal.

## **Domain 3: Session Continuity Across All Interfaces**

A premier AI coding assistant must maintain an unbroken thread of context across volatile environments. Sessions must survive unexpected CLI panics, Context Window exhaustion, IDE extensions handoffs, and transitions to the Tauri desktop app.23 Claude Code and Aider have established baselines for persistent memory, but savant\_cli must extend this paradigm to support multi-agent focus chains and swarm telemetry.23

### **Serialization Architecture: JSONL, SQLite, and Protobuf**

Session serialization requires balancing crash safety, queryability, and payload compactness.25 The architecture will implement a tripartite storage strategy:

1. **Protobuf (Over-the-wire):** Communication between the CLI and the Savant Gateway utilizes Protocol Buffers. Binary serialization is substantially more compact than JSON, minimizing latency when transmitting massive AST payloads or multi-turn conversational histories across WebSocket channels.25 Protobuf's strict schema enforcement prevents type violations during agent handoffs.28  
2. **JSONL (Active Session State):** Local session persistence on disk relies on JSON Lines (JSONL). Because JSONL is an append-only format, it is inherently crash-safe.29 If the CLI panics mid-stream, only the final truncated line is lost, while the historical session state remains perfectly intact.  
3. **SQLite (Knowledge Graph):** The persistent MEMORY.md auto-memory, global constraints, and multi-session analytic data will be indexed in a local SQLite database.30 SQLite provides the ACID compliance required for complex relational queries when agents cross-reference past evolutionary learnings.

### **Session File Format and Minimal Resume State**

The session file format, mirroring the \~/.claude/projects/\<encoded-cwd\>/\<session-id\>.jsonl pattern, encapsulates the absolute minimum state required to rehydrate a session seamlessly.23 Each JSON object appended to the log represents a turn transition containing:

* **Turn Index & Timestamp:** For chronological reconstruction.  
* **Active Files:** A vector of absolute file paths currently loaded into the LLM context.24  
* **Conversation History:** Heavily compressed representations of user prompts and agent responses.  
* **Focus Chain:** The persistent hierarchical to-do list generated by the agentic planning phase.  
* **Tool Results:** Cached stdout/stderr from the last executed bash command to prevent re-execution upon resume.  
* **Swarm State:** The active agent ID, pet health metrics, and streak data.

### **Compaction Mechanics and Focus Chain Survival**

As continuous sessions approach the LLM's context window limit, the CLI must autonomously trigger a compaction routine. The compaction algorithm leverages a fast, localized model to summarize early conversational turns into dense semantic blocks.31 Raw tool outputs and historical diffs are discarded entirely, replaced by high-level abstracts (e.g., "Agent modified auth.rs to support OAuth2").

Crucially, the **Focus Chain** bypasses compaction entirely. As the persistent to-do list driving the agent's workflow, the Focus Chain is treated as an immutable systemic constraint. It is injected natively into the system prompt upon every new turn, ensuring the agent never loses sight of the overarching objective, even if the early conversational context that generated the objective has been discarded.

### **Multi-Project Workspaces, Branching, and Crash Recovery**

Multi-project sessions are managed through namespace isolation bound to the Current Working Directory (CWD).29 If the CLI detects execution in a separate repository, it suspends the active state and initializes or resumes the session mapped to the new CWD hash.29

Session branching allows developers to fork a conversation at any point to explore alternative architectural approaches.29 By invoking a command such as /fork, the CLI duplicates the active JSONL file up to the current turn index and assigns it a new UUID. These branches are tracked as a Directed Acyclic Graph (DAG) in the local SQLite database. If an experimental refactor fails, the user can instantly pivot back to the main branch.

In the event of a hard crash (e.g., a SIGKILL or power failure), the recovery flow is automated. Upon reopening the CLI in the same directory, the system detects an unclosed session ID. It reads the JSONL log, reloads the listed files into memory, restores the Focus Chain, and visually informs the user via the TUI: *"Recovered session from unexpected termination. Last action: cargo test execution."* Any uncommitted text in the input buffer is lost, but the systemic context is preserved entirely.23

## **Domain 4: Perfection Loop & Circuit Breaker Design**

A primary failure mode in modern AI coding assistants is the "doom loop"—an oscillation state where the agent repeatedly applies an invalid fix, triggers a compiler error, and then reverts to a previous broken state, endlessly burning tokens without achieving resolution.32 To guarantee production reliability, the savant\_cli architecture embeds a strict Finite State Machine (FSM) known as the Perfection Loop, safeguarded by deterministic circuit breakers.32

### **The FSM: Red, Green, Audit, Self-Correct**

The autonomous runtime operates under a continuous four-stage state machine:

1. **Red (Hypothesis & Test):** Triggered by a user prompt or a failing test suite. The system analyzes the constraints, forms a hypothesis, and executes the designated testing framework. An exit code greater than 0 transitions the system to Green.  
2. **Green (Execution):** The agent generates a targeted patch utilizing SEARCH/REPLACE blocks. Upon successful application of the patch to the local filesystem, the system loops back to Red to execute the test suite again.  
3. **Audit (Validation):** If the Red phase returns an exit code of 0 (tests pass), the system transitions to Audit. A secondary, highly analytical model reviews the implemented patch for stylistic regressions, silent logic errors, or security vulnerabilities.  
4. **Self-Correct (Loop):** If the Audit detects anomalies, it injects a synthetic prompt and forces the system back to Green. If the Audit passes, the loop terminates, the Focus Chain is advanced, and the UI reports success to the user.

### **Circuit Breaker Design and Oscillation Detection**

To detect oscillation within the Red-Green loop, the system calculates the semantic variance between sequential patches. Relying on simple error hash matching is insufficient, as LLMs frequently generate distinct error traces by making minor, irrelevant modifications.

The circuit breaker relies on the strsim Rust crate to calculate the Levenshtein distance between the current proposed patch and previous failed patches in the active memory buffer.35

![][image1]  
If the distance ![][image2] indicates less than a 5% change from a previously failed patch (signifying the LLM is merely shifting whitespace or making hallucinated, non-functional edits), an internal strike counter increments.

The optimal threshold for requiring human intervention is dynamically calculated based on three factors:

1. **Consecutive Failures:** 4 consecutive loops without a passing test suite.  
2. **Patch Similarity:** 3 consecutive patches with ![][image3] variance.  
3. **Cumulative Token Cost:** If the loop exceeds a predefined token budget for a single focus chain item.

Upon breaching these thresholds, the circuit breaker halts execution. The TUI overlays an alert, presenting the user with the exact failure state and prompting manual intervention.32 This halts runaway API costs immediately.

### **Rust Token Killer (RTK) Output Compression**

Agentic doom loops are frequently exacerbated by massive, uncompressed tool outputs destroying the LLM's context window. Supplying 500 lines of cargo test stack traces or thousands of grep matches overwhelms the model's attention mechanism.37 The Rust Token Killer (RTK) acts as a deterministic compression middleware sitting between the bash sandbox and the LLM context.31

RTK utilizes specific compression strategies based on the executed command:

* **cargo test / npm test:** Truncates excessive stack traces. If a test fails, RTK preserves the test name, the specific assertion failure line, and a maximum of 3 frames of the stack trace. It removes absolute filesystem paths, replacing them with relative paths.  
* **grep / rg:** If a search returns more than 20 matches, RTK applies semantic grouping.39 It compresses the output into a summary (e.g., "Found 45 matches across 3 files. Files: main.rs, utils.rs, auth.ts") rather than dumping the raw strings.  
* **ls / tree:** Strips all metadata (permissions, timestamps) and excludes common ignored directories (node\_modules, target, .git) automatically, returning only a flat structural map.  
* **cat / read:** If file reads exceed 2000 lines, RTK utilizes LLMLingua-inspired summarization to compress the document into structural headers and function signatures.38

When the Perfection Loop successfully resolves a bug or an optimization, it integrates directly with Savant's core systems by emitting a structured learning node to LEARNINGS.md. If a circuit breaker is triggered and subsequently resolved by a human, the system logs the delta and proposes an evolutionary mutation to the Gateway, allowing the swarm to adapt to the failure paradigm.

## **Domain 5: Gateway Integration & WebSocket Protocol**

The savant\_cli operates as a peripheral node to the centralized Savant Gateway. It must securely and efficiently communicate with the Gateway to facilitate multi-agent chats, orchestrate skill installations, and report swarm telemetry.

### **WebSocket Client Architecture and Connection Management**

The networking layer is built upon the tokio-tungstenite asynchronous WebSocket crate.41 Terminal environments are subject to extreme network volatility, laptop sleep cycles, and background process suspensions. A naive WebSocket implementation will panic upon connection resets.42

The client architecture isolates the WebSocket connection into a dedicated Tokio background task, separating the StreamExt (read) and SinkExt (write) halves.41 This allows the TUI to continue rendering LLM tokens simultaneously while the user types, without blocking the main event loop.43

To prevent intermediate load balancers from terminating idle connections, the client injects Ping frames at 15-second intervals.42 When a "Connection reset without closing handshake" occurs due to a network drop, the client suppresses the panic and immediately engages an exponential backoff reconnection loop.42 Session authentication is validated via a JWT payload injected into the initial upgrade request; if the authentication expires mid-session, the gateway dispatches an AuthExpired frame, prompting the user to refresh their credentials via the TUI without destroying the active context state.

### **ControlFrame Protocol Implementation**

All communication over the WebSocket utilizes a strictly typed ControlFrame Protobuf schema.26 The CLI implements specific handlers for the entire protocol suite:

* **ChatMessage / ChatChunk:** Facilitates bidirectional token-by-token streaming.  
* **SoulMutationPropose / Approve / Reject:** Manages the evolutionary lifecycle. When a new learning is generated, the Gateway sends a proposal frame. The TUI triggers an interactive modal allowing the developer to review the mutation diff before dispatching the Approve frame back to the server.  
* **SkillsList / Install / Uninstall:** Drives the dynamic capability matrix, updating local tool sandboxes based on swarm-wide skill availability.  
* **ConfigGet / AgentConfigSet:** Synchronizes global constraints between the CLI and the Tauri desktop application.

### **Streaming LLM Responses and Multi-Agent Handoffs**

Handling streaming LLM responses requires granular UI synchronization. When the gateway transmits a stream of ChatChunk events, the TUI parses the payload types dynamically. Standard conversational tokens are printed to the inline viewport. However, if the chunk contains reasoning tokens (e.g., data from an \<antThinking\> block 45), the TUI styles this text with muted, italicized formatting and places it within a collapsible accordion widget to prevent visual pollution of the main chat area.

When executing complex tasks, Savant orchestrates multiple specialized agents (e.g., a Planning Agent, a Rust Specialist, a QA Agent). The ChatChunk protocol includes an agent\_id parameter. When the Gateway executes a multi-agent handoff, the CLI detects the shifting agent\_id.20 The TUI dynamically updates the system banner to display the new agent's name, subtly shifts the thematic color of the active pet panel, and prints a transition indicator (e.g., \`\`) into the chat log, providing total transparency into the swarm's internal routing.20

### **Offline Mode Capabilities**

In scenarios where the Gateway is unreachable, the CLI degrades gracefully into Offline Mode. The tui-realm state machine disables network-dependent commands and transitions the pet into a "disconnected" visual state. However, the developer retains full local functionality. They can edit files, execute bash scripts, read the local LEARNINGS.md and EVOLUTION.jsonl caches, and view their historical pet state. Natural language commands generated during Offline Mode are stored in a local SQLite spool buffer; upon WebSocket reconnection, the spool is automatically flushed to the Gateway for processing.

## **Domain 6: Coding CLI Core Features**

To effectively compete with OpenCode and Aider, the savant\_cli must possess deep, systemic awareness of the user's local filesystem and execute code modifications with extreme precision.24

### **Diff-First Editing and the SEARCH/REPLACE Parser**

The architectural cornerstone of the editing system is the absolute rejection of "whole file" rewrites. Forcing an LLM to regenerate an entire 1,000-line file to change a single variable is both slow and prohibitively expensive.47 Instead, the system mandates a diff-first approach using strict SEARCH/REPLACE block formatting.45

The parser algorithm isolates text fenced by \<\<\<\<\<\<\< SEARCH, \=======, and \>\>\>\>\>\>\> REPLACE markers.49 The algorithm trims leading and trailing whitespace 50 and attempts an exact substring match against the target file. If the LLM has hallucinated minor formatting changes (e.g., omitted an empty line or altered indentation), the exact match will fail. To recover, the system relies on a semantic fallback strategy using the strsim crate.35 The parser slides a window across the target file, calculating the Jaro-Winkler similarity score.35 The block with the highest similarity exceeding 85% is selected for replacement, drastically reducing the rate of failed edits due to LLM formatting drift.45

### **AST Validation via Tree-sitter**

A severe risk in diff-first editing is the "lazy LLM" phenomenon, where the model truncates vital logic by substituting it with comments like //... existing code....51 To categorically eliminate this risk, savant\_cli utilizes the tree-sitter Rust crate for Abstract Syntax Tree (AST) validation.52

Before any patch is written to disk, tree-sitter generates a complete syntax tree of both the original file and the proposed modified file in memory.52 The validation algorithm traverses the AST and counts the functional nodes within the modified scope. If the original method contained 150 nodes and the proposed replacement contains only 15 nodes (primarily consisting of a comment\_node), the system flags an immediate truncation error.51 The edit is blocked, and the error is fed back into the Perfection Loop's Audit phase, forcing the LLM to rewrite the block completely without truncation.

### **Context Engine and Tool Execution**

The context engine utilizes a three-layer memory hierarchy to optimize prompt formulation:

1. **Global Constraints:** Managed in \~/.config/savant\_cli/, dictating universal developer preferences (e.g., "always use snake\_case for Python").  
2. **Project Root (PROJECT.md):** Repository-specific instructions defining the architecture and testing commands.24  
3. **Auto-Memory (MEMORY.md):** Dynamically maintained knowledge base. During session compaction, high-value insights are promoted into this file to persist indefinitely across all future sessions.

The tool execution system utilizes an isolated bash sandbox. To prevent malicious or unintended command execution, the CLI enforces an approval workflow. Safe commands (ls, cat, grep, cargo test) execute automatically.20 Destructive commands (rm, git push, npm publish) are intercepted by the TUI, requiring explicit user authorization via a \`\` prompt.

### **Auto-Fallback Routing and Command Palette**

The LLM routing system ensures high availability via a 3-tier auto-fallback strategy.

* **Tier 1 (Subscriptions):** Routes heavy reasoning tasks to flagship models (e.g., Claude 3.5 Sonnet) via the Savant Gateway subscription pool.38  
* **Tier 2 (Cheap APIs):** If the Gateway is unreachable or quotas are exhausted, execution automatically falls back to budget-tier models via local API keys.38  
* **Tier 3 (Local Models):** In highly secure, air-gapped environments, the system interfaces with local Ollama or vLLM deployments.

The CLI is driven by an extensive Command Palette accessible from the TUI Command Mode:

| Command | Systemic Action |
| :---- | :---- |
| /chat | Standard conversational input. |
| /edit | Forces the agent into the Green phase of the Perfection Loop. |
| /plan | Triggers a multi-agent architectural analysis, generating a new Focus Chain.20 |
| /compact | Manually triggers the session summarization routine. |
| /evolution | Opens the interactive mutation review modal. |
| /skills | Interfaces with the Gateway to query and install new capabilities. |
| /pet | Interacts directly with the gamification layer (e.g., /pet-feed, /pet-status).21 |

## **Domain 7: Crate Structure & Implementation Phases**

The comprehensive nature of the savant\_cli mandates a highly modular, decoupled Rust workspace architecture. A monolithic crate would result in unacceptable compilation times and tangled abstraction boundaries.

### **Workspace Crate Structure**

The project will be organized into a Cargo workspace comprising six distinct crates:

1. **savant\_cli (Binary Engine):** The core executable. It parses command-line arguments using clap, initializes the async Tokio runtime, and maps the subcommands. It acts as the orchestrator, binding the underlying libraries together.  
2. **savant\_cli\_tui (Presentation Layer):** Encapsulates the entire UI system. Depends heavily on ratatui, tui-realm, and crossterm. It defines the AppState, the React-style UI components, and the event listener threads.  
3. **savant\_cli\_core (Logic & Processing):** Contains the diff-generation algorithms (similar, strsim 35), the tree-sitter AST parsers 52, the SEARCH/REPLACE algorithms 50, and the Perfection Loop FSM.  
4. **savant\_cli\_session (Persistence):** Manages the file I/O operations. Implements the JSONL serialization for append logs, SQLite interactions for the Focus Chain, and logic for multi-project namespace detection.30  
5. **savant\_cli\_gateway (Network Layer):** Houses the tokio-tungstenite 41 WebSocket implementation. It defines the Protobuf ControlFrame structures, the ping/pong keep-alive logic, and the exponential backoff algorithms.42  
6. **savant\_cli\_gamification (Behavioral Engine):** An isolated state machine managing the Hook Model data.22 It tracks the daily streak logic, maps the OCEAN traits to pet visualizations, and tracks the variable reward tables.20

### **New Dependencies**

Integrating the required capabilities will necessitate several robust, community-vetted crates:

* ratatui & tui-realm: Core presentation and state.6  
* crossterm: Terminal manipulation and raw input capturing.8  
* tree-sitter: For multi-pass AST syntactic validation.52  
* strsim: For Levenshtein distance calculations in the circuit breaker and patch matching.35  
* tokio-tungstenite: Asynchronous WebSocket framing.41  
* prost: For Protobuf compilation and typing.26  
* notify: For hot-reloading context based on local filesystem changes.

### **Implementation Priorities and Phasing**

To guarantee a stable release cadence and mitigate architectural risk, deployment will proceed through five strictly gated phases:

* **Phase 1: Gateway Protocol & CLI Harness.** Establish the savant\_cli\_gateway crate. Validate connection stability with the existing Savant WebSocket infrastructure. Ensure bidirectional serialization of the Protobuf ControlFrame payloads works flawlessly across network disruptions. The binary will temporarily operate as a standard CLI, allowing it to exist as a subcommand (e.g., savant-cli agent) alongside legacy commands (start, status, heartbeat) without collision.  
* **Phase 2: Local Context & File Manipulation.** Develop the savant\_cli\_core. Implement the SEARCH/REPLACE parser and rigorously unit-test the semantic fallback matching using strsim. Introduce tree-sitter and build the AST validation middleware to block truncation.  
* **Phase 3: The Production TUI.** Construct the savant\_cli\_tui module using tui-realm. Implement the inline scrollback buffer, the split-pane constraints, and the zero-allocation 60 FPS string rendering loops.12 Integrate the LLM token stream into the UI.  
* **Phase 4: The Perfection Loop.** Connect the local test execution sandboxes. Finalize the Red-Green-Audit FSM and calibrate the Levenshtein thresholds for the circuit breaker.32 Implement the RTK text compression algorithms.38  
* **Phase 5: Gamification & Polish.** Introduce the Tamagotchi pet, the daily streak databases, and the achievement tables. Link the visual representations directly to the Gateway's evolution metrics, fulfilling the Hook Model parameters to drive long-term developer retention.19

#### **Works cited**

1. Looking for a Ratatui based framework : r/rust \- Reddit, accessed May 17, 2026, [https://www.reddit.com/r/rust/comments/1tbygrn/looking\_for\_a\_ratatui\_based\_framework/](https://www.reddit.com/r/rust/comments/1tbygrn/looking_for_a_ratatui_based_framework/)  
2. ratatui-tui-in-rust.md \- GitHub, accessed May 17, 2026, [https://github.com/szabgab/rust.code-maven.com/blob/main/pages/ratatui-tui-in-rust.md](https://github.com/szabgab/rust.code-maven.com/blob/main/pages/ratatui-tui-in-rust.md)  
3. whit3rabbit/bubbletea-rs: A rust implementation of ... \- GitHub, accessed May 17, 2026, [https://github.com/whit3rabbit/bubbletea-rs](https://github.com/whit3rabbit/bubbletea-rs)  
4. bubbletea \- Rust \- Docs.rs, accessed May 17, 2026, [https://docs.rs/charmed-bubbletea](https://docs.rs/charmed-bubbletea)  
5. veeso/tui-realm: A ratatui framework to build stateful ... \- GitHub, accessed May 17, 2026, [https://github.com/veeso/tui-realm](https://github.com/veeso/tui-realm)  
6. tuirealm \- Rust \- Docs.rs, accessed May 17, 2026, [https://docs.rs/tuirealm](https://docs.rs/tuirealm)  
7. App Showcase | Ratatui, accessed May 17, 2026, [https://ratatui.rs/showcase/apps/](https://ratatui.rs/showcase/apps/)  
8. Rust: Playing with tui-rs \- MonkeyPatch, accessed May 17, 2026, [https://www.monkeypatch.io/en/blog/2021-05-31-rust-tui/](https://www.monkeypatch.io/en/blog/2021-05-31-rust-tui/)  
9. How to create a scrollback buffer you can actually scroll through with ANSI? \- Reddit, accessed May 17, 2026, [https://www.reddit.com/r/commandline/comments/1g4wcje/how\_to\_create\_a\_scrollback\_buffer\_you\_can/](https://www.reddit.com/r/commandline/comments/1g4wcje/how_to_create_a_scrollback_buffer_you_can/)  
10. Inline viewport should support an Terminal::insert\_lines\_before method. \#1426 \- GitHub, accessed May 17, 2026, [https://github.com/ratatui/ratatui/issues/1426](https://github.com/ratatui/ratatui/issues/1426)  
11. GitHub \- atuinsh/eye-declare: A declarative inline TUI rendering library for Rust, built on Ratatui, accessed May 17, 2026, [https://github.com/atuinsh/eye-declare](https://github.com/atuinsh/eye-declare)  
12. Layout | Ratatui, accessed May 17, 2026, [https://ratatui.rs/concepts/layout/](https://ratatui.rs/concepts/layout/)  
13. gitui 0.2.5 \- Docs.rs, accessed May 17, 2026, [https://docs.rs/gitui/0.2.5](https://docs.rs/gitui/0.2.5)  
14. Anthropic's Claude Code: The 60 FPS "Game Engine" Architecture that's Breaking Terminals : r/ArtOfVibeCoding \- Reddit, accessed May 17, 2026, [https://www.reddit.com/r/ArtOfVibeCoding/comments/1qvm7d1/anthropics\_claude\_code\_the\_60\_fps\_game\_engine/](https://www.reddit.com/r/ArtOfVibeCoding/comments/1qvm7d1/anthropics_claude_code_the_60_fps_game_engine/)  
15. Displaying in terminal at 60 fps : r/rust \- Reddit, accessed May 17, 2026, [https://www.reddit.com/r/rust/comments/18nsp6q/displaying\_in\_terminal\_at\_60\_fps/](https://www.reddit.com/r/rust/comments/18nsp6q/displaying_in_terminal_at_60_fps/)  
16. termplot-rs \- crates.io: Rust Package Registry, accessed May 17, 2026, [https://crates.io/crates/termplot-rs](https://crates.io/crates/termplot-rs)  
17. Handling Multiple Events in Ratatui: Async Immediate Mode Rendering in Rust \- GitHub, accessed May 17, 2026, [https://github.com/d-holguin/async-ratatui](https://github.com/d-holguin/async-ratatui)  
18. Thanks ratatui, plus rendering best practices \#579 \- GitHub, accessed May 17, 2026, [https://github.com/ratatui/ratatui/discussions/579](https://github.com/ratatui/ratatui/discussions/579)  
19. Adaptive evolving companion pet with gamification · Issue \#59081 · anthropics/claude-code, accessed May 17, 2026, [https://github.com/anthropics/claude-code/issues/59081](https://github.com/anthropics/claude-code/issues/59081)  
20. Anthropic's leaked CLI source code reveals a hidden "Tamagotchi" pet and autonomous multi-agent teams. The bar for developer tools is getting wild. : r/PromptEngineering \- Reddit, accessed May 17, 2026, [https://www.reddit.com/r/PromptEngineering/comments/1s9irpo/anthropics\_leaked\_cli\_source\_code\_reveals\_a/](https://www.reddit.com/r/PromptEngineering/comments/1s9irpo/anthropics_leaked_cli_source_code_reveals_a/)  
21. Meet Claude Code Tamagotchi: The Adorable AI Companion That Reacts to Your Coding Sessions \- Zenn, accessed May 17, 2026, [https://zenn.dev/sexygo/articles/2025-08-22-claude-tamagotchi?locale=en](https://zenn.dev/sexygo/articles/2025-08-22-claude-tamagotchi?locale=en)  
22. I Built a Tamagotchi That Judges Your GitHub Activity (and it's brutally honest), accessed May 17, 2026, [https://dev.to/depapp/i-built-a-tamagotchi-that-judges-your-github-activity-and-its-brutally-honest-oh1](https://dev.to/depapp/i-built-a-tamagotchi-that-judges-your-github-activity-and-its-brutally-honest-oh1)  
23. session persistence · ruvnet/ruflo Wiki \- GitHub, accessed May 17, 2026, [https://github.com/ruvnet/ruflo/wiki/session-persistence](https://github.com/ruvnet/ruflo/wiki/session-persistence)  
24. Claude Code Session Management | Developing with AI Tools \- Steve Kinney, accessed May 17, 2026, [https://stevekinney.com/courses/ai-development/claude-code-session-management](https://stevekinney.com/courses/ai-development/claude-code-session-management)  
25. Efficiency in Data Serialization: Comparing JSON and Protocol Buffers | by Saquib Khan, accessed May 17, 2026, [https://medium.com/@ksaquib/efficiency-in-data-serialization-comparing-json-and-protocol-buffers-9056dff444be](https://medium.com/@ksaquib/efficiency-in-data-serialization-comparing-json-and-protocol-buffers-9056dff444be)  
26. Protobuf vs JSON: Performance, Efficiency & API Speed \- Gravitee, accessed May 17, 2026, [https://www.gravitee.io/blog/protobuf-vs-json](https://www.gravitee.io/blog/protobuf-vs-json)  
27. Protocol Buffers vs JSON: The Serialization Showdown \- YouTube, accessed May 17, 2026, [https://www.youtube.com/watch?v=GNKbPoJOypw](https://www.youtube.com/watch?v=GNKbPoJOypw)  
28. Protocol Buffer vs Json \- when to choose one over the other? \- Stack Overflow, accessed May 17, 2026, [https://stackoverflow.com/questions/52409579/protocol-buffer-vs-json-when-to-choose-one-over-the-other](https://stackoverflow.com/questions/52409579/protocol-buffer-vs-json-when-to-choose-one-over-the-other)  
29. Work with sessions \- Claude Code Docs, accessed May 17, 2026, [https://code.claude.com/docs/en/agent-sdk/sessions](https://code.claude.com/docs/en/agent-sdk/sessions)  
30. Using SQLite or a serialization library for a data format? : r/cpp \- Reddit, accessed May 17, 2026, [https://www.reddit.com/r/cpp/comments/posyg0/using\_sqlite\_or\_a\_serialization\_library\_for\_a/](https://www.reddit.com/r/cpp/comments/posyg0/using_sqlite_or_a_serialization_library_for_a/)  
31. Large Language Model as Token Compressor and Decompressor \- arXiv, accessed May 17, 2026, [https://arxiv.org/html/2603.25340v2](https://arxiv.org/html/2603.25340v2)  
32. kesslerio/vibe-check-mcp: Stop AI coding disasters before they cost you weeks. Real-time anti-pattern detection for vibe coders who love AI tools but need a safety net to avoid expensive overengineering traps. · GitHub, accessed May 17, 2026, [https://github.com/kesslerio/vibe-check-mcp](https://github.com/kesslerio/vibe-check-mcp)  
33. Beyond “Vibe Coding”: A Practical Guide to Principled AI Development \- Medium, accessed May 17, 2026, [https://medium.com/@mustapha786/the-agentic-paradox-an-analysis-of-ai-coding-assistant-performance-scaling-from-simple-prototypes-f4e2f43199d7](https://medium.com/@mustapha786/the-agentic-paradox-an-analysis-of-ai-coding-assistant-performance-scaling-from-simple-prototypes-f4e2f43199d7)  
34. Taming the AI Beast: How Harness Engineering Is Rewriting the Rules of Enterprise AI, accessed May 17, 2026, [https://note.com/betaitohuman/n/ne088989bb77d](https://note.com/betaitohuman/n/ne088989bb77d)  
35. strsim \- Rust, accessed May 17, 2026, [https://kbknapp.github.io/clap-rs/strsim/index.html](https://kbknapp.github.io/clap-rs/strsim/index.html)  
36. similarity \- Keywords \- crates.io: Rust Package Registry, accessed May 17, 2026, [https://crates.io/keywords/similarity?sort=downloads](https://crates.io/keywords/similarity?sort=downloads)  
37. Tool output compression for agents \- 60-70% token reduction on tool-heavy workloads (open source, works with local models) : r/LocalLLaMA \- Reddit, accessed May 17, 2026, [https://www.reddit.com/r/LocalLLaMA/comments/1qbei13/tool\_output\_compression\_for\_agents\_6070\_token/](https://www.reddit.com/r/LocalLLaMA/comments/1qbei13/tool_output_compression_for_agents_6070_token/)  
38. LLM Token Optimization: Cut Costs & Latency in 2026 \- Redis, accessed May 17, 2026, [https://redis.io/blog/llm-token-optimization-speed-up-apps/](https://redis.io/blog/llm-token-optimization-speed-up-apps/)  
39. Prompt Compression – Exploring ways to reduce LLM output tokens through prompt shaping : r/LanguageTechnology \- Reddit, accessed May 17, 2026, [https://www.reddit.com/r/LanguageTechnology/comments/1k2r7yw/prompt\_compression\_exploring\_ways\_to\_reduce\_llm/](https://www.reddit.com/r/LanguageTechnology/comments/1k2r7yw/prompt_compression_exploring_ways_to_reduce_llm/)  
40. New Method Cuts Your LLM Costs in Half Using Smart Text Compression \- Medium, accessed May 17, 2026, [https://medium.com/@JamesStakelum/new-method-cuts-your-llm-costs-in-half-using-smart-text-compression-9b8900af51a9](https://medium.com/@JamesStakelum/new-method-cuts-your-llm-costs-in-half-using-smart-text-compression-9b8900af51a9)  
41. Rust WebSocket Guide: tokio-tungstenite, axum & JoinSet, accessed May 17, 2026, [https://websocket.org/guides/languages/rust/](https://websocket.org/guides/languages/rust/)  
42. automatically reconnect websocket if connection reset · Issue \#101 · snapview/tokio-tungstenite \- GitHub, accessed May 17, 2026, [https://github.com/snapview/tokio-tungstenite/issues/101](https://github.com/snapview/tokio-tungstenite/issues/101)  
43. Trying to understand websocket reconnection with pin\_mut\! and channels, accessed May 17, 2026, [https://users.rust-lang.org/t/trying-to-understand-websocket-reconnection-with-pin-mut-and-channels/49544](https://users.rust-lang.org/t/trying-to-understand-websocket-reconnection-with-pin-mut-and-channels/49544)  
44. trying to learn websocket in rust.. i was wondering if periodic sending of message good enough to detect disconnection? \- Reddit, accessed May 17, 2026, [https://www.reddit.com/r/learnrust/comments/16wapj4/trying\_to\_learn\_websocket\_in\_rust\_i\_was\_wondering/](https://www.reddit.com/r/learnrust/comments/16wapj4/trying_to_learn_websocket_in_rust_i_was_wondering/)  
45. Semantic search & replace code with aider, accessed May 17, 2026, [https://aider.chat/examples/semantic-search-replace.html](https://aider.chat/examples/semantic-search-replace.html)  
46. RExBench: Can coding agents autonomously implement AI research extensions? \- arXiv, accessed May 17, 2026, [https://arxiv.org/html/2506.22598v3](https://arxiv.org/html/2506.22598v3)  
47. Edit formats \- Aider, accessed May 17, 2026, [https://aider.chat/docs/more/edit-formats.html](https://aider.chat/docs/more/edit-formats.html)  
48. aider/aider/coders/editblock\_prompts.py at main \- GitHub, accessed May 17, 2026, [https://github.com/Aider-AI/aider/blob/main/aider/coders/editblock\_prompts.py](https://github.com/Aider-AI/aider/blob/main/aider/coders/editblock_prompts.py)  
49. aider is trying to run search replace blocks rather than make them · Issue \#3811 \- GitHub, accessed May 17, 2026, [https://github.com/Aider-AI/aider/issues/3811](https://github.com/Aider-AI/aider/issues/3811)  
50. SamiLehtinen/fsrb: File Search/Replace Block \- GitHub, accessed May 17, 2026, [https://github.com/SamiLehtinen/fsrb](https://github.com/SamiLehtinen/fsrb)  
51. Unified diffs make GPT-4 Turbo 3X less lazy \- Aider, accessed May 17, 2026, [https://aider.chat/docs/unified-diffs.html](https://aider.chat/docs/unified-diffs.html)  
52. tree\_sitter \- Rust \- Docs.rs, accessed May 17, 2026, [https://docs.rs/tree-sitter](https://docs.rs/tree-sitter)  
53. Introducing Rust Sitter \- Shadaj Laddad, accessed May 17, 2026, [https://www.shadaj.me/writing/introducing-rust-sitter](https://www.shadaj.me/writing/introducing-rust-sitter)  
54. Tree-sitter and Rust: Build AST \- YouTube, accessed May 17, 2026, [https://www.youtube.com/watch?v=bm5r7zdpbts](https://www.youtube.com/watch?v=bm5r7zdpbts)  
55. tree-sitter-rust/examples/ast.rs at master \- GitHub, accessed May 17, 2026, [https://github.com/tree-sitter/tree-sitter-rust/blob/master/examples/ast.rs](https://github.com/tree-sitter/tree-sitter-rust/blob/master/examples/ast.rs)

[image1]: <data:image/png;base64,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>

[image2]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABEAAAAXCAYAAADtNKTnAAAA+ElEQVR4Xu3SP0tCYRTH8SMUJCUugkFNDULgIpIuOgjO0bvo/ThKi1tDi2Dg0BD1GsJVRRAEdTJIKfs+99xreri3254/+MDl/u4fnvM8IvvEpYYx1ltmmPjXS3SQC174LXdYoWLun6ONOUqm20kKr3hDxnQuWfTQRdJ0m1xiigccmC5IS/QZ92xorkXXf2uLrbiPvOPKFkEaEj6PIMd4wgJF03k5wbNEz8PlDH3RXbzYrTR/mUcdX3jEkem8xC2lILq9TRyazkvc1p7iRXQeadNtkhf9i12K++ON6BzuJeIDVQzk55h/YoSh6HH/ED3qZST8d/b5X/kGTpo1fO7baeEAAAAASUVORK5CYII=>

[image3]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAC4AAAAXCAYAAAB0zH1SAAACfElEQVR4Xu2WT0hUURTGv0gXkeYiSsIkKiPaWBIVtQiNiiBSKcKwCKJNuJEKiv5AELSolKJaRBStSjCjpS6EohYlQbRoJ4FFmxa5dlV9H+femXtvr3HGYXDh/ODHvHn3vplzzzvnvgdUWXgspitpXTqQsJTWpifniwb6it6mw/QkXRTNMFpg81anA5XgKN0Fy6SCUVZP0C3BnIt0hNbQ5fQ9fUsP0FV0A71Mf9JOd01FUSDK4J/EF7AsC936cVhgnkt0N22nx+kx2k/vwX6zLIqttcf0C/0Oy+pBWD17lNFvsKx7dHwo+K67pASUVSIbYRl7TlckY1ncp1vTkwH19B3iwC/Qve5Y5XUTcywRXbyDvqYPaHM8XJDZAheD9Bnsf5bQh7TJje2jd1FiieiW7oFlRB2vxikVLfQO/UR/wBqvLZoBrKEf6RVYkOfceZXGS/dZFAq4i36gV+myeLgknsKazde1dpRpuj03w1Cmld3NsMwrw1qELxGN99JbdL07l0PNpg5WdtTFasByUQ2HzajsKfNDKHz7FbAvEcX1iF6jm2ANr7uUo4NO0TOwFVYCv4t8pY3JmCctkZ10kq5z37W/n3XHOcKsq7vLKZNT9DcsER4fuNRxSloiQjUfzteCbuSHY3ydq2nm2pja4vTACQP3pfIG2e8lYYl49Dth4PocQFyC/5BuhVlZ+h+6xbomfFD10Bl6JDjn0aL0oNEDJ+Qw4sC1vV7PDxdGC2ilo/QJXRsPZ6JrzsMWfRr2yP5F+9xYiC8R7Swpem58hvWgrtNOlzVvVhS0tiW9rRWD/rib7kf+HSWlHRZQuiDPNjpBx2D1XczrRpUqVeaLv3pDZAHqSliUAAAAAElFTkSuQmCC>
# CAI-Hobbes: Comprehensive Competitive Analysis Report

> **Date**: 2026-03-24  
> **Version Analyzed**: 0.9.56  
> **Repository**: `dustmoo/cai-hobbes`  
> **License**: Functional Source License (FSL 1.1) — converts to Apache 2.0 on 2028-01-20  

---

## 1. Project Overview

### What It Is

Hobbes is a **private, local-first, hotkey-summoned AI assistant** desktop application. Its core philosophy is "Context Composition" — actively managing conversation context so the AI doesn't degrade over long sessions. The tagline is: *"A local-first key-running AI assistant built in Rust and Dioxus, designed to master context composition."*

### Language & Tech Stack

| Layer | Technology |
|-------|-----------|
| **Core Language** | Rust (Edition 2024) |
| **UI Framework** | Dioxus 0.6.3 (desktop, with TailwindCSS for styling) |
| **Async Runtime** | Tokio (multi-threaded) |
| **HTTP Client** | reqwest |
| **MCP Protocol** | `rmcp` crate (v0.10.0) — official Rust MCP SDK |
| **Serialization** | serde + serde_json |
| **Desktop Windowing** | TAO (via Dioxus desktop) |
| **CSS** | TailwindCSS 3.4.1 (compiled externally, embedded via `include_str!`) |
| **Syntax Highlighting** | syntect |
| **Markdown Rendering** | pulldown-cmark |
| **Platform Integrations** | macOS Keychain (security-framework), Touch ID (objc2-local-authentication), system tray (tray-icon), native menus (muda), global hotkeys (global-hotkey) |
| **Packaging** | Dioxus CLI (`dx build --release`), with custom signing scripts |

### Workspace Structure

```
Cargo workspace:
├── apps/macos_app          # macOS-specific app wrapper
├── apps/windows_app        # Windows-specific app wrapper
├── packages/hobbes_core    # Core data models (ChatSession, Attachment, etc.)
├── packages/feature_clipboard # Clipboard integration
└── src/                    # Main application source (50+ files)
    ├── main.rs             # App entry, initialization, state wiring
    ├── session.rs          # SessionState, Session, ActiveContext, persistence
    ├── settings.rs         # Settings, SettingsManager, UiState, ComposioProfile
    ├── components/         # UI components (31 files)
    ├── llm/                # LLM connectors (Gemini, Claude, OpenAI-compat)
    ├── mcp/                # MCP server management, Composio integration
    ├── processing/         # Conversation summarization, scheduling
    ├── services/           # Tool call summarizer, model validation
    ├── skills/             # Claude-style Skill system (SKILL.md)
    ├── context/            # PromptBuilder, permissions, token estimation
    └── gemini/             # Gemini-specific schema conversion
```

---

## 2. Architecture & Design Patterns

### Core Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      User Interface                          │
│  ChatWindow → MessageList → MessageBubble                    │
│  ChatInput (with drag-drop attachments)                      │
│  TabBar, SessionManager, SettingsPanel, McpMarketplace       │
└──────────────┬──────────────────────────────────────────────┘
               │ Messages / ChatCommands
               ▼
┌──────────────────────────────────────────────────────────────┐
│                     Core State Layer                          │
│  SessionState (Signal<HashMap<Session>>)                      │
│  Settings (Signal<Settings>)                                  │
│  UiState (Signal<UiState>)                                    │
│  SecretManager (Signal<HashMap<String,String>>)               │
│  PermissionManager (Signal<PermissionManager>)                │
│  SkillRegistry (Signal<SkillRegistry>)                        │
│  UsageLog (Signal<UsageLog>)                                  │
└──────────────┬──────────────────────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────────────────────┐
│                   Orchestration Layer                         │
│  StreamManager — manages LLM stream lifecycle                │
│  PromptBuilder — assembles neutral LlmPrompt                 │
│  SummarizationScheduler — background convo summarizer        │
│  ConversationProcessor — generates dialogue summaries        │
│  ToolCallSummarizer — snapshots tool call results            │
│  ContinuationController — multi-turn tool feedback loops     │
└──────────────┬──────────────────────────────────────────────┘
               │
       ┌───────┴───────┐
       ▼               ▼
┌──────────────┐ ┌──────────────────────────────────────────┐
│  LLM Layer   │ │           MCP / Tool Layer               │
│  LlmConnector│ │  McpManager (child processes, SSE)       │
│  (trait)     │ │  ComposioClient (OAuth, tool discovery)  │
│  ├─Gemini    │ │  SmitheryClient (MCP registry)           │
│  ├─Claude    │ │  ToolSelection (LLM-based tool pruning)  │
│  └─OpenAI    │ │  OAuthFlow (local callback server)       │
│              │ │  SkillExecutor (context payload builder)  │
└──────────────┘ └──────────────────────────────────────────┘
```

### Key Design Patterns

#### 1. Reactive State via Dioxus Signals
All application state is held in `Signal<T>` wrappers. Components subscribe reactively to signal changes via `use_effect`, `use_memo`, and `use_resource`. This eliminates manual event buses and ensures the UI always reflects the current state.

**Pattern**: `Signal<SessionState>`, `Signal<Settings>`, `Signal<McpManager>`, etc. are provided via `use_context_provider` in the root `app()` function.

#### 2. Serialize-Then-Move Persistence (Pattern P-009)
Session state is serialized to `Vec<u8>` on the calling thread, then only the bytes are moved to a background thread for atomic file I/O. This avoids deep-cloning the entire state (all sessions + messages) just to persist.

```rust
// Borrow, serialize, move bytes
SessionState::save_async(&state, None);
```

#### 3. Atomic File Writes
All persistence uses `tempfile::NamedTempFile` + `persist()` for atomic writes. The temp file is written in the same directory, synced to disk, then atomically renamed to the target path. If anything fails, the original file remains intact.

#### 4. Provider-Neutral LLM Abstraction
The `LlmConnector` trait and `LlmPrompt` types create a provider-agnostic interface:

```rust
pub trait LlmConnector: Send + Sync {
    async fn generate_content_stream(&self, prompt: LlmPrompt, tx: ..., mcp_context: ...);
    async fn summarize_conversation(&self, previous_summary: String, recent_messages: String) -> Result<Value, ...>;
}
```

`ContentBlock` enum models all content types across providers (Text, Thinking, ToolCall, ToolResult, Image). Each connector implements `LlmFormatConverter` to convert between neutral and native formats.

#### 5. Schema Versioning & Migration
`SessionState` has a `schema_version` field (currently v2). On load, if the persisted version is behind, forward migrations run automatically. The `migrate_from_raw_json()` fallback path handles old field names, missing timestamps, and format changes gracefully.

#### 6. Two-Phase Stream Management
The `StreamManager` implements a "Two-Phase Flush" strategy:
- **Phase 1 (UI responsiveness)**: Text chunks are streamed to the UI immediately
- **Phase 2 (atomic state sync)**: Metadata updates (thought signatures, summaries) are batched and applied atomically to prevent "disappearing bubbles" during state transitions

#### 7. MCP Lock Ordering Invariant (Pattern P-010)
When acquiring multiple MCP manager locks, always follow: `servers → dynamic_local_tools → dynamic_composio_tools`. Violating this order causes ABBA deadlocks.

#### 8. Anti-Pattern Registry
The project maintains a formal registry of anti-patterns with IDs (AP-001 through AP-011), each documenting symptoms and fixes. This is a mature engineering practice.

---

## 3. Top Features (Detailed)

### 3.1 Advanced Memory Architecture — "Context Composition"

**What**: Hobbes maintains a `ConversationSummary` struct within each session's `ActiveContext`:
```rust
pub struct ConversationSummary {
    pub summ

# Hermes Agent — Comprehensive Analysis Report

**Date:** 2026-03-24
**Version Analyzed:** v0.4.0 (latest on main)
**License:** MIT

---

## 1. Project Overview

Hermes Agent is a self-improving AI agent built by Nous Research. It is a Python-based (3.11+) CLI and messaging gateway that wraps LLM providers behind an OpenAI-compatible API with tool-calling capabilities.

### Tech Stack

| Layer | Technology |
|-------|------------|
| Language | Python 3.11+ |
| LLM Interface | OpenAI-compatible API, native Anthropic API |
| CLI/TUI | prompt_toolkit + rich |
| Config | YAML (pyyaml) + .dotenv |
| Session Storage | SQLite with FTS5 full-text search |
| Web Search | Firecrawl, Parallel Web |
| Browser | Browserbase / local Chromium (Node.js agent-browser) |
| Messaging | Telegram, Discord, Slack, WhatsApp, Signal, Email, Home Assistant, SMS |
| TTS | Edge TTS (free), ElevenLabs (premium) |
| STT | faster-whisper (local Whisper) |
| Testing | pytest + pytest-xdist, 3289+ tests |
| RL | Tinker-Atropos integration |

### Provider Support

Nous Portal, OpenRouter (200+ models), OpenAI (incl. Codex), Anthropic (native with prompt caching), z.ai/GLM, Kimi/Moonshot, MiniMax, Azure OpenAI, Vercel AI Gateway, Copilot ACP, custom endpoints.

## 2. Architecture & Design Patterns

### Core Architecture

User Input -> HermesCLI (prompt_toolkit TUI) -> AIAgent.run_conversation() -> Build system prompt -> LLM API Call -> Tool dispatch via registry -> Response

Key flow:
1. Build system prompt (SOUL.md, project context, skills index, memory, platform hints)
2. Call LLM (OpenAI chat.completions or Anthropic Messages API)
3. If tool_calls -> handle_function_call() -> registry.dispatch() -> append results -> loop
4. If text response -> persist session -> return
5. Context compression if approaching model context limit

### Key Design Patterns

**1. Self-Registering Tool Registry**
Each tool file calls registry.register() at import time. model_tools.py triggers discovery by importing all tool modules. The singleton registry collects schemas, handlers, check functions, and toolset membership. Adding a tool requires only 3 files: create tool, add import, add to toolset.

**2. Toolset Composition**
Tools grouped into toolsets with recursive resolution and cycle detection. _HERMES_CORE_TOOLS list shared across all platforms. Supports all/* wildcard, legacy name mapping, and plugin-provided toolsets.

**3. Centralized Slash Command Registry**
All commands defined once as CommandDef dataclasses in COMMAND_REGISTRY. Every consumer derives from this: CLI help, gateway dispatch, Telegram BotCommand menu, Slack subcommand mapping, autocomplete. Adding an alias requires only changing the aliases tuple.

**4. Context Compression with Structured Summarization**
When approaching context limit: prune old tool results (cheap pre-pass) -> protect head (3 msgs) + tail (~20K tokens) -> summarize middle with structured LLM prompt (Goal, Progress, Decisions, Files, Next Steps, Critical Context) -> iterative updates on re-compression -> sanitize orphaned tool_call/tool_result pairs.

**5. Iteration Budget (Shared Across Subagents)**
Thread-safe IterationBudget counter shared between parent and child agents. Every LLM turn consumes one iteration. execute_code iterations are refunded. Prevents runaway costs.

**6. Parallel Tool Execution**
ThreadPoolExecutor with max 8 workers. Read-only tools run concurrently. File tools concurrent if different paths. Interactive tools (clarify) always sequential.

**7. Provider Abstraction with Auto-Detection**
Detects provider from base URL: openrouter.ai -> OpenRouter headers, api.anthropic.com -> Anthropic native, URLs ending /anthropic -> Anthropic-compatible, chatgpt.com -> Codex Responses API.

**8. Plugin Architecture**
Drop Python files in ~/.hermes/plugins/ with plugin.yaml manifest and register(ctx) function. Register tools and lifecycle hooks. Also supports pip entry-point plugins.

**9. Ephemeral Injection Pattern**
System prompts and prefill messages injected at API call time, never persisted. Keeps session store clean.

**10. Atomic Write Pattern**
All critical writes use temp file + fsync + os.replace(). Files never partially written on crash.

## 3. Top Features (Detailed)

### 3.1 Multi-Platform Messaging Gateway
Single gateway process manages 8+ platforms simultaneously: Telegram, Discord, Slack, WhatsApp (Node.js Baileys bridge), Signal (signal-cli-rest-api), Email (IMAP/SMTP), Home Assistant (REST + WebSocket), SMS (Twilio). Per-user session isolation. Background process watchers push status updates. Systemd service support.

### 3.2 Skills Ecosystem (70+ Skills)
Markdown-based procedural memory in ~/.hermes/skills/. SKILL.md files with YAML frontmatter. Auto-scanning at startup, conditional activation (fallback_for_toolsets, requires_toolsets), platform-specific skills, self-improvement (agent creates/patches skills), Skills Hub for community discovery, per-platform enable/disable, dynamic skill slash commands.

### 3.3 Persistent Memory (MEMORY.md + USER.md)
Two markdown files in ~/.hermes/memories/. Agent notes + user profile. Periodic nudges (default 10 turns) to review/update. Section-based entries with character limits. Injected into every turn.

### 3.4 Honcho Integration
AI-native cross-session user modeling. Async memory writes, configurable recall modes, multi-user isolation, per-peer memory modes. Tools gated via check_fn.

### 3.5 Session Search (FTS5)
SQLite FTS5 full-text search across all conversations. Keywords, phrases, boolean operators, prefix matching. Source/role filtering. Surrounding context. Query sanitization.

### 3.6 Scheduled Automations (Cron)
Natural language job descriptions, delivery to any platform, per-job runtime overrides, session persistence, prompt injection scanning, atomic writes.

### 3.7 Subagent Delegation
Isolated child agents with shared iteration budget, configurable toolsets/provider/model, observability metadata, interrupt propagation.

### 3.8 Code Execution Sandbox
Python scripts with RPC tool access. API keys stripped. Configurable timeout (5 min), max RPC calls (50), dynamic schema.

### 3.9 Filesystem Checkpoints & Rollback
Monitors destructive commands, captures file state before modification. /rollback to restore.

### 3.10 Git Worktree Isolation
hermes -w creates isolated worktrees. Auto-prunes stale worktrees. Cleanup on exit.

### 3.11 CLI Skin/Theme Engine
Data-driven customization: colors, spinner, branding. Built-in + user YAML skins. Runtime switching.

### 3.12 MCP Client
stdio/HTTP transports, resource/prompt discovery, sampling, auto-reconnection, selective tool loading.

### 3.13 Browser Automation
Browserbase, local Chromium, CDP connect. 11 browser tools.

### 3.14 Voice Mode
Push-to-talk CLI, voice notes in messaging platforms, local Whisper transcription.

### 3.15 Trajectory Generation & RL Training
Parallel batch processing, trajectory compression, Atropos RL environments.

### 3.16 Smart Approvals
Dangerous command detection, per-session memory, Tirith pre-exec scanning.

### 3.17 ACP Server (IDE Integration)
VS Code, Zed, JetBrains via Agent Communication Protocol.

### 3.18 PII Redaction
Automatic PII scrubbing before sending to LLM providers.

## 4. Strengths

1. **Exceptional Documentation** - AGENTS.md designed for AI coding assistants with dependency chains, known pitfalls, step-by-step guides. Comprehensive CONTRIBUTING.md. Detailed release notes (v0.2.0: 216 PRs/63 contributors, v0.3.0: 50+ bug fixes). 37+ page documentation website.

2. **Self-Improving Skills Loop** - Genuine learning: creates skills from experience (after 5+ tool calls), patches them during use, persists knowledge via MEMORY.md, recalls via session search. This is a genuine differentiator vs. most agents.

3. **Robust Tool Registry** - Self-registering pattern with minimal boilerplate. Check functions handle optional deps gracefully. Plugin tools integrate seamlessly. Toolset composition with cycle detection.

4. **Multi-Platform Gateway** - 8+ platforms from single process. Centralized command registry means one definition drives all consumers (CLI, Telegram, Slack, Discord, etc.).

5. **Comprehensive Security** - Path traversal, shell injection, prompt injection, symlink boundary checks, atomic writes, PII redaction, code sandboxing (API keys stripped), Tirith pre-exec scanning, dangerous command approval.

6. **Production Ready** - 3289+ tests with parallel execution. Atomic writes prevent data loss. Safe stdio wrapper for headless/systemd. Rotating error logs. Config migration (v5). Cross-platform (Linux, macOS, Windows/WSL2).

7. **Provider Flexibility** - 10+ providers with auto-detection, provider routing, fallback models, direct endpoint overrides. Centralized call_llm() API.

8. **Context Compression Quality** - Structured summary template (Goal/Progress/Decisions/Files/Next Steps) with iterative updates, tool pair sanitization, token-budget tail protection.

## 5. Weaknesses

1. **Synchronous Agent Loop** - Core loop blocks on LLM calls. No true async. Gateway needs async wrappers. Concurrent agents require separate processes.

2. **Monolithic run_agent.py** - ~2000+ lines. __init__ is ~500 lines with too many responsibilities (client init, provider detection, tool loading, memory, Honcho, compressor, session logging, checkpoints, budget).

3. **Environment Variable Bridge** - YAML -> env vars -> tool code creates confusing indirection. Three separate config loading functions.

4. **JSON String Tool Results** - All handlers must return JSON strings. Unnecessary serialization, verbose error handling.

5. **Limited Type Safety** - Heavy reliance on strings, JSON dicts, duck typing. Pydantic barely used in core.

6. **No Structured Error Types** - JSON string errors make it hard to distinguish error categories.

7. **Config System Complexity** - load_cli_config() is ~300 lines with complex merging, legacy compat, env var bridging.

8. **Skills Are Markdown** - LLM must interpret correctly. No versioning, dependency management, or testing.

9. **Default Non-Persistent Shell** - cd/env vars don't persist between calls by default.

10. **Single-Process Gateway** - No process isolation between platforms.

## 6. Ideas for Savant

### Adopt from Hermes

1. **Self-Registering Tool Registry** - Eliminates boilerplate, clean extensibility. Implement with schema + handler + check_fn + toolset membership.

2. **Centralized Command Registry** - CommandDef dataclass pattern. One definition -> CLI help, gateway dispatch, Telegram menu, Slack mapping, autocomplete.

3. **Structured Context Compression** - Goal/Progress/Decisions/Files/Next Steps template with iterative updates and tool pair sanitization. Far better than naive truncation.

4. **Atomic Write Pattern** - temp file + fsync + os.replace for all critical state. Essential for crash safety.

5. **Skill System with Conditional Activation** - fallback_for_toolsets/requires_toolsets pattern for showing/hiding capabilities based on available tools.

6. **Plugin Architecture** - Drop-in Python files with manifest + registration + lifecycle hooks.

### Improve Upon Hermes

1. **Async-Native Architecture** - Hermes sync loop is a fundamental limitation. Build async from ground up.

2. **Type Safety with Pydantic** - Use Pydantic for config, tool schemas/results, messages, session metadata.

3. **Structured Error Handling** - Proper error hierarchy (UserError, SystemError, TransientError, PermanentError).

4. **Single Config System** - Pydantic-based, clear precedence, no env var bridging.

5. **Composable Agent Loop** - Avoid monolithic init. Use dependency injection.

### Consider

1. **FTS5 Session Search** - Lightweight, effective for cross-session recall.

2. **Iteration Budget Pattern** - Clean cost/token budgeting across parent/child agents.

## 7. Key Differences vs Savant

| Aspect | Hermes Agent | Savant Opportunity |
|--------|-------------|-------------------|
| Architecture | Synchronous loop | Async-native |
| Tool System | Self-registering, JSON strings | Typed results with Pydantic |
| Memory | Markdown files + Honcho | Structured storage |
| Skills | Markdown, LLM-interpreted | Executable skills |
| Config | YAML + .env + env vars | Single Pydantic system |
| Storage | SQLite FTS5 | FTS5 is solid |
| Context | Structured compression | Adopt pattern |
| Plugins | Python files + hooks | Adopt pattern |
| Testing | 3289+ tests | High bar to match |
| Security | Multi-layer | Learn from |

### Hermes Strongest Differentiators
1. Self-improving skills loop (genuine learning)
2. 8+ messaging platforms from single gateway
3. Centralized command registry
4. Comprehensive security
5. Production-ready test suite

### Where Savant Can Leap Ahead
1. Async-native architecture
2. Type-safe configuration
3. Structured error handling
4. Composable agent loop
5. Better plugin system

## 8. Source Files Read

Core: run_agent.py, model_tools.py, cli.py, toolsets.py, hermes_state.py, hermes_constants.py, hermes_time.py, utils.py, batch_runner.py, trajectory_compressor.py

Agent: prompt_builder.py, context_compressor.py, anthropic_adapter.py, auxiliary_client.py, model_metadata.py, display.py, prompt_caching.py, skill_commands.py, trajectory.py, usage_pricing.py, redact.py

Tools (45 files): registry.py, terminal_tool.py, file_tools.py, web_tools.py, browser_tool.py, code_execution_tool.py, delegate_tool.py, mcp_tool.py, memory_tool.py, todo_tool.py, session_search_tool.py, approval.py, checkpoint_manager.py, cronjob_tools.py, skills_tool.py, tts_tool.py, vision_tools.py, honcho_tools.py, homeassistant_tool.py

Gateway: run.py, session.py, platforms/

CLI: main.py, commands.py, config.py, plugins.py, skin_engine.py, setup.py, callbacks.py, auth.py

Build: pyproject.toml, package.json, requirements.txt

Docs: README.md, AGENTS.md, CONTRIBUTING.md, RELEASE_v0.2.0.md, RELEASE_v0.3.0.md

---

*Report generated by comprehensive READ-ONLY source code audit. All major source files were read in full.*

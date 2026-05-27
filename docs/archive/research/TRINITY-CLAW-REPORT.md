# Trinity-Claw Competitive Analysis Report

**Date:** 2026-03-24
**Version Analyzed:** v1.3.0
**Source:** research/competitors/trinity-claw/

---

## 1. Project Overview

TrinityClaw is a self-modifying AI agent designed for personal local use. It runs as a Docker-based Python/FastAPI service that provides an AI agent with persistent memory, dynamic skill creation, web browsing, browser automation, scheduling, Telegram integration, Google Calendar/Gmail access, a business knowledge base, and more.

- **Language:** Python 3.11
- **Framework:** FastAPI + uvicorn
- **Deployment:** Docker Compose (multi-container: agent, LiteLLM, ChromaDB, PostgreSQL, Ollama, nginx)
- **License:** MIT
- **Version:** 1.3.0

### Tech Stack

| Layer | Technology |
|-------|------------|
| Agent Runtime | Python 3.11, FastAPI, uvicorn |
| LLM Routing | LiteLLM proxy (OpenAI, Anthropic, NVIDIA, Ollama) |
| Vector Memory | ChromaDB HTTP client |
| Conversation Log | JSONL file (session_logs.jsonl) |
| Web Search | Tavily -> DuckDuckGo -> Bing fallback chain |
| Browser Automation | Playwright Chromium + playwright-stealth |
| Document Parsing | pdfplumber, python-docx, pypdf, reportlab |
| OCR | Tesseract multi-language |
| Voice Transcription | faster-whisper local CPU |
| Data Science | pandas, numpy, matplotlib, scikit-learn |
| Scheduling | Custom in-process thread polling every 30s |
| Google APIs | google-api-python-client, google-auth-oauthlib |
| Web UI | Single-page HTML/CSS/JS no framework |
| Config | .env + YAML (litellm_config.yaml) |

---

## 2. Architecture and Design Patterns

### Core Architecture

The system uses 5 Docker containers orchestrated by docker-compose.yml:

1. **trinity-agent** (port 8001): Python FastAPI server running app.py (2947 lines). Main agent with all skills.
2. **litellm** (port 4000): LLM proxy routing to various providers.
3. **chroma** (port 8000): Vector database for semantic memory.
4. **db** (postgres): Database for LiteLLM internal use.
5. **web** (port 8080): nginx serving the static web UI.
6. **ollama** (optional, profile=local): Local LLM inference.

### Key Design Patterns

**1. Plugin/Registry Pattern (Skills)**

Skills are Python modules with a standard interface (NAME, DOC, public functions). The registry dynamically discovers and loads .py files from core/ and dynamic/ directories. Each skill is a standalone module with no class inheritance - just module-level functions returning strings.

- 28 core skills shipped in skills/core/
- Dynamic skills created by the agent stored in skills/dynamic/
- Skills loaded via importlib.util with AST metadata extraction
- Auto-reload after skill creation via HTTP POST to /skills/reload

**2. Dual Invocation Modes**

- Local/Ollama mode: XML-style tags (<skill:name.function>args</skill:name.function>) parsed via regex
- Cloud mode: OpenAI-compatible native function calling via LiteLLM with auto-generated tool schemas
- Tool names use double-underscore separator: skill_name__function_name

**3. Agentic Loop (Reason -> Act -> Observe)**

The chat endpoint implements a multi-iteration loop (up to 20 iterations, configurable via AGENT_MAX_ITERATIONS env var). Each iteration:
1. Calls the LLM with current context
2. Executes any tool calls or XML skill tags
3. Injects results back into conversation
4. Loops until LLM produces final text with no more skill calls
5. Detects planning-without-acting and pushes LLM to actually call tools

**4. Three-Layer Memory System**

- Layer 1: In-process session dict (cleared on restart, auto-expires after 2h, rolling summarization at 40 messages)
- Layer 2: JSONL conversation log (persistent, auto-compaction when >2MB, LLM summarizes old entries)
- Layer 3: ChromaDB vector store (semantic retrieval of past AI responses, never raw user queries)

**5. Identity System (identity.md)**

A markdown file defines agent personality, values, communication style, and 22+ standing orders. Injected into system prompt at runtime. Changes take effect on next chat request (no restart needed). Includes:
- Core values (honesty, precision, self-improvement, transparency)
- Communication style rules
- Business knowledge base behavior
- Web design quality standards with WCAG AA compliance
- Decision support mode (Frame, Evaluate, Recommend, Flip condition)
- Email communication templates (English and Serbian)
- RIPER sequence for complex tasks (Research, Plan, Execute, Review)

**6. Security Layers**

- AST validation for dynamically created skills
- Module ban-list: subprocess, socket, ctypes, shutil, sys, etc.
- Function ban-list: eval(), exec(), compile()
- Docker isolation for code execution
- API key authentication with HMAC comparison
- Rate limiting per API key (configurable RPM)
- Credential file blocking in files skill (.env, _token.json, _credentials.json)

**7. Verification System**

After skill execution, results are formatted as [skill.func Result: ...] blocks. The system also strips hallucinated result blocks that the LLM writes itself. Unclosed skill tags are rescued by appending expected closing tags.

---

## 3. Core Skills Inventory (28 skills)

The project ships with 28 core skills in skills/core/:

| Skill | Description | Functions |
|-------|-------------|----------|
| notes | Persistent note storage with tags, daily journal, user model, activity log | save, load, list_notes, delete, search, tag, list_by_tag, write_daily_entry, get_journal, update_user_model, log_activity, get_activity_log |
| files | File operations (read anywhere, write to memory/dynamic only) | ls, cat, pwd, exists, size, sha256, write, append, patch, patch_all, mkdir, delete, tree, find_duplicates |
| web | Web browsing + multi-engine search + Playwright browser automation | fetch, search (Tavily/DDG/Bing), scrape, scrape_links, scrape_images, browser_goto, browser_click, browser_type, browser_screenshot, browser_evaluate, parse_feed, find_and_download_image |
| code_executor | Safe Python sandbox + math evaluator | run_snippet, test_skill, run_bash, calc, status |
| create_skill | Create new skills dynamically with AST validation | create_new_skill, reload |
| scheduler | Schedule one-time and recurring tasks with natural language | schedule, schedule_recurring, remove, list_tasks, get_task, edit_task_prompt, parse_preview |
| git_manager | Git repository management | status, branches, log, add, commit, pull, push |
| document_parser | Parse PDF, DOCX, XLSX, CSV, TXT, JSON, YAML | read, extract_tables, metadata, summarize, convert_to_text |
| data_science | EDA, ML prediction, NLP, charts | analyze_dataset, predict_column, create_chart |
| image_viewer | View and inspect local images | view_image, list_images, inspect_image, extract_text (OCR) |
| url_monitor | Monitor URLs for changes and health | add_url, check_url_now, get_status_summary, get_recent_changes |
| self_improvement | Audit, auto-fix, and learn from skill errors | audit, fix, daily_review, prevent, report, suggest_tests, learn_from_feedback |
| dashboard | System health and monitoring | health, alerts, processes |
| terminal | Whitelisted shell commands inside container | run |
| google_calendar | Google Calendar CRUD via OAuth | authorize, list_events, create_event, delete_event, update_event, find_free_slots |
| telegram_bot | Telegram messaging and polling | setup, send, start_polling, stop, status |
| email_sender | Email via SMTP or SendGrid | send, send_html, send_with_attachment |
| web_builder | Build and preview HTML/CSS/JS websites with live preview | scaffold, write_file, patch_file, serve, stop_server, validate, analyze_design_folder, get_design_system |
| knowledge_base | RAG over business documents via ChromaDB | ingest_folder, ingest_file, search, list_ingested, delete_document, get_summary |
| gmail_reader | Read Gmail inbox via Gmail API | authorize, list_inbox, read_email, search_emails, get_attachments, mark_read, summarize_inbox |
| competitive_intel | Monitor competitor websites for changes | add_site, remove_site, list_watchlist, check_site, run_check, get_alerts, schedule_daily |
| google_drive | Google Drive file management | authorize, list_files, search_files, upload_file, download_file, delete_file |
| meeting_notes | Extract structured info from meeting transcripts | extract, save_meeting |
| youtube | YouTube Data API search | search_videos, search_channels |
| browser_session | CDP-based Chrome automation + stealth mode | list_tabs, goto, get_snapshot, click_ref, fill_ref, click_accessible, type_accessible, tweet, like_tweet, send_gmail, stealth_start, stealth_goto |
| database | SQL database operations | query, execute |
| weather_api | Weather data | get_weather |
| google_maps | Maps and directions | search, directions, nearby |

---

## 4. Top Features (Detailed)

### 4.1 Self-Modifying Skill System

The agent can create new Python skills at runtime via the create_skill skill. When the LLM generates a skill tag, the code goes through:
1. Unicode sanitization (em dashes, smart quotes, etc.)
2. AST parsing for syntax validation
3. Import blocklist checking (12+ dangerous modules)
4. Dangerous function pattern blocking (os.system, os.popen, eval, exec)
5. Protected core skill filename check (cannot overwrite core skills)
6. Write to skills/dynamic/
7. Auto-reload via HTTP POST to /skills/reload
8. Immediate availability for use

The self_improvement skill can then auto-audit newly created skills for anti-patterns (bare except, missing timeouts, hardcoded paths, missing docstrings) and auto-fix safe issues.

### 4.2 Browser Automation (CDP + Stealth)

**CDP Mode (browser_session skill):**
- Connects to user existing logged-in Chrome via Chrome DevTools Protocol
- Works through Docker via netsh portproxy (Windows) or direct address binding (Linux)
- CSS-based element snapshot system: stamps data-tc-ref attributes onto DOM nodes
- Works across iframes (Gmail compose, LinkedIn DMs)
- Platform-specific helpers: tweet(), like_tweet(), reply_tweet(), send_gmail(), tiktok_like()
- get_snapshot() returns structured page state with @eN refs for reliable clicking
- click_ref() and fill_ref() use CSS locators (frame.locator([data-tc-ref=@eN]))

**Stealth Mode:**
- Launches own Chromium with playwright-stealth patches
- Cookie persistence across sessions (/app/memory/stealth_sessions/)
- Multiple parallel sessions with different names
- CAPTCHA handling via 2captcha API

**Fresh Browser (web skill):**
- Playwright-based headless browser for general web tasks
- Singleton pattern with background async event loop
- Rate limiting with exponential backoff and user-agent rotation
- Screenshot limit (3 per navigation) to prevent context bloat

### 4.3 Multi-Engine Web Search

The search function implements a priority fallback chain:
1. Tavily API (if TAVILY_API_KEY set) - highest quality, includes direct answers
2. DuckDuckGo HTML scraping - no API key needed
3. Bing HTML scraping - last resort fallback

Time-sensitive queries automatically append today date for fresh results. Rate limiting with exponential backoff on consecutive errors. RSS/Atom feed parser for news sites.

### 4.4 Knowledge Base (RAG)

- Drop documents into memory/knowledge/ (PDF, DOCX, XLSX, CSV, TXT, MD, images)
- Chunked at 800 chars with 100 char overlap
- Stored in dedicated ChromaDB collection (business_knowledge)
- MD5 hash-based change detection (skip unchanged files)
- Optional LLM summarization per document
- Semantic search with relevance scoring
- Agent configured to ALWAYS search KB first for business questions

### 4.5 Self-Improvement System

- AST-based code analysis detects: bare except, missing docstrings, missing type hints, hardcoded paths, SQL injection risks, missing timeouts
- Health scoring (0-100) per skill with severity weights (critical=25, high=15, medium=8, low=3)
- Auto-patch generation for safe fixes (bare_except, missing_timeout)
- Pattern learning from lessons.jsonl
- daily_review() scans all skills, surfaces patterns, recommends fixes
- Auto-runs on new session start for dynamic skills

### 4.6 Competitive Intelligence

- Watchlist of competitor URLs with CSS selectors and priority levels (high/medium/low)
- Content hashing with SHA-256 + similarity detection (noise threshold 97%)
- Per-domain rate limiting (5s delay between requests)
- Deduplication prevents re-alerting on same content state
- Telegram alerts for high-priority changes
- Structured reports for strategic analysis

### 4.7 Web Builder with Vision Analysis

- Scaffold HTML/CSS/JS projects from templates (professional, blank, landing, dashboard)
- patch_file() for targeted edits (preferred over write_file() to preserve template)
- Live preview server on port 8090
- Vision LLM-based design analysis (analyze_design_folder() returns JSON design brief)
- 161-rule design system generator (get_design_system())

### 4.8 Scheduling System

- Custom in-process thread polling every 30 seconds
- Natural language time parsing (tomorrow at 3pm, in 2 hours, next monday)
- One-time and recurring tasks with interval support
- Tasks dispatch prompts to /chat endpoint (self-calling pattern)
- Activity logging per execution

---

## 5. Strengths

1. **Extremely comprehensive feature set** - 28 core skills covering nearly every use case imaginable, from browser automation to competitive intelligence to data science.

2. **Dual LLM support** - Cloud (LiteLLM) and local (Ollama) with different invocation strategies (native function calling vs XML tags). Well-engineered for different model capabilities.

3. **Robust error learning** - Errors are automatically recorded, deduplicated, injected into system prompts, and the agent learns from them. The self_improvement skill provides AST-based code analysis with auto-patching.

4. **Well-designed browser automation** - The CDP attachment to user Chrome is a killer feature. The CSS-based snapshot system (stamping data-tc-ref attributes) is innovative and avoids fragile accessibility tree parsing. Platform-specific helpers (tweet, send_gmail) dramatically reduce LLM error rates.

5. **Sophisticated identity system** - The identity.md file with 22+ standing orders covers decision support, email communication, social media interaction, self-verification checklists, and the RIPER sequence for complex tasks.

6. **Security-conscious design** - AST validation, import blocklists, Docker isolation, HMAC API key verification, rate limiting, credential file blocking.

7. **Three-layer memory** - In-process sessions, JSONL logs, and ChromaDB vectors with auto-summarization when context grows too large.

8. **Practical deployment** - One-line installers for Windows/Mac/Linux. Docker Compose handles all dependencies. Auto-start on boot.

9. **Web builder with vision** - Analyzing design mockup images and generating JSON design briefs is a unique feature.

10. **Competitive intelligence** - Noise filtering, per-domain rate limiting, and deduplication are well-implemented for monitoring.

---

## 6. Weaknesses

1. **Monolithic app.py (2947 lines)** - All endpoints, LLM calls, memory management, session handling, skill loading, tag parsing, and the agent loop are in a single file. Very hard to maintain and extend.

2. **No structured agent framework** - No abstraction for the agent loop, tool execution, or memory management. Everything is procedural code.

3. **In-process session store** - Session memory is a Python dict. Lost on restart, cannot be shared across instances.

4. **Polling-based scheduler and Telegram** - Scheduler checks every 30s in a background thread. Telegram uses polling. Resource-inefficient.

5. **Single-user design** - No multi-tenancy, no user isolation, no workspace concept.

6. **No testing infrastructure** - No test files, no test framework, no CI/CD.

7. **Hardcoded paths** - Docker uses /app/ as root with all paths fixed. files.py has _APP = Path("/app") hardcoded.

8. **Limited type safety** - Skills use *args, **kwargs with string-based argument parsing. No Pydantic models for skill I/O.

9. **Fragile XML tag parsing** - Regex-based parser can be confused by nested tags, unclosed tags, or tags inside code blocks.

10. **No traditional streaming** - /chat/stream runs entire chat in background thread and queues events. Not true LLM streaming.

11. **No conversation branching or undo** - Once a skill executes, no way to undo or branch.

12. **No skill versioning** - No version tracking or rollback capability.

13. **JSONL compaction uses LLM** - Costs API calls and can fail silently.

14. **No observability** - No structured logging, metrics, or tracing. Just print() statements.

---

## 7. Ideas for Savant

### Adopt These Patterns

1. **Identity.md system** - A markdown file for agent personality/behavior configuration injected into the system prompt. Much more flexible than hardcoding behavior. Include standing orders, communication style, and decision support rules.

2. **Lessons/error memory** - Auto-recording errors with fixes, deduplicating by skill+error_type, and injecting into system prompts. The lessons.jsonl approach is simple and effective. Savant should implement something similar from the start.

3. **Daily journal + user model** - write_daily_entry() and update_user_model() patterns for accumulating context about the user across sessions. This makes the agent genuinely improve over time.

4. **AST-based skill validation** - For any dynamic code creation, AST parsing + import blocklisting is essential. Apply this to any code-execution feature.

5. **Multi-engine search fallback** - Tavily -> DuckDuckGo -> Bing fallback chain ensures search always works even if one provider is down.

6. **CDP browser attachment** - Connecting to user existing Chrome via CDP is a killer feature for social media automation. The data-tc-ref stamping approach for reliable element targeting is innovative.

7. **Platform-specific browser helpers** - tweet(), send_gmail(), like_tweet() as single-call functions reduce LLM error rates dramatically. Create similar helpers for common platforms.

8. **Rolling session summarization** - When context gets too long, summarize older turns and keep recent ones verbatim. Good pattern for context window management.

9. **Noise filtering for monitoring** - The 97% similarity threshold for competitive intelligence is a good pattern for any monitoring or change-detection feature.

10. **Standing Orders as behavioral rules** - The 22+ standing orders in identity.md that control agent behavior (when to search, when to act vs plan, how to handle errors) are extremely valuable. Savant should build a structured version of this.

### Do NOT Adopt

1. **Monolithic main file** - Maintain clean separation of concerns from the start.
2. **XML skill tag parsing** - Native function calling (tool_calls) is more reliable. Use it as primary invocation.
3. **In-process session store** - Use Redis or a database from the start.
4. **Polling-based scheduling** - Use a proper task queue or cron-based approach.
5. **Single-user design** - Design for multi-tenancy from the start.

### Specific Feature Ideas

1. **Standing Orders Database**: Like identity.md but as a structured database of behavioral rules that can be dynamically added/removed/modified per user or per conversation.

2. **Self-Audit on Skill/Tool Creation**: When creating new tools, automatically run security and quality analysis before making them available.

3. **Proactive Search Decision Rules**: Implement rules for when to search vs answer from memory (weather, prices, news -> always search; stable facts, concepts -> answer directly).

4. **Design System Generator**: The get_design_system() function that generates industry-matched design tokens is unique and valuable for any web-building feature.

5. **Competitive Intelligence**: Website monitoring with change detection, noise filtering, and alerting could be a premium feature.

6. **Knowledge Base with Auto-Ingestion**: RAG over user documents with hash-based change detection and optional LLM summarization per document.

---

## 8. Key Differences vs Savant

| Aspect | Trinity-Claw | Savant |
|--------|-------------|--------|
| Language | Python | TypeScript/Node.js |
| Architecture | Monolithic (single 2947-line app.py) | Modular (separate packages) |
| Deployment | Docker Compose (self-hosted only) | TBD |
| User model | Single-user local | Multi-user / cloud |
| LLM routing | LiteLLM proxy | Direct API calls |
| Agent loop | Custom procedural loop | Framework-based |
| Memory | ChromaDB + JSONL | Structured vector store + DB |
| Skills | Python modules (flat files) | Structured tool definitions |
| Browser | Playwright + CDP | TBD |
| Streaming | SSE with background thread | True streaming |
| Testing | None visible | Test framework |
| Observability | print() statements | Structured logging/tracing |
| Session store | In-process dict | Database-backed |
| Config | .env + YAML | Structured config |

### Savant Potential Advantages

1. Cleaner architecture if designed with separation of concerns from the start
2. Multi-user support if designed for it from the ground up
3. Better streaming with true LLM streaming vs background-thread SSE
4. Type safety with TypeScript + structured tool definitions
5. Testing if tests are written from day one
6. Observability if structured logging/metrics are built in

### Trinity-Claw Current Advantages

1. Feature breadth - 28 skills covering almost every use case
2. Browser automation maturity - CDP + stealth + platform helpers
3. Self-improvement system - AST analysis + auto-patching + lessons
4. Identity/personality system - 22+ standing orders in markdown
5. Knowledge base - RAG with document ingestion and semantic search
6. One-click deployment - Installers for all platforms

---

## 9. Code Quality Summary

**app.py**: 2947 lines. Contains all API endpoints, the agent loop, memory management, session handling, skill loading, tag parsing, LLM call helper, and Whisper transcription. This should be at least 10 separate modules.

**Skills**: Each skill is a standalone Python module (100-800 lines). Good docstrings on public functions. Consistent error handling with try/except. Functions return strings (not dicts/lists).

**Security**: Well-designed. AST validation on dynamic code, import blocklists, Docker isolation, HMAC auth, rate limiting. The create_skill module has a thorough security pipeline.

**lessons.jsonl**: Contains 14 entries primarily about browser_session issues (wrong selectors, stopping before task completion, timeout issues). Shows the self-learning system is actively used.

**identity.md**: 169 lines of detailed behavioral configuration. Covers values, communication style, knowledge base rules, web design standards, email templates, standing orders, and decision support mode. This is the most sophisticated agent personality configuration I have seen.

---

*Report generated by comprehensive analysis of all source files in the Trinity-Claw repository.*

# Changelog

All notable changes to the Savant project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

#### LLM Parameter Configuration (v2.0.0)
- **Per-agent LLM parameters** (`crates/core/src/types/mod.rs`)
  - `LlmParams` struct with: temperature, top_p, frequency_penalty, presence_penalty, max_tokens, stop sequences
  - `ParameterDescriptor` struct for UI explanations
  - `get_parameter_descriptors()` returns human-readable descriptions for all parameters
  - Parameters configurable via `agent.config.json` per-agent
  - Hot-reload support when agent spawns

- **Provider LLM parameter support** (`crates/agent/src/providers/mod.rs`)
  - All providers include LLM params in request JSON
  - OpenAI, Anthropic, Groq, Google, Mistral, Deepseek, Cohere, Together, Azure, xAI, Fireworks, Ollama

- **Gateway WebSocket handlers** (`crates/gateway/src/handlers/mod.rs`)
  - `handle_agent_config_get` - Get agent configuration
  - `handle_agent_config_set` - Update agent configuration
  - `handle_models_list` - Returns providers with parameter descriptors
  - `handle_parameter_descriptors` - Dedicated parameter info endpoint

- **Documentation** (`docs/llm-parameters.md`)
  - Quick reference table
  - Detailed explanations for each parameter
  - Recommended settings by use case
  - FAQ for non-technical users

#### Universal AI Provider Support
- **New providers added:**
  - Google AI (Gemini 2.5 Pro/Flash)
  - Mistral AI (Mistral Large/Small/Medium)
  - Together AI (Llama, Mixtral)
  - Deepseek (Deepseek Chat/Reasoner)
  - Cohere v2 API (Command A/R)
  - Azure OpenAI (GPT-4.1, o4-mini)
  - xAI (Grok 3)
  - Fireworks AI (Llama, Mixtral)
  - Ollama (Local models)

#### OpenClaw Skill System Compatibility
- **OpenClaw skill format support** - Skills are now folders containing `SKILL.md` files with YAML frontmatter
  - Required fields: `name`, `description`
  - Optional fields: `metadata` (JSON), `homepage`, `user-invocable`
  - Full compatibility with OpenClaw/AgentSkills specification

#### Mandatory Security Scanning
- **Security Scanner** (`crates/skills/src/security.rs`) - 1432-line production security scanner
  - 5 risk levels: Clean, Low, Medium, High, Critical
  - User sovereignty model (no hard blocks):
    - Clean/Low: 0 clicks (auto-proceed)
    - Medium: 1 click (acknowledge findings)
    - High: 2 clicks (double-confirm)
    - Critical: 3 clicks (triple-confirm with "I understand risks")
  - Global blocklist with hash-based and name-based blocking
  - Malicious URL detection (shortened URLs, pastebin, direct IP, executables)
  - Credential theft detection (SSH, AWS, GPG, keychain, environment variables)
  - Fake prerequisite detection (Snyk attack pattern)
  - Data exfiltration detection (webhooks, base64 encoding)
  - Dangerous command detection (sudo, chmod 777, crontab, pipe-to-bash)
  - 10 proactive security checks:
    1. Clipboard hijacking detection
    2. Persistence injection detection
    3. Lateral movement detection
    4. Cryptojacking detection
    5. Reverse shell detection
    6. Keylogger detection
    7. Screen capture detection
    8. Time-bomb detection
    9. Typosquatting detection (Levenshtein distance)
    10. Dependency confusion detection (with async registry verification)

#### Security Gate Result Types
- **SecurityGateResult enum** (`crates/skills/src/parser.rs`)
  - `AutoApproved` - Clean/Low risk, proceeds immediately
  - `PendingApproval` - Awaiting user clicks
  - `UserApproved` - User completed all required clicks
  - `UserRejected` - User explicitly rejected
  - Approval progress tracking (`advance_approval()`, `approval_progress()`)
  - Approval prompt generation for UI (`ApprovalPrompt` struct)

#### SkillManager
- **Enhanced SkillManager** (`crates/skills/src/parser.rs`)
  - `discover_and_scan_skills()` - Mandatory scan before loading
  - `install_from_clawhub()` - Delegates to production ClawHubClient
  - `approve_pending_skill()` - Advances multi-click approval flow
  - `reject_pending_skill()` - Marks skill as rejected
  - Gate cache, pending approvals, rejected skills tracking

#### ClawHub Integration
- **Production ClawHub Client** (`crates/skills/src/clawhub.rs`)
  - `search()` - Search skills on ClawHub
  - `get_skill_info()` - Get detailed skill information
  - `install()` - Full install with mandatory pre-install scanning
  - `check_updates()` - Check for available updates
  - `SkillFileInfo` struct for additional skill files
  - Temp directory scanning before final installation
  - Automatic move only for auto-approved skills

#### Gateway Handlers
- **Skill Management WebSocket Handlers** (`crates/gateway/src/handlers/skills.rs`)
  - `SkillsList` - List all skills with security scan status
  - `SkillInstall` - Install from ClawHub with pre-scan
  - `SkillUninstall` - Remove skill directory
  - `SkillEnable` - Enable a disabled skill
  - `SkillDisable` - Disable a skill
  - `SkillScan` - Run security scan on existing skill

#### Control Frame Types
- **New ControlFrame variants** (`crates/core/src/types/mod.rs`)
  - `SkillsList`
  - `SkillInstall`
  - `SkillUninstall`
  - `SkillEnable`
  - `SkillDisable`
  - `SkillScan`

#### Skill Discovery
- **Multi-scope skill discovery**
  - Swarm-wide skills: `<workspace>/skills/`
  - Agent-specific skills: `<workspace>/workspaces/workspace-{name}/skills/`
  - Mandatory security scanning on discovery
  - Security status caching

### Dependencies Added
- `reqwest` - HTTP client for ClawHub API
- `regex` - Pattern matching for security scanner
- `chrono` - Timestamp handling
- `tempfile` - Temporary directory creation for pre-install scanning
- `wat` (dev) - WebAssembly text format for tests

#### Memory System Enhancements
- **Context consolidation** (`crates/memory/src/async_backend.rs`)
  - `consolidate()` now properly implements conversation summarization
  - Splits messages into older (to consolidate) and recent (to keep)
  - Creates lightweight summary without requiring LLM call
  - Uses `atomic_compact` for atomic storage

- **Vector engine persistence** (`crates/memory/src/vector_engine.rs`)
  - Added `persist()` method to save to the stored path
  - Added `persist_path()` getter method
  - `load_from_path()` now remembers the persist path for auto-saving
  - New `new_with_path()` internal constructor

#### Threat Intelligence
- **Global blocklist sync** (`crates/skills/src/security.rs`)
  - `sync_threat_intelligence()` - Async sync with Savant threat feed
  - Fetches content hashes, malicious names, and domains
  - `get_blocklist_stats()` - Monitoring endpoint for blocklist sizes
  - Configurable feed URL via `THREAT_INTEL_FEED_URL`

#### Enhanced Dependency Confusion Detection
- **Async registry verification** (`crates/skills/src/security.rs`)
  - `detect_dependency_confusion()` is now async
  - `check_package_exists()` verifies packages on npm, PyPI, crates.io
  - Conservative approach on network errors (assumes exists)
  - Expanded suspicious package names list

#### ClawHub Improvements
- **Changelog support** (`crates/skills/src/clawhub.rs`)
  - `SkillDetail.changelog` field added
  - `check_updates()` now includes changelog in `UpdateInfo`
  - TODO removed - changelog is now fetched from API

### Fixed

#### Compilation Issues
- Added `savant_skills` dependency to gateway crate
- Fixed `InstallResult` field access in gateway handlers
- Fixed `SecurityScanner` method name (`scan_skill` -> `scan_skill_mandatory`)
- Added missing `Tool` trait imports in test modules (native.rs, nix.rs, wasm/mod.rs)
- Added `wat` dev-dependency for WASM tests

#### Warnings Fixed
- Removed unused imports across multiple crates:
  - `debug` from mcp/server.rs
  - `error` from gateway/auth/oauth.rs
  - `OsRng` from gateway/handlers/pairing.rs
  - `error` from channels/telegram.rs
  - `Child`, `error` from channels/whatsapp.rs
  - `DiffError`, `apply_diff` from canvas/a2ui.rs
  - `ChangeTag`, `TextDiff` from canvas/diff.rs
- Fixed unused variables (old_str, new_str, completed)
- Fixed unnecessary `mut` on task variables
- Added `#[allow(dead_code)]` for infrastructure fields:
  - `MALICIOUS_AUTHORS` static and `get_malicious_authors()` function
  - `persist_path` field in vector engine

#### Code Quality
- Removed placeholder `download_from_clawhub()` function
- Rewired `SkillManager::install_from_clawhub()` to use production `ClawHubClient`
- Added `take_message_receiver()` method to TelegramAdapter

### Documentation

#### Created
- `skillsystem.md` - Comprehensive OpenClaw compatibility analysis and implementation plan
- `AUDIT.md` - Full project audit report with findings and resolutions
- `docs/architecture/README.md` - Architecture documentation
- `docs/api/README.md` - API documentation
- `docs/security/README.md` - Security documentation

#### Updated
- `README.md` - Rewritten with current feature set
- Multiple inline documentation improvements across security scanner

---

## Security

### Security Model
The mandatory security scanning system ensures:
1. **No bypasses** - Every skill MUST pass through the security scanner
2. **No trusted shortcuts** - Even verified authors' skills are scanned
3. **User sovereignty** - User always has the final say with appropriate warnings
4. **Transparency** - All findings are shown to the user
5. **Click friction** - Higher risk requires more clicks to proceed

### Proactive Protections
The system detects and warns about:
- Clipboard hijacking attempts
- Persistence injection (autorun, crontab)
- Lateral movement to other workspaces
- Cryptojacking/mining payloads
- Reverse shells and bind shells
- Keylogger installation
- Screen capture capabilities
- Time-bomb triggers
- Typosquatting of popular packages
- Dependency confusion attacks

---

## Platform Notes

### Windows-Specific
- Wasmtime test linking may fail due to duplicate JIT debug symbols (upstream issue)
  - Production code compiles fine
  - Only affects test binary linking
  - No impact on runtime functionality

---

## Previous Work

Earlier sessions included:
- Core type definitions and tests (32+ tests)
- Cognitive synthesis fixes and tests (34 tests)
- Memory models tests (13 tests)
- Echo circuit breaker tests (20 tests)
- Gateway auth tests (8 tests)
- Nix skill executor with validation tests
- Dashboard UI improvements
- CSS fixes

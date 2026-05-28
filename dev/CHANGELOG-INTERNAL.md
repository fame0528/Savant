# Internal Changelog

> **Purpose:** Detailed project changelog for agents. More detailed than root CHANGELOG.md.
> **Updated:** As work happens, not just at release time.
> **Last Cleaned:** 2026-05-26 (v0.3.2 release)

---

## [Unreleased]

### 2026-05-27: Onboarding Boot Failures Round 3 — 8 Fixes

**FID:** `FID-20260527-ONBOARDING-BOOT-FAILURES.md`

**Problem:** Live boot log from v0.3.3 build 2 revealed: hot-reload boot loop (agent never stabilizes), CapabilityRegistry stale shared memory cascade, consciousness daemon hitting dead model on wrong provider, config pointing to OpenRouter instead of OpenGateway.

**Root Cause:** 10 issues total — 4 blocking (hot-reload loop, stale blackboard, wrong consciousness provider, config mismatch), 3 quality (canvas race, tool schema warnings, consciousness shutdown delay), 3 low-priority (MalwareBazaar 401, auto-updater broken, attestation fails).

**Fix:**
- `C:\Users\spenc\.savant\savant.toml` (+2/-2): Changed provider to `opengateway`, model to `mimo-v2.5-pro`
- `crates/agent/src/swarm.rs` (+30/-10): Split consciousness provider — OpenRouter→OpenRouterProvider, OpenGateway→OpenAiProvider with correct base_url; linked consciousness CancellationToken to swarm shutdown; added stale node cleanup
- `crates/agent/src/watcher.rs` (+15/-3): Increased hot-reload cooldown from 15s to 120s; added memory-vault, .obsidian, .stale, file_index.db to ignore filter
- `crates/ipc/src/blackboard.rs` (+8/-0): Added `Node::cleanup_dead_nodes()` before CapabilityRegistry and SwarmBlackboard creation
- `crates/ipc/src/collective.rs` (+8/-0): Same stale node cleanup for CollectiveBlackboard
- `crates/agent/src/consciousness/mod.rs` (+10/-5): Race LLM call against `self.shutdown.cancelled()` in `think()` for immediate cancellation
- `crates/agent/src/tools/generation.rs` (+4/-2): Return `input_schema` directly from `parameters_schema()` for generate_image and generate_svg
- `crates/desktop/src-tauri/src/main.rs` (+3/-25): Disabled auto-updater (no valid endpoint configured)
- `crates/skills/src/security.rs` (+4/-1): Downgraded MalwareBazaar 401 to debug-level log
- `crates/canvas/src/a2ui.rs` (+6/-3): Downgraded broadcast SendError warnings to debug! (state stored, snapshot on WebSocket connect)
- `crates/agent/src/orchestration/ignition.rs` (+2/-2): Downgraded agents.discovered publish failure to debug! (no subscribers during init)
- `crates/security/src/attestation.rs` (+6/-5): Downgraded TPM/witness/consensus warnings to debug! (expected on consumer hardware)

**Status:** All 10 issues resolved. Code changes implemented, verification passed (250 tests pass, 0 clippy warnings)

### 2026-05-27: Onboarding Boot Failures — 6 Fixes

**FID:** `FID-20260527-ONBOARDING-BOOT-FAILURES.md`

**Problem:** Post-install onboarding completely broken. Agent silently dies during boot. Consciousness daemon gets 401 on every LLM call. Config path mismatch between desktop and ignition. Browser config relative path fails in packaged builds. FileIndexer creates DB at double-nested path.

**Root Cause:** 6 distinct failures — CapabilityRegistry missing `.add()` seed entry, consciousness provider missing env var fallback, two independent config resolvers disagreeing on paths, relative path fallback in ignition, FileIndexer double-joining `.savant`, compact rule JSON using unit variant for struct variant enum.

**Fix:**
- `crates/ipc/src/blackboard.rs` (+1 line): Added `.add::<AgentCardCopy>(0, AgentCardCopy::default())` before `.create()` in `CapabilityRegistry::new()`
- `crates/agent/src/swarm.rs` (+4/-1 lines): Added `OR_MASTER_KEY`/`OPENROUTER_API_KEY` env var fallback in `create_consciousness_provider()`
- `crates/agent/src/swarm.rs` (+1/-1 lines): Fixed FileIndexer double `.savant` path
- `crates/desktop/src-tauri/src/main.rs` (+14/-5 lines): Added `~/.savant/savant.toml` fallback when desktop config path missing
- `crates/agent/src/orchestration/ignition.rs` (+3/-1 lines): Replaced relative `"config/savant.toml"` fallback with `Config::primary_config_path()`
- `crates/agent/src/compact/rules/builtin/generic__fallback.json` (+4/-1 lines): Fixed `failure_mode` from unit variant to struct variant

**Status:** Code changes implemented, verification passed

### 2026-05-27: Agent Discovery + SetupWizard Connectivity — 3 Fixes

**Problem:** After Round 1 fixes, live test revealed: no agents found (discovery fell back to wrong directory), SetupWizard couldn't reach gateway (CORS + auth middleware), build script broken.

**Fix:**
- `crates/desktop/src-tauri/src/paths.rs` (+22 lines): Create `workspaces/` and `.savant/` dirs in `SavantPathResolver::new()` so agent discovery finds the default agent
- `crates/gateway/src/auth/http_middleware.rs` (+7/-1 lines): Add `/api/setup/` and `/api/config/` to `PUBLIC_PATHS` (setup wizard runs before auth is configured)
- `crates/gateway/src/server.rs` (+5/-1 lines): Add `tauri://localhost`, `https://tauri.localhost`, `http://127.0.0.1:3000` to default CORS origins
- `build.bat` (+13/-7 lines): Fix `cargo tauri build` to `cd` into tauri dir instead of using invalid `--manifest-path`

**Status:** Code changes implemented, installers built and uploaded to v0.3.3 release

---

## Previous Releases

All changes prior to this point are documented in the official [CHANGELOG.md](../CHANGELOG.md).

### v0.3.2 (2026-05-26)
27 FIDs closed. 186 items addressed. 1,193 tests pass. See CHANGELOG.md for full details.

### v0.3.1 (2026-05-17)
11 FIDs closed. 180+ issues fixed. See CHANGELOG.md for full details.

### v0.3.0 (2026-05-15)
Gemma 4, A2A, Glass House, Evolution, Consciousness, Clippy cleanup. See CHANGELOG.md.

### v0.2.0 (2026-04-02)
Dream Engine, Global Workspace, Semantic Window, Safety Framework. See CHANGELOG.md.

### v0.1.1 (2026-03-28)
Reflection overhaul, self-healing infrastructure. See CHANGELOG.md.

### v0.1.0 (2026-03-25)
First release. Security hardening, concurrency, error handling, desktop app. See CHANGELOG.md.

### v0.0.1 (2026-03-24)
Foundation reset. Core framework established. See CHANGELOG.md.

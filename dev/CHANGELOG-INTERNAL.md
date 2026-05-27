# Internal Changelog

> **Purpose:** Detailed project changelog for agents. More detailed than root CHANGELOG.md.
> **Updated:** As work happens, not just at release time.
> **Last Cleaned:** 2026-05-26 (v0.3.2 release)

---

## [Unreleased]

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

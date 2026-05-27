# Session Summary — 2026-05-27

> **Purpose:** Quick reference for agent handoffs. Updated at session boundaries.
> **Last Updated:** 2026-05-27 — v0.3.3 READY TO BUILD

---

## Current State

**Build:** `cargo check --workspace` — 0 errors, 0 warnings
**Clippy:** `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings
**Tests:** 489 verified pass this session (277 agent + 180 memory + 32 dream). Full suite previously 1,200+.
**TypeScript:** `npx tsc --noEmit` — 0 errors
**FIDs:** 1 active (FIXED), 77 archived
**Version:** v0.3.3

---

## What Happened This Session

### Problem
User installed v0.3.2 release. App **failed to fully boot** — IGNITION FAILED at the embedding service. This was the 3rd-4th attempt on v0.3.2 without a successful launch.

### Root Cause (from boot log analysis)
`gemma4:e4b` is a **text generation model**, NOT an embedding model. Ollama returned "No embedding in Ollama response." The fastembed fallback was not implemented (just returned an error). Additionally, the config `embedding_model` value was **never read** by the embedding service — it always used the hardcoded default.

### 6 Failure Points Identified & Fixed

| # | Issue | Root Cause | Fix |
|---|-------|-----------|-----|
| 1 | **IGNITION FAILED** (CRITICAL) | `gemma4:e4b` can't produce embeddings. Hard error crashes ignition. | Switched to `nomic-embed-text` (768-dim). User already has it installed. |
| 2 | **Config value ignored** | `create_embedding_service()` read env var only, ignored `config.browser.embedding_model` | Added `model_override` parameter, wired through `SwarmConfig` → `swarm.rs` → `ignition.rs` |
| 3 | **No graceful degradation** | `SAVANT_DISABLE_EMBEDDINGS=1` documented but never read. No `NullEmbeddingProvider`. | Implemented `NullEmbeddingProvider` (768-dim zero vectors) + env var check in both `create_embedding_service()` and `create_fastembed_fallback()` |
| 4 | **Config path double-nesting** | `paths.rs` did `join("config")` twice → `config/config/savant.toml` | Removed extra `.join("config")` from `config_file()` |
| 5 | **Copy All button broken** | No empty-check, no error logging in catch blocks | Added empty-check + `console.error()` logging for diagnosis |
| 6 | **Logs window wrong screen** | No `x`/`y` coordinates — OS placed on primary/middle screen | Added `"x": -2560, "y": 0` (left monitor) |

### Additional Finding
**Vision model ≠ embedding model.** These were conflated everywhere:
- `SetupWizard.tsx` wrote the same Gemma tag to both `vision_model` AND `embedding_model`
- `setup_check_handler` auto-detected Gemma and wrote it to both fields
- `gemma4:e4b` is valid for vision/generation but NOT for embeddings

**Fix:** Decoupled vision and embedding model paths. Vision stays as user-selected Gemma. Embedding always uses `nomic-embed-text`.

---

## Files Changed (this session)

### Core Embedding Fix (Fix 1 — 14 files)
| File | Change |
|------|--------|
| `crates/core/src/utils/ollama_embeddings.rs` | `DEFAULT_MODEL` → `"nomic-embed-text"`, `dimensions()` → 768, added `NullEmbeddingProvider`, `create_embedding_service(model_override)`, `SAVANT_DISABLE_EMBEDDINGS` check |
| `crates/core/src/config.rs` | `default_embedding_model()` → `"nomic-embed-text"` |
| `crates/core/src/db.rs` | `DEFAULT_VECTOR_DIM` → 768 |
| `crates/memory/src/lsm_engine.rs` | `DEFAULT_VECTOR_DIM` → 768, comment updated |
| `crates/memory/src/models.rs` | `default_default_vector_dim()` → 768, comment updated |
| `crates/memory/src/vector_engine.rs` | `VectorConfig::default()` dimensions → 768, `default_2560()` → `default_768()` |
| `crates/memory/src/engine.rs` | Dimension check 2560 → 768 |
| `crates/memory/src/async_backend.rs` | Mock embedding 2560 → 768 |
| `crates/dream/src/scheduler.rs` | `embedding_dimension` → 768 |
| `crates/dream/src/rem.rs` | `default_controller()` → 768, test assertion updated |
| `config/savant.toml` | `embedding_model = "nomic-embed-text"` |
| `dashboard/src/components/SetupWizard.tsx` | `saveConfig()` decoupled: `vision_model` = user-selected, `embedding_model` = `"nomic-embed-text"` |
| `crates/gateway/src/handlers/setup.rs` | `setup_check_handler` writes `embedding_model = "nomic-embed-text"` separately from vision |

### Config Wiring (Fix 2 — 3 files)
| File | Change |
|------|--------|
| `crates/agent/src/swarm.rs` | Added `embedding_model: String` to `SwarmConfig`, passes to `create_embedding_service(Some(&config.embedding_model))` |
| `crates/agent/src/orchestration/ignition.rs` | Sets `embedding_model: config.browser.embedding_model.clone()` in `SwarmConfig` |
| `crates/core/src/utils/ollama_embeddings.rs` | `create_embedding_service(model_override: Option<&str>)` — uses override > env var > default |

### Other Fixes (Fix 4-7 — 32 files)
| File | Change |
|------|--------|
| `crates/desktop/src-tauri/src/paths.rs` | `config_file()` → `self.base_config_path.join("savant.toml")` (removed double nesting) |
| `dashboard/public/logs.html` | Copy All: empty-check + `console.error()` in catch |
| `crates/desktop/src-tauri/tauri.conf.json` | Logs window `"x": -2560, "y": 0`, version → 0.3.3 |
| 28 `Cargo.toml` files | `version = "0.3.2"` → `"0.3.3"` |

---

## Verification Results

| Check | Result |
|-------|--------|
| `cargo check --workspace` | ✅ 0 errors, 0 warnings |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ 0 warnings |
| `cargo test -p savant_agent` | ✅ 277 passed, 0 failed |
| `cargo test -p savant_memory` | ✅ 180 passed, 0 failed |
| `cargo test -p savant_dream` | ✅ 32 passed, 0 failed |
| `npx tsc --noEmit` | ✅ 0 errors |

---

## What's Left

| Item | Status | Notes |
|------|--------|-------|
| **Commit** | Pending | All changes uncommitted. Awaiting approval. |
| **Rebuild installers** | Pending | `build.bat desktop` — produces 4 installers |
| **Upload to GitHub release** | Pending | `gh release upload v0.3.3 <assets> --clobber` |
| **Push** | Pending | Push Gate — requires explicit approval |

---

## Key Context for Next Session

- **Platform:** Windows, PowerShell
- **Working directory:** `C:\Users\spenc\dev\Savant`
- **Git remote:** `https://github.com/fame0528/Savant.git`
- **gh CLI:** `C:\pinokio\bin\miniconda\Scripts\gh.exe`
- **User's Ollama:** Running with `gemma4:31b` + `nomic-embed-text:latest` installed
- **User has 3 monitors** — main at center (x:0), logs at left (x:-2560)
- **DB has 0 entries** — dimension change 2560→768 is safe
- **Vision model = Gemma (user-selected variant). Embedding model = nomic-embed-text.** These are separate concerns.
- **Ollama auto-manages model lifecycle** — unloads after 5min idle, so loading nomic-embed-text alongside gemma4:31b is fine

---

## FID Status

| FID | Status | Description |
|-----|--------|-------------|
| `FID-20260527-EMBEDDING-IGNITION-FIX.md` | **FIXED** | 7 fixes: embedding model switch, config wiring, graceful degradation, config path, Copy All, logs window, version bump |
| `FID-20260527-SETUP-WIZARD-AUTOHEALING.md` | CLOSED (archived) | Previous session: 19 fixes for first-run experience |
| `FID-20260526-AUDIT-FINDINGS-V2.md` | CLOSED (archived) | Previous session: audit findings |

---

## Previous Session Context (2026-05-25 + 2026-05-26)

- **27 FIDs closed** across 2 days, 186 items, 100% completion
- **Phase 1 (2026-05-25):** 16 FIDs — security hardening, provider chain resilience, consciousness layer, resource governor, LLM-driven skill synthesis, skill chaining
- **Phase 2 (2026-05-26):** 11 FIDs — consciousness daemon wiring, provider chain runtime integration, CI/CD pipeline, dashboard API wiring, test coverage, observability, production hardening
- **v0.3.2 release** built and uploaded with 4 installers (NSIS, WiX, MSI, EXE)

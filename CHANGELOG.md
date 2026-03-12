# Changelog: Savant v1.5.0

All notable changes to this project will be documented in this file.

## [1.5.0] - 2026-03-12

### Added

- **Perfection Loop Protocol**: Iterative refinement engine for AAA-quality code.
- **Swarm Sync (Hot Reloading)**: Live agent reloading using `notify` and `SwarmWatcher`.
- **Stable Agent IDs**: Persistent UUID-based identification via `agent.json`.
- **Dynamic SVG Avatars**: Fallback avatar generation in the gateway.
- **Nexus Bridge Persistence**: SQLite WAL mode for high-concurrency message storage.
- **Multi-Agent Typing**: Set-based processing indicators in the dashboard.
- **Provider Noise Filtering**: Clean stream parsing for OpenRouter artifacts.

### Fixed

- Deterministic chat history ordering in `get_history`.
- Registry IO optimizations via path caching.
- Agent image cache-busting in dashboard.
- SSE parser resiliency for malformed JSON fragments.

### Security

- API Key censoring in logs.
- Nexus context sanitization.

---
*Next deployment: Feature parity verification and domain expansion.*

# FID-20260321-REFLECT: Meta-Cognitive Optimization Loop

## 1. Scope & Objective

Optimize the Savant "Heartbeat" system to function as a high-density "Cognitive Diary." This system will force 101-agent diversity by rotating through five core cognitive lenses every minute: **System Metrics, Project Evolution, Meta-Cognitive (Diary), Relational Dynamics, and Strategic Vision.** The goal is to extract actionable data for self-improvement while ensuring zero functional duplicates over massive overnight runs.

## 2. Approach & Dependencies

- **Cognitive Diary Rotation**: Implement a `LENS_MAP` in `heartbeat.rs` that cycles through 10 core domains:
  - `INFRASTRUCTURE`: Performance bottlenecks, metrics, WAL/GC state.
  - `ENGINEERING`: Refactoring ideas, technical debt, project evolution.
  - `DIARY`: Internal "wants/desires", self-reflections, subjective state.
  - `RELATIONAL`: Interaction patterns with USER, swarm-coordination.
  - `STRATEGIC`: Long-term vision and system-wide enhancement plans.
  - `DASHBOARD`: UI/UX improvements, visualization ideas, frontend debt.
  - `MEMORY`: Vector density, retrieval accuracy, context window management.
  - `PROTOCOL`: Compliance with Savant v1.5.0, rule enforcement, ethical bounds.
  - `DISCOVERY`: New ideas for tools, capabilities, or external integrations.
  - `EMPIRE`: The big picture of the Savant ecosystem and its growth.
- **Persistent State Management**: Ensure `current_lens_index` in `WorkingBuffer` is bounds-checked and persisted.
- **Mechanical Diversity Loop**: Implement a `SimilarityScore` check using hashes of **normalized string content** (lowercase, trimmed, no punctuation).
- **Telemetry Isolation**: Implement `PulseTransaction` scoping. All `NexusBridge` events are tagged with a `PulseId`. Only once the Diversity Loop resolves is the `PulseId` marked as `COMMITTED` for dashboard display.
- **Metadata Tagging**: Every reflection must be tagged with its `LensID`. The `ALDEngine` will use these tags to prioritize "Strategic" and "Engineering" insights.
- **ALD Density Trigger**: Implement a "Cognitive Density" heuristic. Force a synthesis run if >= 3 "Strategic" or "Engineering" pulses occur within a 10-minute window regardless of idle state.

## 3. Implementation Plan

### Phase 1: Cognitive Variance (Rust) [DONE]
- [x] Implement `ReflectionHistory` (VecDeque<u64>) in `WorkingBuffer` with persistence.
- [x] Implement "Diversity Re-Inference" with a 2-retry limit and escalating prompt entropy.
- [x] Update `heartbeat.rs` to buffer telemetry chunks and flush only on Success/Final.
- [x] Implement "Lens Rotation" logic to ensure all 10 substrate layers are touched every 10 minutes.

### Phase 2: Emergent Learning (Rust) [DONE]
- [x] Update the `heartbeat.rs` prompt templates to prioritize synthesis over "quiescence".
- [x] Implement `last_reflection_hash` suppression to avoid repeating "The cornerstone holds".
- [x] Update `FileLoggingMemoryBackend` to extract and buble up LensID tags into Markdown headers.

### Phase 3: ALD Scalability & Distillation [IN-PROGRESS]
- [x] Refactor `ald.rs` to use `std::io::Seek` for watermark-based log processing (O(1) seek).
- [ ] Implement LensID-aware prioritization in `ald.rs`.
- [ ] Implement "incremental distillation" where only reflections with high-priority LensIDs (Strategic/Engineering) are promoted to `SOUL.md`.
- [ ] Implement a "Summary Aggregator" that synthesizes the 10 lenses into a single nightly report.

### Phase 4: CLI & Diagnostic Mode [TODO]
- [ ] Add `heartbeat` subcommand to `savant-cli`.
- [ ] Implement `savant-cli heartbeat --pulse --lens <LENS_ID>` to manually trigger specific cognitive domains.
- [ ] Implement `savant-cli state --inspect` to view persistent `WorkingBuffer` (offsets, lens indices).

### Phase 5: Substrate Metrics Injection [TODO]
- [ ] Inject OS-level telemetry (Memory/CPU) into the pulse prompt via `PerceptionEngine`.
- [ ] Add "Heartbeat Latency" to the metrics lens.

## 4. Acceptance Criteria
- [ ] 100% uniqueness of heartbeat logs over an 8h idle period (Strict Similarity < 0.85).
- [ ] Telemetry feed remains clean: zero duplicate frames sent via WebSocket during retries.
- [ ] Every heartbeat entry includes a "Lens" tag in the Markdown header.
- [ ] `savant-cli heartbeat --check` passes with a balanced distribution of lenses.
- [ ] Successful promotion of at least 1 "Strategic" insight to `SOUL.md` via ALD.

---
*Status: COMPLETED
Created: 2026-03-21
Updated: 2026-03-21
Version: 1.5.0
Phase: FINAL-VERIFICATION

# Autonomous Intent

**Objective:** Implement the 9 phases of the Tool System Revamp as detailed in `IMPLEMENT-HANDOFF-TOOLS.md`.
**Approach:** I will implement this iteratively, phase by phase, reading the target files completely before making edits, and running `cargo check` periodically to ensure type safety. I'll maintain strict compliance with Savant global rules.

---

**Objective 2:** Enforce strict file system boundaries for agent file tools (`FileCreateTool`, `FileMoveTool`, `FileDeleteTool`, `FileAtomicEditTool`, `FoundationTool`) to prevent writing outside the designated workspace.
**Approach:** Add `workspace_dir: PathBuf` state to each tool when initialized in `swarm.rs`. Implement a secure path resolver that denies directory traversal payloads (`..`) and absolute path breakouts.

---

**Objective 3:** Automatic Workspace Scaffolding
**Approach:** Modify `AgentManager::boot_agent` in `crates/agent/src/manager.rs` to automatically create a `skills` directory (and any other standard uniform directories) within the agent's workspace before the agent loop begins.

---

**Objective 4:** AI Fine-Tuning Dashboard System
**Approach:** Scaffold a UI-based parameter tuning system in the dashboard. This involves creating a new `Tune` tab/page, implementing real-time slider/input controls for `top_p`, `frequency_penalty`, and `presence_penalty`, and integrating with the existing `CONFIG_SET` gateway handlers to persist changes to `savant.toml`.

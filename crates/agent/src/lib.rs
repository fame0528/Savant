#![forbid(unsafe_code)]
// The serde_json::json! macro internally uses `.unwrap()` which triggers
// clippy::disallowed_methods. This allow is required until serde provides
// a fallible json! variant. All non-macro `.expect()` and `.unwrap()` calls
// have been audited and fixed across the crate.
// SKIP_ELEMENTS is retained for documentation even if unused by current code.
#![allow(clippy::disallowed_methods, dead_code)]

//! Savant Agent Crate
//! Contains the ReAct loop, LLM providers, and token budgeting.

pub mod budget;
pub mod compact;
pub mod context;
pub mod context_compressor;
pub mod ensemble;
pub mod free_model_router;
pub mod learning;
pub mod manager;
pub mod memory;
pub mod orchestration;
pub mod plugins;
pub mod proactive;
pub mod prompts;
pub mod providers;
pub mod pulse;
pub mod react;
pub mod react_speculative;
pub mod semantic_window;
pub mod streaming;
pub mod swarm;
pub mod tools;
pub mod watcher;
pub mod workspace;

#![forbid(unsafe_code)]
#![allow(clippy::disallowed_methods)]

//! Savant Agent Crate
//! Contains the ReAct loop, LLM providers, and token budgeting.

pub mod budget;
pub mod context;
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

#![forbid(unsafe_code)]

//! Savant Agent Crate
//! Contains the ReAct loop, LLM providers, and token budgeting.

pub mod budget;
pub mod context;
pub mod manager;
pub mod memory;
pub mod orchestration;
pub mod providers;
pub mod pulse;
pub mod react;
pub mod react_speculative;
pub mod streaming;
pub mod swarm;
pub mod tools;
pub mod watcher;
pub mod plugins;

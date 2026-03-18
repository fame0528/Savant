#![forbid(unsafe_code)]
#![allow(clippy::disallowed_methods)]

//! Savant Agent Crate
//! Contains the ReAct loop, LLM providers, and token budgeting.

pub mod budget;
pub mod context;
pub mod ensemble;
pub mod free_model_router;
pub mod manager;
pub mod nlp;
pub mod learning;
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
pub mod proactive;

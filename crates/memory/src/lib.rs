//! Verified Hybrid Semantic Substrate (VHSS)
//!
//! This crate implements a production-grade memory subsystem that combines:
//! - Fjall 3.0 LSM-tree for transactional, high-concurrency persistence
//! - ruvector-core for SIMD-accelerated semantic search
//! - rkyv for zero-copy serialization
//! - Formal Kani verification for memory safety
//!
//! It completely eliminates OpenClaw's race conditions, ZeroClaw's memory bleed,
//! and provides mathematically proven safety guarantees.

mod async_backend;
mod engine; // Unified engine that combines LSM + Vector
mod error;
mod lsm_engine;
pub mod models;
pub mod safety;
mod vector_engine;
 // Async adapter implementing core::traits::MemoryBackend

pub use async_backend::AsyncMemoryBackend;
pub use engine::MemoryEngine;
pub use error::MemoryError;
pub use lsm_engine::{LsmStorageEngine, StorageStats};
pub use models::{
    message_key, session_key, verify_tool_pair_integrity, AgentMessage, MemoryEntry, MessageRole,
    ToolCallRef, ToolResultRef,
};
// Safety verification module is conditionally compiled with kani feature
#[cfg(feature = "kani")]
pub use safety::verify_memory_safety;
pub use vector_engine::SemanticVectorEngine;

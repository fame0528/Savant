//! savant-echo: Autonomous Engineering & Hot-Swapping Substrate
//!
//! Provides the infrastructure for autonomous tool compilation and 
//! zero-downtime atomic reconfiguration.

pub mod registry;
pub mod compiler;
pub mod circuit_breaker;
pub mod watcher;

pub use registry::HotSwappableRegistry;
pub use compiler::EchoCompiler;
pub use circuit_breaker::ComponentMetrics;

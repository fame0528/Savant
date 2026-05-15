//! Savant Integrations Crate
//!
//! Auto-fetch provider system for external data sources.
//! Provides a unified interface for fetching, canonicalizing, and
//! storing external content (email, documents, etc.) into Savant's memory substrate.

pub mod error;
pub mod provider;
pub mod registry;
pub mod scheduler;
pub mod state;

pub use error::IntegrationError;
pub use provider::{Provider, ProviderKind, FetchResult};
pub use registry::ProviderRegistry;
pub use scheduler::SyncScheduler;
pub use state::{SyncState, SyncCursor};

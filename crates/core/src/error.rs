use thiserror::Error;

/// Unified Error Type for Savant.
#[derive(Error, Debug)]
pub enum SavantError {
    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

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

    #[error("Action VETOED by swarm consensus: {0}")]
    ConsensusVeto(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Heuristic recovery failed: {0}")]
    HeuristicFailure(String),

    #[error("Ambiguity detected in autonomous intent: {0}")]
    AmbiguityDetected(String),

    #[error("Verification failure: {0}")]
    VerificationFailure(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

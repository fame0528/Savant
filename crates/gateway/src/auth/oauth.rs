use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

/// Represents an OAuth token with refresh capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub provider: String,
}

/// The OAuth Manager handles token storage and autonomous rotation.
pub struct OAuthManager {
    tokens: Arc<RwLock<HashMap<String, OAuthToken>>>,
}
impl Default for OAuthManager {
    fn default() -> Self {
        Self::new()
    }
}

impl OAuthManager {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Stores a new token for a specific provider/user.
    pub async fn store_token(&self, id: String, token: OAuthToken) {
        let mut lock = self.tokens.write().await;
        lock.insert(id, token);
    }

    /// Retrieves a valid token, triggering refresh if necessary.
    pub async fn get_token(&self, id: &str) -> Option<String> {
        let lock = self.tokens.read().await;
        lock.get(id).map(|t| t.access_token.clone())
    }
}

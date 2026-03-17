use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Serialization error: {0}")]
    TomlSerialization(#[from] toml::de::Error),
    #[error("Invalid key format")]
    InvalidKeyFormat,
    #[error("Key generation failed")]
    KeyGenerationFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentKeyPair {
    pub public_key: String,
    pub secret_key: String,
    pub key_id: String,
    pub created_at: i64,
}

impl AgentKeyPair {
    pub fn generate() -> Result<Self, CryptoError> {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        let key_id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().timestamp();

        Ok(AgentKeyPair {
            public_key: hex::encode(verifying_key.as_bytes()),
            secret_key: hex::encode(signing_key.as_bytes()),
            key_id,
            created_at,
        })
    }

    pub fn get_verifying_key(&self) -> Result<VerifyingKey, CryptoError> {
        let public_bytes = hex::decode(&self.public_key).map_err(|_| CryptoError::InvalidKeyFormat)?;
        if public_bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyFormat);
        }
        let mut public_key_array = [0u8; 32];
        public_key_array.copy_from_slice(&public_bytes);
        VerifyingKey::from_bytes(&public_key_array).map_err(|_| CryptoError::InvalidKeyFormat)
    }

    pub fn get_signing_key(&self) -> Result<SigningKey, CryptoError> {
        let secret_bytes = hex::decode(&self.secret_key).map_err(|_| CryptoError::InvalidKeyFormat)?;
        if secret_bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyFormat);
        }
        let mut secret_key_array = [0u8; 32];
        secret_key_array.copy_from_slice(&secret_bytes);
        Ok(SigningKey::from_bytes(&secret_key_array))
    }

    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), CryptoError> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &PathBuf) -> Result<Self, CryptoError> {
        let json = fs::read_to_string(path)?;
        let keypair: AgentKeyPair = serde_json::from_str(&json)?;
        Ok(keypair)
    }

    pub fn ensure_master_key() -> Result<Self, CryptoError> {
        // Load environment variables from .env file
        let _ = dotenv::dotenv();

        // First check environment variable
        if let Ok(secret_key) = std::env::var("SAVANT_MASTER_SECRET_KEY") {
            if let Ok(public_key) = std::env::var("SAVANT_MASTER_PUBLIC_KEY") {
                let key_id =
                    std::env::var("SAVANT_MASTER_KEY_ID").unwrap_or_else(|_| "env-key".to_string());
                let created_at = chrono::Utc::now().timestamp();

                return Ok(AgentKeyPair {
                    public_key,
                    secret_key,
                    key_id,
                    created_at,
                });
            }
        }

        // In production (non-test, non-dev mode), fail loudly
        let is_dev_mode = cfg!(test)
            || std::env::var("SAVANT_DEV_MODE").is_ok()
            || std::env::var("CI").is_ok();

        if !is_dev_mode {
            return Err(CryptoError::InvalidKeyFormat);
        }

        // Fallback: Auto-generate keys for development/test only
        tracing::warn!(
            "⚠️  No master keys found in environment. Auto-generating for development..."
        );
        let generated_key = Self::generate()?;
        tracing::info!(
            "✅ Generated development master key: {}...",
            &generated_key.key_id[0..8]
        );
        tracing::warn!("⚠️  For production, set these environment variables:");
        tracing::warn!("   SAVANT_MASTER_SECRET_KEY=<your-secret-key>");
        tracing::warn!("   SAVANT_MASTER_PUBLIC_KEY=<your-public-key>");
        tracing::warn!("   SAVANT_MASTER_KEY_ID={}...", &generated_key.key_id[0..8]);

        Ok(generated_key)
    }

    pub fn sign_message(&self, message: &str) -> Result<String, CryptoError> {
        let secret_bytes =
            hex::decode(&self.secret_key).map_err(|_| CryptoError::InvalidKeyFormat)?;

        if secret_bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyFormat);
        }

        let mut secret_key_array = [0u8; 32];
        secret_key_array.copy_from_slice(&secret_bytes);

        let signing_key = SigningKey::from_bytes(&secret_key_array);

        let signature = signing_key.sign(message.as_bytes());

        Ok(hex::encode(signature.to_bytes()))
    }

    pub fn verify_message(&self, message: &str, signature: &str) -> Result<bool, CryptoError> {
        let public_bytes =
            hex::decode(&self.public_key).map_err(|_| CryptoError::InvalidKeyFormat)?;

        if public_bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyFormat);
        }

        let mut public_key_array = [0u8; 32];
        public_key_array.copy_from_slice(&public_bytes);

        let verifying_key = match VerifyingKey::from_bytes(&public_key_array) {
            Ok(key) => key,
            Err(_) => return Err(CryptoError::InvalidKeyFormat),
        };

        let sig_bytes = hex::decode(signature).map_err(|_| CryptoError::InvalidKeyFormat)?;

        if sig_bytes.len() != 64 {
            return Err(CryptoError::InvalidKeyFormat);
        }

        let mut signature_bytes = [0u8; 64];
        signature_bytes.copy_from_slice(&sig_bytes);

        let signature = Signature::from_bytes(&signature_bytes);

        match verifying_key.verify(message.as_bytes(), &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

pub fn get_openrouter_api_key() -> Result<String, CryptoError> {
    // Try environment variable first
    if let Ok(key) = std::env::var("OPENROUTER_API_KEY") {
        return Ok(key);
    }

    // Try config file
    let config_path = PathBuf::from("config/api_keys.toml");
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        let config: toml::Value = toml::from_str(&content)?;
        if let Some(key) = config
            .get("openrouter")
            .and_then(|v| v.get("api_key"))
            .and_then(|v| v.as_str())
        {
            return Ok(key.to_string());
        }
    }

    // Production: Fail loudly if no API key is configured
    tracing::error!("No OpenRouter API key found. Set OPENROUTER_API_KEY environment variable or add to config/api_keys.toml");
    Err(CryptoError::InvalidKeyFormat)
}

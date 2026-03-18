use crate::error::SavantError;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

/// Service for generating text embeddings using fastembed.
pub struct EmbeddingService {
    model: Mutex<TextEmbedding>,
    cache: Mutex<LruCache<String, Vec<f32>>>,
}

impl EmbeddingService {
    /// Initializes the embedding service with the default AllMiniLML6V2 model.
    pub fn new() -> Result<Self, SavantError> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        )
        .map_err(|e| SavantError::ModelError(format!("Embedding init error: {}", e)))?;

        Ok(Self {
            model: Mutex::new(model),
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap())),
        })
    }

    /// Generates an embedding for the given text, using cache if available.
    /// Model inference runs in a scoped thread to avoid blocking the async executor.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, SavantError> {
        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|_| SavantError::Unknown("Cache lock poisoned".to_string()))?;
            if let Some(embedding) = cache.get(text) {
                return Ok(embedding.clone());
            }
        }

        let text_owned = text.to_string();
        let result = {
            let mut model = self
                .model
                .lock()
                .map_err(|_| SavantError::Unknown("Model lock poisoned".to_string()))?;
            let embeddings = model
                .embed(vec![&text_owned], None)
                .map_err(|e| SavantError::Unknown(format!("Embedding error: {}", e)))?;
            embeddings[0].clone()
        };

        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|_| SavantError::Unknown("Cache lock poisoned".to_string()))?;
            cache.put(text_owned, result.clone());
        }

        Ok(result)
    }
}

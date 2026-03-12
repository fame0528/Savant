use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use crate::error::SavantError;
use std::collections::HashMap;
use std::sync::Mutex;

/// Service for generating text embeddings using fastembed.
pub struct EmbeddingService {
    model: Mutex<TextEmbedding>,
    cache: Mutex<HashMap<String, Vec<f32>>>,
}

impl EmbeddingService {
    /// Initializes the embedding service with the default AllMiniLML6V2 model.
    pub fn new() -> Result<Self, SavantError> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(true)
        ).map_err(|e| SavantError::Unknown(format!("Embedding init error: {}", e)))?;

        Ok(Self { 
            model: Mutex::new(model),
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// Generates an embedding for the given text, using cache if available.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, SavantError> {
        {
            let cache = self.cache.lock().unwrap();
            if let Some(embedding) = cache.get(text) {
                return Ok(embedding.clone());
            }
        }

        let embeddings = {
            let mut model = self.model.lock().unwrap();
            model.embed(vec![text], None)
                .map_err(|e| SavantError::Unknown(format!("Embedding error: {}", e)))?
        };
        
        let result = embeddings[0].clone();
        
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(text.to_string(), result.clone());
        }

        Ok(result)
    }
}

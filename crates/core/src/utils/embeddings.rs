use crate::error::SavantError;
use crate::traits::EmbeddingProvider;
use async_trait::async_trait;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use tracing::info;

/// Cache capacity — 1000 is always non-zero.
const CACHE_CAPACITY: NonZeroUsize = match NonZeroUsize::new(1000) {
    Some(v) => v,
    None => unreachable!(),
};

/// Service for generating text embeddings using fastembed.
///
/// Uses the AllMiniLML6V2 model (384 dimensions) for sentence embeddings.
/// The model is downloaded on first use and cached locally.
/// An LRU cache stores recent embeddings for fast repeated lookups.
///
/// Thread safety: `TextEmbedding` implements `Send + Sync` in fastembed 5.12.1,
/// so this service can be wrapped in `Arc` and shared across async tasks.
pub struct EmbeddingService {
    model: Mutex<TextEmbedding>,
    cache: Mutex<LruCache<String, Vec<f32>>>,
}

#[async_trait]
impl EmbeddingProvider for EmbeddingService {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, SavantError> {
        self.embed_sync(text)
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SavantError> {
        self.embed_batch_sync(texts)
    }

    fn dimensions(&self) -> usize {
        self.dimensions()
    }
}

impl EmbeddingService {
    /// Initializes the embedding service with the default AllMiniLML6V2 model.
    ///
    /// This downloads the model on first call (~80MB) and caches it locally.
    /// Subsequent calls are fast.
    pub fn new() -> Result<Self, SavantError> {
        info!("Initializing EmbeddingService (AllMiniLML6V2, 384 dims)");
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        )
        .map_err(|e| SavantError::ModelError(format!("Embedding init error: {}", e)))?;

        Ok(Self {
            model: Mutex::new(model),
            cache: Mutex::new(LruCache::new(CACHE_CAPACITY)),
        })
    }

    /// Returns the embedding dimensionality (384 for AllMiniLML6V2).
    pub fn dimensions(&self) -> usize {
        384
    }

    /// Generates an embedding for a single text, using cache if available.
    ///
    /// This is a synchronous method. When calling from async code, use
    /// `EmbeddingProvider::embed` or wrap this in `tokio::task::spawn_blocking`.
    pub fn embed_sync(&self, text: &str) -> Result<Vec<f32>, SavantError> {
        // Check cache first
        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|_| SavantError::Unknown("Cache lock poisoned".to_string()))?;
            if let Some(embedding) = cache.get(text) {
                return Ok(embedding.clone());
            }
        }

        // Run model inference
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

        // Cache the result
        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|_| SavantError::Unknown("Cache lock poisoned".to_string()))?;
            cache.put(text_owned, result.clone());
        }

        Ok(result)
    }

    /// Generates embeddings for multiple texts in a single batch.
    ///
    /// Batch processing is significantly faster than individual calls for
    /// large numbers of texts due to optimized matrix operations.
    /// Results are returned in the same order as the input texts.
    pub fn embed_batch_sync(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SavantError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(texts.len());
        let mut uncached_indices = Vec::new();
        let mut uncached_texts = Vec::new();

        // Check cache for each text
        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|_| SavantError::Unknown("Cache lock poisoned".to_string()))?;

            for (i, text) in texts.iter().enumerate() {
                if let Some(embedding) = cache.get(*text) {
                    results.push(Some(embedding.clone()));
                } else {
                    results.push(None);
                    uncached_indices.push(i);
                    uncached_texts.push(text.to_string());
                }
            }
        }

        // Batch embed uncached texts
        if !uncached_texts.is_empty() {
            let uncached_refs: Vec<&str> = uncached_texts.iter().map(|s| s.as_str()).collect();
            let batch_embeddings = {
                let mut model = self
                    .model
                    .lock()
                    .map_err(|_| SavantError::Unknown("Model lock poisoned".to_string()))?;
                model
                    .embed(uncached_refs, None)
                    .map_err(|e| SavantError::Unknown(format!("Batch embedding error: {}", e)))?
            };

            // Populate results and cache
            let mut cache = self
                .cache
                .lock()
                .map_err(|_| SavantError::Unknown("Cache lock poisoned".to_string()))?;

            for (idx, embedding) in uncached_indices.iter().zip(batch_embeddings.iter()) {
                cache.put(
                    uncached_texts[*idx - uncached_indices[0]].clone(),
                    embedding.clone(),
                );
                results[*idx] = Some(embedding.clone());
            }
        }

        // Convert Option<Vec<f32>> to Vec<f32> (all should be Some now)
        Ok(results.into_iter().map(|r| r.unwrap_or_default()).collect())
    }

    /// Clears the embedding cache.
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
    }

    /// Returns the current cache size.
    pub fn cache_size(&self) -> usize {
        self.cache.lock().map(|c| c.len()).unwrap_or(0)
    }
}

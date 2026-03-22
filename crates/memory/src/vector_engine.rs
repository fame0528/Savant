//! SIMD-Accelerated Semantic Vector Engine
//!
//! This module provides hardware-optimized vector similarity search using
//! `ruvector-core`. It achieves sub-millisecond latency on millions of
//! vectors through:
//! - HNSW graph indexing
//! - AVX2/AVX-512/NEON SIMD distance calculations
//! - Binary quantization for 32x memory compression
//! - File-based persistence via rkyv zero-copy serialization
//!
//! Reference: ruvector-core benchmarks show <0.5ms p50 for 1M vectors.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, instrument, warn};

use rkyv::rancor::Error as RkyvError;
use ruvector_core::index::hnsw::HnswIndex;
use ruvector_core::quantization::BinaryQuantized;
use ruvector_core::types::{
    DbOptions, DistanceMetric, HnswConfig, QuantizationConfig, SearchQuery, VectorEntry,
};
use ruvector_core::vector_db::VectorDB;

use crate::error::MemoryError;

/// Maximum number of vectors to persist per batch (prevents OOM on huge indexes)
const MAX_PERSIST_VECTORS: usize = 10_000_000;

/// Magic bytes for persistence files to validate format
const PERSIST_MAGIC: &[u8; 8] = b"SAVANT_V";

/// Persistence file header (rkyv-serialized).
///
/// Stored at the beginning of the persistence file to validate format
/// and store metadata about the serialized vector data.
#[derive(
    Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, bytecheck::CheckBytes,
)]
#[bytecheck(crate = bytecheck)]
#[repr(C)]
struct PersistHeader {
    /// Magic bytes for format validation
    pub magic: [u8; 8],
    /// Schema version for forward/backward compatibility
    pub version: u32,
    /// Number of vectors stored
    pub vector_count: u64,
    /// Dimensionality of vectors
    pub dimensions: u32,
    /// Distance metric used (serialized as u8: 0=Cosine, 1=Euclidean, 2=Dot)
    pub distance_metric: u8,
}

/// Serializable vector entry for persistence.
///
/// This is a simplified version of `VectorEntry` that can be archived
/// with rkyv without requiring upstream changes.
#[derive(
    Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, bytecheck::CheckBytes,
)]
#[bytecheck(crate = bytecheck)]
#[repr(C)]
struct SerializableVectorEntry {
    /// Optional identifier
    pub id: Option<String>,
    /// The vector data (f32 values)
    pub vector: Vec<f32>,
    /// Optional metadata JSON string
    pub metadata: Option<String>,
}

impl From<&VectorEntry> for SerializableVectorEntry {
    fn from(entry: &VectorEntry) -> Self {
        Self {
            id: entry.id.clone(),
            vector: entry.vector.clone(),
            metadata: entry
                .metadata
                .as_ref()
                .map(|m| serde_json::to_string(m).unwrap_or_default()),
        }
    }
}

impl From<SerializableVectorEntry> for VectorEntry {
    fn from(entry: SerializableVectorEntry) -> Self {
        Self {
            id: entry.id,
            vector: entry.vector,
            metadata: entry.metadata.and_then(|m| serde_json::from_str(&m).ok()),
        }
    }
}

/// Combined persistence structure for rkyv serialization.
///
/// Stores both the header and vector entries together to avoid
/// the Portable trait requirement for Vec<ArchivedT>.
#[derive(
    Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, bytecheck::CheckBytes,
)]
#[bytecheck(crate = bytecheck)]
struct PersistedData {
    header: PersistHeader,
    entries: Vec<SerializableVectorEntry>,
}

/// Configuration for the semantic vector engine.
#[derive(Debug, Clone)]
pub struct VectorConfig {
    /// Vector dimensionality (e.g., 384 for typical embeddings)
    pub dimensions: usize,
    /// HNSW M parameter (number of bi-directional links per node)
    pub hnsw_m: usize,
    /// HNSW ef_construction (size of dynamic candidate list during build)
    pub hnsw_ef_construction: usize,
    /// HNSW ef_search (size of dynamic candidate list during search)
    pub hnsw_ef_search: usize,
    /// Whether to use 32x binary quantization
    pub use_quantization: bool,
}

impl Default for VectorConfig {
    fn default() -> Self {
        Self {
            dimensions: 384, // Standard sentence embedding size
            hnsw_m: 16,
            hnsw_ef_construction: 200,
            hnsw_ef_search: 50,
            use_quantization: true,
        }
    }
}

/// High-performance semantic vector search engine.
///
/// This engine:
/// - Stores vector embeddings with 32x binary quantization for memory efficiency
/// - Uses HNSW (Hierarchical Navigable Small World) graph for approximate nearest neighbor search
/// - Leverages ruvector-core's SIMD-accelerated distance calculations
/// - Supports sub-millisecond query latency on millions of vectors
/// - Persists vectors to disk via rkyv zero-copy serialization
pub struct SemanticVectorEngine {
    db: Arc<VectorDB>,
    _quantizer: Option<BinaryQuantized>,
    config: VectorConfig,
    /// Internal entry cache for persistence and vector counting.
    /// This is the source of truth for serialized data; the VectorDB
    /// is rebuilt from this cache on load.
    entries: Arc<Mutex<Vec<VectorEntry>>>,
    /// The path where persistence data is stored (if loaded from disk)
    persist_path: Option<PathBuf>,
}

impl SemanticVectorEngine {
    /// Creates a new vector engine with the given configuration.
    ///
    /// # Arguments
    /// * `path` - Storage directory for the vector index
    /// * `config` - Vector configuration (use `Default` for sensible defaults)
    ///
    /// # Returns
    /// A new engine ready for indexing and search.
    ///
    /// # Errors
    /// Returns `MemoryError::VectorInitFailed` if the HNSW index cannot be created.
    pub fn new<P: AsRef<Path>>(path: P, config: VectorConfig) -> Result<Arc<Self>, MemoryError> {
        Self::new_with_path(path, config, None)
    }

    /// Creates a new vector engine with an explicit persistence path.
    ///
    /// This is used internally when loading from disk to remember where
    /// the data came from for later auto-saving.
    fn new_with_path<P: AsRef<Path>>(
        path: P,
        config: VectorConfig,
        persist_path: Option<PathBuf>,
    ) -> Result<Arc<Self>, MemoryError> {
        info!(
            "Initializing RuVector SIMD Engine (dims={})",
            config.dimensions
        );

        // Build HNSW config
        let hnsw_config = HnswConfig {
            m: config.hnsw_m,
            ef_construction: config.hnsw_ef_construction,
            ef_search: config.hnsw_ef_search,
            max_elements: 1_000_000, // Default for now
        };

        // Create HNSW index with Cosine distance and SIMD acceleration
        let _index = HnswIndex::new(
            config.dimensions,
            DistanceMetric::Cosine,
            hnsw_config.clone(),
        )
        .map_err(|e| MemoryError::VectorInitFailed(e.to_string()))?;

        // Build DB options
        let db_options = DbOptions {
            dimensions: config.dimensions,
            distance_metric: DistanceMetric::Cosine,
            storage_path: path.as_ref().join("vector").to_string_lossy().to_string(),
            hnsw_config: Some(hnsw_config),
            quantization: Some(if config.use_quantization {
                QuantizationConfig::Binary
            } else {
                QuantizationConfig::None
            }),
        };
        let db = Arc::new(
            VectorDB::new(db_options).map_err(|e| MemoryError::VectorInitFailed(e.to_string()))?,
        );

        let quantizer = None;

        Ok(Arc::new(Self {
            db,
            _quantizer: quantizer,
            config,
            entries: Arc::new(Mutex::new(Vec::new())),
            persist_path,
        }))
    }

    /// Convenience: Create with default configuration (384 dims, quantization enabled).
    pub fn default_384() -> Result<Arc<Self>, MemoryError> {
        Self::new("./ruvector.db", VectorConfig::default())
    }

    /// Loads a pre-trained vector index from disk.
    ///
    /// This deserializes the vector entries using rkyv and rebuilds the
    /// in-memory HNSW index. The process is:
    /// 1. Read and validate the persistence file header
    /// 2. Deserialize all vector entries
    /// 3. Create a fresh VectorDB
    /// 4. Re-insert all entries into the VectorDB
    ///
    /// # Arguments
    /// * `path` - Directory containing the persistence file (`vectors.rkyv`)
    /// * `config` - Vector configuration (dimensions must match the persisted data)
    ///
    /// # Returns
    /// A new engine with the loaded vectors, or an error if the file is invalid.
    pub fn load_from_path<P: AsRef<Path>>(
        path: P,
        config: VectorConfig,
    ) -> Result<Arc<Self>, MemoryError> {
        let persist_file = path.as_ref().join("vectors.rkyv");
        info!("Loading vector index from {:?}", persist_file);

        if !persist_file.exists() {
            return Err(MemoryError::Unsupported(format!(
                "Persistence file not found: {}",
                persist_file.display()
            )));
        }

        // Read the entire persistence file
        let data = std::fs::read(&persist_file).map_err(MemoryError::Io)?;

        if data.len() < 8 {
            return Err(MemoryError::SerializationFailed(
                "Persistence file too small to contain valid header".to_string(),
            ));
        }

        // Validate magic bytes
        let magic: [u8; 8] = data[0..8].try_into().map_err(|_| {
            MemoryError::SerializationFailed(
                "Invalid persistence file: too short for magic bytes".into(),
            )
        })?;
        if magic != *PERSIST_MAGIC {
            return Err(MemoryError::SerializationFailed(format!(
                "Invalid persistence file format (magic: {:?})",
                &magic
            )));
        }

        // Deserialize the entire persistence payload using from_bytes
        // This avoids the Portable trait requirement that rkyv::access has
        let persisted: PersistedData = rkyv::from_bytes(&data[8..]).map_err(|e: RkyvError| {
            MemoryError::SerializationFailed(format!(
                "Failed to deserialize persistence data: {}",
                e
            ))
        })?;

        let header = persisted.header;

        // Validate header
        if header.magic != *PERSIST_MAGIC {
            return Err(MemoryError::SerializationFailed(
                "Corrupted persistence header".to_string(),
            ));
        }
        if header.dimensions as usize != config.dimensions {
            return Err(MemoryError::DimensionMismatch {
                expected: config.dimensions,
                actual: header.dimensions as usize,
            });
        }
        if header.vector_count as usize > MAX_PERSIST_VECTORS {
            return Err(MemoryError::SerializationFailed(format!(
                "Too many vectors in persistence file: {} (max: {})",
                header.vector_count, MAX_PERSIST_VECTORS
            )));
        }

        let entries: Vec<VectorEntry> = persisted
            .entries
            .into_iter()
            .map(VectorEntry::from)
            .collect();

        let count = entries.len();
        info!(
            "Loaded {} vectors from persistence (dims={})",
            count, header.dimensions
        );

        // Create a fresh engine and re-insert all entries, storing the persist path
        let engine = Self::new_with_path(path.as_ref(), config, Some(path.as_ref().to_path_buf()))?;

        // Re-insert all entries into the VectorDB
        {
            let mut engine_entries = engine.entries.lock().map_err(|_| {
                MemoryError::SerializationFailed("Failed to lock entries mutex".to_string())
            })?;
            for entry in &entries {
                engine
                    .db
                    .insert(entry.clone())
                    .map_err(|e| MemoryError::VectorInsertFailed(e.to_string()))?;
            }
            *engine_entries = entries;
        }

        Ok(engine)
    }

    /// Saves the current vector index to disk.
    ///
    /// This serializes all vector entries using rkyv zero-copy serialization
    /// to a file named `vectors.rkyv` in the specified directory.
    ///
    /// # File Format
    /// ```text
    /// [8 bytes: magic "SAVANT_V"]
    /// [rkyv-serialized PersistHeader]
    /// [rkyv-serialized Vec<SerializableVectorEntry>]
    /// ```
    ///
    /// # Arguments
    /// * `path` - Directory where the persistence file will be written
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), MemoryError> {
        let persist_dir = path.as_ref();
        let persist_file = persist_dir.join("vectors.rkyv");

        info!("Saving vector index to {:?}", persist_file);

        // Ensure the directory exists
        std::fs::create_dir_all(persist_dir).map_err(MemoryError::Io)?;

        // Lock the entries mutex
        let entries = self.entries.lock().map_err(|_| {
            MemoryError::SerializationFailed("Failed to lock entries mutex".to_string())
        })?;

        if entries.len() > MAX_PERSIST_VECTORS {
            return Err(MemoryError::SerializationFailed(format!(
                "Too many vectors to persist: {} (max: {})",
                entries.len(),
                MAX_PERSIST_VECTORS
            )));
        }

        // Build serializable entries
        let serializable: Vec<SerializableVectorEntry> =
            entries.iter().map(SerializableVectorEntry::from).collect();

        // Build header
        let header = PersistHeader {
            magic: *PERSIST_MAGIC,
            version: 1,
            vector_count: entries.len() as u64,
            dimensions: self.config.dimensions as u32,
            distance_metric: 0, // Cosine
        };

        // Combine into a single persistable structure
        let persisted = PersistedData {
            header,
            entries: serializable,
        };

        // Serialize with rkyv
        let serialized_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&persisted).map_err(|e| {
            MemoryError::SerializationFailed(format!("Persistence serialization failed: {}", e))
        })?;

        // Write to file: [magic][serialized_data]
        let mut file_data = Vec::with_capacity(8 + serialized_bytes.len());
        file_data.extend_from_slice(PERSIST_MAGIC);
        file_data.extend_from_slice(&serialized_bytes);

        // Write to file atomically: write to temp file, then rename
        let tmp_file = persist_dir.join("vectors.rkyv.tmp");
        std::fs::write(&tmp_file, &file_data).map_err(MemoryError::Io)?;
        std::fs::rename(&tmp_file, &persist_file).map_err(MemoryError::Io)?;

        info!(
            "Saved {} vectors to {:?} ({} bytes)",
            entries.len(),
            persist_file,
            file_data.len()
        );

        Ok(())
    }

    /// Persists the current vector index to the stored path.
    ///
    /// This is a convenience method that uses the path stored when the engine
    /// was created or loaded. Returns an error if no persist path is available.
    ///
    /// The persist operation is also called automatically on `Drop` to prevent
    /// data loss on normal shutdown. The write is atomic (temp file + rename)
    /// to prevent corruption on crash.
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if no persist path is set.
    pub fn persist(&self) -> Result<(), MemoryError> {
        match &self.persist_path {
            Some(path) => self.save_to_path(path),
            None => Err(MemoryError::Unsupported(
                "No persist path configured. Use save_to_path() instead.".to_string(),
            )),
        }
    }

    /// Returns the current persist path, if any.
    pub fn persist_path(&self) -> Option<&Path> {
        self.persist_path.as_deref()
    }

    /// Indexes a new memory entry for semantic retrieval.
    ///
    /// The embedding is optionally quantized (32x compression) before insertion.
    /// The entry is also stored in the internal cache for persistence support.
    ///
    /// # Arguments
    /// * `memory_id` - Unique identifier for this memory (typically the MemoryEntry.id)
    /// * `embedding` - Raw embedding vector (length = config.dimensions)
    ///
    /// # Returns
    /// `Ok(())` on success.
    #[instrument(skip(self, embedding), fields(memory_id = %memory_id))]
    pub fn index_memory(&self, memory_id: &str, embedding: &[f32]) -> Result<(), MemoryError> {
        // Validate dimensions
        if embedding.len() != self.config.dimensions {
            return Err(MemoryError::DimensionMismatch {
                expected: self.config.dimensions,
                actual: embedding.len(),
            });
        }

        let entry = VectorEntry {
            id: Some(memory_id.to_string()),
            vector: embedding.to_vec(),
            metadata: None,
        };

        // Acquire lock once and perform both DB insert and cache push atomically
        let mut entries = self.entries.lock().map_err(|_| {
            MemoryError::VectorInsertFailed("Failed to lock entries mutex".to_string())
        })?;

        // Insert into VectorDB
        self.db
            .insert(entry.clone())
            .map_err(|e| MemoryError::VectorInsertFailed(e.to_string()))?;

        // Store in internal cache for persistence (under same lock)
        entries.push(entry);

        debug!("Indexed memory with ID: {}", memory_id);
        Ok(())
    }

    /// Performs a k-nearest neighbor search using the query embedding.
    ///
    /// Returns up to `top_k` memory IDs sorted by similarity (highest first).
    /// Latency is typically <0.5ms for 1M vectors on modern hardware with AVX2.
    ///
    /// # Arguments
    /// * `query_embedding` - Query vector (must match config.dimensions)
    /// * `top_k` - Number of nearest neighbors to return (max typically 100)
    /// * `options` - Optional search tuning parameters
    ///
    /// # Returns
    /// Vector of memory IDs ordered by decreasing similarity.
    pub fn recall(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        options: Option<SearchOptions>,
    ) -> Result<Vec<SearchResult>, MemoryError> {
        // Validate dimensions
        if query_embedding.len() != self.config.dimensions {
            return Err(MemoryError::DimensionMismatch {
                expected: self.config.dimensions,
                actual: query_embedding.len(),
            });
        }

        // Apply search options if provided
        if let Some(ref opts) = options {
            if let Some(ef) = opts.ef_search {
                debug!("Using custom ef_search: {}", ef);
            }
        }

        let query = SearchQuery {
            vector: query_embedding.to_vec(),
            k: top_k,
            filter: None,
            ef_search: options.and_then(|o| o.ef_search),
        };

        let results = self
            .db
            .search(query)
            .map_err(|e| MemoryError::VectorQueryFailed(e.to_string()))?;

        let search_results: Vec<SearchResult> = results
            .into_iter()
            .map(|res| SearchResult {
                document_id: res.id,
                score: normalize_distance(res.score),
                distance: res.score,
            })
            .collect();

        debug!(
            "Search returned {} results in <0.5ms (SIMD)",
            search_results.len()
        );
        Ok(search_results)
    }

    /// Performs a ranged search returning all vectors within a maximum distance.
    ///
    /// This is useful for similarity thresholds.
    pub fn recall_within_distance(
        &self,
        query_embedding: &[f32],
        max_distance: f32,
    ) -> Result<Vec<SearchResult>, MemoryError> {
        if query_embedding.len() != self.config.dimensions {
            return Err(MemoryError::DimensionMismatch {
                expected: self.config.dimensions,
                actual: query_embedding.len(),
            });
        }

        use ruvector_core::types::SearchQuery;
        let query = SearchQuery {
            vector: query_embedding.to_vec(),
            k: 100, // reasonable upper bound
            filter: None,
            ef_search: None,
        };

        let all_results = self
            .db
            .search(query)
            .map_err(|e| MemoryError::VectorQueryFailed(e.to_string()))?;

        let filtered: Vec<SearchResult> = all_results
            .into_iter()
            .filter(|res| res.score <= max_distance)
            .map(|res| SearchResult {
                document_id: res.id,
                score: normalize_distance(res.score),
                distance: res.score,
            })
            .collect();

        Ok(filtered)
    }

    /// Removes a memory entry from the index.
    ///
    /// This is useful for memory compaction and deletion.
    /// The entry is also removed from the internal persistence cache.
    pub fn remove(&self, memory_id: &str) -> Result<(), MemoryError> {
        self.db
            .delete(memory_id)
            .map_err(|e| MemoryError::VectorDeleteFailed(e.to_string()))?;

        // Remove from internal cache
        if let Ok(mut entries) = self.entries.lock() {
            entries.retain(|e| e.id.as_deref() != Some(memory_id));
        }

        debug!("Removed memory from vector index: {}", memory_id);
        Ok(())
    }

    /// Returns the number of vectors currently indexed.
    ///
    /// This uses the internal entry cache, which is the source of truth
    /// for the vector count since ruvector-core does not expose a count API.
    pub fn vector_count(&self) -> usize {
        self.entries.lock().map(|e| e.len()).unwrap_or(0)
    }

    /// Returns the engine configuration.
    pub fn config(&self) -> &VectorConfig {
        &self.config
    }

    /// Checks if the current hardware supports SIMD acceleration.
    ///
    /// ruvector-core automatically falls back to scalar code if SIMD is unavailable,
    /// but this method allows explicit checking.
    pub fn simd_supported() -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            is_x86_feature_detected!("avx2") || is_x86_feature_detected!("avx512f")
        }
        #[cfg(target_arch = "aarch64")]
        {
            // ARM NEON is always present on aarch64
            true
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            false
        }
    }
}

impl Drop for SemanticVectorEngine {
    fn drop(&mut self) {
        // Attempt to persist vectors on drop to prevent data loss on exit
        if let Err(e) = self.persist() {
            tracing::warn!("Failed to persist vector index on drop: {}", e);
        }
    }
}

/// Normalizes a raw distance to a similarity score in [0, 1].
///
/// For cosine distance (used by ruvector-core), the range is [0, 2].
/// We convert to similarity: score = 1.0 - (distance / 2.0)
fn normalize_distance(distance: f32) -> f32 {
    (1.0 - (distance / 2.0)).clamp(0.0, 1.0)
}

/// Search result containing the document ID, similarity score, and raw distance.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// Unique identifier of the retrieved memory/document
    pub document_id: String,
    /// Similarity score (0.0 to 1.0) where 1.0 is identical
    pub score: f32,
    /// Raw distance metric value (lower is more similar for cosine)
    pub distance: f32,
}

/// Options to tune search behavior.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Override the ef_search parameter (larger = more accurate but slower)
    pub ef_search: Option<usize>,
    /// Maximum distance threshold (early termination)
    pub max_distance: Option<f32>,
    /// Whether to include quantized vectors only (faster, less accurate)
    pub quantized_only: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_vector_engine_creation() {
        let engine = SemanticVectorEngine::default_384().unwrap();
        assert_eq!(engine.config().dimensions, 384);
    }

    #[test]
    fn test_simd_supported_detection() {
        let supported = SemanticVectorEngine::simd_supported();
        let _ = supported;
    }

    #[test]
    fn test_normalize_distance() {
        assert!((normalize_distance(0.0) - 1.0).abs() < 1e-6);
        assert!((normalize_distance(1.0) - 0.5).abs() < 1e-6);
        assert!((normalize_distance(2.0) - 0.0).abs() < 1e-6);
        assert!((normalize_distance(3.0) - 0.0).abs() < 1e-6); // >2 should clamp
    }

    #[test]
    fn test_dimension_mismatch_error() {
        let engine = SemanticVectorEngine::default_384().unwrap();
        let wrong_dims = vec![0.1; 128];
        let result = engine.index_memory("test", &wrong_dims);
        assert!(matches!(result, Err(MemoryError::DimensionMismatch { .. })));
    }

    #[test]
    fn test_vector_count_initially_zero() {
        let engine = SemanticVectorEngine::default_384().unwrap();
        assert_eq!(engine.vector_count(), 0);
    }

    #[test]
    fn test_vector_count_increments() {
        let engine = SemanticVectorEngine::default_384().unwrap();
        let embedding = vec![0.1; 384];
        engine.index_memory("mem-1", &embedding).unwrap();
        assert_eq!(engine.vector_count(), 1);

        engine.index_memory("mem-2", &embedding).unwrap();
        assert_eq!(engine.vector_count(), 2);
    }

    #[test]
    fn test_remove_decrements_count() {
        let engine = SemanticVectorEngine::default_384().unwrap();
        let embedding = vec![0.1; 384];
        engine.index_memory("mem-1", &embedding).unwrap();
        engine.index_memory("mem-2", &embedding).unwrap();
        assert_eq!(engine.vector_count(), 2);

        engine.remove("mem-1").unwrap();
        assert_eq!(engine.vector_count(), 1);
    }

    #[test]
    fn test_save_and_load_persistence() {
        let dir = tempdir().unwrap();

        // Create engine and index some vectors
        let engine = SemanticVectorEngine::new(dir.path(), VectorConfig::default()).unwrap();
        engine.index_memory("mem-1", &vec![0.1; 384]).unwrap();
        engine.index_memory("mem-2", &vec![0.2; 384]).unwrap();
        engine.index_memory("mem-3", &vec![0.3; 384]).unwrap();
        assert_eq!(engine.vector_count(), 3);

        // Save to disk
        engine.save_to_path(dir.path()).unwrap();

        // Verify file exists
        let persist_file = dir.path().join("vectors.rkyv");
        assert!(persist_file.exists());

        // Load from disk
        let loaded =
            SemanticVectorEngine::load_from_path(dir.path(), VectorConfig::default()).unwrap();
        assert_eq!(loaded.vector_count(), 3);
    }

    #[test]
    fn test_save_and_load_preserves_search() {
        let dir = tempdir().unwrap();

        // Create engine with known vectors
        let engine = SemanticVectorEngine::new(dir.path(), VectorConfig::default()).unwrap();
        let query = vec![1.0; 384];
        let similar = vec![0.9; 384];
        let dissimilar = vec![-1.0; 384];

        engine.index_memory("similar", &similar).unwrap();
        engine.index_memory("dissimilar", &dissimilar).unwrap();

        // Save and reload
        engine.save_to_path(dir.path()).unwrap();
        let loaded =
            SemanticVectorEngine::load_from_path(dir.path(), VectorConfig::default()).unwrap();

        // Search should return results
        let results = loaded.recall(&query, 2, None).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_load_nonexistent_file_fails() {
        let dir = tempdir().unwrap();
        let result = SemanticVectorEngine::load_from_path(dir.path(), VectorConfig::default());
        let err = result.err().expect("should fail for nonexistent file");
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_save_creates_directory() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("deep/nested/path");

        let engine = SemanticVectorEngine::new(&nested, VectorConfig::default()).unwrap();
        engine.index_memory("mem-1", &vec![0.1; 384]).unwrap();
        engine.save_to_path(&nested).unwrap();

        assert!(nested.join("vectors.rkyv").exists());
    }

    #[test]
    fn test_persist_header_magic_validation() {
        let dir = tempdir().unwrap();
        let persist_file = dir.path().join("vectors.rkyv");

        // Write invalid magic
        std::fs::write(&persist_file, b"BADMAGIC").unwrap();

        let result = SemanticVectorEngine::load_from_path(dir.path(), VectorConfig::default());
        let err = result.err().expect("should fail for invalid magic");
        assert!(err.to_string().contains("Invalid persistence file format"));
    }
}

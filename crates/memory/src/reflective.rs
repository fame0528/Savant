//! Reflective/Semantic Memory Layer
//!
//! A graph-based store for synthesized rules, core identity constraints,
//! and generalized concepts derived from episodic memory consolidation.
//! Written by the background consolidation thread, read by the workspace
//! executive monitor.

use serde::{Deserialize, Serialize};

/// A concept node in the reflective memory graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    /// Unique identifier.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Source memory entry IDs that contributed to this concept.
    pub source_entries: Vec<u64>,
    /// Creation timestamp.
    pub created_at: i64,
    /// Last access timestamp.
    pub last_accessed: i64,
}

/// A relation between two concepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    /// Type of relation (e.g., "is_a", "part_of", "contradicts").
    pub relation_type: String,
    /// Strength of the relation [0.0, 1.0].
    pub weight: f32,
    /// Source concept ID.
    pub source_concept: String,
    /// Target concept ID.
    pub target_concept: String,
}

/// Reflective memory graph storing generalized concepts and relations.
///
/// This is NOT the same as the episodic memory (LSM/vector). This layer
/// stores high-level abstractions derived during memory consolidation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectiveMemory {
    /// Concept nodes.
    pub concepts: Vec<Concept>,
    /// Relations between concepts.
    pub relations: Vec<Relation>,
    /// Last consolidation timestamp.
    pub last_consolidation: i64,
}

impl ReflectiveMemory {
    /// Creates an empty reflective memory.
    pub fn new() -> Self {
        Self {
            concepts: Vec::new(),
            relations: Vec::new(),
            last_consolidation: chrono::Utc::now().timestamp(),
        }
    }

    /// Adds a concept to the graph.
    pub fn add_concept(&mut self, concept: Concept) {
        // Deduplicate by ID
        self.concepts.retain(|c| c.id != concept.id);
        self.concepts.push(concept);
    }

    /// Adds a relation to the graph.
    pub fn add_relation(&mut self, relation: Relation) {
        self.relations.push(relation);
    }

    /// Finds concepts by label substring match.
    pub fn find_concepts(&self, query: &str) -> Vec<&Concept> {
        let query_lower = query.to_lowercase();
        self.concepts
            .iter()
            .filter(|c| c.label.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Finds all relations for a concept.
    pub fn find_relations(&self, concept_id: &str) -> Vec<&Relation> {
        self.relations
            .iter()
            .filter(|r| r.source_concept == concept_id || r.target_concept == concept_id)
            .collect()
    }

    /// Returns the number of concepts.
    pub fn concept_count(&self) -> usize {
        self.concepts.len()
    }

    /// Returns the number of relations.
    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }
}

impl Default for ReflectiveMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_find_concept() {
        let mut memory = ReflectiveMemory::new();
        memory.add_concept(Concept {
            id: "c1".to_string(),
            label: "Build System".to_string(),
            source_entries: vec![1, 2, 3],
            created_at: 0,
            last_accessed: 0,
        });

        let found = memory.find_concepts("build");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].label, "Build System");
    }

    #[test]
    fn test_relations() {
        let mut memory = ReflectiveMemory::new();
        memory.add_relation(Relation {
            relation_type: "is_a".to_string(),
            weight: 0.9,
            source_concept: "c1".to_string(),
            target_concept: "c2".to_string(),
        });

        let relations = memory.find_relations("c1");
        assert_eq!(relations.len(), 1);
    }

    #[test]
    fn test_deduplicate_concepts() {
        let mut memory = ReflectiveMemory::new();
        memory.add_concept(Concept {
            id: "c1".to_string(),
            label: "Original".to_string(),
            source_entries: vec![],
            created_at: 0,
            last_accessed: 0,
        });
        memory.add_concept(Concept {
            id: "c1".to_string(),
            label: "Updated".to_string(),
            source_entries: vec![1],
            created_at: 1,
            last_accessed: 1,
        });

        assert_eq!(memory.concept_count(), 1);
        assert_eq!(memory.concepts[0].label, "Updated");
    }
}

//! Entity Extraction and Relationship Tracking
//!
//! Extracts entities (people, projects, services, keys) from agent messages
//! and tracks them across sessions using a graph structure.
//!
//! # Architecture
//! Rule-based extraction by default. Can be swapped for NER (gline-rs) when
//! dependency is verified.
//!
//! # Usage
//! ```text
//! New message arrives
//!     ↓
//! Rule-based entity extraction ("Project Alpha", "OpenRouter API Key")
//!     ↓
//! Normalize + hash → entity ID
//!     ↓
//! petgraph: add node, create edges to related memories
//!     ↓
//! Query "Project Alpha" → traverse graph → return all related memories
//! ```

use serde::{Deserialize, Serialize};

/// An extracted entity from agent messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entity {
    /// Unique entity ID (hash of normalized name)
    pub entity_id: String,
    /// Normalized entity name
    pub canonical_name: String,
    /// Entity type (project, service, person, key, tool, file, etc.)
    pub entity_type: EntityType,
    /// Number of times this entity was mentioned
    pub mention_count: u32,
    /// First seen timestamp
    pub first_seen: i64,
    /// Last seen timestamp
    pub last_seen: i64,
    /// Session IDs where this entity was mentioned
    pub sessions: Vec<String>,
}

/// Entity types for categorization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EntityType {
    /// Software project or repository
    Project,
    /// External service or API
    Service,
    /// Person or user
    Person,
    /// API key or credential
    Credential,
    /// Tool or command
    Tool,
    /// File or path
    File,
    /// Configuration value
    Config,
    /// Generic entity
    Other(String),
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::Project => write!(f, "project"),
            EntityType::Service => write!(f, "service"),
            EntityType::Person => write!(f, "person"),
            EntityType::Credential => write!(f, "credential"),
            EntityType::Tool => write!(f, "tool"),
            EntityType::File => write!(f, "file"),
            EntityType::Config => write!(f, "config"),
            EntityType::Other(name) => write!(f, "{}", name),
        }
    }
}

/// Rule-based entity extractor.
pub struct EntityExtractor {
    /// Patterns for entity detection
    patterns: Vec<EntityPattern>,
}

/// A pattern for detecting entities in text.
struct EntityPattern {
    /// Keywords that indicate this entity type
    keywords: Vec<String>,
    /// Entity type when matched
    entity_type: EntityType,
}

impl EntityExtractor {
    /// Creates a new entity extractor with default patterns.
    pub fn new() -> Self {
        Self {
            patterns: vec![
                EntityPattern {
                    keywords: vec![
                        "project".to_string(),
                        "repo".to_string(),
                        "repository".to_string(),
                    ],
                    entity_type: EntityType::Project,
                },
                EntityPattern {
                    keywords: vec![
                        "api".to_string(),
                        "endpoint".to_string(),
                        "service".to_string(),
                        "server".to_string(),
                    ],
                    entity_type: EntityType::Service,
                },
                EntityPattern {
                    keywords: vec![
                        "key".to_string(),
                        "token".to_string(),
                        "secret".to_string(),
                        "credential".to_string(),
                    ],
                    entity_type: EntityType::Credential,
                },
                EntityPattern {
                    keywords: vec![
                        "file".to_string(),
                        "path".to_string(),
                        "directory".to_string(),
                    ],
                    entity_type: EntityType::File,
                },
                EntityPattern {
                    keywords: vec![
                        "config".to_string(),
                        "setting".to_string(),
                        "option".to_string(),
                    ],
                    entity_type: EntityType::Config,
                },
            ],
        }
    }

    /// Extracts entities from text content.
    pub fn extract(&self, text: &str, session_id: &str) -> Vec<Entity> {
        let mut entities = Vec::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        // Split into sentences and look for entity indicators
        for sentence in text.split('.') {
            let lower = sentence.to_lowercase();
            for pattern in &self.patterns {
                for keyword in &pattern.keywords {
                    if lower.contains(keyword) {
                        // Extract the entity name (word after keyword or before)
                        if let Some(name) = Self::extract_entity_name(sentence, keyword) {
                            let entity_id = Self::normalize_id(&name, &pattern.entity_type);
                            entities.push(Entity {
                                entity_id,
                                canonical_name: name,
                                entity_type: pattern.entity_type.clone(),
                                mention_count: 1,
                                first_seen: now,
                                last_seen: now,
                                sessions: vec![session_id.to_string()],
                            });
                        }
                    }
                }
            }
        }

        entities
    }

    /// Extracts the entity name from a sentence near a keyword.
    fn extract_entity_name(sentence: &str, keyword: &str) -> Option<String> {
        let words: Vec<&str> = sentence.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            if word.to_lowercase().contains(keyword) {
                // Take the next 2-3 words as the entity name
                if i + 1 < words.len() {
                    let name: String = words[i + 1..]
                        .iter()
                        .take(3)
                        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-'))
                        .filter(|w| !w.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !name.is_empty() {
                        return Some(name);
                    }
                }
            }
        }
        None
    }

    /// Normalizes an entity name to an ID.
    fn normalize_id(name: &str, entity_type: &EntityType) -> String {
        let normalized = name.to_lowercase().replace(' ', "_");
        format!("{}:{}", entity_type, normalized)
    }
}

impl Default for EntityExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_extractor_creation() {
        let extractor = EntityExtractor::new();
        assert!(!extractor.patterns.is_empty());
    }

    #[test]
    fn test_extract_project() {
        let extractor = EntityExtractor::new();
        let entities = extractor.extract("Working on project Savant today.", "sess-1");
        assert!(!entities.is_empty());
        assert_eq!(entities[0].entity_type, EntityType::Project);
    }

    #[test]
    fn test_extract_api() {
        let extractor = EntityExtractor::new();
        let entities = extractor.extract("The API endpoint is returning 500 errors.", "sess-1");
        assert!(!entities.is_empty());
        assert_eq!(entities[0].entity_type, EntityType::Service);
    }

    #[test]
    fn test_extract_key() {
        let extractor = EntityExtractor::new();
        let entities = extractor.extract("Need to rotate the secret key for production.", "sess-1");
        assert!(!entities.is_empty());
        // "secret" matches Credential pattern
        assert!(entities.iter().any(|e| e.entity_type == EntityType::Credential));
    }

    #[test]
    fn test_extract_no_entities() {
        let extractor = EntityExtractor::new();
        let entities = extractor.extract("Hello world, how are you?", "sess-1");
        assert!(entities.is_empty());
    }

    #[test]
    fn test_entity_type_display() {
        assert_eq!(EntityType::Project.to_string(), "project");
        assert_eq!(EntityType::Service.to_string(), "service");
    }
}

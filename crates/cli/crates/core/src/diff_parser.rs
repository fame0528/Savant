use anyhow::{Context, Result};
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::Path;
use tracing::{debug, warn};

/// A parsed SEARCH/REPLACE block
#[derive(Debug, Clone)]
pub struct DiffBlock {
    pub search: String,
    pub replace: String,
    pub file_path: Option<String>,
}

/// Result of applying a diff block
#[derive(Debug, Clone)]
pub enum DiffResult {
    /// Exact match applied successfully
    ExactMatch { lines_changed: usize },
    /// Fuzzy match applied (with similarity score)
    FuzzyMatch {
        similarity: f64,
        lines_changed: usize,
    },
    /// No match found
    NoMatch { search_lines: usize },
}

/// SEARCH/REPLACE block parser
pub struct DiffParser;

impl DiffParser {
    /// Parse a single SEARCH/REPLACE block from text
    pub fn parse_block(text: &str) -> Option<DiffBlock> {
        let search_start = text.find("<<<<<<< SEARCH")?;
        let separator = text.find("\n=======\n")?;
        let replace_end = text.find(">>>>>>> REPLACE")?;

        let search = text[search_start + "<<<<<<< SEARCH".len()..separator]
            .trim()
            .to_string();
        let replace = text[separator + "\n=======\n".len()..replace_end]
            .trim()
            .to_string();

        // Check for file path hint before the block
        let file_path = text[..search_start]
            .lines()
            .rev()
            .find(|line| line.starts_with("File:") || line.starts_with("```"))
            .map(|line| {
                line.trim_start_matches("File:")
                    .trim()
                    .trim_start_matches("```")
                    .trim()
                    .to_string()
            });

        Some(DiffBlock {
            search,
            replace,
            file_path,
        })
    }

    /// Parse multiple SEARCH/REPLACE blocks from text
    pub fn parse_all(text: &str) -> Vec<DiffBlock> {
        let mut blocks = Vec::new();
        let mut remaining = text;

        while let Some(block) = Self::parse_block(remaining) {
            let end_marker = remaining.find(">>>>>>> REPLACE").unwrap_or(remaining.len());
            blocks.push(block);
            remaining = &remaining[end_marker + ">>>>>>> REPLACE".len()..];
        }

        blocks
    }

    /// Apply a diff block to a file with exact match first, then fuzzy fallback
    pub fn apply_to_file(block: &DiffBlock, file_path: &Path) -> Result<DiffResult> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {:?}", file_path))?;

        // Try exact match first
        if let Some(result) = Self::apply_exact(&content, block, file_path)? {
            return Ok(result);
        }

        // Fallback to fuzzy match
        Self::apply_fuzzy(&content, block, file_path)
    }

    /// Apply with exact string matching
    fn apply_exact(
        content: &str,
        block: &DiffBlock,
        file_path: &Path,
    ) -> Result<Option<DiffResult>> {
        // Normalize whitespace for matching
        let normalized_search = Self::normalize_whitespace(&block.search);
        let normalized_content = Self::normalize_whitespace(content);

        if normalized_content.contains(&normalized_search) {
            // Find the actual position in the original content
            let search_bytes = block.search.as_bytes();
            if let Some(actual_pos) = Self::find_subsequence(content.as_bytes(), search_bytes) {
                let new_content = format!(
                    "{}{}{}",
                    &content[..actual_pos],
                    block.replace,
                    &content[actual_pos + block.search.len()..]
                );

                fs::write(file_path, &new_content)
                    .with_context(|| format!("Failed to write file: {:?}", file_path))?;

                let lines_changed =
                    block.replace.lines().count() as isize - block.search.lines().count() as isize;

                debug!(
                    "Exact match applied to {:?} ({} lines changed)",
                    file_path, lines_changed
                );
                return Ok(Some(DiffResult::ExactMatch {
                    lines_changed: lines_changed.unsigned_abs(),
                }));
            }
        }

        Ok(None)
    }

    /// Apply with fuzzy matching using Jaro-Winkler similarity
    fn apply_fuzzy(content: &str, block: &DiffBlock, file_path: &Path) -> Result<DiffResult> {
        let search_lines: Vec<&str> = block.search.lines().collect();
        let content_lines: Vec<&str> = content.lines().collect();

        let mut best_match: Option<(usize, f64)> = None;
        let threshold = 0.85;

        // Slide window over content lines
        for window_start in 0..=content_lines.len().saturating_sub(search_lines.len()) {
            let window = &content_lines[window_start..window_start + search_lines.len()];
            let window_text = window.join("\n");

            let similarity = strsim::jaro_winkler(&block.search, &window_text);

            let should_update = match best_match {
                None => true,
                Some((_, best_similarity)) => similarity > best_similarity,
            };
            if similarity >= threshold && should_update {
                best_match = Some((window_start, similarity));
            }
        }

        if let Some((start_idx, similarity)) = best_match {
            let end_idx = start_idx + search_lines.len();

            // Reconstruct file with replacement
            let mut new_lines: Vec<String> = content_lines[..start_idx]
                .iter()
                .map(|s| s.to_string())
                .collect();

            for line in block.replace.lines() {
                new_lines.push(line.to_string());
            }

            if end_idx < content_lines.len() {
                new_lines.extend(content_lines[end_idx..].iter().map(|s| s.to_string()));
            }

            let new_content = new_lines.join("\n");
            fs::write(file_path, &new_content)
                .with_context(|| format!("Failed to write file: {:?}", file_path))?;

            let lines_changed =
                block.replace.lines().count() as isize - block.search.lines().count() as isize;

            debug!(
                "Fuzzy match applied to {:?} (similarity: {:.2}, {} lines changed)",
                file_path, similarity, lines_changed
            );
            return Ok(DiffResult::FuzzyMatch {
                similarity,
                lines_changed: lines_changed.unsigned_abs(),
            });
        }

        warn!(
            "No match found for SEARCH block ({} lines) in {:?}",
            search_lines.len(),
            file_path
        );
        Ok(DiffResult::NoMatch {
            search_lines: search_lines.len(),
        })
    }

    /// Normalize whitespace for comparison
    fn normalize_whitespace(s: &str) -> String {
        s.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }

    /// Find byte subsequence
    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    /// Generate a diff summary for LLM context
    pub fn diff_summary(original: &str, modified: &str) -> String {
        let diff = TextDiff::from_lines(original, modified);

        let mut additions = 0usize;
        let mut deletions = 0usize;

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Insert => additions += 1,
                ChangeTag::Delete => deletions += 1,
                ChangeTag::Equal => {}
            }
        }

        format!("+{} -{}", additions, deletions)
    }
}

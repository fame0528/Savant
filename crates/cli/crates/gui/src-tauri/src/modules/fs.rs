//! File System module — file operations, directory listing, search

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::{Mutex, MutexGuard};
use tracing;

fn lock_mutex<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!("Mutex was poisoned, recovering inner data");
            poisoned.into_inner()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub children: Option<Vec<FileNode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file: String,
    pub line: u32,
    pub content: String,
}

pub struct FsService;

impl FsService {
    pub fn new() -> Self {
        Self
    }

    pub async fn list_dir(&mut self, path: &str) -> Result<Vec<FileNode>> {
        let mut nodes = Vec::new();
        let entries =
            fs::read_dir(path).with_context(|| format!("Failed to read directory: {}", path))?;

        for entry in entries.flatten() {
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path().to_string_lossy().to_string();

            nodes.push(FileNode {
                name,
                path,
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                children: None,
            });
        }

        nodes.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        Ok(nodes)
    }

    pub async fn search_files(
        &self,
        query: &str,
        dir: &str,
        max_results: usize,
    ) -> Result<Vec<String>> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        let walker = WalkBuilder::new(dir)
            .hidden(false)
            .ignore(true)
            .git_ignore(true)
            .max_depth(Some(10))
            .build();

        for entry in walker.flatten() {
            if results.len() >= max_results {
                break;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.to_lowercase().contains(&query_lower) {
                results.push(entry.path().to_string_lossy().to_string());
            }
        }

        Ok(results)
    }

    pub async fn search_content(
        &self,
        pattern: &str,
        dir: &str,
        _file_glob: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        use grep_regex::RegexMatcher;
        use grep_searcher::Searcher;

        let matcher = RegexMatcher::new(pattern).context("Invalid search pattern")?;
        let mut searcher = Searcher::new();
        let results = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let max_results = 100;

        let walker = WalkBuilder::new(dir)
            .hidden(false)
            .ignore(true)
            .git_ignore(true)
            .max_depth(Some(10))
            .build();

        for entry in walker.flatten() {
            let current_len = lock_mutex(&results).len();
            if current_len >= max_results {
                break;
            }
            if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                let path = entry.path().to_string_lossy().to_string();
                let results_clone = results.clone();
                let path_for_closure = path.clone();
                let sink = grep_searcher::sinks::UTF8(move |line_num, line| {
                    let mut r = lock_mutex(&results_clone);
                    r.push(SearchResult {
                        file: path_for_closure.clone(),
                        line: line_num as u32,
                        content: line.to_string(),
                    });
                    Ok(true)
                });
                if let Err(e) = searcher.search_path(&matcher, entry.path(), sink) {
                    tracing::warn!("Search error in {}: {}", path, e);
                }
            }
        }

        let final_results = match std::sync::Arc::try_unwrap(results) {
            Ok(mutex) => match mutex.into_inner() {
                Ok(vec) => vec,
                Err(poisoned) => {
                    tracing::warn!("Results mutex was poisoned, recovering data");
                    poisoned.into_inner()
                }
            },
            Err(arc) => {
                // Arc still has references, just read what we can
                let guard = lock_mutex(&arc);
                guard.clone()
            }
        };
        Ok(final_results)
    }
}

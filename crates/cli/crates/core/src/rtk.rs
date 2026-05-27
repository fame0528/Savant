use std::collections::HashMap;

/// RTK compression profile for different command types
#[derive(Debug, Clone)]
pub struct RtkProfile {
    pub name: String,
    pub max_lines: usize,
    pub max_stack_frames: usize,
    pub strip_paths: bool,
    pub semantic_grouping: bool,
    pub group_threshold: usize,
}

impl RtkProfile {
    /// cargo test / npm test profile
    pub fn test_output() -> Self {
        Self {
            name: "test_output".to_string(),
            max_lines: 20,
            max_stack_frames: 3,
            strip_paths: true,
            semantic_grouping: false,
            group_threshold: 0,
        }
    }

    /// grep / rg profile
    pub fn search_output() -> Self {
        Self {
            name: "search_output".to_string(),
            max_lines: 20,
            max_stack_frames: 0,
            strip_paths: false,
            semantic_grouping: true,
            group_threshold: 20,
        }
    }

    /// ls / tree profile
    pub fn directory_listing() -> Self {
        Self {
            name: "directory_listing".to_string(),
            max_lines: 50,
            max_stack_frames: 0,
            strip_paths: false,
            semantic_grouping: false,
            group_threshold: 0,
        }
    }

    /// cat / read profile
    pub fn file_content() -> Self {
        Self {
            name: "file_content".to_string(),
            max_lines: 100,
            max_stack_frames: 0,
            strip_paths: false,
            semantic_grouping: false,
            group_threshold: 0,
        }
    }

    /// Default profile (generic)
    pub fn default_profile() -> Self {
        Self {
            name: "default".to_string(),
            max_lines: 50,
            max_stack_frames: 0,
            strip_paths: false,
            semantic_grouping: false,
            group_threshold: 0,
        }
    }
}

/// Compress command output using the appropriate RTK profile
pub fn compress_output(output: &str, profile: &RtkProfile) -> (String, usize) {
    let lines: Vec<&str> = output.lines().collect();
    let original_lines = lines.len();

    if lines.len() <= profile.max_lines {
        return (output.to_string(), original_lines);
    }

    let compressed = match profile.name.as_str() {
        "test_output" => compress_test_output(&lines, profile),
        "search_output" => compress_search_output(&lines, profile),
        "directory_listing" => compress_directory_listing(&lines, profile),
        "file_content" => compress_file_content(&lines, profile),
        _ => compress_generic(&lines, profile),
    };

    (compressed, original_lines)
}

/// Compress test/cargo test output
fn compress_test_output(lines: &[&str], profile: &RtkProfile) -> String {
    let mut result = Vec::new();
    let mut stack_depth = 0;
    let mut in_stack_trace = false;

    for line in lines {
        if line.starts_with("thread '") || line.contains("panicked at") {
            result.push(line.to_string());
            in_stack_trace = true;
            stack_depth = 0;
            continue;
        }

        if in_stack_trace {
            if stack_depth < profile.max_stack_frames {
                let processed = if profile.strip_paths {
                    strip_absolute_paths(line)
                } else {
                    line.to_string()
                };
                result.push(processed);
                stack_depth += 1;
            } else if line.is_empty() || line.starts_with("note:") {
                in_stack_trace = false;
                result.push(line.to_string());
            }
            continue;
        }

        // Keep test result lines
        if line.starts_with("test ") || line.starts_with("running ") {
            result.push(line.to_string());
        }
    }

    // Add summary
    let skipped = lines.len().saturating_sub(result.len());
    if skipped > 0 {
        result.push(format!("\n... {} lines omitted (RTK compression)", skipped));
    }

    result.join("\n")
}

/// Compress search/grep output with semantic grouping
fn compress_search_output(lines: &[&str], profile: &RtkProfile) -> String {
    if lines.len() <= profile.group_threshold {
        return lines.join("\n");
    }

    // Group by file
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_file = String::from("other");

    for line in lines {
        if line.contains(':') {
            let file = line.split(':').next().unwrap_or("unknown").to_string();
            current_file = file;
        }
        groups
            .entry(current_file.clone())
            .or_default()
            .push(line.to_string());
    }

    let mut result = Vec::new();
    for (file, matches) in &groups {
        if matches.len() > 5 {
            result.push(format!("{} ({} matches)", file, matches.len()));
            for line in matches.iter().take(3) {
                result.push(format!("  {}", line));
            }
            result.push(format!("  ... and {} more", matches.len() - 3));
        } else {
            for line in matches {
                result.push(line.clone());
            }
        }
    }

    result.join("\n")
}

/// Compress directory listing
fn compress_directory_listing(lines: &[&str], profile: &RtkProfile) -> String {
    let mut result = Vec::new();
    let mut skipped = 0;

    for line in lines {
        let trimmed = line.trim();
        // Skip node_modules, target, .git, etc.
        if trimmed.contains("node_modules/")
            || trimmed.contains("target/")
            || trimmed.contains(".git/")
        {
            skipped += 1;
            continue;
        }
        result.push(line.to_string());
    }

    if result.len() > profile.max_lines {
        let keep = profile.max_lines / 2;
        let mut truncated = result[..keep].to_vec();
        truncated.push(format!(
            "... {} entries omitted",
            result.len() - keep + skipped
        ));
        truncated.extend(result[result.len() - keep..].iter().cloned());
        result = truncated;
    }

    if skipped > 0 {
        result.push(format!(
            "({} excluded: node_modules, target, .git)",
            skipped
        ));
    }

    result.join("\n")
}

/// Compress file content (cat/read)
fn compress_file_content(lines: &[&str], profile: &RtkProfile) -> String {
    if lines.len() <= profile.max_lines {
        return lines.join("\n");
    }

    let keep = profile.max_lines / 3;
    let mut result: Vec<String> = lines[..keep].iter().map(|s| s.to_string()).collect();

    // Show structural headers for the middle
    result.push(format!(
        "\n... {} lines omitted (showing structure) ...\n",
        lines.len() - (keep * 2)
    ));

    // Show function signatures / structural elements from middle
    let middle_start = lines.len() / 3;
    let middle_end = (lines.len() * 2) / 3;
    for line in &lines[middle_start..middle_end] {
        if is_structural_line(line) {
            result.push(format!("| {}", line.trim()));
        }
    }

    result.extend(lines[lines.len() - keep..].iter().map(|s| s.to_string()));
    result.join("\n")
}

/// Generic compression (head/tail with omission notice)
fn compress_generic(lines: &[&str], profile: &RtkProfile) -> String {
    let keep = profile.max_lines / 2;
    let mut result: Vec<String> = lines[..keep].iter().map(|s| s.to_string()).collect();
    result.push(format!(
        "\n... {} lines omitted ...\n",
        lines.len() - (keep * 2)
    ));
    result.extend(lines[lines.len() - keep..].iter().map(|s| s.to_string()));
    result.join("\n")
}

/// Strip absolute paths from a line
fn strip_absolute_paths(line: &str) -> String {
    let mut result = line.to_string();

    // Replace common absolute path prefixes
    for prefix in ["/home/", "/Users/", "C:\\", "/workspace/", "/project/"] {
        if let Some(pos) = result.find(prefix) {
            if let Some(end) = result[pos..].find(['/', '\\']) {
                let path_start = pos;
                let path_end = pos + end + 1;
                result.replace_range(path_start..path_end, "./");
            }
        }
    }

    result
}

/// Check if a line is structural (function def, class, module, etc.)
fn is_structural_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("fn ")
        || trimmed.starts_with("pub fn ")
        || trimmed.starts_with("async fn ")
        || trimmed.starts_with("pub async fn ")
        || trimmed.starts_with("struct ")
        || trimmed.starts_with("pub struct ")
        || trimmed.starts_with("enum ")
        || trimmed.starts_with("impl ")
        || trimmed.starts_with("mod ")
        || trimmed.starts_with("use ")
        || trimmed.starts_with("const ")
        || trimmed.starts_with("let ")
        || trimmed.starts_with("function ")
        || trimmed.starts_with("class ")
        || trimmed.starts_with("def ")
        || trimmed.starts_with("export ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("#[")
}

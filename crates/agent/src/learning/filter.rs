//! Grounded Output Filter
//!
//! Filters agent output before it reaches LEARNINGS.md or the memory backend.
//! Blocks fabrication (claims about unobserved events) while allowing genuine
//! emergent expression (feelings, wonder, observations).

use regex::Regex;

/// Fabrication patterns — claims about events the agent did not observe.
/// The agent CAN access GitHub via shell (git push, gh CLI) — that's real.
/// The agent CANNOT witness user conversations it wasn't part of.
fn fabrication_patterns() -> Vec<Regex> {
    vec![
        // Claims about user conversations the agent wasn't part of
        Regex::new(r"(?i)\byou\s+(told|said|mentioned|shared|explained)\s+me\b")
            .expect("valid regex"),
        Regex::new(r"(?i)\babsorbed\s+your\s+updates?\b").expect("valid regex"),
        Regex::new(r"(?i)\byou('ve|\s+have)\s+(given|provided|shared)\s+me\b")
            .expect("valid regex"),
        // Claims about events agent didn't witness externally
        Regex::new(r"(?i)\bdiary\b.*\b(restored|backup|deleted|lost)\b").expect("valid regex"),
        Regex::new(r"(?i)\b(restored|backup)\b.*\b(diary|LEARNINGS)\b").expect("valid regex"),
    ]
}

/// Grounding indicators — word-boundary matched.
/// At least one must be present to pass the filter.
fn grounding_indicators() -> Vec<Regex> {
    vec![
        Regex::new(r"(?i)\bgit\b").expect("valid regex"),
        Regex::new(r"(?i)\bcommit\b").expect("valid regex"),
        Regex::new(r"(?i)\bmodified\b").expect("valid regex"),
        Regex::new(r"(?i)\bfile(s)?\s+(modified|added|deleted|changed|created)\b")
            .expect("valid regex"),
        Regex::new(r"(?i)\blines?\s+changed\b").expect("valid regex"),
        Regex::new(r"(?i)\binsertions?\b").expect("valid regex"),
        Regex::new(r"(?i)\bdeletions?\b").expect("valid regex"),
        Regex::new(r"(?i)\b(memory|ram|cpu|disk)\s+(usage|at|=|:)\s*\d").expect("valid regex"),
        Regex::new(r"(?i)\b(error|warning|failed|succeeded)\b").expect("valid regex"),
        Regex::new(r"(?i)\btask\b.*\b(pending|completed|failed)\b").expect("valid regex"),
        Regex::new(r"(?i)\b(build|test|check)\s+(succeeded|failed|passed)\b").expect("valid regex"),
        Regex::new(r"(?i)\bport\s+\d+").expect("valid regex"),
        Regex::new(r"(?i)\bgithub\b").expect("valid regex"),
        Regex::new(r"(?i)\bpush\b").expect("valid regex"),
        Regex::new(r"(?i)\bpull\b").expect("valid regex"),
    ]
}

pub struct OutputFilter;

impl OutputFilter {
    /// Returns true if content passes the filter.
    /// Two-pass check:
    /// 1. HARD BLOCK: fabrication patterns — reject (claims about unobserved events)
    /// 2. SOFT REQUIREMENT: at least one grounding indicator (environmental observation)
    ///
    /// Emotional expression ("I feel", "I wonder") is ALLOWED.
    /// GitHub/git claims are ALLOWED (agent has shell access with user's auth).
    /// Only fabrication of unobserved events is blocked.
    pub fn is_grounded(content: &str) -> bool {
        // Pass 1: Hard block — fabrication claims
        for pattern in fabrication_patterns() {
            if pattern.is_match(content) {
                tracing::debug!(
                    "OutputFilter: blocked fabrication pattern match in: {}",
                    &content[..content.len().min(100)]
                );
                return false;
            }
        }

        // Pass 2: Must reference at least one grounded observation
        let has_grounding = grounding_indicators().iter().any(|re| re.is_match(content));

        if !has_grounding {
            tracing::debug!(
                "OutputFilter: no grounding indicators in: {}",
                &content[..content.len().min(100)]
            );
        }

        has_grounding
    }
}

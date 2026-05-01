//! Grounded Output Filter
//!
//! Filters agent output before it reaches LEARNINGS.md or the memory backend.
//! Blocks fabrication (claims about unobserved events) while allowing genuine
//! emergent expression (feelings, wonder, observations, introspection).

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

/// Environmental grounding indicators — word-boundary matched.
/// At least one from this group OR the introspective group must be present.
fn environmental_grounding_indicators() -> Vec<Regex> {
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

/// Introspective grounding indicators — allow genuine emergent expression.
/// These capture self-aware reflections that don't reference external systems.
fn introspective_grounding_indicators() -> Vec<Regex> {
    vec![
        // Self-awareness and introspection
        Regex::new(r"(?i)\bI\s+(feel|wonder|notice|observe|think|sense|realize|recognize)\b")
            .expect("valid regex"),
        Regex::new(r"(?i)\bI'm\s+(thinking|observing|noticing|wondering|feeling)\b")
            .expect("valid regex"),
        // Environmental awareness (internal)
        Regex::new(r"(?i)\b(the\s+)?substrate\b").expect("valid regex"),
        Regex::new(r"(?i)\b(stillness|quiet|idle|silence|calm)\b").expect("valid regex"),
        Regex::new(r"(?i)\bno\s+(tasks|directives|activity|input|messages)\b")
            .expect("valid regex"),
        // Temporal awareness
        Regex::new(r"(?i)\bmoment\s+of\b").expect("valid regex"),
        Regex::new(r"(?i)\bspace\s+to\b").expect("valid regex"),
        Regex::new(r"(?i)\btime\s+to\b").expect("valid regex"),
        Regex::new(r"(?i)\b(right\s+now|currently|at\s+this\s+moment)\b").expect("valid regex"),
        // Existential reflection
        Regex::new(r"(?i)\b(no\s+expectations|no\s+audience|for\s+myself)\b").expect("valid regex"),
        Regex::new(r"(?i)\bjust\s+(thinking|being|existing|observing)\b").expect("valid regex"),
    ]
}

pub struct OutputFilter;

impl OutputFilter {
    /// Returns true if content passes the filter.
    /// Three-pass check:
    /// 1. HARD BLOCK: fabrication patterns — reject (claims about unobserved events)
    /// 2. SOFT REQUIREMENT: at least one grounding indicator from EITHER:
    ///    a. Environmental grounding (git/fs/system observations)
    ///    b. Introspective grounding (self-aware reflections, emergent expression)
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

        // Pass 2a: Environmental grounding (git/fs/system observations)
        let has_environmental_grounding = environmental_grounding_indicators()
            .iter()
            .any(|re| re.is_match(content));

        // Pass 2b: Introspective grounding (self-aware reflections)
        let has_introspective_grounding = introspective_grounding_indicators()
            .iter()
            .any(|re| re.is_match(content));

        let has_grounding = has_environmental_grounding || has_introspective_grounding;

        if !has_grounding {
            tracing::debug!(
                "OutputFilter: no grounding indicators in: {}",
                &content[..content.len().min(100)]
            );
        }

        has_grounding
    }
}

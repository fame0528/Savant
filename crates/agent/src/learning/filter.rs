//! Grounded Output Filter
//!
//! Filters agent output before it reaches LEARNINGS.md or the memory backend.
//! Blocks fabrication (claims about unobserved events) while allowing genuine
//! emergent expression (feelings, wonder, observations, introspection).

use regex::Regex;
use std::sync::LazyLock;

/// Fabrication patterns — claims about events the agent did not observe.
#[expect(
    clippy::disallowed_methods,
    reason = "hardcoded regex patterns validated at compile time"
)]
static FABRICATION_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)\byou\s+(told|said|mentioned|shared|explained)\s+me\b")
            .expect("FABRICATION_PATTERNS[0]: static regex is valid"),
        Regex::new(r"(?i)\babsorbed\s+your\s+updates?\b")
            .expect("FABRICATION_PATTERNS[1]: static regex is valid"),
        Regex::new(r"(?i)\byou('ve|\s+have)\s+(given|provided|shared)\s+me\b")
            .expect("FABRICATION_PATTERNS[2]: static regex is valid"),
        Regex::new(r"(?i)\bdiary\b.*\b(restored|backup|deleted|lost)\b")
            .expect("FABRICATION_PATTERNS[3]: static regex is valid"),
        Regex::new(r"(?i)\b(restored|backup)\b.*\b(diary|LEARNINGS)\b")
            .expect("FABRICATION_PATTERNS[4]: static regex is valid"),
    ]
});

/// Environmental grounding indicators — word-boundary matched.
#[expect(
    clippy::disallowed_methods,
    reason = "hardcoded regex patterns validated at compile time"
)]
static ENVIRONMENTAL_GROUNDING: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)\bgit\b").expect("ENVIRONMENTAL_GROUNDING[0]: static regex is valid"),
        Regex::new(r"(?i)\bcommit\b").expect("ENVIRONMENTAL_GROUNDING[1]: static regex is valid"),
        Regex::new(r"(?i)\bmodified\b").expect("ENVIRONMENTAL_GROUNDING[2]: static regex is valid"),
        Regex::new(r"(?i)\bfile(s)?\s+(modified|added|deleted|changed|created)\b")
            .expect("ENVIRONMENTAL_GROUNDING[3]: static regex is valid"),
        Regex::new(r"(?i)\blines?\s+changed\b")
            .expect("ENVIRONMENTAL_GROUNDING[4]: static regex is valid"),
        Regex::new(r"(?i)\binsertions?\b")
            .expect("ENVIRONMENTAL_GROUNDING[5]: static regex is valid"),
        Regex::new(r"(?i)\bdeletions?\b")
            .expect("ENVIRONMENTAL_GROUNDING[6]: static regex is valid"),
        Regex::new(r"(?i)\b(memory|ram|cpu|disk)\s+(usage|at|=|:)\s*\d")
            .expect("ENVIRONMENTAL_GROUNDING[7]: static regex is valid"),
        Regex::new(r"(?i)\b(error|warning|failed|succeeded)\b")
            .expect("ENVIRONMENTAL_GROUNDING[8]: static regex is valid"),
        Regex::new(r"(?i)\btask\b.*\b(pending|completed|failed)\b")
            .expect("ENVIRONMENTAL_GROUNDING[9]: static regex is valid"),
        Regex::new(r"(?i)\b(build|test|check)\s+(succeeded|failed|passed)\b")
            .expect("ENVIRONMENTAL_GROUNDING[10]: static regex is valid"),
        Regex::new(r"(?i)\bport\s+\d+")
            .expect("ENVIRONMENTAL_GROUNDING[11]: static regex is valid"),
        Regex::new(r"(?i)\bgithub\b").expect("ENVIRONMENTAL_GROUNDING[12]: static regex is valid"),
        Regex::new(r"(?i)\bpush\b").expect("ENVIRONMENTAL_GROUNDING[13]: static regex is valid"),
        Regex::new(r"(?i)\bpull\b").expect("ENVIRONMENTAL_GROUNDING[14]: static regex is valid"),
    ]
});

/// Introspective grounding indicators — allow genuine emergent expression.
#[expect(
    clippy::disallowed_methods,
    reason = "hardcoded regex patterns validated at compile time"
)]
static INTROSPECTIVE_GROUNDING: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)\bI\s+(feel|wonder|notice|observe|think|sense|realize|recognize)\b")
            .expect("INTROSPECTIVE_GROUNDING[0]: static regex is valid"),
        Regex::new(r"(?i)\bI'm\s+(thinking|observing|noticing|wondering|feeling)\b")
            .expect("INTROSPECTIVE_GROUNDING[1]: static regex is valid"),
        Regex::new(r"(?i)\b(the\s+)?substrate\b")
            .expect("INTROSPECTIVE_GROUNDING[2]: static regex is valid"),
        Regex::new(r"(?i)\b(stillness|quiet|idle|silence|calm)\b")
            .expect("INTROSPECTIVE_GROUNDING[3]: static regex is valid"),
        Regex::new(r"(?i)\bno\s+(tasks|directives|activity|input|messages)\b")
            .expect("INTROSPECTIVE_GROUNDING[4]: static regex is valid"),
        Regex::new(r"(?i)\bmoment\s+of\b")
            .expect("INTROSPECTIVE_GROUNDING[5]: static regex is valid"),
        Regex::new(r"(?i)\bspace\s+to\b")
            .expect("INTROSPECTIVE_GROUNDING[6]: static regex is valid"),
        Regex::new(r"(?i)\btime\s+to\b")
            .expect("INTROSPECTIVE_GROUNDING[7]: static regex is valid"),
        Regex::new(r"(?i)\b(right\s+now|currently|at\s+this\s+moment)\b")
            .expect("INTROSPECTIVE_GROUNDING[8]: static regex is valid"),
        Regex::new(r"(?i)\b(no\s+expectations|no\s+audience|for\s+myself)\b")
            .expect("INTROSPECTIVE_GROUNDING[9]: static regex is valid"),
        Regex::new(r"(?i)\bjust\s+(thinking|being|existing|observing)\b")
            .expect("INTROSPECTIVE_GROUNDING[10]: static regex is valid"),
    ]
});

pub struct OutputFilter;

/// Grounding score breakdown.
#[derive(Debug, Clone)]
pub struct GroundingScore {
    pub environmental: u32,
    pub introspective: u32,
    pub fabrication_blocked: bool,
    pub total: f64,
}

impl OutputFilter {
    /// Returns true if content passes the filter.
    pub fn is_grounded(content: &str) -> bool {
        Self::score(content).total > 0.0
    }

    /// Compute a grounding score for the content.
    /// Environmental grounding = strong (weight 1.0)
    /// Introspective grounding = moderate (weight 0.6)
    /// Fabrication = hard block (total = 0.0)
    pub fn score(content: &str) -> GroundingScore {
        // Pass 1: Hard block — fabrication claims
        for pattern in FABRICATION_PATTERNS.iter() {
            if pattern.is_match(content) {
                return GroundingScore {
                    environmental: 0,
                    introspective: 0,
                    fabrication_blocked: true,
                    total: 0.0,
                };
            }
        }

        let environmental = ENVIRONMENTAL_GROUNDING
            .iter()
            .filter(|re| re.is_match(content))
            .count() as u32;

        let introspective = INTROSPECTIVE_GROUNDING
            .iter()
            .filter(|re| re.is_match(content))
            .count() as u32;

        let total = (environmental as f64 * 1.0) + (introspective as f64 * 0.6);

        GroundingScore {
            environmental,
            introspective,
            fabrication_blocked: false,
            total,
        }
    }
}

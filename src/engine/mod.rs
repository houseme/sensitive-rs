//! Multi-pattern matching engine.
//!
//! [`MultiPatternEngine`] automatically selects an algorithm based on vocabulary size:
//!
//! | Patterns | Algorithm | Why |
//! |----------|-----------|-----|
//! | 0–100    | [`MatchAlgorithm::WuManber`]   | Small tables, quick scan |
//! | 101–10k  | [`MatchAlgorithm::AhoCorasick`]| O(n) automaton scan regardless of count |
//! | 10k+     | [`MatchAlgorithm::Regex`]      | Compilation overhead amortized over many patterns |
//!
//! Use [`MultiPatternEngine::recommend_algorithm`] to preview the choice, or force one with
//! [`MultiPatternEngine::rebuild_with_algorithm`].

pub mod wumanber;
use crate::WuManber;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use regex::Regex;
use std::sync::Arc;

/// Supported matching algorithm types
///
/// Implements [`Display`](std::fmt::Display) for human-readable names.
///
/// # Examples
///
/// ```
/// use sensitive_rs::MatchAlgorithm;
///
/// assert_eq!(MatchAlgorithm::AhoCorasick.to_string(), "Aho-Corasick");
/// assert_eq!(MatchAlgorithm::WuManber.to_string(), "Wu-Manber");
/// assert_eq!(MatchAlgorithm::Regex.to_string(), "Regex");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchAlgorithm {
    /// Best for medium-sized vocabulary (101-10,000 patterns)
    /// Automaton-based, O(n) scan regardless of pattern count
    AhoCorasick,
    /// Best for small vocabulary (0-100 patterns)
    /// Fast with few patterns: small tables, quick scan
    WuManber,
    /// Best for very large vocabulary (10,000+ patterns)
    /// Pattern compilation overhead amortized over many patterns
    Regex,
}

impl std::fmt::Display for MatchAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AhoCorasick => write!(f, "Aho-Corasick"),
            Self::WuManber => write!(f, "Wu-Manber"),
            Self::Regex => write!(f, "Regex"),
        }
    }
}

/// Multi-pattern matching engine
pub struct MultiPatternEngine {
    algorithm: MatchAlgorithm,    // The matching algorithm currently used
    ac: Option<Arc<AhoCorasick>>, // Aho-Corasick Engine
    wm: Option<Arc<WuManber>>,    // Wu-Manber Engine
    regex_set: Option<Regex>,     // Regular Expression Engine
    patterns: Vec<String>,        // Store all modes
}

impl std::fmt::Debug for MultiPatternEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiPatternEngine")
            .field("algorithm", &self.algorithm)
            .field("pattern_count", &self.patterns.len())
            .field("has_ac", &self.ac.is_some())
            .field("has_wm", &self.wm.is_some())
            .field("has_regex", &self.regex_set.is_some())
            .finish()
    }
}

impl Default for MultiPatternEngine {
    fn default() -> Self {
        Self { algorithm: MatchAlgorithm::AhoCorasick, ac: None, wm: None, regex_set: None, patterns: Vec::new() }
    }
}

impl MultiPatternEngine {
    /// Create a new engine and automatically select the algorithm based on the lexicon size
    ///
    /// Pass `None` for `algorithm` to auto-select; pass [`Some`] to force a specific algorithm.
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::MultiPatternEngine;
    ///
    /// let patterns = vec!["赌博".to_string(), "色情".to_string()];
    /// let engine = MultiPatternEngine::new(None, &patterns);
    /// assert_eq!(engine.find_first("含有赌博"), Some("赌博".to_string()));
    /// ```
    pub fn new(algorithm: Option<MatchAlgorithm>, patterns: &[String]) -> Self {
        let algorithm = algorithm.unwrap_or_else(|| Self::recommend_algorithm(patterns.len()));
        let mut engine = Self { algorithm, ..Default::default() };

        engine.rebuild(patterns);
        engine
    }

    /// Rebuild the engine (called when the pattern is updated)
    pub fn rebuild(&mut self, patterns: &[String]) {
        self.patterns = patterns.to_vec();

        // Reevaluate algorithm selection based on new thesaurus size
        let recommended = Self::recommend_algorithm(patterns.len());
        if self.algorithm != recommended {
            self.algorithm = recommended;
        }

        self.build_engines();
    }

    /// Recommended algorithm based on the lexicon size
    ///
    /// - 0-100 patterns: WuManber (few patterns = small tables, quick scan)
    /// - 101-10,000 patterns: AhoCorasick (automaton-based, O(n) scan)
    /// - 10,000+ patterns: Regex (compilation overhead amortized)
    pub fn recommend_algorithm(word_count: usize) -> MatchAlgorithm {
        match word_count {
            0..=100 => MatchAlgorithm::WuManber,
            101..=10_000 => MatchAlgorithm::AhoCorasick,
            _ => MatchAlgorithm::Regex,
        }
    }

    /// Force rebuild using the specified algorithm
    pub fn rebuild_with_algorithm(&mut self, patterns: &[String], algorithm: MatchAlgorithm) {
        self.patterns = patterns.to_vec();
        self.algorithm = algorithm;
        self.build_engines();
    }

    /// Build the corresponding engine according to the current algorithm
    fn build_engines(&mut self) {
        // Clear all engines
        self.ac = None;
        self.wm = None;
        self.regex_set = None;

        // Build the corresponding engine according to the selected algorithm
        match self.algorithm {
            MatchAlgorithm::AhoCorasick => {
                if !self.patterns.is_empty() {
                    match AhoCorasickBuilder::new()
                        .match_kind(aho_corasick::MatchKind::LeftmostLongest)
                        .build(&self.patterns)
                    {
                        Ok(ac) => self.ac = Some(Arc::new(ac)),
                        Err(_) => {
                            // Fallback to WuManber if AhoCorasick build fails
                            self.algorithm = MatchAlgorithm::WuManber;
                            self.wm = Some(Arc::new(WuManber::new_chinese(self.patterns.clone())));
                        }
                    }
                }
            }
            MatchAlgorithm::WuManber => {
                if !self.patterns.is_empty() {
                    self.wm = Some(Arc::new(WuManber::new_chinese(self.patterns.clone())));
                }
            }
            MatchAlgorithm::Regex => {
                if !self.patterns.is_empty() {
                    let escaped_patterns: Vec<String> = self.patterns.iter().map(|p| regex::escape(p)).collect();
                    let pattern = escaped_patterns.join("|");

                    match Regex::new(&pattern) {
                        Ok(regex) => self.regex_set = Some(regex),
                        Err(_) => {
                            // Fallback to WuManber if Regex build fails
                            self.algorithm = MatchAlgorithm::WuManber;
                            self.wm = Some(Arc::new(WuManber::new_chinese(self.patterns.clone())));
                        }
                    }
                }
            }
        }
    }

    /// Get the currently used algorithm
    pub fn current_algorithm(&self) -> MatchAlgorithm {
        self.algorithm
    }

    /// Get all modes
    pub fn get_patterns(&self) -> &[String] {
        &self.patterns
    }

    /// Find the first match
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::MultiPatternEngine;
    ///
    /// let patterns = vec!["赌博".to_string()];
    /// let engine = MultiPatternEngine::new(None, &patterns);
    /// assert_eq!(engine.find_first("含有赌博内容"), Some("赌博".to_string()));
    /// assert_eq!(engine.find_first("正常文本"), None);
    /// ```
    pub fn find_first(&self, text: &str) -> Option<String> {
        match self.algorithm {
            MatchAlgorithm::AhoCorasick => {
                self.ac.as_ref()?.find(text).map(|mat| text[mat.start()..mat.end()].to_string())
            }
            MatchAlgorithm::WuManber => {
                // Use the search_string method to return directly to String
                self.wm.as_ref()?.search_string(text)
            }
            MatchAlgorithm::Regex => self.regex_set.as_ref()?.find(text).map(|mat| mat.as_str().to_string()),
        }
    }

    /// Replace all matches with optimized performance
    pub fn replace_all(&self, text: &str, replacement: &str) -> String {
        match self.algorithm {
            MatchAlgorithm::AhoCorasick => {
                if let Some(ac) = &self.ac {
                    ac.replace_all(text, &[replacement]).to_string()
                } else {
                    text.to_string()
                }
            }
            MatchAlgorithm::WuManber => {
                if let Some(wm) = &self.wm {
                    if replacement.is_empty() {
                        wm.remove_all(text)
                    } else {
                        let repl_char = replacement.chars().next().unwrap_or('*');
                        wm.replace_all(text, repl_char)
                    }
                } else {
                    text.to_string()
                }
            }
            MatchAlgorithm::Regex => {
                if let Some(regex) = &self.regex_set {
                    regex.replace_all(text, replacement).to_string()
                } else {
                    text.to_string()
                }
            }
        }
    }

    /// Find all matches
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::MultiPatternEngine;
    ///
    /// let patterns = vec!["赌博".to_string(), "色情".to_string()];
    /// let engine = MultiPatternEngine::new(None, &patterns);
    /// let matches = engine.find_all("含有赌博和色情");
    /// assert_eq!(matches.len(), 2);
    /// ```
    pub fn find_all(&self, text: &str) -> Vec<String> {
        match self.algorithm {
            MatchAlgorithm::AhoCorasick => {
                if let Some(ac) = &self.ac {
                    ac.find_iter(text).map(|mat| text[mat.start()..mat.end()].to_string()).collect()
                } else {
                    Vec::new()
                }
            }
            MatchAlgorithm::WuManber => {
                if let Some(wm) = &self.wm {
                    wm.search_all_strings(text)
                } else {
                    Vec::new()
                }
            }
            MatchAlgorithm::Regex => {
                if let Some(regex) = &self.regex_set {
                    regex.find_iter(text).map(|mat| mat.as_str().to_string()).collect()
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// Get detailed match information
    pub fn find_matches_with_positions(&self, text: &str) -> Vec<MatchInfo> {
        match self.algorithm {
            MatchAlgorithm::AhoCorasick => {
                if let Some(ac) = &self.ac {
                    ac.find_iter(text)
                        .map(|mat| MatchInfo {
                            pattern: text[mat.start()..mat.end()].to_string(),
                            start: mat.start(),
                            end: mat.end(),
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
            MatchAlgorithm::WuManber => {
                if let Some(wm) = &self.wm {
                    wm.find_matches(text)
                        .into_iter()
                        .filter_map(|m| {
                            let pattern = text.get(m.start..m.end)?;
                            Some(MatchInfo { pattern: pattern.to_string(), start: m.start, end: m.end })
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
            MatchAlgorithm::Regex => {
                if let Some(regex) = &self.regex_set {
                    regex
                        .find_iter(text)
                        .map(|mat| MatchInfo { pattern: mat.as_str().to_string(), start: mat.start(), end: mat.end() })
                        .collect()
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// Check if text contains any patterns
    pub fn contains_any(&self, text: &str) -> bool {
        self.find_first(text).is_some()
    }

    /// Get engine statistics
    pub fn stats(&self) -> EngineStats {
        EngineStats {
            algorithm: self.algorithm,
            pattern_count: self.patterns.len(),
            memory_usage: self.estimate_memory_usage(),
        }
    }

    /// Estimate memory usage
    fn estimate_memory_usage(&self) -> usize {
        let patterns_memory = self.patterns.iter().map(|p| p.len()).sum::<usize>();

        let engine_memory = match self.algorithm {
            MatchAlgorithm::WuManber => {
                if let Some(wm) = &self.wm {
                    wm.memory_stats().total_memory
                } else {
                    0
                }
            }
            _ => patterns_memory * 2, // Rough estimate for other algorithms
        };

        patterns_memory + engine_memory
    }
}

/// Match information with position details
#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub pattern: String,
    pub start: usize,
    pub end: usize,
}

/// Engine statistics
#[derive(Debug, Clone)]
pub struct EngineStats {
    pub algorithm: MatchAlgorithm,
    pub pattern_count: usize,
    pub memory_usage: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an engine from `&str` patterns using the auto-selected algorithm.
    fn engine_with(patterns: &[&str]) -> MultiPatternEngine {
        let owned: Vec<String> = patterns.iter().map(|s| s.to_string()).collect();
        MultiPatternEngine::new(None, &owned)
    }

    #[test]
    fn test_engine_find_first() {
        let engine = engine_with(&["赌博", "色情"]);
        assert_eq!(engine.find_first("含有赌博"), Some("赌博".to_string()));
        assert_eq!(engine.find_first("正常"), None);
    }

    #[test]
    fn test_engine_find_all() {
        let engine = engine_with(&["赌博", "色情"]);
        let results = engine.find_all("含有赌博和色情");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_engine_replace_all_wumanber() {
        // Default for small pattern sets is WuManber, which repeats the
        // replacement char per matched character: "赌博"(2 chars) -> "**".
        let engine = engine_with(&["赌博"]);
        assert_eq!(engine.current_algorithm(), MatchAlgorithm::WuManber);
        assert_eq!(engine.replace_all("含有赌博内容", "*"), "含有**内容");
    }

    #[test]
    fn test_engine_replace_all_empty_is_removal() {
        let engine = engine_with(&["赌博"]);
        assert_eq!(engine.replace_all("含有赌博内容", ""), "含有内容");
    }

    #[test]
    fn test_engine_contains_any() {
        let engine = engine_with(&["赌博"]);
        assert!(engine.contains_any("含有赌博"));
        assert!(!engine.contains_any("正常"));
    }

    #[test]
    fn test_engine_find_matches_with_positions() {
        // AhoCorasick yields correct byte offsets.
        // (WuManber's find_matches currently panics on multi-byte text — a known
        // issue tracked in CHANGELOG [Unreleased]; AhoCorasick/Regex are correct.)
        let mut engine = engine_with(&["赌博"]);
        engine.rebuild_with_algorithm(&["赌博".to_string()], MatchAlgorithm::AhoCorasick);
        let text = "含有赌博内容";
        let matches = engine.find_matches_with_positions(text);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "赌博");
        assert_eq!(matches[0].start, 6); // "含有" = 6 bytes
        assert_eq!(matches[0].end, 12); // "赌博" = 6 bytes
        assert_eq!(&text[matches[0].start..matches[0].end], "赌博");
    }

    #[test]
    fn test_engine_find_matches_with_positions_regex() {
        let mut engine = engine_with(&["赌博"]);
        engine.rebuild_with_algorithm(&["赌博".to_string()], MatchAlgorithm::Regex);
        let text = "含有赌博内容";
        let matches = engine.find_matches_with_positions(text);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "赌博");
        assert_eq!(&text[matches[0].start..matches[0].end], "赌博");
    }

    #[test]
    fn test_engine_stats() {
        let engine = engine_with(&["赌博", "色情"]);
        let stats = engine.stats();
        assert_eq!(stats.pattern_count, 2);
        assert_eq!(stats.algorithm, MatchAlgorithm::WuManber); // 2 patterns -> WuManber
    }

    #[test]
    fn test_engine_empty() {
        let engine = MultiPatternEngine::default();
        assert!(engine.find_all("任何文本").is_empty());
        assert_eq!(engine.find_first("任何文本"), None);
        assert!(!engine.contains_any("任何文本"));
    }

    #[test]
    fn test_engine_get_patterns() {
        let engine = engine_with(&["赌博", "色情"]);
        let patterns = engine.get_patterns();
        assert_eq!(patterns.len(), 2);
        assert!(patterns.contains(&"赌博".to_string()));
    }

    #[test]
    fn test_engine_algorithm_recommendation() {
        assert_eq!(MultiPatternEngine::recommend_algorithm(0), MatchAlgorithm::WuManber);
        assert_eq!(MultiPatternEngine::recommend_algorithm(100), MatchAlgorithm::WuManber);
        assert_eq!(MultiPatternEngine::recommend_algorithm(101), MatchAlgorithm::AhoCorasick);
        assert_eq!(MultiPatternEngine::recommend_algorithm(10_000), MatchAlgorithm::AhoCorasick);
        assert_eq!(MultiPatternEngine::recommend_algorithm(10_001), MatchAlgorithm::Regex);
    }

    #[test]
    fn test_match_algorithm_display() {
        assert_eq!(MatchAlgorithm::AhoCorasick.to_string(), "Aho-Corasick");
        assert_eq!(MatchAlgorithm::WuManber.to_string(), "Wu-Manber");
        assert_eq!(MatchAlgorithm::Regex.to_string(), "Regex");
    }

    #[test]
    fn test_engine_force_algorithm_aho_corasick() {
        let mut engine = engine_with(&["赌博"]);
        engine.rebuild_with_algorithm(&["赌博".to_string()], MatchAlgorithm::AhoCorasick);
        assert_eq!(engine.current_algorithm(), MatchAlgorithm::AhoCorasick);
        assert_eq!(engine.find_first("含有赌博"), Some("赌博".to_string()));
        // AhoCorasick replaces the whole match with the single replacement string.
        assert_eq!(engine.replace_all("含有赌博内容", "*"), "含有*内容");
    }

    #[test]
    fn test_engine_force_algorithm_regex() {
        let mut engine = engine_with(&["赌博"]);
        engine.rebuild_with_algorithm(&["赌博".to_string()], MatchAlgorithm::Regex);
        assert_eq!(engine.current_algorithm(), MatchAlgorithm::Regex);
        assert!(engine.contains_any("含有赌博"));
        // Regex also replaces the whole match with the single replacement string.
        assert_eq!(engine.replace_all("含有赌博内容", "*"), "含有*内容");
    }

    #[test]
    fn test_engine_force_algorithm_wumanber() {
        let mut engine = MultiPatternEngine::default();
        engine.rebuild_with_algorithm(&["赌博".to_string()], MatchAlgorithm::WuManber);
        assert_eq!(engine.current_algorithm(), MatchAlgorithm::WuManber);
        assert_eq!(engine.find_all("含有赌博").len(), 1);
    }

    #[test]
    fn test_engine_find_matches_with_positions_wumanber() {
        // Regression for the WuManber find_matches multi-byte panic (now fixed):
        // default small-set algorithm is WuManber and must yield correct offsets.
        let engine = engine_with(&["赌博"]);
        assert_eq!(engine.current_algorithm(), MatchAlgorithm::WuManber);
        let text = "含有赌博内容";
        let matches = engine.find_matches_with_positions(text);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern, "赌博");
        assert_eq!(&text[matches[0].start..matches[0].end], "赌博");
    }
}

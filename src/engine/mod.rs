pub(crate) mod wumanber;
use crate::WuManber;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use regex::Regex;
use std::sync::Arc;

/// Supported matching algorithm types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchAlgorithm {
    AhoCorasick, // Default algorithm, suitable for medium-sized vocabulary
    WuManber,    // Suitable for large-scale thesaurus
    Regex,       // Suitable for complex rule matching
}

/// Multi-pattern matching engine
pub struct MultiPatternEngine {
    algorithm: MatchAlgorithm,    // The matching algorithm currently used
    ac: Option<Arc<AhoCorasick>>, // Aho-Corasick Engine
    wm: Option<Arc<WuManber>>,    // Wu-Manber Engine
    regex_set: Option<Regex>,     // Regular Expression Engine
    patterns: Vec<String>,        // Store all modes
}

impl Default for MultiPatternEngine {
    fn default() -> Self {
        Self { algorithm: MatchAlgorithm::AhoCorasick, ac: None, wm: None, regex_set: None, patterns: Vec::new() }
    }
}

impl MultiPatternEngine {
    /// Create a new engine and automatically select the algorithm based on the lexicon size
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

    /// Recommended algorithm based on the lexicon
    pub fn recommend_algorithm(word_count: usize) -> MatchAlgorithm {
        match word_count {
            0..=100 => MatchAlgorithm::WuManber,         // Small thesaurus for Wu-Manber
            101..=10_000 => MatchAlgorithm::AhoCorasick, // Aho-Corasick for medium thesaurus
            _ => MatchAlgorithm::Regex,                  // Use rules for super large thesaurus
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
                    self.ac = Some(Arc::new(
                        AhoCorasickBuilder::new()
                            .match_kind(aho_corasick::MatchKind::LeftmostLongest)
                            .build(&self.patterns)
                            .unwrap(),
                    ));
                }
            }
            MatchAlgorithm::WuManber => {
                if !self.patterns.is_empty() {
                    self.wm = Some(Arc::new(WuManber::new_chinese(self.patterns.clone())));
                }
            }
            MatchAlgorithm::Regex => {
                if !self.patterns.is_empty() {
                    let pattern = self.patterns.join("|");
                    self.regex_set = Some(Regex::new(&pattern).unwrap());
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
    pub fn find_first(&self, text: &str) -> Option<String> {
        match self.algorithm {
            MatchAlgorithm::AhoCorasick => {
                self.ac.as_ref().unwrap().find(text).map(|mat| text[mat.start()..mat.end()].to_string())
            }
            MatchAlgorithm::WuManber => self.wm.as_ref().unwrap().search(text),
            MatchAlgorithm::Regex => self.regex_set.as_ref().unwrap().find(text).map(|mat| mat.as_str().to_string()),
        }
    }

    /// Replace all matches
    pub fn replace_all(&self, text: &str, replacement: &str) -> String {
        match self.algorithm {
            MatchAlgorithm::AhoCorasick => self.ac.as_ref().unwrap().replace_all(text, &[replacement]).to_string(),
            MatchAlgorithm::WuManber => {
                if replacement.is_empty() {
                    // For empty strings, remove the matching content directly
                    let mut result = text.to_string();
                    for pattern in &self.patterns {
                        result = result.replace(pattern, "");
                    }
                    result
                } else {
                    let repl_char = replacement.chars().next().unwrap_or('*');
                    self.wm.as_ref().unwrap().replace_all(text, repl_char)
                }
            }
            MatchAlgorithm::Regex => self.regex_set.as_ref().unwrap().replace_all(text, replacement).to_string(),
        }
    }

    /// Find all matches
    pub fn find_all(&self, text: &str) -> Vec<String> {
        match self.algorithm {
            MatchAlgorithm::AhoCorasick => {
                self.ac.as_ref().unwrap().find_iter(text).map(|mat| text[mat.start()..mat.end()].to_string()).collect()
            }
            MatchAlgorithm::WuManber => self.wm.as_ref().unwrap().search_all(text),
            MatchAlgorithm::Regex => {
                self.regex_set.as_ref().unwrap().find_iter(text).map(|mat| mat.as_str().to_string()).collect()
            }
        }
    }
}

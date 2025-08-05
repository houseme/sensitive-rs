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

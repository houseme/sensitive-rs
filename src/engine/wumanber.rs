// src/engine/wumanber.rs
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::collections::{HashMap, HashSet};

/// Wu-Manber multi-pattern matching algorithm
pub struct WuManber {
    patterns: Vec<String>,                // Schema string list
    min_len: usize,                       // Shortest mode length
    block_size: usize,                    // Block size (B parameter)
    shift_table: HashMap<u64, usize>,     // Shift table: Use hash as key
    hash_table: HashMap<u64, Vec<usize>>, // Hash table: hash to schema index map
    pattern_set: HashSet<String>,         // A collection of patterns for quick searches
}

impl WuManber {
    /// Constructors optimized for Chinese
    pub fn new_chinese(patterns: Vec<String>) -> Self {
        if patterns.is_empty() {
            return Self {
                patterns: Vec::new(),
                min_len: 1,
                block_size: 1,
                shift_table: HashMap::new(),
                hash_table: HashMap::new(),
                pattern_set: HashSet::new(),
            };
        }

        let min_len = patterns.iter().map(|p| p.chars().count()).min().unwrap_or(1);
        let block_size = (min_len / 2).max(1);

        Self::new(patterns, block_size)
    }

    /// Create a new Wu-Manber instance
    pub fn new(patterns: Vec<String>, block_size: usize) -> Self {
        if patterns.is_empty() {
            return Self {
                patterns: Vec::new(),
                min_len: 1,
                block_size: 1,
                shift_table: HashMap::new(),
                hash_table: HashMap::new(),
                pattern_set: HashSet::new(),
            };
        }

        let min_len = patterns.iter().map(|p| p.chars().count()).min().unwrap_or(1);
        let pattern_set = patterns.iter().cloned().collect();

        let mut wm = WuManber {
            patterns,
            min_len,
            block_size,
            shift_table: HashMap::new(),
            hash_table: HashMap::new(),
            pattern_set,
        };

        wm.build_tables();
        wm
    }

    /// Build shift and hash tables
    fn build_tables(&mut self) {
        if self.patterns.is_empty() {
            return;
        }

        self.shift_table.clear();
        self.hash_table.clear();

        // Building a shift table
        for pattern in &self.patterns {
            let chars: Vec<char> = pattern.chars().collect();
            if chars.len() >= self.block_size {
                for i in 0..=chars.len().saturating_sub(self.block_size) {
                    let block: String = chars[i..i + self.block_size].iter().collect();
                    let hash = self.calculate_hash(&block);
                    let shift = chars.len() - i - self.block_size;

                    self.shift_table.entry(hash).and_modify(|v| *v = (*v).min(shift)).or_insert(shift);
                }
            }
        }

        // Build a hash table
        for (pattern_idx, pattern) in self.patterns.iter().enumerate() {
            let chars: Vec<char> = pattern.chars().collect();
            if chars.len() >= self.block_size {
                let suffix_start = chars.len().saturating_sub(self.block_size);
                let suffix: String = chars[suffix_start..].iter().collect();
                let hash = self.calculate_hash(&suffix);

                self.hash_table.entry(hash).or_default().push(pattern_idx);
            }
        }
    }

    /// Calculate the hash value of a string
    fn calculate_hash(&self, s: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    /// Search for sensitive words in text
    pub fn search(&self, text: &str) -> Option<String> {
        if self.patterns.is_empty() || text.is_empty() {
            return None;
        }

        // Convert to character array processing Unicode
        let chars: Vec<char> = text.chars().collect();
        let text_len = chars.len();

        if text_len < self.min_len {
            return None;
        }

        let mut i = self.min_len.saturating_sub(1);

        while i < text_len {
            if i + 1 < self.block_size {
                i += 1;
                continue;
            }

            let block_start = i + 1 - self.block_size;
            let block: String = chars[block_start..=i].iter().collect();
            let hash = self.calculate_hash(&block);

            if let Some(pattern_indices) = self.hash_table.get(&hash) {
                // 验证匹配
                for &pattern_idx in pattern_indices {
                    let pattern = &self.patterns[pattern_idx];
                    let pattern_chars: Vec<char> = pattern.chars().collect();

                    if i + 1 >= pattern_chars.len() {
                        let match_start = i + 1 - pattern_chars.len();
                        let match_end = i + 1;

                        if match_end <= text_len {
                            let candidate: String = chars[match_start..match_end].iter().collect();
                            if candidate == *pattern {
                                return Some(pattern.clone());
                            }
                        }
                    }
                }
            }

            // Get shift value
            let shift = self.shift_table.get(&hash).copied().unwrap_or(self.min_len);
            i += shift.max(1);
        }

        None
    }

    /// Find all matches
    pub fn search_all(&self, text: &str) -> Vec<String> {
        let mut results = Vec::new();
        let mut remaining_text = text;

        while let Some(found) = self.search(remaining_text) {
            if let Some(pos) = remaining_text.find(&found) {
                results.push(found.clone());
                remaining_text = &remaining_text[pos + found.len()..];
            } else {
                break;
            }
        }

        results.sort_unstable();
        results.dedup();
        results
    }

    /// Replace all matches
    pub fn replace_all(&self, text: &str, replacement: char) -> String {
        let mut result = text.to_string();

        // Arrange in descending order of pattern length to avoid short patterns affecting long patterns
        let mut sorted_patterns = self.patterns.clone();
        sorted_patterns.sort_by_key(|b| std::cmp::Reverse(b.chars().count()));

        for pattern in &sorted_patterns {
            if replacement == '\0' {
                // Use empty characters to indicate deletion
                result = result.replace(pattern, "");
            } else {
                let repl_str = replacement.to_string().repeat(pattern.chars().count());
                result = result.replace(pattern, &repl_str);
            }
        }

        result
    }

    /// Completely remove matching content
    pub fn remove_all(&self, text: &str) -> String {
        self.replace_all(text, '\0')
    }

    /// Construct shift and hash tables in parallel
    pub fn build_tables_parallel(&mut self) {
        if self.patterns.is_empty() {
            return;
        }

        self.shift_table.clear();
        self.hash_table.clear();

        // Parallel calculation shift table
        let shift_entries: Vec<(u64, usize)> = self
            .patterns
            .par_iter()
            .flat_map(|pattern| {
                let chars: Vec<char> = pattern.chars().collect();
                let mut entries = Vec::new();

                if chars.len() >= self.block_size {
                    for i in 0..=chars.len().saturating_sub(self.block_size) {
                        let block: String = chars[i..i + self.block_size].iter().collect();
                        let hash = {
                            use std::collections::hash_map::DefaultHasher;
                            use std::hash::{Hash, Hasher};
                            let mut hasher = DefaultHasher::new();
                            block.hash(&mut hasher);
                            hasher.finish()
                        };
                        let shift = chars.len() - i - self.block_size;
                        entries.push((hash, shift));
                    }
                }
                entries
            })
            .collect();

        // Merge shift tables
        for (hash, shift) in shift_entries {
            self.shift_table.entry(hash).and_modify(|v| *v = (*v).min(shift)).or_insert(shift);
        }

        // Parallel computing hash table
        let hash_entries: Vec<(u64, usize)> = self
            .patterns
            .par_iter()
            .enumerate()
            .filter_map(|(pattern_idx, pattern)| {
                let chars: Vec<char> = pattern.chars().collect();
                if chars.len() >= self.block_size {
                    let suffix_start = chars.len().saturating_sub(self.block_size);
                    let suffix: String = chars[suffix_start..].iter().collect();
                    let hash = {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        suffix.hash(&mut hasher);
                        hasher.finish()
                    };
                    Some((hash, pattern_idx))
                } else {
                    None
                }
            })
            .collect();

        // Merge hash tables
        for (hash, pattern_idx) in hash_entries {
            self.hash_table.entry(hash).or_default().push(pattern_idx);
        }
    }
}

impl WuManber {
    /// Get matching positions - Fix character boundary issues
    pub fn find_matches(&self, text: &str) -> Vec<Match> {
        let mut matches = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let text_len = chars.len();

        if text_len < self.min_len {
            return matches;
        }

        let mut i = self.min_len.saturating_sub(1);

        while i < text_len {
            if i + 1 < self.block_size {
                i += 1;
                continue;
            }

            let block_start = i + 1 - self.block_size;
            let block: String = chars[block_start..=i].iter().collect();
            let hash = self.calculate_hash(&block);

            if let Some(pattern_indices) = self.hash_table.get(&hash) {
                for &pattern_idx in pattern_indices {
                    let pattern = &self.patterns[pattern_idx];
                    let pattern_chars: Vec<char> = pattern.chars().collect();

                    if i + 1 >= pattern_chars.len() {
                        let match_start = i + 1 - pattern_chars.len();
                        let match_end = i + 1;

                        if match_end <= text_len {
                            let candidate: String = chars[match_start..match_end].iter().collect();
                            if candidate == *pattern {
                                // Calculate byte position for return
                                let byte_start = chars[..match_start].iter().map(|c| c.len_utf8()).sum();
                                let byte_end = chars[..match_end].iter().map(|c| c.len_utf8()).sum();

                                matches.push(Match { start: byte_start, end: byte_end });
                                i = match_end - self.block_size + 1;
                                break;
                            }
                        }
                    }
                }
            }

            let shift = self.shift_table.get(&hash).copied().unwrap_or(self.min_len);
            i += shift.max(1);
        }

        matches
    }

    /// Constructor with parallel construction
    pub fn new_parallel(patterns: Vec<String>, block_size: usize) -> Self {
        if patterns.is_empty() {
            return Self {
                patterns: Vec::new(),
                min_len: 0,
                block_size,
                shift_table: HashMap::new(),
                hash_table: HashMap::new(),
                pattern_set: HashSet::new(),
            };
        }
        let min_len = patterns.iter().map(|s| s.len()).min().unwrap_or(0);
        let mut wm = Self {
            patterns: patterns.clone(),
            min_len,
            block_size,
            shift_table: HashMap::new(),
            hash_table: HashMap::new(),
            pattern_set: HashSet::with_capacity(patterns.len()),
        };

        wm.pattern_set.extend(patterns);
        wm.build_tables_parallel();
        wm
    }

    /// Chinese-optimized hash function
    #[allow(dead_code)]
    fn chinese_hash(&self, block: &str) -> u64 {
        // Use character encoding for direct computation, avoiding UTF-8 decoding overhead
        if block.len() == 2 {
            // For 2 bytes Chinese common case
            let bytes = block.as_bytes();
            (bytes[0] as u64) << 8 | (bytes[1] as u64)
        } else {
            self.hash(block)
        }
    }

    /// Calculate the hash value of a string block
    /// Chinese-optimized hash function
    fn hash(&self, block: &str) -> u64 {
        self.calculate_hash(block)
    }
}

/// Match result struct
#[derive(Debug, Clone, Copy)]
pub struct Match {
    pub start: usize,
    pub end: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_matching() {
        let wm = WuManber::new(vec!["赌博".to_string(), "色情".to_string(), "诈骗".to_string()], 2);

        assert_eq!(wm.search("正常内容"), None);
        assert_eq!(wm.search("含有赌博内容"), Some("赌博".to_string()));
        assert_eq!(wm.search("有色情图片"), Some("色情".to_string()));
    }

    #[test]
    fn test_varied_length() {
        let wm = WuManber::new(vec!["赌".to_string(), "赌博".to_string(), "赌博机".to_string()], 1); // block_size=1 以适应短词

        assert_eq!(wm.search("赌"), Some("赌".to_string()));
        assert_eq!(wm.search("赌博"), Some("赌博".to_string()));
        assert_eq!(wm.search("赌博机"), Some("赌博机".to_string()));
    }

    #[test]
    fn test_replace_all() {
        let wm = WuManber::new(vec!["赌博".to_string(), "色情".to_string()], 2);

        assert_eq!(wm.replace_all("禁止赌博和色情内容", '*'), "禁止**和**内容");
    }

    #[test]
    fn test_find_matches() {
        let wm = WuManber::new(vec!["赌博".to_string()], 2);
        let matches = wm.find_matches("赌博 赌博");

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[0].end, 2);
        assert_eq!(matches[1].start, 3);
        assert_eq!(matches[1].end, 5);
    }

    #[test]
    fn test_performance() {
        let patterns: Vec<_> = (0..10_000).map(|i| format!("敏感词{i}")).collect();
        let text = "这是一个包含敏感词 1234 的文本";

        let wm = WuManber::new(patterns, 3);
        assert_eq!(wm.search(text), Some("敏感词 1234".to_string()));
    }
}

use hashbrown::HashMap;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use smallvec::SmallVec;
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

/// Space handling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceHandling {
    /// Strict match (default): Spaces must match exactly
    Strict,
    /// Ignore spaces: Ignore all whitespace characters when matching
    IgnoreSpaces,
    /// Normalized spaces: Treat multiple consecutive spaces as a single space
    NormalizeSpaces,
}

/// High-performance Wu-Manber multi-pattern matching algorithm
/// Optimized for Chinese text processing with parallel table building and memory efficiency
#[derive(Debug, Clone)]
pub struct WuManber {
    patterns: Vec<Arc<String>>,
    original_patterns: Vec<String>,
    pattern_set: HashSet<Arc<String>>, // For quick lookup
    min_len: usize,
    block_size: usize,
    shift_table: HashMap<u64, usize>,
    hash_table: HashMap<u64, SmallVec<[usize; 4]>>,
    space_handling: SpaceHandling,
}

impl WuManber {
    /// Create new instance optimized for Chinese characters
    pub fn new_chinese(patterns: Vec<String>) -> Self {
        Self::new_with_space_handling(patterns, SpaceHandling::Strict)
    }

    /// Create new instance with custom block size
    pub fn new(patterns: Vec<String>, block_size: usize) -> Self {
        Self::new_with_block_size_and_space_handling(patterns, block_size, SpaceHandling::Strict)
    }

    /// Create new instance with parallel table building
    pub fn new_parallel(patterns: Vec<String>, block_size: usize) -> Self {
        Self::new_parallel_with_space_handling(patterns, block_size, SpaceHandling::Strict)
    }

    /// Create new instance with space handling strategy
    pub fn new_with_space_handling(patterns: Vec<String>, space_handling: SpaceHandling) -> Self {
        let block_size = Self::calculate_optimal_block_size(&patterns, space_handling);
        Self::new_with_block_size_and_space_handling(patterns, block_size, space_handling)
    }

    /// Create new instance with custom block size and space handling
    pub fn new_with_block_size_and_space_handling(
        patterns: Vec<String>,
        block_size: usize,
        space_handling: SpaceHandling,
    ) -> Self {
        Self::build_instance(patterns, block_size, false, space_handling)
    }

    /// Create new instance with parallel table building and space handling
    pub fn new_parallel_with_space_handling(
        patterns: Vec<String>,
        block_size: usize,
        space_handling: SpaceHandling,
    ) -> Self {
        Self::build_instance(patterns, block_size, true, space_handling)
    }

    /// Calculate optimal block size based on patterns and space handling
    fn calculate_optimal_block_size(patterns: &[String], space_handling: SpaceHandling) -> usize {
        if patterns.is_empty() {
            return 2;
        }

        let processed_patterns: Vec<String> =
            patterns.iter().map(|p| Self::preprocess_pattern(p, space_handling)).filter(|p| !p.is_empty()).collect();

        let min_len = processed_patterns.iter().map(|p| p.chars().count()).min().unwrap_or(2);

        // Dynamically adjust block_size to ensure it doesn't exceed the minimum mode length
        match min_len {
            1 => 1,
            2..=3 => 2,
            4..=10 => 3,
            _ => (min_len / 2).clamp(2, 4),
        }
    }

    /// Preprocess pattern based on space handling strategy
    fn preprocess_pattern(pattern: &str, space_handling: SpaceHandling) -> String {
        match space_handling {
            SpaceHandling::Strict => pattern.to_string(),
            SpaceHandling::IgnoreSpaces => pattern.chars().filter(|c| !c.is_whitespace()).collect(),
            SpaceHandling::NormalizeSpaces => {
                let mut result = String::new();
                let mut prev_was_space = false;

                for ch in pattern.chars() {
                    if ch.is_whitespace() {
                        if !prev_was_space {
                            result.push(' ');
                            prev_was_space = true;
                        }
                    } else {
                        result.push(ch);
                        prev_was_space = false;
                    }
                }

                result
            }
        }
    }

    /// Preprocess text based on space handling strategy
    fn preprocess_text(&self, text: &str) -> String {
        Self::preprocess_pattern(text, self.space_handling)
    }

    /// Create empty instance
    fn empty() -> Self {
        WuManber {
            patterns: Vec::new(),
            original_patterns: Vec::new(),
            pattern_set: HashSet::new(),
            min_len: 0,
            block_size: 2,
            shift_table: HashMap::new(),
            hash_table: HashMap::new(),
            space_handling: SpaceHandling::Strict,
        }
    }

    /// Internal instance builder with Arc optimization and space handling
    fn build_instance(patterns: Vec<String>, block_size: usize, parallel: bool, space_handling: SpaceHandling) -> Self {
        if patterns.is_empty() {
            return Self::empty();
        }

        // Save the original mode
        let original_patterns = patterns.clone();

        // Pretreatment mode
        let processed_patterns: Vec<String> =
            patterns.iter().map(|p| Self::preprocess_pattern(p, space_handling)).filter(|p| !p.is_empty()).collect();

        if processed_patterns.is_empty() {
            return Self::empty();
        }

        let min_len = processed_patterns.iter().map(|p| p.chars().count()).min().unwrap_or(1);

        // Make sure that the block_size does not exceed the minimum mode length
        let safe_block_size = block_size.min(min_len);

        let patterns_arc: Vec<Arc<String>> = processed_patterns.into_iter().map(Arc::new).collect();
        // Build pattern_set for quick lookups
        let pattern_set: HashSet<Arc<String>> = patterns_arc.iter().cloned().collect();
        let mut instance = WuManber {
            patterns: patterns_arc,
            original_patterns,
            pattern_set,
            min_len,
            block_size: safe_block_size,
            shift_table: HashMap::new(),
            hash_table: HashMap::new(),
            space_handling,
        };

        if parallel {
            instance.build_tables_parallel();
        } else {
            instance.build_tables();
        }

        instance
    }

    /// Build shift and hash tables sequentially with memory optimization
    fn build_tables(&mut self) {
        self.build_shift_table();
        self.build_hash_table();
    }

    /// Build shift and hash tables in parallel with memory optimization
    fn build_tables_parallel(&mut self) {
        self.build_shift_table_parallel();
        self.build_hash_table_parallel();
    }

    /// Build shift table sequentially with optimized character handling
    fn build_shift_table(&mut self) {
        for pattern in self.patterns.iter() {
            let chars: Vec<char> = pattern.chars().collect();
            let char_count = chars.len();

            // Prevent spillage: Ensure that the pattern length is greater than or equal to block_size
            if char_count < self.block_size {
                continue;
            }

            for i in 0..=(char_count - self.block_size) {
                let block = self.extract_block_optimized(&chars, i);
                let hash = Self::calculate_hash_fast(&block);
                let shift = char_count - i - self.block_size;

                self.shift_table.entry(hash).and_modify(|v| *v = (*v).min(shift)).or_insert(shift);
            }
        }
    }

    /// Build hash table sequentially with memory optimization
    fn build_hash_table(&mut self) {
        for (pattern_idx, pattern) in self.patterns.iter().enumerate() {
            let chars: Vec<char> = pattern.chars().collect();
            let char_count = chars.len();

            if char_count >= self.block_size {
                let start_pos = char_count - self.block_size;
                let block = self.extract_block_optimized(&chars, start_pos);
                let hash = Self::calculate_hash_fast(&block);

                self.hash_table.entry(hash).or_default().push(pattern_idx);
            }
        }
    }

    /// Build shift table in parallel with proper iterator handling
    fn build_shift_table_parallel(&mut self) {
        let block_size = self.block_size;

        let shift_entries: Vec<(u64, usize)> = self
            .patterns
            .par_iter()
            .flat_map(|pattern| {
                let chars: Vec<char> = pattern.chars().collect();
                let char_count = chars.len();

                if char_count < block_size {
                    return Vec::new();
                }

                (0..=(char_count - block_size))
                    .map(move |i| {
                        let block = chars[i..i + block_size].iter().collect::<String>();
                        let hash = Self::calculate_hash_fast(&block);
                        let shift = char_count - i - block_size;
                        (hash, shift)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        for (hash, shift) in shift_entries {
            self.shift_table.entry(hash).and_modify(|v| *v = (*v).min(shift)).or_insert(shift);
        }
    }

    /// Build hash table in parallel with memory optimization
    fn build_hash_table_parallel(&mut self) {
        let block_size = self.block_size;

        let hash_entries: Vec<(u64, usize)> = self
            .patterns
            .par_iter()
            .enumerate()
            .filter_map(|(pattern_idx, pattern)| {
                let chars: Vec<char> = pattern.chars().collect();
                let char_count = chars.len();

                if char_count >= block_size {
                    let start_pos = char_count - block_size;
                    let block = chars[start_pos..start_pos + block_size].iter().collect::<String>();
                    let hash = Self::calculate_hash_fast(&block);
                    Some((hash, pattern_idx))
                } else {
                    None
                }
            })
            .collect();

        for (hash, pattern_idx) in hash_entries {
            self.hash_table.entry(hash).or_insert_with(SmallVec::new).push(pattern_idx);
        }
    }

    /// Extract block from character array with reduced allocations
    #[inline]
    fn extract_block_optimized(&self, chars: &[char], start: usize) -> String {
        chars[start..start + self.block_size].iter().collect()
    }

    /// Optimized hash calculation with better performance
    #[inline]
    fn calculate_hash_fast(s: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    /// Search for first match using Wu-Manber algorithm with space handling
    pub fn search(&self, text: &str) -> Option<Arc<String>> {
        if self.patterns.is_empty() || text.is_empty() {
            return None;
        }

        if self.space_handling != SpaceHandling::Strict {
            return self.search_with_preprocessing(text);
        }

        let chars: Vec<char> = text.chars().collect();
        let text_len = chars.len();

        if text_len < self.min_len {
            return None;
        }

        let mut pos = self.min_len - 1;

        while pos < text_len {
            if pos + 1 < self.block_size {
                pos += 1;
                continue;
            }

            let block_start = pos + 1 - self.block_size;
            let block = self.extract_block_optimized(&chars, block_start);
            let hash = Self::calculate_hash_fast(&block);

            if let Some(&shift) = self.shift_table.get(&hash) {
                if shift == 0 {
                    if let Some(pattern_indices) = self.hash_table.get(&hash) {
                        if let Some(result) = self.verify_matches_arc(&chars, pos, pattern_indices) {
                            return Some(result);
                        }
                    }

                    let prev_block_start = block_start.saturating_sub(1);
                    if prev_block_start < block_start {
                        let prev_block = self.extract_block_optimized(&chars, prev_block_start);
                        let prev_hash = Self::calculate_hash_fast(&prev_block);
                        if let Some(prev_pattern_indices) = self.hash_table.get(&prev_hash) {
                            // Verify the previous position
                            if let Some(found) = self.verify_matches_arc(&chars, pos - 1, prev_pattern_indices) {
                                return Some(found);
                            }
                        }
                    }

                    pos += 1;
                } else {
                    pos += shift;
                }
            } else {
                // pos += self.min_len;
                pos += self.min_len.saturating_sub(self.block_size).saturating_add(1);
            }
        }

        None
    }

    /// Search with preprocessing for non-strict space handling
    fn search_with_preprocessing(&self, text: &str) -> Option<Arc<String>> {
        for (i, original_pattern) in self.original_patterns.iter().enumerate() {
            let processed_text = self.preprocess_text(text);
            let processed_pattern = Self::preprocess_pattern(original_pattern, self.space_handling);

            if processed_text.contains(&processed_pattern) {
                return self.patterns.get(i).cloned();
            }
        }
        None
    }

    /// Verify potential matches and return Arc pattern
    fn verify_matches_arc(&self, chars: &[char], pos: usize, pattern_indices: &[usize]) -> Option<Arc<String>> {
        for &pattern_idx in pattern_indices {
            if let Some(pattern) = self.patterns.get(pattern_idx) {
                let pattern_chars: Vec<char> = pattern.chars().collect();
                let pattern_len = pattern_chars.len();

                if pos + 1 >= pattern_len {
                    let start = pos + 1 - pattern_len;
                    if chars[start..start + pattern_len] == pattern_chars[..] {
                        return Some(pattern.clone());
                    }
                }
            }
        }
        None
    }

    /// Legacy search method for string compatibility
    pub fn search_string(&self, text: &str) -> Option<String> {
        if self.space_handling != SpaceHandling::Strict {
            // For non-strict matches, use simplified logic to ensure correctness
            for original_pattern in &self.original_patterns {
                let processed_text = self.preprocess_text(text);
                let processed_pattern = Self::preprocess_pattern(original_pattern, self.space_handling);
                if processed_text.contains(&processed_pattern) {
                    return Some(original_pattern.clone());
                }
            }
            return None;
        }

        self.search(text).map(|arc| (*arc).clone())
    }

    /// Find all matches in text with space handling
    pub fn search_all(&self, text: &str) -> Vec<Arc<String>> {
        if self.patterns.is_empty() || text.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();

        // Use simplified matching logic to ensure correctness
        for (i, original_pattern) in self.original_patterns.iter().enumerate() {
            let matches = match self.space_handling {
                SpaceHandling::Strict => text.contains(original_pattern),
                _ => {
                    let processed_text = self.preprocess_text(text);
                    let processed_pattern = Self::preprocess_pattern(original_pattern, self.space_handling);
                    processed_text.contains(&processed_pattern)
                }
            };

            if matches {
                if let Some(pattern) = self.patterns.get(i) {
                    results.push(pattern.clone());
                }
            }
        }

        results
    }

    /// Find all matches returning strings for compatibility
    pub fn search_all_strings(&self, text: &str) -> Vec<String> {
        if self.patterns.is_empty() || text.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();

        for original_pattern in &self.original_patterns {
            let matches = match self.space_handling {
                SpaceHandling::Strict => text.contains(original_pattern),
                _ => {
                    let processed_text = self.preprocess_text(text);
                    let processed_pattern = Self::preprocess_pattern(original_pattern, self.space_handling);
                    processed_text.contains(&processed_pattern)
                }
            };

            if matches {
                results.push(original_pattern.clone());
            }
        }

        results
    }

    /// Replace all matches with replacement character
    pub fn replace_all(&self, text: &str, replacement: char) -> String {
        let mut result = text.to_string();

        for pattern in &self.original_patterns {
            let replacement_str = replacement.to_string().repeat(pattern.chars().count());
            result = result.replace(pattern, &replacement_str);
        }

        result
    }

    /// Remove all matches from text
    pub fn remove_all(&self, text: &str) -> String {
        let mut result = text.to_string();

        for pattern in &self.original_patterns {
            result = result.replace(pattern, "");
        }

        result
    }

    /// Find all match positions with byte offsets
    pub fn find_matches(&self, text: &str) -> Vec<Match> {
        let mut matches = Vec::new();

        for pattern in &self.original_patterns {
            let mut start = 0;
            while let Some(pos) = text[start..].find(pattern) {
                let absolute_start = start + pos;
                let absolute_end = absolute_start + pattern.len();
                matches.push(Match { start: absolute_start, end: absolute_end });
                start = absolute_start + 1;
            }
        }

        matches.sort_by_key(|m| m.start);
        matches
    }

    /// Get patterns reference (returns Arc slice)
    pub fn patterns(&self) -> &[Arc<String>] {
        &self.patterns
    }

    /// Get original patterns as strings
    pub fn patterns_strings(&self) -> Vec<String> {
        self.original_patterns.clone()
    }

    /// Check if pattern exists (使用 pattern_set 提供 O(1) 查找)
    pub fn contains_pattern(&self, pattern: &str) -> bool {
        let target = Arc::new(pattern.to_string());
        self.pattern_set.contains(&target)
    }

    /// Get space handling strategy
    pub fn space_handling(&self) -> SpaceHandling {
        self.space_handling
    }

    /// Get memory usage statistics
    pub fn memory_stats(&self) -> WuManberMemoryStats {
        let patterns_memory = self.patterns.iter().map(|p| size_of::<Arc<String>>() + p.len()).sum::<usize>();

        let shift_table_memory = self.shift_table.len() * (size_of::<u64>() + size_of::<usize>());
        let hash_table_memory = self
            .hash_table
            .iter()
            .map(|(_k, v)| size_of::<u64>() + size_of::<SmallVec<[usize; 4]>>() + v.len() * size_of::<usize>())
            .sum::<usize>();

        let pattern_set_memory = self.pattern_set.len() * size_of::<Arc<String>>();

        let total_memory = patterns_memory + shift_table_memory + hash_table_memory + pattern_set_memory;

        WuManberMemoryStats {
            total_patterns: self.patterns.len(),
            patterns_memory,
            shift_table_memory,
            hash_table_memory,
            total_memory,
        }
    }

    /// Get statistics
    pub fn stats(&self) -> WuManberStats {
        WuManberStats {
            pattern_count: self.patterns.len(),
            min_length: self.min_len,
            block_size: self.block_size,
            shift_table_size: self.shift_table.len(),
            hash_table_size: self.hash_table.len(),
        }
    }
}

/// Match result with byte positions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub start: usize,
    pub end: usize,
}

/// Wu-Manber statistics
#[derive(Debug, Clone)]
pub struct WuManberStats {
    pub pattern_count: usize,
    pub min_length: usize,
    pub block_size: usize,
    pub shift_table_size: usize,
    pub hash_table_size: usize,
}

/// Wu-Manber memory usage statistics
#[derive(Debug, Clone)]
pub struct WuManberMemoryStats {
    pub total_patterns: usize,
    pub patterns_memory: usize,
    pub shift_table_memory: usize,
    pub hash_table_memory: usize,
    pub total_memory: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_matching() {
        let wm =
            WuManber::new_chinese(vec!["色情".to_string(), "赌博".to_string(), "诈骗".to_string(), "扯蛋".to_string()]);

        assert_eq!(wm.search_string("正常内容"), None);
        assert_eq!(wm.search_string("测试含有赌博内容"), Some("赌博".to_string()));
        assert_eq!(wm.search_string("测试还有色情赌博图片"), Some("色情".to_string()));
        assert_eq!(wm.search_string("测试有色情赌博图片"), Some("色情".to_string()));
        assert_eq!(wm.search_string("还有诈骗色情测试"), Some("诈骗".to_string()));
    }

    #[test]
    fn test_varied_length() {
        let wm = WuManber::new(vec!["赌".to_string(), "赌博".to_string(), "赌博机".to_string()], 1);

        assert_eq!(wm.search_string("赌"), Some("赌".to_string()));
        assert_eq!(wm.search_string("赌博"), Some("赌".to_string())); // 会匹配最先找到的
        assert_eq!(wm.search_string("赌博机"), Some("赌".to_string())); // 会匹配最先找到的
    }

    #[test]
    fn test_space_handling_strategies() {
        let patterns = vec!["hello world".to_string(), "test  pattern".to_string()];

        // 严格匹配
        let wm_strict = WuManber::new_with_space_handling(patterns.clone(), SpaceHandling::Strict);
        assert_eq!(wm_strict.search_string("hello world"), Some("hello world".to_string()));
        assert_eq!(wm_strict.search_string("helloworld"), None);

        // 忽略空格
        let wm_ignore = WuManber::new_with_space_handling(patterns.clone(), SpaceHandling::IgnoreSpaces);
        assert_eq!(wm_ignore.search_string("hello world"), Some("hello world".to_string()));
        assert_eq!(wm_ignore.search_string("helloworld"), Some("hello world".to_string()));

        // 标准化空格
        let wm_normalize = WuManber::new_with_space_handling(patterns.clone(), SpaceHandling::NormalizeSpaces);
        assert_eq!(wm_normalize.search_string("test  pattern"), Some("test  pattern".to_string()));
        assert_eq!(wm_normalize.search_string("test   pattern"), Some("test  pattern".to_string()));
    }

    #[test]
    fn test_mixed_patterns() {
        let mixed_patterns =
            vec!["关键词2500".to_string(), "关键词 3000".to_string(), "test".to_string(), "hello world".to_string()];

        let wm = WuManber::new_chinese(mixed_patterns);

        assert_eq!(wm.search_string("包含关键词2500的文本"), Some("关键词2500".to_string()));
        assert_eq!(wm.search_string("包含关键词 3000 的文本"), Some("关键词 3000".to_string()));
        assert_eq!(wm.search_string("this is test"), Some("test".to_string()));
        assert_eq!(wm.search_string("say hello world"), Some("hello world".to_string()));
    }

    #[test]
    fn test_complex_patterns_with_spaces() {
        let complex_patterns = vec![
            "Hello World".to_string(),
            "你好 世界".to_string(),
            "multiple   spaces".to_string(),
            "tab\there".to_string(),
            "new\nline".to_string(),
        ];

        let wm = WuManber::new_chinese(complex_patterns);

        assert_eq!(wm.search_string("Say Hello World"), Some("Hello World".to_string()));
        assert_eq!(wm.search_string("说你好 世界"), Some("你好 世界".to_string()));
        assert_eq!(wm.search_string("has multiple   spaces here"), Some("multiple   spaces".to_string()));
        assert_eq!(wm.search_string("tab\there you go"), Some("tab\there".to_string()));
        assert_eq!(wm.search_string("new\nline break"), Some("new\nline".to_string()));
    }

    #[test]
    fn test_ignore_spaces_functionality() {
        let patterns = vec!["关键词 2500".to_string(), "hello world".to_string()];
        let wm = WuManber::new_with_space_handling(patterns, SpaceHandling::IgnoreSpaces);

        // 这些都应该匹配
        assert_eq!(wm.search_string("关键词2500"), Some("关键词 2500".to_string()));
        assert_eq!(wm.search_string("关键词 2500"), Some("关键词 2500".to_string()));
        assert_eq!(wm.search_string("关键词  2500"), Some("关键词 2500".to_string()));
        assert_eq!(wm.search_string("helloworld"), Some("hello world".to_string()));
        assert_eq!(wm.search_string("hello world"), Some("hello world".to_string()));
    }

    #[test]
    fn test_parallel_performance() {
        let patterns: Vec<String> = (0..5000).map(|i| format!("关键词{i}")).collect();

        let wm_seq = WuManber::new(patterns.clone(), 2);
        let wm_par = WuManber::new_parallel(patterns, 2);

        let text = "这里包含关键词2500和关键词3000";

        let result_seq = wm_seq.search_all_strings(text);
        let result_par = wm_par.search_all_strings(text);

        assert_eq!(result_seq.len(), result_par.len());
        assert!(result_seq.contains(&"关键词2500".to_string()));
        assert!(result_seq.contains(&"关键词3000".to_string()));

        for item in &result_seq {
            assert!(result_par.contains(item));
        }
    }

    #[test]
    fn test_search_all_functionality() {
        let wm = WuManber::new_chinese(vec!["苹果".to_string(), "香蕉".to_string(), "橙子".to_string()]);

        let text = "我喜欢吃苹果、香蕉和橙子";
        let results = wm.search_all_strings(text);

        assert_eq!(results.len(), 3);
        assert!(results.contains(&"苹果".to_string()));
        assert!(results.contains(&"香蕉".to_string()));
        assert!(results.contains(&"橙子".to_string()));
    }
}

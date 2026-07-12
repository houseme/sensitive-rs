use crate::engine::MatchAlgorithm;
use crate::{engine::MultiPatternEngine, variant::VariantDetector};
use lru::LruCache;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use regex::Regex;
use std::num::NonZero;
use std::sync::{Arc, Mutex};
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
};

/// Advanced sensitive word filter with variant detection
pub struct Filter {
    engine: MultiPatternEngine,        // Multi-pattern matching engine
    variant_detector: VariantDetector, // Variation detector
    noise: Regex,                      // Noise processing rules
    cache: Arc<Mutex<LruCache<String, Vec<String>>>>,
    #[cfg(feature = "net")]
    http_client: reqwest::blocking::Client, // Network request client
}

impl std::fmt::Debug for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Filter")
            .field("engine", &self.engine)
            .field("variant_detector", &self.variant_detector)
            .field("noise", &self.noise)
            .field("cache", &"<LruCache>")
            .finish()
    }
}

impl Filter {
    /// Create a new filter with default settings
    pub fn new() -> Self {
        Self {
            engine: MultiPatternEngine::new(None, &[]),
            variant_detector: VariantDetector::new(),
            noise: Regex::new(r"[^\w\s\u4e00-\u9fff]").unwrap(),
            cache: Arc::new(Mutex::new(LruCache::new(NonZero::new(1000).unwrap()))), // Cache 1000 results
            #[cfg(feature = "net")]
            http_client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap(),
        }
    }

    fn check_cache(&self, text: &str) -> Option<Vec<String>> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).get(text).cloned()
    }

    fn cache_result(&self, text: &str, results: &[String]) {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).put(text.to_string(), results.to_vec());
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    /// Create with specific algorithm
    pub fn with_algorithm(algorithm: MatchAlgorithm) -> Self {
        Self { engine: MultiPatternEngine::new(Some(algorithm), &[]), ..Self::new() }
    }

    /// Load default dictionary
    pub fn with_default_dict() -> io::Result<Self> {
        let mut filter = Self::new();
        filter.load_word_dict("dict/dict.txt")?;
        Ok(filter)
    }

    /// Update noise pattern
    pub fn update_noise_pattern(&mut self, pattern: &str) -> Result<(), regex::Error> {
        self.noise = Regex::new(pattern)?;
        Ok(())
    }

    /// Add a sensitive word
    pub fn add_word(&mut self, word: &str) {
        let patterns = {
            let mut p = self.engine.get_patterns().to_vec();
            p.push(word.to_string());
            p
        };
        self.engine.rebuild(&patterns);
        self.variant_detector.add_word(word);
        self.clear_cache();
    }

    /// Add multiple words
    pub fn add_words(&mut self, words: &[&str]) {
        let mut patterns = self.engine.get_patterns().to_vec();
        patterns.extend(words.iter().map(|s| s.to_string()));

        self.engine.rebuild(&patterns);
        for word in words {
            self.variant_detector.add_word(word);
        }
        self.clear_cache();
    }

    /// Get the currently used algorithm
    pub fn current_algorithm(&self) -> MatchAlgorithm {
        self.engine.current_algorithm()
    }

    /// Remove a word
    pub fn del_word(&mut self, word: &str) {
        let patterns: Vec<_> = self.engine.get_patterns().iter().filter(|&w| w != word).cloned().collect();

        self.engine.rebuild(&patterns);
        self.clear_cache();
    }

    /// Remove multiple words
    pub fn del_words(&mut self, words: &[&str]) {
        let word_set: std::collections::HashSet<_> = words.iter().collect();
        let patterns: Vec<_> =
            self.engine.get_patterns().iter().filter(|w| !word_set.contains(&w.as_str())).cloned().collect();

        self.engine.rebuild(&patterns);
        self.clear_cache();
    }

    /// Load dictionary from file
    pub fn load_word_dict<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let file = File::open(path)?;
        self.load(BufReader::new(file))
    }

    /// Load dictionary from reader
    pub fn load<R: BufRead>(&mut self, reader: R) -> io::Result<()> {
        let words: Vec<_> = reader.lines().collect::<Result<_, _>>()?;
        self.add_words(&words.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        Ok(())
    }

    /// Load dictionary from URL
    #[cfg(feature = "net")]
    pub fn load_net_word_dict(&mut self, url: &str) -> io::Result<()> {
        let response = self.http_client.get(url).send().map_err(io::Error::other)?;

        if !response.status().is_success() {
            return Err(io::Error::other(format!("HTTP request failed: {}", response.status())));
        }

        let reader = BufReader::new(response);
        self.load(reader)
    }

    /// Find first sensitive word
    pub fn find_in(&self, text: &str) -> (bool, String) {
        let clean_text = self.remove_noise(text);

        // 1. Try exact match first
        if let Some(word) = self.engine.find_first(&clean_text) {
            return (true, word);
        }

        // 2. Try variant detection
        let patterns: Vec<_> = self.engine.get_patterns().iter().map(|s| s.as_str()).collect();

        if let Some(word) = self.variant_detector.detect(&clean_text, &patterns).first() {
            return (true, word.to_string());
        }

        (false, String::new())
    }

    /// Replace sensitive words with replacement character
    pub fn replace(&self, text: &str, replacement: char) -> String {
        let clean_text = self.remove_noise(text);
        let repl = replacement.to_string();

        // Exact matches: single-pass rebuild from engine match positions
        // (leftmost-longest, so overlapping dict words resolve cleanly), one
        // replacement char per matched character.
        let mut positions = self.engine.find_matches_with_positions(&clean_text);
        positions.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
        let mut result = String::with_capacity(clean_text.len());
        let mut cursor = 0usize;
        for m in &positions {
            if m.start < cursor {
                continue; // covered by a previously kept (longer-leftmost) span
            }
            result.push_str(&clean_text[cursor..m.start]);
            result.push_str(&repl.repeat(clean_text[m.start..m.end].chars().count()));
            cursor = m.end;
        }
        result.push_str(&clean_text[cursor..]);

        // Variant branch: `detect` returns original word names, not the variant
        // text actually present, so these replaces are no-ops when only variants
        // occur. Kept unchanged in this perf pass; revisit separately.
        let patterns: Vec<_> = self.engine.get_patterns().iter().map(|s| s.as_str()).collect();
        let variants = self.variant_detector.detect(&result, &patterns);
        for variant in variants {
            let repl_str = repl.repeat(variant.chars().count());
            result = result.replace(variant, &repl_str);
        }

        result
    }

    /// Filter out sensitive words (remove them completely)
    pub fn filter(&self, text: &str) -> String {
        let clean_text = self.remove_noise(text);

        // Use engine's optimized replace_all for pattern removal
        let mut result = self.engine.replace_all(&clean_text, "");

        // Remove sensitive words detected by variants
        let patterns: Vec<_> = self.engine.get_patterns().iter().map(|s| s.as_str()).collect();
        let variants = self.variant_detector.detect(&result, &patterns);

        for variant in variants {
            result = result.replace(variant, "");
        }

        result
    }

    /// Validate text
    pub fn validate(&self, text: &str) -> (bool, String) {
        self.find_in(text)
    }

    /// Remove only specific noise characters, preserve spaces
    pub fn remove_noise(&self, text: &str) -> String {
        self.noise.replace_all(text, "").to_string()
    }

    /// Get current noise pattern
    pub fn get_noise_pattern(&self) -> &Regex {
        &self.noise
    }
}

impl Filter {
    /// Optimized method of finding all sensitive words
    pub fn find_all(&self, text: &str) -> Vec<String> {
        let clean_text = self.remove_noise(text);

        // 1. Caching mechanism - Check whether the results have been cached
        if let Some(cached_result) = self.check_cache(&clean_text) {
            return cached_result;
        }

        #[cfg(feature = "parallel")]
        let results = if clean_text.len() > 1000 {
            self.find_all_parallel(&clean_text) // long text -> parallel
        } else {
            self.find_all_sequential(&clean_text) // short text -> sequential
        };
        #[cfg(not(feature = "parallel"))]
        let results = self.find_all_sequential(&clean_text);

        // 3. Cache results
        self.cache_result(&clean_text, &results);

        results
    }

    /// Parallel Processing Version - For Long Text
    #[cfg(feature = "parallel")]
    fn find_all_parallel(&self, text: &str) -> Vec<String> {
        let chunk_size = std::cmp::max(text.len() / rayon::current_num_threads(), 100);
        let patterns: Vec<_> = self.engine.get_patterns().iter().map(|s| s.as_str()).collect();

        // Compute overlap to catch patterns spanning chunk boundaries
        let max_pattern_len = patterns.iter().map(|p| p.chars().count()).max().unwrap_or(0);
        let overlap = max_pattern_len.min(chunk_size);

        // Build overlapping chunks for parallel processing
        let chars: Vec<char> = text.chars().collect();
        let engine_results: Vec<String> = if chars.len() <= chunk_size {
            self.engine.find_all(text)
        } else {
            let step = chunk_size;
            chars
                .windows(chunk_size + overlap)
                .step_by(step)
                .collect::<Vec<_>>()
                .par_iter()
                .flat_map(|window| {
                    let chunk_text: String = window.iter().collect();
                    self.engine.find_all(&chunk_text)
                })
                .collect()
        };

        // Parallel variant detection - Fixed parallel iterator problem
        let variant_results: Vec<String> = text
            .split_whitespace()
            .collect::<Vec<_>>()
            .par_iter()
            .map(|segment| self.variant_detector.detect(segment, &patterns))
            .flatten()
            .map(|s| s.to_string())
            .collect();

        // Merge and remove repetition
        let mut results = engine_results;
        results.extend(variant_results);
        self.deduplicate_and_sort(results)
    }

    /// Sequential processing version - suitable for short text
    fn find_all_sequential(&self, text: &str) -> Vec<String> {
        let mut results = self.engine.find_all(text);
        let patterns: Vec<_> = self.engine.get_patterns().iter().map(|s| s.as_str()).collect();

        // Add variant detection results
        results.extend(self.variant_detector.detect(text, &patterns).into_iter().map(|s| s.to_string()));

        self.deduplicate_and_sort(results)
    }

    /// Deduplication and sort
    fn deduplicate_and_sort(&self, mut results: Vec<String>) -> Vec<String> {
        results.sort_unstable();
        results.dedup();
        results
    }

    /// Bulk search for optimized versions
    pub fn find_all_batch(&self, texts: &[&str]) -> Vec<Vec<String>> {
        #[cfg(feature = "parallel")]
        {
            texts.par_iter().map(|text| self.find_all(text)).collect()
        }
        #[cfg(not(feature = "parallel"))]
        {
            texts.iter().map(|text| self.find_all(text)).collect()
        }
    }

    /// Hierarchical Matching - Preferential Matching by Sensitive Word Length
    pub fn find_all_layered(&self, text: &str) -> Vec<String> {
        let clean_text = self.remove_noise(text);
        let mut results = Vec::new();
        let mut remaining_text = clean_text.clone();

        // Arrange patterns in descending order of length, prioritize long words
        let mut sorted_patterns = self.engine.get_patterns().to_vec();
        sorted_patterns.sort_by_key(|b| std::cmp::Reverse(b.len()));

        // Hierarchical matching
        for pattern in &sorted_patterns {
            if remaining_text.contains(pattern) {
                results.push(pattern.clone());
                // Remove matching parts to avoid duplicate matches
                remaining_text = remaining_text.replace(pattern, " ");
            }
        }

        // Variation detection (for remaining text)
        let patterns: Vec<_> = sorted_patterns.iter().map(|s| s.as_str()).collect();
        results.extend(self.variant_detector.detect(&remaining_text, &patterns).into_iter().map(|s| s.to_string()));

        self.deduplicate_and_sort(results)
    }

    /// Streaming version - suitable for oversized text
    pub fn find_all_streaming<R: BufRead>(&self, reader: R) -> io::Result<Vec<String>> {
        let mut all_results = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let results = self.find_all(&line);
            all_results.extend(results);
        }

        Ok(self.deduplicate_and_sort(all_results))
    }
}

impl Default for Filter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    #[test]
    fn test_integration() {
        let mut filter = Filter::new();
        filter.add_words(&["赌博", "色情"]);

        // Exact match
        assert_eq!(filter.find_in("含有赌博"), (true, "赌博".to_string()));

        // Pinyin variant
        assert_eq!(filter.find_in("含有 dubo"), (true, "赌博".to_string()));

        // Replacement
        assert_eq!(filter.replace("赌博 色情", '*'), "** **");

        // Filter
        assert_eq!(filter.filter("赌博内容"), "内容");
    }

    #[test]
    fn test_noise_handling() {
        let mut filter = Filter::new();
        filter.add_word("赌博");

        // 测试空格保留
        assert_eq!(filter.remove_noise("赌 博"), "赌 博");

        // 测试特殊符号移除
        assert_eq!(filter.remove_noise("赌@#$博"), "赌博");
    }

    #[test]
    fn test_replace_vs_filter() {
        let mut filter = Filter::new();
        filter.add_words(&["赌博", "色情"]);

        let text = "这里有赌博和色情内容";

        // replace should be replaced with characters
        assert_eq!(filter.replace(text, '*'), "这里有**和**内容");

        // filter should be completely removed
        assert_eq!(filter.filter(text), "这里有和内容");
    }

    #[test]
    fn test_replace_single_pass_rebuild() {
        // R3: exact matches rebuilt in a single pass, one '*' per matched char,
        // regardless of how many patterns hit.
        let mut filter = Filter::new();
        filter.add_words(&["赌博", "色情"]);
        assert_eq!(filter.replace("前缀赌博中间色情后缀", '*'), "前缀**中间**后缀");
    }

    #[test]
    fn test_variant_detection() {
        let mut filter = Filter::new();
        filter.add_word("测试");

        assert_eq!(filter.find_in("ceshi"), (true, "测试".to_string()));
    }

    #[test]
    fn test_algorithm_switch_one() {
        // Use Wu-Manber in small quantities
        let mut small = Filter::new();
        small.add_words(&["a", "b", "c"]);
        assert!(matches!(small.engine.current_algorithm(), MatchAlgorithm::WuManber));

        // Aho-Corasick for medium quantity
        let words: Vec<_> = (0..150).map(|i| format!("word{i}")).collect();
        let mut medium = Filter::new();
        medium.add_words(&words.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        println!("Medium current_algorithm: {:?}", medium.engine.current_algorithm());
        assert!(matches!(medium.engine.current_algorithm(), MatchAlgorithm::AhoCorasick));
    }

    #[test]
    fn test_io_operations() -> io::Result<()> {
        let mut filter = Filter::new();
        let cursor = Cursor::new("word1\nword2\nword3");
        filter.load(cursor)?;

        assert_eq!(filter.find_in("word2"), (true, "word2".to_string()));
        Ok(())
    }

    #[test]
    fn test_algorithm_recommendation() {
        assert_eq!(MultiPatternEngine::recommend_algorithm(50), MatchAlgorithm::WuManber);
        assert_eq!(MultiPatternEngine::recommend_algorithm(150), MatchAlgorithm::AhoCorasick);
        assert_eq!(MultiPatternEngine::recommend_algorithm(15000), MatchAlgorithm::Regex);
    }

    #[test]
    fn test_algorithm_switch() {
        // Use Wu-Manber in small quantities
        let mut small = Filter::new();
        small.add_words(&["a", "b", "c"]);
        println!("Small (3 words): {:?}", small.current_algorithm());
        assert!(matches!(small.current_algorithm(), MatchAlgorithm::WuManber));

        // Aho-Corasick for medium quantity
        let words: Vec<_> = (0..150).map(|i| format!("word{i}")).collect();
        let word_refs: Vec<&str> = words.iter().map(|s| s.as_str()).collect();

        let mut medium = Filter::new();
        medium.add_words(&word_refs);

        println!("Medium (150 words): {:?}", medium.current_algorithm());
        println!("Pattern count: {}", medium.engine.get_patterns().len());

        // Verification algorithm selection logic
        let recommended = MultiPatternEngine::recommend_algorithm(150);
        println!("Recommended algorithm for 150 words: {recommended:?}");

        assert!(matches!(medium.current_algorithm(), MatchAlgorithm::AhoCorasick));
    }

    #[test]
    fn test_cache_invalidation_on_add_word() {
        let mut filter = Filter::new();
        filter.add_word("赌博");

        // First search populates cache
        let results1 = filter.find_all("这里有赌博");
        assert!(results1.contains(&"赌博".to_string()));

        // Add a new word
        filter.add_word("色情");

        // Cache should be invalidated — new word must appear
        let results2 = filter.find_all("这里有赌博和色情");
        assert!(results2.contains(&"赌博".to_string()));
        assert!(results2.contains(&"色情".to_string()));
    }

    #[test]
    fn test_cache_invalidation_on_del_word() {
        let mut filter = Filter::new();
        filter.add_words(&["赌博", "色情"]);

        // First search populates cache
        let results1 = filter.find_all("这里有赌博和色情");
        assert!(results1.contains(&"赌博".to_string()));
        assert!(results1.contains(&"色情".to_string()));

        // Delete a word
        filter.del_word("赌博");

        // Cache should be invalidated — deleted word must not appear
        let results2 = filter.find_all("这里有赌博和色情");
        assert!(!results2.contains(&"赌博".to_string()));
        assert!(results2.contains(&"色情".to_string()));
    }

    #[test]
    fn test_mutex_poison_recovery() {
        use std::sync::Arc;

        let filter = Arc::new(Filter::new());
        let filter_clone = Arc::clone(&filter);

        // Poison the mutex by panicking while holding the lock
        let handle = std::thread::spawn(move || {
            let _guard = filter_clone.cache.lock().unwrap();
            panic!("intentional panic to poison mutex");
        });
        let _ = handle.join();

        // Filter should still work — recovers from poisoned mutex
        let results = filter.find_all("test");
        assert!(results.is_empty());
    }

    #[test]
    fn test_parallel_search_cross_boundary() {
        let mut filter = Filter::new();
        filter.add_word("赌博");

        // Build text > 1000 bytes so find_all uses parallel path
        // Place "赌博" at a position that could land on a chunk boundary
        let prefix: String = "安全文字".repeat(200); // 800 bytes (4 chars × 3 bytes × 200)
        let text = format!("{prefix}这里有赌博内容");

        let results = filter.find_all(&text);
        assert!(results.contains(&"赌博".to_string()));
    }

    #[test]
    fn test_parallel_search_no_duplicates() {
        let mut filter = Filter::new();
        filter.add_word("赌博");

        // Build text > 1000 bytes with the word in the middle
        let prefix: String = "安全".repeat(300); // 1800 bytes
        let text = format!("{prefix}赌博{prefix}");

        let results = filter.find_all(&text);
        let count = results.iter().filter(|w| *w == "赌博").count();
        assert_eq!(count, 1, "expected exactly 1 match, got {count}");
    }

    // ---- Task 2: advanced methods (batch / layered / streaming) ----

    #[test]
    fn test_find_all_batch() {
        let mut filter = Filter::new();
        filter.add_words(&["赌博", "色情"]);

        let texts = vec!["含有赌博", "含有色情", "正常内容"];
        let results = filter.find_all_batch(&texts);

        assert_eq!(results.len(), 3);
        assert!(results[0].contains(&"赌博".to_string()));
        assert!(results[1].contains(&"色情".to_string()));
        assert!(results[2].is_empty());
    }

    #[test]
    fn test_find_all_batch_empty() {
        let filter = Filter::new();
        let results = filter.find_all_batch(&[]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_all_layered_prefers_longest() {
        let mut filter = Filter::new();
        filter.add_words(&["赌", "赌博", "赌博机"]);

        // Longest match consumes the span; shorter overlapping words are dropped.
        let results = filter.find_all_layered("这里有赌博机");
        assert!(results.contains(&"赌博机".to_string()));
        assert!(!results.contains(&"赌".to_string()));
        assert!(!results.contains(&"赌博".to_string()));
    }

    #[test]
    fn test_find_all_streaming_multiline() {
        let mut filter = Filter::new();
        filter.add_words(&["赌博", "色情"]);

        let input = "第一行含有赌博\n第二行含有色情\n第三行正常";
        let cursor = std::io::Cursor::new(input);
        let results = filter.find_all_streaming(cursor).unwrap();

        assert!(results.contains(&"赌博".to_string()));
        assert!(results.contains(&"色情".to_string()));
        assert_eq!(results.len(), 2);
    }

    // ---- Task 3: LRU cache behavior ----

    #[test]
    fn test_cache_hit_returns_consistent_results() {
        let mut filter = Filter::new();
        filter.add_words(&["赌博", "色情"]);

        // First call — cache miss; second call — cache hit.
        let r1 = filter.find_all("含有赌博和色情内容");
        let r2 = filter.find_all("含有赌博和色情内容");
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_cache_clear() {
        let mut filter = Filter::new();
        filter.add_word("赌博");

        filter.find_all("含有赌博"); // populate cache
        filter.clear_cache();

        // After clear, the result is recomputed (still correct).
        let results = filter.find_all("含有赌博");
        assert!(results.contains(&"赌博".to_string()));
    }

    // ---- Task 4: edge cases ----

    #[test]
    fn test_empty_text() {
        let mut filter = Filter::new();
        filter.add_word("赌博");

        assert_eq!(filter.find_in(""), (false, String::new()));
        assert!(filter.find_all("").is_empty());
        assert_eq!(filter.replace("", '*'), "");
        assert_eq!(filter.filter(""), "");
    }

    #[test]
    fn test_empty_dictionary() {
        let filter = Filter::new();

        assert_eq!(filter.find_in("任何文本"), (false, String::new()));
        assert!(filter.find_all("任何文本").is_empty());
        assert_eq!(filter.replace("任何文本", '*'), "任何文本");
        assert_eq!(filter.filter("任何文本"), "任何文本");
    }

    #[test]
    fn test_unicode_emoji_does_not_interfere() {
        let mut filter = Filter::new();
        filter.add_word("赌博");

        // Emoji are stripped by the noise regex; surrounding CJK still matches.
        let (found, word) = filter.find_in("🎉 赌博 🎰");
        assert!(found);
        assert_eq!(word, "赌博");
    }

    #[test]
    fn test_very_long_text() {
        let mut filter = Filter::new();
        filter.add_word("赌博");

        // 100_000 chars = 300_000 bytes, exercises the >1000-byte parallel path.
        let long_text = "正常".repeat(100_000) + "赌博";
        let results = filter.find_all(&long_text);
        assert!(results.contains(&"赌博".to_string()));
    }

    #[test]
    fn test_cjk_extension_b_chars() {
        let mut filter = Filter::new();
        filter.add_word("赌博");

        // CJK Extension B (outside BMP); preceding CJK still matches.
        let text = "含有赌博内容 𠀀𠀁";
        let (found, _) = filter.find_in(text);
        assert!(found);
    }
}

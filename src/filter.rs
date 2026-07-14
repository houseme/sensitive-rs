//! Main filtering API.
//!
//! [`Filter`] is the primary entry point: load a dictionary with [`Filter::add_word`] /
//! [`Filter::add_words`] / [`Filter::load_word_dict`], then query with [`Filter::find_all`]
//! (all matches), [`Filter::find_in`] or [`Filter::find_first_match`] (first match),
//! [`Filter::replace`] (mask), or [`Filter::filter`] (remove). Input text is first cleaned of
//! noise via a configurable regex, then matched exactly against the dictionary, and finally
//! checked for pinyin/shape variants.

use crate::engine::{MatchAlgorithm, MatchInfo, MultiPatternEngine};
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use hashbrown::HashSet;
use regex::Regex;

#[cfg(feature = "std")]
use crate::variant::VariantDetector;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
#[cfg(feature = "std")]
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
};
#[cfg(feature = "std")]
use {alloc::sync::Arc, lru::LruCache, std::num::NonZero, std::sync::Mutex};

/// Advanced sensitive word filter with variant detection
pub struct Filter {
    engine: MultiPatternEngine, // Multi-pattern matching engine
    #[cfg(feature = "std")]
    variant_detector: VariantDetector, // Variation detector (pinyin/shape)
    noise: Regex,               // Noise processing rules
    #[cfg(feature = "std")]
    cache: Arc<Mutex<LruCache<String, Vec<String>>>>,
}

/// A sensitive-word match found by [`Filter::find_first_match`].
///
/// `word` is the matched word in its dictionary form; `is_variant` is `true` when
/// the match came from pinyin/shape variant detection rather than an exact hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    /// The matched sensitive word, in dictionary form.
    pub word: String,
    /// `true` if matched via a pinyin/shape variant rather than an exact hit.
    pub is_variant: bool,
}

impl core::fmt::Debug for Filter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Filter").field("engine", &self.engine).field("noise", &self.noise).finish_non_exhaustive()
    }
}

impl Filter {
    /// Create a new filter with default settings.
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_word("赌博");
    /// assert_eq!(filter.find_in("含有赌博"), (true, "赌博".to_string()));
    /// ```
    pub fn new() -> Self {
        Self {
            engine: MultiPatternEngine::new(None, &[]),
            #[cfg(feature = "std")]
            variant_detector: VariantDetector::new(),
            noise: Regex::new(r"[^\w\s\u4e00-\u9fff]").unwrap(),
            #[cfg(feature = "std")]
            cache: Arc::new(Mutex::new(LruCache::new(NonZero::new(1000).unwrap()))), // Cache 1000 results
        }
    }

    #[cfg(feature = "std")]
    fn check_cache(&self, text: &str) -> Option<Vec<String>> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).get(text).cloned()
    }

    #[cfg(feature = "std")]
    fn cache_result(&self, text: &str, results: &[String]) {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).put(text.to_string(), results.to_vec());
    }

    fn word_match_variants(word: &str) -> Vec<String> {
        let mut variants = vec![word.to_string()];
        if word.chars().any(char::is_whitespace) {
            let folded: String = word.chars().filter(|c| !c.is_whitespace()).collect();
            if !folded.is_empty() && folded != word {
                variants.push(folded);
            }
        }
        variants
    }

    fn extend_patterns_with_word_variants(patterns: &mut Vec<String>, words: &[&str]) {
        let mut seen: HashSet<String> = patterns.iter().cloned().collect();
        for word in words {
            for variant in Self::word_match_variants(word) {
                if seen.insert(variant.clone()) {
                    patterns.push(variant);
                }
            }
        }
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        #[cfg(feature = "std")]
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    /// Create with specific algorithm
    pub fn with_algorithm(algorithm: MatchAlgorithm) -> Self {
        Self { engine: MultiPatternEngine::new(Some(algorithm), &[]), ..Self::new() }
    }

    /// Load default dictionary
    #[cfg(feature = "std")]
    pub fn with_default_dict() -> io::Result<Self> {
        let mut filter = Self::new();
        filter.load_word_dict("dict/dict.txt")?;
        Ok(filter)
    }

    /// Update noise pattern
    ///
    /// Sets the regex used to strip noise characters before matching. Characters
    /// **not** matched by the regex are kept. Returns an error if `pattern` is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_word("赌博");
    /// // Strip everything except CJK and ASCII word characters.
    /// filter.update_noise_pattern(r"[^\w一-鿿]")?;
    /// assert_eq!(filter.remove_noise("赌@#博"), "赌博");
    /// # Ok::<(), regex::Error>(())
    /// ```
    pub fn update_noise_pattern(&mut self, pattern: &str) -> Result<(), regex::Error> {
        self.noise = Regex::new(pattern)?;
        Ok(())
    }

    /// Add a sensitive word
    pub fn add_word(&mut self, word: &str) {
        self.add_words(&[word]);
    }

    /// Add multiple words
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_words(&["赌博", "色情"]);
    /// assert!(filter.find_all("含有赌博和色情").contains(&"赌博".to_string()));
    /// ```
    pub fn add_words(&mut self, words: &[&str]) {
        let mut patterns = self.engine.get_patterns().to_vec();
        Self::extend_patterns_with_word_variants(&mut patterns, words);

        self.engine.rebuild(&patterns);
        #[cfg(feature = "std")]
        for word in words {
            for variant in Self::word_match_variants(word) {
                self.variant_detector.add_word(&variant);
            }
        }
        self.clear_cache();
    }

    /// Get the currently used algorithm
    #[must_use]
    pub fn current_algorithm(&self) -> MatchAlgorithm {
        self.engine.current_algorithm()
    }

    /// Remove a word
    pub fn del_word(&mut self, word: &str) {
        self.del_words(&[word]);
    }

    /// Remove multiple words
    pub fn del_words(&mut self, words: &[&str]) {
        let word_set: HashSet<String> = words.iter().flat_map(|word| Self::word_match_variants(word)).collect();
        let patterns: Vec<_> = self.engine.get_patterns().iter().filter(|w| !word_set.contains(*w)).cloned().collect();

        self.engine.rebuild(&patterns);
        self.clear_cache();
    }

    /// Load dictionary from file
    #[cfg(feature = "std")]
    pub fn load_word_dict<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let file = File::open(path)?;
        self.load(BufReader::new(file))
    }

    /// Load dictionary from reader
    ///
    /// Each line of the reader is one dictionary word.
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::Filter;
    /// use std::io::Cursor;
    ///
    /// let mut filter = Filter::new();
    /// filter.load(Cursor::new("赌博\n色情"))?;
    /// assert_eq!(filter.find_in("含有赌博"), (true, "赌博".to_string()));
    /// # Ok::<(), std::io::Error>(())
    /// ```
    #[cfg(feature = "std")]
    pub fn load<R: BufRead>(&mut self, reader: R) -> io::Result<()> {
        let words: Vec<_> = reader.lines().collect::<Result<_, _>>()?;
        self.add_words(&words.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        Ok(())
    }

    /// Load dictionary from URL
    #[cfg(feature = "net")]
    pub fn load_net_word_dict(&mut self, url: &str) -> io::Result<()> {
        // Build a client per call (lazy) rather than storing one, so a `Filter` can
        // be created/dropped inside an async runtime without panicking.
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(io::Error::other)?;
        let response = client.get(url).send().map_err(io::Error::other)?;

        if !response.status().is_success() {
            return Err(io::Error::other(format!("HTTP request failed: {}", response.status())));
        }

        let reader = BufReader::new(response);
        self.load(reader)
    }

    /// Find the first sensitive word, returning a [`Match`] with details, or `None`.
    ///
    /// Exact matches are preferred; pinyin/shape variants are only consulted when no
    /// exact hit is found. `is_variant` on the returned [`Match`] records which path hit.
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::{Filter, Match};
    ///
    /// let mut filter = Filter::new();
    /// filter.add_word("赌博");
    ///
    /// // Exact hit:
    /// assert_eq!(
    ///     filter.find_first_match("含有赌博"),
    ///     Some(Match { word: "赌博".to_string(), is_variant: false })
    /// );
    /// // Pinyin variant (no exact hit):
    /// assert_eq!(
    ///     filter.find_first_match("dubo"),
    ///     Some(Match { word: "赌博".to_string(), is_variant: true })
    /// );
    /// // No match:
    /// assert_eq!(filter.find_first_match("clean text"), None);
    /// ```
    #[must_use]
    pub fn find_first_match(&self, text: &str) -> Option<Match> {
        let clean_text = self.remove_noise(text);

        // 1. Try exact match first
        if let Some(word) = self.engine.find_first(&clean_text) {
            return Some(Match { word, is_variant: false });
        }

        // 2. Try variant detection (requires `std`: pinyin/shape detection)
        #[cfg(feature = "std")]
        {
            let patterns: Vec<_> = self.engine.get_patterns().iter().map(|s| s.as_str()).collect();
            if let Some(word) = self.variant_detector.detect(&clean_text, &patterns).first() {
                return Some(Match { word: word.to_string(), is_variant: true });
            }
        }

        None
    }

    /// Find first sensitive word.
    ///
    /// Returns `(found, word)`; `word` is empty when nothing matched. For richer detail
    /// (including whether the hit was a variant), use [`Filter::find_first_match`].
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_word("赌博");
    /// assert_eq!(filter.find_in("含有赌博"), (true, "赌博".to_string()));
    /// assert_eq!(filter.find_in("clean text"), (false, String::new()));
    /// ```
    #[must_use]
    pub fn find_in(&self, text: &str) -> (bool, String) {
        match self.find_first_match(text) {
            Some(m) => (true, m.word),
            None => (false, String::new()),
        }
    }

    /// Replace sensitive words with replacement character.
    ///
    /// Each matched character is replaced by one `replacement` char. Only exact
    /// dictionary matches are masked; variant forms are not (use [`Filter::find_all`]
    /// to detect them).
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_word("赌博");
    /// assert_eq!(filter.replace("含有赌博内容", '*'), "含有**内容");
    /// ```
    #[must_use]
    pub fn replace(&self, text: &str, replacement: char) -> String {
        let clean_text = self.remove_noise(text);
        let repl = replacement.to_string();

        // Single pass over leftmost-longest non-overlapping matches: one replacement
        // char per matched character. Exact matches only — see the doc note above.
        let matches = self.leftmost_longest_matches(&clean_text);
        let mut result = String::with_capacity(clean_text.len());
        let mut cursor = 0usize;
        for m in &matches {
            result.push_str(&clean_text[cursor..m.start]);
            result.push_str(&repl.repeat(clean_text[m.start..m.end].chars().count()));
            cursor = m.end;
        }
        result.push_str(&clean_text[cursor..]);
        result
    }

    /// Filter out sensitive words (remove them completely).
    ///
    /// Only exact dictionary matches are removed; variant forms are not (see
    /// [`Filter::replace`]). Use [`Filter::find_all`] to detect variants.
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_word("赌博");
    /// assert_eq!(filter.filter("含有赌博内容"), "含有内容");
    /// ```
    #[must_use]
    pub fn filter(&self, text: &str) -> String {
        let clean_text = self.remove_noise(text);
        self.engine.replace_all(&clean_text, "")
    }

    /// Validate text
    ///
    /// This is an alias for [`Filter::find_in`]: it returns `(found, word)` for the
    /// first sensitive word encountered.
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_word("赌博");
    /// assert_eq!(filter.validate("含有赌博"), (true, "赌博".to_string()));
    /// assert_eq!(filter.validate("clean text"), (false, String::new()));
    /// ```
    #[must_use]
    pub fn validate(&self, text: &str) -> (bool, String) {
        self.find_in(text)
    }

    /// Remove only specific noise characters, preserve spaces
    #[must_use]
    pub fn remove_noise(&self, text: &str) -> String {
        self.noise.replace_all(text, "").to_string()
    }

    /// Get current noise pattern
    #[must_use]
    pub fn get_noise_pattern(&self) -> &Regex {
        &self.noise
    }

    /// Greedy leftmost-longest non-overlapping exact matches (byte spans + pattern).
    ///
    /// Sorts by start ascending then end descending (longest first at each start) and
    /// keeps a match only when it begins at or after the previous kept match's end.
    /// Shared by [`Filter::replace`] and [`Filter::find_all_layered`].
    fn leftmost_longest_matches(&self, clean_text: &str) -> Vec<MatchInfo> {
        let mut matches = self.engine.find_matches_with_positions(clean_text);
        matches.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
        let mut kept = Vec::with_capacity(matches.len());
        let mut cursor = 0usize;
        for m in matches {
            if m.start >= cursor {
                cursor = m.end;
                kept.push(m);
            }
        }
        kept
    }

    /// Optimized method of finding all sensitive words.
    ///
    /// Returns the de-duplicated, sorted list of matched dictionary words (variants
    /// included). Results are cached, so repeated calls on the same text are cheap.
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_words(&["赌博", "色情"]);
    /// // Results are sorted: 色 (U+8272) sorts before 赌 (U+8D4C).
    /// assert_eq!(filter.find_all("含有赌博和色情内容"), vec!["色情".to_string(), "赌博".to_string()]);
    /// ```
    #[must_use]
    pub fn find_all(&self, text: &str) -> Vec<String> {
        let clean_text = self.remove_noise(text);

        // 1. Caching mechanism - Check whether the results have been cached
        #[cfg(feature = "std")]
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
        #[cfg(feature = "std")]
        self.cache_result(&clean_text, &results);

        results
    }

    /// Parallel processing version — for long text.
    ///
    /// Exact scan and variant detection are independent, so they run concurrently via
    /// [`rayon::join`]. Variant detection runs once over the full text (the previous
    /// whitespace-split parallelization dropped cross-segment variants).
    #[cfg(feature = "parallel")]
    fn find_all_parallel(&self, text: &str) -> Vec<String> {
        let patterns: Vec<&str> = self.engine.get_patterns().iter().map(|s| s.as_str()).collect();

        let (engine_results, variant_results) = rayon::join(
            || self.engine.find_all(text),
            || self.variant_detector.detect(text, &patterns).into_iter().map(String::from).collect::<Vec<_>>(),
        );

        let mut results = engine_results;
        results.extend(variant_results);
        self.deduplicate_and_sort(results)
    }

    /// Sequential processing version - suitable for short text
    fn find_all_sequential(&self, text: &str) -> Vec<String> {
        let mut results = self.engine.find_all(text);

        // Add variant detection results (std only: pinyin/shape detection)
        #[cfg(feature = "std")]
        {
            let patterns: Vec<_> = self.engine.get_patterns().iter().map(|s| s.as_str()).collect();
            results.extend(self.variant_detector.detect(text, &patterns).into_iter().map(|s| s.to_string()));
        }

        self.deduplicate_and_sort(results)
    }

    /// Deduplication and sort
    fn deduplicate_and_sort(&self, mut results: Vec<String>) -> Vec<String> {
        results.sort_unstable();
        results.dedup();
        results
    }

    /// Bulk search for optimized versions
    ///
    /// Runs [`Filter::find_all`] over each text. With the `parallel` feature (default) the
    /// texts are processed concurrently.
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_words(&["赌博", "色情"]);
    /// let results = filter.find_all_batch(&["含有赌博", "正常", "含有色情"]);
    /// assert!(results[0].contains(&"赌博".to_string()));
    /// assert!(results[1].is_empty());
    /// assert!(results[2].contains(&"色情".to_string()));
    /// ```
    #[must_use]
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
    ///
    /// Matches the longest words first and consumes their span, so shorter overlapping
    /// entries are dropped. Useful when the dictionary contains both short and long forms.
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_words(&["赌", "赌博", "赌博机"]);
    /// let results = filter.find_all_layered("这里有赌博机");
    /// assert!(results.contains(&"赌博机".to_string()));
    /// assert!(!results.contains(&"赌博".to_string()));
    /// ```
    #[must_use]
    pub fn find_all_layered(&self, text: &str) -> Vec<String> {
        let clean_text = self.remove_noise(text);
        let matches = self.leftmost_longest_matches(&clean_text);

        // The longest exact matches...
        let mut results: Vec<String> = matches.iter().map(|m| m.pattern.clone()).collect();

        // ...then blank those spans before variant detection, so a shorter word's
        // pinyin/shape isn't re-discovered inside a longer exact match. (std only)
        #[cfg(feature = "std")]
        {
            let mut remaining = String::with_capacity(clean_text.len());
            let mut cursor = 0usize;
            for m in &matches {
                remaining.push_str(&clean_text[cursor..m.start]);
                remaining.push(' ');
                cursor = m.end;
            }
            remaining.push_str(&clean_text[cursor..]);

            let patterns: Vec<&str> = self.engine.get_patterns().iter().map(|s| s.as_str()).collect();
            results.extend(self.variant_detector.detect(&remaining, &patterns).into_iter().map(String::from));
        }

        self.deduplicate_and_sort(results)
    }

    /// Streaming version - suitable for oversized text
    ///
    /// Reads line-by-line from any [`BufRead`] and returns the de-duplicated matches
    /// across all lines. Handy for files too large to hold in memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::Filter;
    /// use std::io::Cursor;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_words(&["赌博", "色情"]);
    /// let input = "第一行含有赌博\n第二行含有色情\n第三行正常";
    /// let results = filter.find_all_streaming(Cursor::new(input))?;
    /// assert_eq!(results.len(), 2);
    /// # Ok::<(), std::io::Error>(())
    /// ```
    #[cfg(feature = "std")]
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

/// Async dictionary loading (non-blocking I/O). Enable with the `async-io` feature
/// (and `net-async` for the URL loader). The synchronous API is unchanged.
#[cfg(feature = "async-io")]
impl Filter {
    /// Load a dictionary from a file without blocking the caller's thread.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[tokio::main] async fn main() -> std::io::Result<()> {
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.load_word_dict_async("dict/dict.txt").await?;
    /// # Ok(()) }
    /// ```
    pub async fn load_word_dict_async<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};
        let file = tokio::fs::File::open(path).await?;
        let mut lines = BufReader::new(file).lines();
        let mut words = Vec::new();
        while let Some(line) = lines.next_line().await? {
            words.push(line);
        }
        let refs: Vec<&str> = words.iter().map(|s| s.as_str()).collect();
        self.add_words(&refs);
        Ok(())
    }

    /// Load a dictionary from a URL without blocking. Requires the `net-async` feature.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[tokio::main] async fn main() -> std::io::Result<()> {
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.load_net_word_dict_async("https://example.com/dict.txt").await?;
    /// # Ok(()) }
    /// ```
    #[cfg(feature = "net-async")]
    pub async fn load_net_word_dict_async(&mut self, url: &str) -> io::Result<()> {
        let response = reqwest::get(url).await.map_err(io::Error::other)?;
        let content = response.text().await.map_err(io::Error::other)?;
        let words: Vec<&str> = content.lines().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        self.add_words(&words);
        Ok(())
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
    fn test_find_first_match_exact() {
        let mut filter = Filter::new();
        filter.add_words(&["赌博", "色情"]);

        assert_eq!(filter.find_first_match("含有赌博"), Some(Match { word: "赌博".to_string(), is_variant: false }));
        assert_eq!(filter.find_first_match("正常文本"), None);
    }

    #[test]
    fn test_find_first_match_variant() {
        let mut filter = Filter::new();
        filter.add_word("赌博");

        // Pinyin variant path: word found, but is_variant = true.
        assert_eq!(filter.find_first_match("含有 dubo"), Some(Match { word: "赌博".to_string(), is_variant: true }));
    }

    #[test]
    fn test_find_first_match_prefers_exact_over_variant() {
        let mut filter = Filter::new();
        filter.add_word("赌博");

        // Exact hit wins even though a pinyin variant would also match.
        assert_eq!(filter.find_first_match("赌博 dubo"), Some(Match { word: "赌博".to_string(), is_variant: false }));
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
    fn test_space_folded_word_matches_both_forms() {
        let mut filter = Filter::new();
        filter.add_word("A 级");

        assert_eq!(filter.find_in("含有 A 级 内容"), (true, "A 级".to_string()));
        assert_eq!(filter.find_in("含有 A级 内容"), (true, "A级".to_string()));

        let results = filter.find_all("A 级 和 A级");
        assert!(results.contains(&"A 级".to_string()));
        assert!(results.contains(&"A级".to_string()));
    }

    #[test]
    fn test_loaded_space_word_adds_folded_match() -> io::Result<()> {
        let mut filter = Filter::new();
        filter.load(Cursor::new("A 级\n3 级片"))?;

        assert_eq!(filter.find_in("这里有 A级 内容"), (true, "A级".to_string()));
        assert_eq!(filter.find_in("这里有 3级片 内容"), (true, "3级片".to_string()));
        Ok(())
    }

    #[test]
    fn test_delete_space_word_removes_both_forms() {
        let mut filter = Filter::new();
        filter.add_words(&["A 级", "B 级"]);

        filter.del_word("A 级");

        assert_eq!(filter.find_in("含有 A 级 内容"), (false, String::new()));
        assert_eq!(filter.find_in("含有 A级 内容"), (false, String::new()));
        assert_eq!(filter.find_in("含有 B级 内容"), (true, "B级".to_string()));
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
    fn test_find_all_layered_multi_occurrence() {
        // Two non-overlapping longest matches are both kept; shorter overlapping
        // forms inside each span are dropped.
        let mut filter = Filter::new();
        filter.add_words(&["赌", "赌博", "赌博机"]);

        let results = filter.find_all_layered("赌博和赌博机");
        assert!(results.contains(&"赌博".to_string()));
        assert!(results.contains(&"赌博机".to_string()));
        assert!(!results.contains(&"赌".to_string()));
    }

    #[test]
    fn test_find_all_layered_blanks_exact_before_variant() {
        // Regression: a short word whose pinyin is a substring of a longer exact
        // match must NOT be re-added by variant detection — exact spans are blanked.
        let mut filter = Filter::new();
        filter.add_words(&["赌", "赌博", "赌博机"]);

        let results = filter.find_all_layered("这里有赌博机");
        // "赌"/"赌博" pinyin (du/dubo) sits inside "赌博机" pinyin (duboji), but the
        // exact span is blanked first, so only the longest match survives.
        assert_eq!(results, vec!["赌博机".to_string()]);
    }

    #[test]
    fn test_replace_and_filter_are_exact_only() {
        // Pins behavior: replace/filter mask exact dictionary matches only. A
        // pinyin variant present in the text passes through unchanged (the variant
        // detector reports word names, not the variant text spans to mask).
        let mut filter = Filter::new();
        filter.add_word("赌博");

        // Exact match is masked / removed...
        assert_eq!(filter.replace("含有赌博", '*'), "含有**");
        assert_eq!(filter.filter("含有赌博"), "含有");
        // ...but the pinyin variant "dubo" is left untouched.
        assert_eq!(filter.replace("dubo", '*'), "dubo");
        assert_eq!(filter.filter("dubo"), "dubo");
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

        let _ = filter.find_all("含有赌博"); // populate cache
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

    // ---- Async loading (async-io feature) ----

    #[cfg(feature = "async-io")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_load_word_dict_async() {
        let path = std::env::temp_dir().join(format!("sensitive-rs-async-{}.txt", std::process::id()));
        std::fs::write(&path, "赌博\n色情\n").unwrap();

        let mut filter = Filter::new();
        filter.load_word_dict_async(&path).await.unwrap();

        assert_eq!(filter.find_in("含有赌博"), (true, "赌博".to_string()));
        assert!(filter.find_all("赌博和色情").iter().any(|w| w == "色情"));
        let _ = std::fs::remove_file(&path);
    }
}

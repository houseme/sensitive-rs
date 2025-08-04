use crate::engine::MatchAlgorithm;
use crate::{engine::MultiPatternEngine, variant::VariantDetector};
use regex::Regex;
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
};

/// Advanced sensitive word filter with variant detection
pub struct Filter {
    engine: MultiPatternEngine,        // 多模式匹配引擎
    variant_detector: VariantDetector, // 变体检测器
    noise: Regex,                      // 噪音处理正则
    #[cfg(feature = "net")]
    http_client: reqwest::blocking::Client, // 网络请求客户端
}

impl Filter {
    /// Create a new filter with default settings
    pub fn new() -> Self {
        Self {
            engine: MultiPatternEngine::new(None, &[]),
            variant_detector: VariantDetector::new(),
            noise: Regex::new(r"[^\w\s\u4e00-\u9fff]").unwrap(),
            #[cfg(feature = "net")]
            http_client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap(),
        }
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
    pub fn update_noise_pattern(&mut self, pattern: &str) {
        self.noise = Regex::new(pattern).unwrap();
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
    }

    /// Add multiple words
    pub fn add_words(&mut self, words: &[&str]) {
        let mut patterns = self.engine.get_patterns().to_vec();
        patterns.extend(words.iter().map(|s| s.to_string()));

        self.engine.rebuild(&patterns);
        for word in words {
            self.variant_detector.add_word(word);
        }
    }

    /// 获取当前使用的算法
    pub fn current_algorithm(&self) -> MatchAlgorithm {
        self.engine.current_algorithm()
    }

    /// Remove a word
    pub fn del_word(&mut self, word: &str) {
        let patterns: Vec<_> = self.engine.get_patterns().iter().filter(|&w| w != word).cloned().collect();

        self.engine.rebuild(&patterns);
    }

    /// Remove multiple words
    pub fn del_words(&mut self, words: &[&str]) {
        let word_set: std::collections::HashSet<_> = words.iter().collect();
        let patterns: Vec<_> =
            self.engine.get_patterns().iter().filter(|w| !word_set.contains(&w.as_str())).cloned().collect();

        self.engine.rebuild(&patterns);
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

    /// Find all sensitive words
    pub fn find_all(&self, text: &str) -> Vec<String> {
        let clean_text = self.remove_noise(text);
        let mut results = self.engine.find_all(&clean_text);

        let patterns: Vec<_> = self.engine.get_patterns().iter().map(|s| s.as_str()).collect();

        results.extend(self.variant_detector.detect(&clean_text, &patterns).into_iter().map(|s| s.to_string()));
        results.sort_unstable();
        results.dedup();
        results
    }

    /// Replace sensitive words with replacement character
    pub fn replace(&self, text: &str, replacement: char) -> String {
        let clean_text = self.remove_noise(text);

        // 获取所有需要处理的敏感词（包括变体）
        let patterns: Vec<_> = self.engine.get_patterns().iter().map(|s| s.as_str()).collect();
        let variants = self.variant_detector.detect(&clean_text, &patterns);

        let mut result = clean_text;

        // 替换引擎检测到的敏感词
        for pattern in self.engine.get_patterns() {
            let repl_str = replacement.to_string().repeat(pattern.chars().count());
            result = result.replace(pattern, &repl_str);
        }

        // 替换变体检测到的敏感词
        for variant in variants {
            let repl_str = replacement.to_string().repeat(variant.chars().count());
            result = result.replace(variant, &repl_str);
        }

        result
    }

    /// Filter out sensitive words (remove them completely)
    pub fn filter(&self, text: &str) -> String {
        let clean_text = self.remove_noise(text);

        // 获取所有需要处理的敏感词（包括变体）
        let patterns: Vec<_> = self.engine.get_patterns().iter().map(|s| s.as_str()).collect();
        let variants = self.variant_detector.detect(&clean_text, &patterns);

        let mut result = clean_text;

        // 移除引擎检测到的敏感词
        for pattern in self.engine.get_patterns() {
            result = result.replace(pattern, "");
        }

        // 移除变体检测到的敏感词
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

        // replace 应该用字符替换
        assert_eq!(filter.replace(text, '*'), "这里有**和**内容");

        // filter 应该完全移除
        assert_eq!(filter.filter(text), "这里有和内容");
    }

    #[test]
    fn test_variant_detection() {
        let mut filter = Filter::new();
        filter.add_word("测试");

        assert_eq!(filter.find_in("ceshi"), (true, "测试".to_string()));
    }

    #[test]
    fn test_algorithm_switch_one() {
        // 少量词使用 Wu-Manber
        let mut small = Filter::new();
        small.add_words(&["a", "b", "c"]);
        assert!(matches!(small.engine.current_algorithm(), MatchAlgorithm::WuManber));

        // 中等数量用 Aho-Corasick
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
        // 少量词使用 Wu-Manber
        let mut small = Filter::new();
        small.add_words(&["a", "b", "c"]);
        println!("Small (3 words): {:?}", small.current_algorithm());
        assert!(matches!(small.current_algorithm(), MatchAlgorithm::WuManber));

        // 中等数量用 Aho-Corasick
        let words: Vec<_> = (0..150).map(|i| format!("word{i}")).collect();
        let word_refs: Vec<&str> = words.iter().map(|s| s.as_str()).collect();

        let mut medium = Filter::new();
        medium.add_words(&word_refs);

        println!("Medium (150 words): {:?}", medium.current_algorithm());
        println!("Pattern count: {}", medium.engine.get_patterns().len());

        // 验证算法选择逻辑
        let recommended = MultiPatternEngine::recommend_algorithm(150);
        println!("Recommended algorithm for 150 words: {recommended:?}");

        assert!(matches!(medium.current_algorithm(), MatchAlgorithm::AhoCorasick));
    }
}

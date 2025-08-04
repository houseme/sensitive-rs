pub(crate) mod wumanber;
use crate::WuManber;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use regex::Regex;
use std::sync::Arc;

/// 支持的匹配算法类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchAlgorithm {
    AhoCorasick, // 默认算法，适合中等规模词库
    WuManber,    // 适合大规模词库
    Regex,       // 适合复杂规则匹配
}

/// 多模式匹配引擎
pub struct MultiPatternEngine {
    algorithm: MatchAlgorithm,    // 当前使用的匹配算法
    ac: Option<Arc<AhoCorasick>>, // Aho-Corasick 引擎
    wm: Option<Arc<WuManber>>,    // Wu-Manber 引擎
    regex_set: Option<Regex>,     // 正则表达式引擎
    patterns: Vec<String>,        // 存储所有模式
}

impl Default for MultiPatternEngine {
    fn default() -> Self {
        Self { algorithm: MatchAlgorithm::AhoCorasick, ac: None, wm: None, regex_set: None, patterns: Vec::new() }
    }
}

impl MultiPatternEngine {
    /// 创建新引擎并根据词库大小自动选择算法
    pub fn new(algorithm: Option<MatchAlgorithm>, patterns: &[String]) -> Self {
        let algorithm = algorithm.unwrap_or_else(|| Self::recommend_algorithm(patterns.len()));
        let mut engine = Self { algorithm, ..Default::default() };

        engine.rebuild(patterns);
        engine
    }

    /// 重新构建引擎（当模式更新时调用）
    pub fn rebuild(&mut self, patterns: &[String]) {
        self.patterns = patterns.to_vec();

        // 根据新的词库大小重新评估算法选择
        let recommended = Self::recommend_algorithm(patterns.len());
        if self.algorithm != recommended {
            self.algorithm = recommended;
        }

        self.build_engines();
    }

    /// 根据词库大小推荐算法
    pub fn recommend_algorithm(word_count: usize) -> MatchAlgorithm {
        match word_count {
            0..=100 => MatchAlgorithm::WuManber,         // 小词库用 Wu-Manber
            101..=10_000 => MatchAlgorithm::AhoCorasick, // 中等词库用 Aho-Corasick
            _ => MatchAlgorithm::Regex,                  // 超大词库用正则
        }
    }

    /// 强制使用指定算法重建
    pub fn rebuild_with_algorithm(&mut self, patterns: &[String], algorithm: MatchAlgorithm) {
        self.patterns = patterns.to_vec();
        self.algorithm = algorithm;
        self.build_engines();
    }

    /// 根据当前算法构建相应的引擎
    fn build_engines(&mut self) {
        // 清空所有引擎
        self.ac = None;
        self.wm = None;
        self.regex_set = None;

        // 根据选择的算法构建对应引擎
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

    /// 获取当前使用的算法
    pub fn current_algorithm(&self) -> MatchAlgorithm {
        self.algorithm
    }

    /// 获取所有模式
    pub fn get_patterns(&self) -> &[String] {
        &self.patterns
    }

    /// 查找第一个匹配
    pub fn find_first(&self, text: &str) -> Option<String> {
        match self.algorithm {
            MatchAlgorithm::AhoCorasick => {
                self.ac.as_ref().unwrap().find(text).map(|mat| text[mat.start()..mat.end()].to_string())
            }
            MatchAlgorithm::WuManber => self.wm.as_ref().unwrap().search(text),
            MatchAlgorithm::Regex => self.regex_set.as_ref().unwrap().find(text).map(|mat| mat.as_str().to_string()),
        }
    }

    /// 替换所有匹配
    pub fn replace_all(&self, text: &str, replacement: &str) -> String {
        match self.algorithm {
            MatchAlgorithm::AhoCorasick => self.ac.as_ref().unwrap().replace_all(text, &[replacement]).to_string(),
            MatchAlgorithm::WuManber => {
                // 修复：正确处理空字符串替换
                if replacement.is_empty() {
                    // 对于空字符串，直接移除匹配的内容
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

    /// 查找所有匹配
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

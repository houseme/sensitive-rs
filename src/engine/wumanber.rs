// src/engine/wumanber.rs
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::collections::{HashMap, HashSet};

/// Wu-Manber 多模式匹配算法
pub struct WuManber {
    patterns: Vec<String>,                // 模式字符串列表
    min_len: usize,                       // 最短模式长度
    block_size: usize,                    // 块大小（B 参数）
    shift_table: HashMap<u64, usize>,     // Shift 表：使用 hash 作为 key
    hash_table: HashMap<u64, Vec<usize>>, // Hash 表：hash 到模式索引的映射
    pattern_set: HashSet<String>,         // 用于快速查找的模式集合
}

impl WuManber {
    /// 为中文优化的构造函数
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

    /// 创建新的 Wu-Manber 实例
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

    /// 构建 shift 和 hash 表
    fn build_tables(&mut self) {
        if self.patterns.is_empty() {
            return;
        }

        self.shift_table.clear();
        self.hash_table.clear();

        // 构建 shift 表
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

        // 构建 hash 表
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

    /// 计算字符串的哈希值
    fn calculate_hash(&self, s: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    /// 搜索文本中的敏感词
    pub fn search(&self, text: &str) -> Option<String> {
        if self.patterns.is_empty() || text.is_empty() {
            return None;
        }

        // 转换为字符数组处理 Unicode
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

            // 获取 shift 值
            let shift = self.shift_table.get(&hash).copied().unwrap_or(self.min_len);
            i += shift.max(1);
        }

        None
    }

    /// 查找所有匹配
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

    /// 替换所有匹配
    pub fn replace_all(&self, text: &str, replacement: char) -> String {
        let mut result = text.to_string();

        // 按模式长度降序排列，避免短模式影响长模式的匹配
        let mut sorted_patterns = self.patterns.clone();
        sorted_patterns.sort_by_key(|b| std::cmp::Reverse(b.chars().count()));

        for pattern in &sorted_patterns {
            if replacement == '\0' {
                // 使用空字符表示删除
                result = result.replace(pattern, "");
            } else {
                let repl_str = replacement.to_string().repeat(pattern.chars().count());
                result = result.replace(pattern, &repl_str);
            }
        }

        result
    }

    /// 完全移除匹配的内容
    pub fn remove_all(&self, text: &str) -> String {
        self.replace_all(text, '\0')
    }

    /// 并行构建 shift 和 hash 表
    pub fn build_tables_parallel(&mut self) {
        if self.patterns.is_empty() {
            return;
        }

        self.shift_table.clear();
        self.hash_table.clear();

        // 并行计算 shift 表
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

        // 合并 shift 表
        for (hash, shift) in shift_entries {
            self.shift_table.entry(hash).and_modify(|v| *v = (*v).min(shift)).or_insert(shift);
        }

        // 并行计算 hash 表
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

        // 合并 hash 表
        for (hash, pattern_idx) in hash_entries {
            self.hash_table.entry(hash).or_default().push(pattern_idx);
        }
    }
}

impl WuManber {
    /// 获取匹配位置 - 修复字符边界问题
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
                                // 计算字节位置用于返回
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

    /// 带并行构建的构造函数
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

    /// 中文优化的哈希函数
    #[allow(dead_code)]
    fn chinese_hash(&self, block: &str) -> u64 {
        // 使用字符编码直接计算，避免 UTF-8 解码开销
        if block.len() == 2 {
            // 对于 2 字节中文常见情况
            let bytes = block.as_bytes();
            (bytes[0] as u64) << 8 | (bytes[1] as u64)
        } else {
            self.hash(block)
        }
    }

    /// 计算字符串块的哈希值
    /// 中文优化的哈希函数
    fn hash(&self, block: &str) -> u64 {
        self.calculate_hash(block)
    }
}

/// 匹配结果结构体
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

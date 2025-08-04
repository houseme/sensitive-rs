use pinyin::ToPinyin;
use std::collections::HashMap;

/// 变体检测器
pub struct VariantDetector {
    pinyin_map: HashMap<String, Vec<String>>, // 拼音到原词的映射
    shape_map: HashMap<char, Vec<char>>,      // 形近字映射
    char_to_pinyin: HashMap<char, String>,    // 字符到拼音的映射
}

impl Default for VariantDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl VariantDetector {
    /// 创建新检测器
    pub fn new() -> Self {
        VariantDetector {
            pinyin_map: HashMap::new(),
            shape_map: Self::build_shape_map(),
            char_to_pinyin: HashMap::new(),
        }
    }

    /// 添加敏感词时构建拼音索引
    pub fn add_word(&mut self, word: &str) {
        // 修复 clippy 警告并安全处理拼音转换
        let pinyins: Vec<String> = word
            .chars()
            .filter_map(|c| {
                if let Some(py) = c.to_pinyin() {
                    let pinyin = py.plain().to_string();
                    // 建立字符到拼音的映射
                    self.char_to_pinyin.insert(c, pinyin.clone());
                    Some(pinyin)
                } else {
                    // 对于无法转换的字符，返回 None
                    None
                }
            })
            .collect();

        if !pinyins.is_empty() {
            let pinyin_key = pinyins.join("");
            self.pinyin_map.entry(pinyin_key).or_default().push(word.to_string());
        }
    }

    /// 检测文本中的变体
    pub fn detect<'a>(&'a self, text: &str, original_words: &[&'a str]) -> Vec<&'a str> {
        let mut variants = Vec::new();

        // 1. 检测拼音变体
        variants.extend(self.detect_pinyin_variants(text, original_words));

        // 2. 检测形近字变体
        variants.extend(self.detect_shape_variants(text, original_words));

        variants.sort_unstable();
        variants.dedup();
        variants
    }

    /// 检测拼音变体
    fn detect_pinyin_variants<'a>(&'a self, text: &str, original_words: &[&'a str]) -> Vec<&'a str> {
        let text_pinyin = self.text_to_pinyin(text);

        original_words
            .iter()
            .filter(|&&word| {
                // 构建词的拼音
                let word_pinyin: String = word
                    .chars()
                    .map(|c| {
                        self.char_to_pinyin.get(&c).cloned().unwrap_or_else(|| c.to_string())
                        // 安全处理：返回原字符
                    })
                    .collect();

                text_pinyin.contains(&word_pinyin)
            })
            .copied()
            .collect()
    }

    /// 将文本转换为拼音
    fn text_to_pinyin(&self, text: &str) -> String {
        text.chars()
            .map(|c| {
                self.char_to_pinyin.get(&c).cloned().unwrap_or_else(|| {
                    // 实时转换未缓存的字符
                    if let Some(py) = c.to_pinyin() {
                        py.plain().to_string()
                    } else {
                        c.to_string() // 保持原字符
                    }
                })
            })
            .collect()
    }

    /// 检测形近字变体
    fn detect_shape_variants<'a>(&'a self, text: &str, original_words: &[&'a str]) -> Vec<&'a str> {
        original_words.iter().filter(|&&word| self.is_shape_variant(text, word)).copied().collect()
    }

    /// 判断是否为形近字变体
    fn is_shape_variant(&self, text: &str, word: &str) -> bool {
        let text_chars: Vec<char> = text.chars().collect();
        let word_chars: Vec<char> = word.chars().collect();

        if text_chars.len() != word_chars.len() {
            return false;
        }

        text_chars
            .iter()
            .zip(word_chars.iter())
            .all(|(&tc, &wc)| tc == wc || self.shape_map.get(&wc).is_some_and(|variants| variants.contains(&tc)))
    }

    /// 构建形近字映射表
    fn build_shape_map() -> HashMap<char, Vec<char>> {
        let mut map = HashMap::new();
        // 示例：添加一些常见形近字
        map.insert('赌', vec!['渧', '睹', '堵']);
        map.insert('博', vec!['搏', '傅', '膊']);
        map.insert('有', vec!['友', '右']);
        map.insert('色', vec!['涩']);
        map.insert('情', vec!['请', '清']);
        map
    }
}

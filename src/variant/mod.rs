use pinyin::Pinyin;
use std::collections::HashMap;

/// Variation detector
pub struct VariantDetector {
    pinyin_map: HashMap<String, Vec<String>>, // The mapping of pinyin to original word
    shape_map: HashMap<char, Vec<char>>,      // SHAPED CLOSE CHARACTER MAPPING
    char_to_pinyin: HashMap<char, String>,    // Character to pinyin mapping
}

impl Default for VariantDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl VariantDetector {
    /// Create a new detector
    pub fn new() -> Self {
        VariantDetector {
            pinyin_map: HashMap::new(),
            shape_map: Self::build_shape_map(),
            char_to_pinyin: HashMap::new(),
        }
    }

    /// Construct pinyin index when adding sensitive words
    pub fn add_word(&mut self, word: &str) {
        let chars_result = Pinyin::chars(word).with_tone_style(pinyin::ToneStyle::None);
        let han_chars: Vec<char> = word.chars().filter(|c| !c.is_ascii()).collect();
        let pinyin_lookup: HashMap<char, String> = han_chars.into_iter().zip(chars_result.iter()).collect();

        let pinyins: Vec<String> = word
            .chars()
            .filter_map(|c| {
                pinyin_lookup.get(&c).map(|pinyin| {
                    self.char_to_pinyin.insert(c, pinyin.clone());
                    pinyin.clone()
                })
            })
            .collect();

        if !pinyins.is_empty() {
            let pinyin_key = pinyins.join("");
            self.pinyin_map.entry(pinyin_key).or_default().push(word.to_string());
        }
    }

    /// Detect variants in text
    pub fn detect<'a>(&'a self, text: &str, original_words: &[&'a str]) -> Vec<&'a str> {
        let mut variants = Vec::new();

        // 1. Detect pinyin variants
        variants.extend(self.detect_pinyin_variants(text, original_words));

        // 2. Detect shape-near-word variant
        variants.extend(self.detect_shape_variants(text, original_words));

        variants.sort_unstable();
        variants.dedup();
        variants
    }

    /// Detect pinyin variants
    fn detect_pinyin_variants<'a>(&'a self, text: &str, original_words: &[&'a str]) -> Vec<&'a str> {
        let text_pinyin = self.text_to_pinyin(text);

        original_words
            .iter()
            .filter(|&&word| {
                // Construct the pinyin of the word
                let word_pinyin: String = word
                    .chars()
                    .map(|c| {
                        self.char_to_pinyin.get(&c).cloned().unwrap_or_else(|| c.to_string())
                        // Safe processing: Return original characters
                    })
                    .collect();

                text_pinyin.contains(&word_pinyin)
            })
            .copied()
            .collect()
    }

    /// Convert text to pinyin
    fn text_to_pinyin(&self, text: &str) -> String {
        // Build pinyin for uncached characters in batch
        let uncached: Vec<char> =
            text.chars().filter(|c| !c.is_ascii() && !self.char_to_pinyin.contains_key(c)).collect();
        let extra: HashMap<char, String> = if uncached.is_empty() {
            HashMap::new()
        } else {
            let uncached_str: String = uncached.iter().collect();
            uncached
                .into_iter()
                .zip(Pinyin::chars(&uncached_str).with_tone_style(pinyin::ToneStyle::None).iter())
                .collect()
        };

        text.chars()
            .map(|c| {
                self.char_to_pinyin.get(&c).cloned().or_else(|| extra.get(&c).cloned()).unwrap_or_else(|| c.to_string())
            })
            .collect()
    }

    /// Detect shape-near-word variant
    fn detect_shape_variants<'a>(&'a self, text: &str, original_words: &[&'a str]) -> Vec<&'a str> {
        original_words.iter().filter(|&&word| self.is_shape_variant(text, word)).copied().collect()
    }

    /// Determine whether it is a variant of the shape and character
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

    /// Constructing a shape-size-word mapping table
    fn build_shape_map() -> HashMap<char, Vec<char>> {
        let mut map = HashMap::new();
        // Example: Add some common characters
        map.insert('赌', vec!['渧', '睹', '堵']);
        map.insert('博', vec!['搏', '傅', '膊']);
        map.insert('有', vec!['友', '右']);
        map.insert('色', vec!['涩']);
        map.insert('情', vec!['请', '清']);
        map
    }
}

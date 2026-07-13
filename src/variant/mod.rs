//! Variant (evasion) detection.
//!
//! [`VariantDetector`] catches sensitive words that have been obfuscated. Two channels run
//! in parallel and the results are merged and de-duplicated:
//!
//! - **Pinyin**: the text and each dictionary word are converted to tone-less pinyin; a word
//!   matches if its pinyin appears as a substring (e.g. `dubo` → `赌博`).
//! - **Shape** (形似字): characters are compared against a shape-confusable map loaded from
//!   `dict/shape_map.txt`, where each entry is a full equivalence class (e.g. `睹` ↔ `赌`).

use pinyin::Pinyin;
use std::collections::HashMap;

/// Variation detector
pub struct VariantDetector {
    pinyin_map: HashMap<String, Vec<String>>, // The mapping of pinyin to original word
    shape_map: HashMap<char, Vec<char>>,      // SHAPED CLOSE CHARACTER MAPPING
    char_to_pinyin: HashMap<char, String>,    // Character to pinyin mapping
}

impl std::fmt::Debug for VariantDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VariantDetector")
            .field("pinyin_map_size", &self.pinyin_map.len())
            .field("shape_map_size", &self.shape_map.len())
            .field("char_to_pinyin_size", &self.char_to_pinyin.len())
            .finish()
    }
}

impl Default for VariantDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl VariantDetector {
    /// Create a new detector
    ///
    /// The detector starts empty; register words with [`VariantDetector::add_word`] before
    /// calling [`VariantDetector::detect`].
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::VariantDetector;
    ///
    /// let mut vd = VariantDetector::new();
    /// vd.add_word("赌博");
    /// assert_eq!(vd.detect("dubo", &["赌博"]), vec!["赌博"]); // pinyin variant
    /// ```
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
    ///
    /// Returns the subset of `original_words` whose pinyin or shape variant appears in `text`.
    /// The returned slices borrow from `original_words`.
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::VariantDetector;
    ///
    /// let mut vd = VariantDetector::new();
    /// vd.add_word("赌博");
    /// // Shape variant: 睹 is shape-confusable with 赌.
    /// assert_eq!(vd.detect("睹博", &["赌博"]), vec!["赌博"]);
    /// ```
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
    ///
    /// Loaded from the embedded `dict/shape_map.txt`. Each non-comment line is one
    /// shape-confusable equivalence class (`key:v1,v2,...`); every character in the
    /// line is treated as confusable with every other (bidirectional), so evasions
    /// are caught in both directions. Overlapping lines union naturally.
    fn build_shape_map() -> HashMap<char, Vec<char>> {
        let mut map: HashMap<char, Vec<char>> = HashMap::new();

        for line in include_str!("../../dict/shape_map.txt").lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Format: `key:v1,v2,...` — only the first char of key/value is used.
            let Some((key_part, vals_part)) = line.split_once(':') else { continue };
            let Some(key) = key_part.trim().chars().next() else { continue };
            let group: Vec<char> =
                std::iter::once(key).chain(vals_part.split(',').filter_map(|s| s.trim().chars().next())).collect();
            if group.len() < 2 {
                continue;
            }

            // Full equivalence class: every char maps to all the others.
            for &c in &group {
                let others: Vec<char> = group.iter().filter(|&&x| x != c).copied().collect();
                map.entry(c).or_default().extend(others);
            }
        }

        // Lines may overlap on shared chars; dedup each value list.
        for vals in map.values_mut() {
            vals.sort_unstable();
            vals.dedup();
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pinyin_detection() {
        let mut vd = VariantDetector::new();
        vd.add_word("赌博");
        let results = vd.detect("dubo", &["赌博"]);
        assert_eq!(results, vec!["赌博"]);
    }

    #[test]
    fn test_pinyin_no_match() {
        let mut vd = VariantDetector::new();
        vd.add_word("赌博");
        let results = vd.detect("hello", &["赌博"]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_shape_variant_detection() {
        let mut vd = VariantDetector::new();
        vd.add_word("赌博");
        // "睹" is a shape variant of "赌"
        let results = vd.detect("睹博", &["赌博"]);
        assert_eq!(results, vec!["赌博"]);
    }

    #[test]
    fn test_shape_no_match_different_length() {
        let mut vd = VariantDetector::new();
        vd.add_word("赌博");
        let results = vd.detect("赌", &["赌博"]);
        assert!(results.is_empty()); // different length
    }

    #[test]
    fn test_shape_map_loaded_from_file() {
        // The shape map is loaded from dict/shape_map.txt and must cover 50+ groups.
        let vd = VariantDetector::new();
        assert!(vd.shape_map.len() >= 50, "shape_map has {} entries, expected >= 50", vd.shape_map.len());
        // The original hard-coded entries are preserved.
        assert!(vd.shape_map.get(&'赌').is_some_and(|v| v.contains(&'睹')));
        assert!(vd.shape_map.get(&'博').is_some_and(|v| v.contains(&'膊')));
    }

    #[test]
    fn test_shape_variant_bidirectional() {
        // Full equivalence-class symmetry: a word built from a "variant" char is
        // detected when the text uses the canonical char, and vice versa.
        let mut vd = VariantDetector::new();
        vd.add_word("睹博"); // text form of 赌博
        assert_eq!(vd.detect("赌博", &["睹博"]), vec!["睹博"]);
    }

    #[test]
    fn test_shape_group_full_symmetry() {
        // 人:入,八 is one equivalence class — every char matches every other.
        let mut vd = VariantDetector::new();
        vd.add_word("人");
        assert_eq!(vd.detect("入", &["人"]), vec!["人"]);
        assert_eq!(vd.detect("八", &["人"]), vec!["人"]);
    }

    #[test]
    fn test_empty_input() {
        let mut vd = VariantDetector::new();
        vd.add_word("赌博");
        let results = vd.detect("", &["赌博"]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_all_ascii_input() {
        let mut vd = VariantDetector::new();
        vd.add_word("赌博");
        let results = vd.detect("hello world", &["赌博"]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_mixed_script() {
        let mut vd = VariantDetector::new();
        vd.add_word("测试");
        let results = vd.detect("这是 test 内容", &["测试"]);
        // "test" pinyin doesn't match "测试"
        assert!(results.is_empty());
    }

    #[test]
    fn test_multiple_words() {
        let mut vd = VariantDetector::new();
        vd.add_word("赌博");
        vd.add_word("色情");
        let results = vd.detect("dubo and seqing", &["赌博", "色情"]);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_detect_dedup() {
        // A single original word can be matched by both the pinyin and shape
        // paths simultaneously; `detect` must sort+dedup so it appears once.
        let mut vd = VariantDetector::new();
        vd.add_word("赌博");
        let results = vd.detect("睹博", &["赌博"]);
        assert_eq!(results.len(), 1);
        assert_eq!(results, vec!["赌博"]);
    }

    #[test]
    fn test_detect_returns_borrowed_slices() {
        // detect returns references into the caller's `original_words` slice,
        // not freshly allocated Strings.
        let mut vd = VariantDetector::new();
        vd.add_word("赌博");
        let words = ["赌博"];
        let results = vd.detect("dubo", &words);
        assert!(results.iter().all(|r| words.contains(r)));
    }
}

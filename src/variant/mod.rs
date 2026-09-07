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
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};

/// Variation detector
pub struct VariantDetector {
    pinyin_map: HashMap<String, Vec<String>>, // The mapping of pinyin to original word
    shape_map: HashMap<char, Vec<char>>,      // SHAPED CLOSE CHARACTER MAPPING
    char_to_pinyin: HashMap<char, String>,    // Character to pinyin mapping
    /// Words registered via [`VariantDetector::add_word`], keyed by the polynomial rolling
    /// hash of their full pinyin key (Han chars → pinyin, every other char kept as-is).
    /// Detection hashes sliding windows of the text's pinyin and verifies candidates on
    /// hash hits, so the scan costs O(text × key_lengths) cheap integer ops instead of
    /// re-deriving each word's pinyin per call.
    pinyin_index: HashMap<u64, Vec<PinyinKeyEntry>>,
    /// Sorted, distinct char lengths of the registered pinyin keys (window widths).
    pinyin_key_lengths: Vec<u16>,
    /// Powers of the rolling-hash multiplier, `pinyin_pow[i] == MULT^i`.
    pinyin_pow: Vec<u64>,
    /// Every word registered for shape detection, with its char decomposition.
    shape_words: Vec<(String, Box<[char]>)>,
    /// Char → indices into `shape_words` whose first char equals the key or is
    /// shape-confusable with it; anchors the substring shape scan.
    shape_index: HashMap<char, SmallVec<[u32; 4]>>,
    /// Words registered via [`VariantDetector::add_word`]; unregistered words passed to
    /// [`VariantDetector::detect`] fall back to the legacy per-word computation.
    registered: HashSet<String>,
}

/// Rolling-hash multiplier for the pinyin window index (odd 64-bit constant).
const PINYIN_HASH_MULT: u64 = 0x100000001b3;

/// One registered pinyin key and the words that share it. `key` is verified against the
/// scanned window on hash hits, so hash collisions can only cost a comparison.
#[derive(Debug, Clone)]
struct PinyinKeyEntry {
    key: String,
    words: Vec<String>,
}

impl PinyinKeyEntry {
    /// Does `key` equal the `len` chars of `window` starting at `start`?
    fn key_matches(&self, window: &[char], start: usize, len: usize) -> bool {
        self.key.chars().count() == len && self.key.chars().eq(window[start..start + len].iter().copied())
    }
}

impl std::fmt::Debug for VariantDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VariantDetector")
            .field("pinyin_map_size", &self.pinyin_map.len())
            .field("shape_map_size", &self.shape_map.len())
            .field("char_to_pinyin_size", &self.char_to_pinyin.len())
            .field("pinyin_index_keys", &self.pinyin_index.len())
            .field("pinyin_key_lengths", &self.pinyin_key_lengths)
            .field("shape_words_size", &self.shape_words.len())
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
            pinyin_index: HashMap::new(),
            pinyin_key_lengths: Vec::new(),
            pinyin_pow: vec![1],
            shape_words: Vec::new(),
            shape_index: HashMap::new(),
            registered: HashSet::new(),
        }
    }

    /// Construct pinyin index when adding sensitive words
    pub fn add_word(&mut self, word: &str) {
        // One batch call gives (char, pinyin) pairs for every Han char; the result's
        // `text` field names the source char, so the pairing needs no position math.
        let result = Pinyin::chars(word).with_tone_style(pinyin::ToneStyle::None);
        let mut lookup: HashMap<char, String> = HashMap::with_capacity(result.len());
        for (pw, pinyin) in result.words().iter().zip(result.iter()) {
            if let Some(c) = pw.text.chars().next() {
                lookup.insert(c, pinyin.clone());
                self.char_to_pinyin.insert(c, pinyin);
            }
        }

        // Han-only key (legacy index).
        let pinyins: Vec<String> = word.chars().filter_map(|c| lookup.get(&c).cloned()).collect();
        if !pinyins.is_empty() {
            let pinyin_key = pinyins.join("");
            self.pinyin_map.entry(pinyin_key).or_default().push(word.to_string());
        }

        // Full key: Han chars → pinyin, everything else kept as-is. This mirrors exactly
        // what detection derives per word per call, so window lookups on it reproduce the
        // legacy substring semantics.
        let mut full_key = String::with_capacity(word.len());
        for c in word.chars() {
            match lookup.get(&c) {
                Some(p) => full_key.push_str(p),
                None => full_key.push(c),
            }
        }
        if !full_key.is_empty() {
            let key_len = full_key.chars().count() as u16;
            let hash = Self::rolling_hash(full_key.chars());
            let entries = self.pinyin_index.entry(hash).or_default();
            match entries.iter_mut().find(|entry| entry.key == full_key) {
                Some(entry) => entry.words.push(word.to_string()),
                None => entries.push(PinyinKeyEntry { key: full_key, words: vec![word.to_string()] }),
            }
            // Track the distinct window widths the scan must try.
            match self.pinyin_key_lengths.binary_search(&key_len) {
                Ok(_) => {}
                Err(pos) => self.pinyin_key_lengths.insert(pos, key_len),
            }
            self.ensure_pinyin_pow(key_len as usize);
        }

        // Shape index: index the word under its first char and every char shape-confusable
        // with it, so the scan only verifies windows anchored on a confusable first char.
        if let Some(first) = word.chars().next() {
            let idx = self.shape_words.len() as u32;
            self.shape_words.push((word.to_string(), word.chars().collect()));
            let mut keys: SmallVec<[char; 8]> = SmallVec::new();
            keys.push(first);
            if let Some(confusables) = self.shape_map.get(&first) {
                keys.extend(confusables.iter().copied());
            }
            for key in keys {
                self.shape_index.entry(key).or_default().push(idx);
            }
        }

        self.registered.insert(word.to_string());
    }

    /// Drop every registration, keeping only the built-in shape map.
    ///
    /// [`crate::Filter`] calls this before re-registering its whole pattern set, so the
    /// detector's active words always equal the engine's patterns (deleted words cannot
    /// resurface as variants).
    pub fn clear(&mut self) {
        self.pinyin_map.clear();
        self.char_to_pinyin.clear();
        self.pinyin_index.clear();
        self.pinyin_key_lengths.clear();
        self.pinyin_pow.clear();
        self.pinyin_pow.push(1);
        self.shape_words.clear();
        self.shape_index.clear();
        self.registered.clear();
    }

    /// Grow `pinyin_pow` so `pinyin_pow[len - 1]` exists (rolling-hash slide constant).
    fn ensure_pinyin_pow(&mut self, len: usize) {
        while self.pinyin_pow.len() < len {
            let last = *self.pinyin_pow.last().unwrap();
            self.pinyin_pow.push(last.wrapping_mul(PINYIN_HASH_MULT));
        }
    }

    /// Polynomial hash of a char sequence — matches the slide update in
    /// [`VariantDetector::pinyin_candidates`].
    fn rolling_hash(chars: impl Iterator<Item = char>) -> u64 {
        chars.fold(0u64, |h, c| h.wrapping_mul(PINYIN_HASH_MULT).wrapping_add(c as u64))
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
        let mut variants: Vec<&'a str> = Vec::new();

        // Registered fast path: index candidates filtered to the caller's word universe.
        let candidates = self.variant_candidates(text);
        if !candidates.is_empty() {
            let word_set: HashSet<&'a str> = original_words.iter().copied().collect();
            variants.extend(candidates.into_iter().filter_map(|word| word_set.get(word.as_str()).copied()));
        }

        // Words never registered through add_word keep the legacy per-word behavior.
        variants.extend(self.detect_unregistered(text, original_words));

        variants.sort_unstable();
        variants.dedup();
        variants
    }

    /// Internal hot path for [`crate::Filter`], which guarantees that every pattern was
    /// registered through [`VariantDetector::add_word`] and that deleted words were
    /// unregistered — so candidates need no filtering against a word universe. Returns
    /// the sorted, de-duplicated variant words (borrowed from the detector).
    pub(crate) fn detect_internal(&self, text: &str) -> Vec<&str> {
        let mut variants = self.variant_candidates(text);
        variants.sort_unstable();
        variants.dedup();
        variants.into_iter().map(|word| word.as_str()).collect()
    }

    /// First variant match, if any.
    ///
    /// Equivalent to `detect(text, original_words).first()` but tracks the minimum while
    /// scanning instead of collecting and sorting every hit.
    ///
    /// # Examples
    ///
    /// ```
    /// use sensitive_rs::VariantDetector;
    ///
    /// let mut vd = VariantDetector::new();
    /// vd.add_word("赌博");
    /// assert_eq!(vd.detect_first("dubo", &["赌博"]), Some("赌博"));
    /// assert_eq!(vd.detect_first("clean", &["赌博"]), None);
    /// ```
    pub fn detect_first<'a>(&'a self, text: &str, original_words: &[&'a str]) -> Option<&'a str> {
        let candidates = self.variant_candidates(text);
        let registered_min = if candidates.is_empty() {
            None
        } else {
            let word_set: HashSet<&'a str> = original_words.iter().copied().collect();
            candidates.into_iter().filter_map(|word| word_set.get(word.as_str()).copied()).min()
        };
        let unregistered_min = self.detect_unregistered(text, original_words).into_iter().min();
        match (registered_min, unregistered_min) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (found, None) => found,
            (None, found) => found,
        }
    }

    /// [`VariantDetector::detect_first`] restricted to registered words — the hot path
    /// used by [`crate::Filter::find_first_match`].
    pub(crate) fn detect_first_internal(&self, text: &str) -> Option<&str> {
        self.variant_candidates(text).into_iter().min().map(|word| word.as_str())
    }

    /// Pinyin + shape candidates from the indexes (unsorted, may repeat). Every returned
    /// word is an active registration.
    fn variant_candidates(&self, text: &str) -> Vec<&String> {
        let mut candidates = self.pinyin_candidates(text);
        candidates.extend(self.shape_candidates(text));
        candidates
    }

    /// Registered-word pinyin candidates: every indexed word whose full pinyin key appears
    /// as a substring of the text's pinyin. Scans fixed-width windows of the text pinyin
    /// against the key index — O(text × max_key_len) lookups instead of one substring
    /// search per word. Candidates borrow from the index; filter against the caller's
    /// word list before treating them as results.
    fn pinyin_candidates(&self, text: &str) -> Vec<&String> {
        let mut candidates = Vec::new();
        if self.pinyin_key_lengths.is_empty() {
            return candidates;
        }
        let text_pinyin = self.text_to_pinyin(text);
        let chars: Vec<char> = text_pinyin.chars().collect();
        if chars.is_empty() {
            return candidates;
        }

        // Sliding-window scan: for every registered key length, roll a polynomial hash
        // across the text pinyin (O(1) per slide, no per-window string) and verify only
        // on hash hits.
        for &len in &self.pinyin_key_lengths {
            let len = len as usize;
            if len > chars.len() {
                break; // lengths are sorted ascending; the rest are longer
            }
            let mut hash = Self::rolling_hash(chars[..len].iter().copied());
            let mut start = 0usize;
            loop {
                if let Some(entries) = self.pinyin_index.get(&hash) {
                    for entry in entries {
                        if entry.key_matches(&chars, start, len) {
                            candidates.extend(entry.words.iter());
                        }
                    }
                }
                if start + len >= chars.len() {
                    break;
                }
                hash = hash
                    .wrapping_sub((chars[start] as u64).wrapping_mul(self.pinyin_pow[len - 1]))
                    .wrapping_mul(PINYIN_HASH_MULT)
                    .wrapping_add(chars[start + len] as u64);
                start += 1;
            }
        }
        candidates
    }

    /// Registered-word shape candidates: every indexed word that appears somewhere in the
    /// text as a same-length window of pairwise-equal-or-confusable characters. Windows
    /// are anchored via the first-char index, so unrelated text positions are never
    /// scanned. Candidates borrow from the index.
    fn shape_candidates(&self, text: &str) -> Vec<&String> {
        let mut candidates = Vec::new();
        let text_chars: Vec<char> = text.chars().collect();

        for (i, text_char) in text_chars.iter().enumerate() {
            let Some(anchor_list) = self.shape_index.get(text_char) else { continue };
            for &idx in anchor_list {
                let Some((word, word_chars)) = self.shape_words.get(idx as usize) else { continue };
                let end = i + word_chars.len();
                if end > text_chars.len() {
                    continue;
                }
                if text_chars[i..end].iter().zip(word_chars.iter()).all(|(&tc, &wc)| {
                    tc == wc || self.shape_map.get(&wc).is_some_and(|confusables| confusables.contains(&tc))
                }) {
                    candidates.push(word);
                }
            }
        }
        candidates
    }

    /// Legacy handling for words never registered through [`VariantDetector::add_word`]
    /// (kept for API compatibility): pinyin is derived per call with unmapped chars
    /// passing through unchanged, and the shape check compares the whole text only when
    /// its length equals the word's.
    fn detect_unregistered<'a>(&'a self, text: &str, original_words: &[&'a str]) -> Vec<&'a str> {
        let mut variants = Vec::new();
        let unregistered: Vec<&'a str> =
            original_words.iter().copied().filter(|w| !self.registered.contains(*w)).collect();
        if unregistered.is_empty() {
            return variants;
        }

        // Legacy pinyin path.
        let text_pinyin = self.text_to_pinyin(text);
        for word in &unregistered {
            if word.is_empty() {
                continue;
            }
            let word_pinyin: String =
                word.chars().map(|c| self.char_to_pinyin.get(&c).cloned().unwrap_or_else(|| c.to_string())).collect();
            if text_pinyin.contains(&word_pinyin) {
                variants.push(*word);
            }
        }

        // Legacy shape path (whole text, equal length).
        let text_chars: Vec<char> = text.chars().collect();
        for word in &unregistered {
            let word_chars: Vec<char> = word.chars().collect();
            if word_chars.is_empty() || word_chars.len() != text_chars.len() {
                continue;
            }
            if text_chars.iter().zip(word_chars.iter()).all(|(&tc, &wc)| {
                tc == wc || self.shape_map.get(&wc).is_some_and(|confusables| confusables.contains(&tc))
            }) {
                variants.push(*word);
            }
        }

        variants
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
    fn test_shape_variant_substring_in_longer_text() {
        // Shape variants are now found as substrings of longer text, not only when the
        // whole text is exactly the word's length. 人 is shape-confusable with 八, so
        // "八卦" surfaces inside "这是人卦".
        let mut vd = VariantDetector::new();
        vd.add_word("八卦");
        let words = ["八卦"];
        assert!(vd.detect("这是人卦", &words).contains(&"八卦"));
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

    #[test]
    fn test_detect_first_matches_detect_minimum() {
        // detect_first must agree with detect().first() (the lexicographic minimum).
        let mut vd = VariantDetector::new();
        vd.add_word("赌博");
        vd.add_word("赌博机");
        let words = ["赌博", "赌博机"];
        assert_eq!(vd.detect_first("dubo", &words), vd.detect("dubo", &words).first().copied());
        assert_eq!(vd.detect_first("clean", &words), None);
    }

    #[test]
    fn test_unregistered_words_keep_legacy_behavior() {
        // A word never registered through add_word keeps the legacy behavior: ASCII
        // passes through on both sides so it is found verbatim, while Han chars of the
        // unregistered word are never converted, so they cannot match converted text.
        let vd = VariantDetector::new();
        let words = ["spam"];
        assert_eq!(vd.detect("hello spam", &words), vec!["spam"]);
        assert!(vd.detect("无关内容", &words).is_empty());

        let han_words = ["反贪"];
        assert!(vd.detect("这是反贪内容", &han_words).is_empty());
    }
}

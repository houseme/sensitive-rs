use crate::trie::Trie;
use regex::Regex;
#[cfg(feature = "net")]
use reqwest::blocking::Client;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
#[cfg(feature = "net")]
use std::time::Duration;

/// A filter that can be used to filter out sensitive words from text.
/// The filter is case-insensitive.
/// The filter is fast and efficient.
/// # Example
/// ```
/// use crate::Filter;
///
/// let mut filter = Filter::new();
/// filter.add_word("bad");
/// filter.add_word("worse");
///
/// assert_eq!(filter.find_in("This is bad."), (true, "bad".to_string()));
/// assert_eq!(filter.find_in("This is worse."), (true, "worse".to_string()));
/// assert_eq!(filter.find_in("This is good."), (false, "".to_string()));
/// ```
/// # Safety
/// The filter is not thread-safe.
/// # Examples
/// ```
/// use sensitive_rs::Filter;
///
/// let mut filter = Filter::new();
/// filter.add_word("bad");
/// filter.add_word("worse");
///
/// assert_eq!(filter.find_in("This is bad."), (true, "bad".to_string()));
/// assert_eq!(filter.find_in("This is worse."), (true, "worse".to_string()));
/// assert_eq!(filter.find_in("This is good."), (false, "".to_string()));
/// ```
/// # Panics
/// Panics if the noise pattern is invalid.
/// # Errors
/// Returns an error if the noise pattern is invalid.
/// # Safety
/// The noise pattern must be a valid regular expression.
/// # Examples
/// ```
/// use sensitive_rs::Filter;
///
/// let mut filter = Filter::new();
/// filter.update_noise_pattern(r"[\|\s&%$@*]+");
/// ```
/// # Panics
/// Panics if the pattern is invalid.
/// # Errors
/// Returns an error if the pattern is invalid.
/// # Arguments
/// * `pattern` - A regular expression pattern.
/// # Returns
/// * `()` - Returns nothing.
/// # Safety
/// The pattern must be a valid regular expression.
pub struct Filter {
    trie: Trie,
    noise: Regex,
}

impl Filter {
    /// Create a new filter.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    /// let mut filter = Filter::new();
    /// ```
    /// # Safety
    /// The filter is not thread-safe.
    /// # Examples
    /// ```
    /// use sensitive_rs::Filter;
    /// let mut filter = Filter::new();
    /// ```
    pub fn new() -> Self {
        Filter { trie: Trie::new(), noise: Regex::new(r"[\|\s&%$@*]+").unwrap() }
    }

    /// Create a new filter and load the default dictionary.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let filter = Filter::with_default_dict().unwrap();
    /// ```
    /// # Returns
    /// * `Result<Self, io::Error>` - Returns the filter object or an error if the dictionary cannot be loaded.
    /// # Errors
    /// Returns an error if the dictionary cannot be loaded.
    /// # Safety
    /// The dictionary must be valid.
    pub fn with_default_dict() -> Result<Self, io::Error> {
        let mut filter = Filter::new();
        filter.load_word_dict("dict/dict.txt")?;
        Ok(filter)
    }

    /// Update the noise pattern used to remove unwanted characters from the text.
    /// The default pattern is r"[\|\s&%$@*]+".
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    /// let mut filter = Filter::new();
    /// filter.update_noise_pattern(r"[\|\s&%$@*]+");
    /// ```
    /// # Panics
    /// Panics if the pattern is invalid.
    /// # Errors
    /// Returns an error if the pattern is invalid.
    /// # Arguments
    /// * `pattern` - A regular expression pattern.
    /// # Returns
    /// * `()` - Returns nothing.
    /// # Safety
    /// The pattern must be a valid regular expression.
    /// # Examples
    /// ```
    /// use keyword_filter::Filter;
    /// let mut filter = Filter::new();
    /// filter.update_noise_pattern(r"[\|\s&%$@*]+");
    /// ```
    pub fn update_noise_pattern(&mut self, pattern: &str) {
        self.noise = Regex::new(pattern).unwrap();
    }

    /// Load a word dictionary from a file.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.load_word_dict("dict.txt").unwrap();
    /// ```
    /// # Arguments
    /// * `path` - The path to the word dictionary file.
    /// # Returns
    /// * `()` - Returns nothing.
    /// # Errors
    /// Returns an error if the file cannot be opened.
    pub fn load_word_dict(&mut self, path: &str) -> io::Result<()> {
        let file = File::open(path)?;
        self.load(file)
    }

    /// Load a word dictionary from a URL.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.load_net_word_dict("https://example.com/dict.txt").unwrap();
    /// ```
    /// # Arguments
    /// * `url` - The URL to the word dictionary file.
    /// # Returns
    /// * `()` - Returns nothing.
    /// # Errors
    /// Returns an error if the URL cannot be opened.
    /// # Safety
    /// The URL must be valid.
    #[cfg(feature = "net")]
    pub fn load_net_word_dict(&mut self, url: &str) -> Result<(), io::Error> {
        let client = Client::builder().timeout(Duration::from_secs(5)).build().unwrap();
        let response = client.get(url).send().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        self.load(response)
    }

    /// Load a word dictionary from a reader.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    /// use std::fs::File;
    ///
    /// let mut filter = Filter::new();
    /// filter.load(File::open("dict.txt").unwrap()).unwrap();
    /// ```
    /// # Arguments
    /// * `reader` - A reader that implements the Read trait.
    /// # Returns
    /// * `()` - Returns nothing.
    /// # Errors
    /// Returns an error if the reader cannot be read.
    /// # Safety
    /// The reader must be valid.
    pub fn load<R: io::Read>(&mut self, reader: R) -> io::Result<()> {
        let buf_reader = BufReader::new(reader);
        for line in buf_reader.lines() {
            if let Ok(word) = line {
                self.trie.add_word(word.as_str());
            }
        }
        Ok(())
    }

    /// Add a word to the filter.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_word("apple");
    /// ```
    /// # Arguments
    /// * `words` - The word to add to the filter.
    /// # Returns
    /// * `()` - Returns nothing.
    /// # Safety
    /// The word must be valid.
    pub fn add_word(&mut self, words: &str) {
        self.trie.add_word(words);
    }

    /// Add words to the filter.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_words(&["apple", "app", "banana"]);
    /// ```
    ///
    /// # Arguments
    /// * `words` - The words to add to the filter.
    /// # Returns
    /// * `()` - Returns nothing.
    /// # Safety
    /// The words must be valid.
    pub fn add_words(&mut self, words: &[&str]) {
        for i in 0..words.len() {
            self.trie.add_word(words[i]);
        }
    }

    /// Delete a word from the filter.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_word("apple");
    /// filter.del_word("apple");
    /// ```
    /// # Arguments
    /// * `words` - The word to delete from the filter.
    /// # Returns
    /// * `()` - Returns nothing.
    /// # Safety
    /// The word must be valid.
    /// # Errors
    /// Returns an error if the word is not found.
    pub fn del_word(&mut self, words: &str) {
        self.trie.del_word(words);
    }

    /// Delete words from the filter.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let mut filter = Filter::new();
    /// filter.add_words(&["apple", "app", "banana"]);
    /// filter.del_words(&["app"]);
    /// ```
    /// # Arguments
    /// * `words` - The words to delete from the filter.
    /// # Returns
    /// * `()` - Returns nothing.
    /// # Safety
    /// The words must be valid.
    /// # Errors
    /// Returns an error if the word is not found.
    pub fn del_words(&mut self, words: &[&str]) {
        for i in 0..words.len() {
            self.trie.del_word(words[i]);
        }
    }

    /// Filter words from a string.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let filter = Filter::new();
    /// filter.add_words(&["apple", "app", "banana"]);
    ///
    /// assert_eq!(filter.filter("I have an apple and a banana"), "I have an and a");
    /// ```
    /// # Arguments
    /// * `text` - The text to filter words from.
    /// # Returns
    /// * `String` - The text with words filtered out.
    /// # Safety
    /// The text must be valid.
    pub fn filter(&self, text: &str) -> String {
        self.trie.filter(text)
    }

    /// Replace words in a string.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let filter = Filter::new();
    /// filter.add_words(&["apple", "app", "banana"]);
    ///
    /// assert_eq!(filter.replace("I have an apple and a banana", '*'), "I have an ***** and a ******");
    /// ```
    /// # Arguments
    /// * `text` - The text to replace words in.
    /// * `repl` - The character to replace words with.
    /// # Returns
    /// * `String` - The text with words replaced.
    ///
    /// # Safety
    /// The text must be valid.
    pub fn replace(&self, text: &str, repl: char) -> String {
        self.trie.replace(text, repl)
    }

    /// Find a word in a string.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let filter = Filter::new();
    /// filter.add_words(&["apple", "app", "banana"]);
    ///
    /// assert_eq!(filter.find_in("I have an apple and a banana"), (true, "apple".to_string()));
    /// ```
    /// # Arguments
    /// * `text` - The text to find the word in.
    /// # Returns
    /// * `(bool, String)` - A tuple containing a boolean and the word found.
    /// # Safety
    /// The text must be valid.
    ///
    /// # Errors
    /// Returns an error if the word is not found.
    /// # Panics
    /// Panics if the word is not found.
    pub fn find_in(&self, text: &str) -> (bool, String) {
        let text = self.remove_noise(text);
        let result = self.trie.find_in(&text);
        if let Some(word) = result {
            (true, word.to_string())
        } else {
            (false, "".to_string())
        }
    }

    /// Find all words in a string.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let filter = Filter::new();
    /// filter.add_words(&["apple", "app", "banana"]);
    ///
    /// assert_eq!(filter.find_all("I have an apple and a banana"), vec!["apple", "banana"]);
    /// ```
    /// # Arguments
    /// * `text` - The text to find words in.
    /// # Returns
    /// * `Vec<String>` - A vector containing the words found.
    /// # Safety
    /// The text must be valid.
    /// # Errors
    /// Returns an error if the word is not found.
    /// # Panics
    /// Panics if the word is not found.
    pub fn find_all(&self, text: &str) -> Vec<String> {
        self.trie.find_all(text)
    }

    /// Validate a string.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let filter = Filter::new();
    /// filter.add_words(&["apple", "app", "banana"]);
    ///
    /// assert_eq!(filter.validate("I have an apple and a banana"), (false, "apple".to_string()));
    /// ```
    /// # Arguments
    /// * `text` - The text to validate.
    /// # Returns
    /// * `(bool, String)` - A tuple containing a boolean and the word found.
    /// # Safety
    /// The text must be valid.
    /// # Errors
    /// Returns an error if the word is not found.
    /// # Panics
    /// Panics if the word is not found.
    pub fn validate(&self, text: &str) -> (bool, String) {
        let text = self.remove_noise(text);
        let result = self.trie.validate(text.as_str());
        if let Some(word) = result {
            (true, word.to_string())
        } else {
            (false, "".to_string())
        }
    }

    /// Remove unwanted characters from the text.
    /// # Example
    /// ```
    /// use sensitive_rs::Filter;
    ///
    /// let filter = Filter::new();
    /// filter.update_noise_pattern(r"[^\w]");
    ///
    /// assert_eq!(filter.remove_noise("I |have& %an$ @apple*"), "Ihaveanapple");
    /// ```
    /// # Arguments
    /// * `text` - The text to remove unwanted characters from.
    /// # Returns
    /// * `String` - The text with unwanted characters removed.
    /// # Safety
    /// The noise pattern must be a valid regular expression.
    /// # Panics
    /// Panics if the noise pattern is invalid.
    /// # Errors
    /// Returns an error if the noise pattern is invalid.
    pub fn remove_noise(&self, text: &str) -> String {
        self.noise.replace_all(text, "").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_find_in() {
        let mut filter = Filter::new();
        filter.add_words(&["apple", "app", "banana"]);

        assert_eq!(filter.find_in("apple"), (true, "apple".to_string()));
        assert_eq!(filter.find_in("app"), (true, "app".to_string()));
        assert_eq!(filter.find_in("appl"), (false, "app".to_string()));
        assert_eq!(filter.find_in("banana"), (true, "banana".to_string()));
    }

    #[test]
    fn test_del() {
        let mut filter = Filter::new();
        filter.add_words(&["apple", "app", "banana"]);

        filter.del_words(&["app"]);
        assert_eq!(filter.find_in("app"), (false, "".to_string()));
        assert_eq!(filter.find_in("apple"), (true, "apple".to_string()));
    }

    #[test]
    fn test_update_noise_pattern() {
        let mut filter = Filter::new();
        filter.update_noise_pattern(r"[^\w]");
        assert_eq!(filter.remove_noise("I |have& %an$ @apple*"), "Ihaveanapple");
    }

    #[test]
    fn test_add_word() {
        let mut filter = Filter::new();
        filter.add_word("apple");
        assert_eq!(filter.find_in("I have an apple"), (true, "apple".to_string()));
    }

    #[test]
    fn test_del_word() {
        let mut filter = Filter::new();
        filter.add_word("apple");
        filter.del_word("apple");
        assert_eq!(filter.find_in("I have an apple"), (false, "".to_string()));
    }

    #[test]
    fn test_replace() {
        let mut filter = Filter::new();
        filter.add_words(&["apple", "app", "banana"]);

        assert_eq!(filter.replace("I have an apple and a banana", '*'), "I have an ***** and a ******");
    }

    #[test]
    fn test_filter() {
        let mut filter = Filter::new();
        filter.add_words(&["apple", "app", "banana"]);

        assert_eq!(filter.filter("I have an apple and a banana"), "I have an and a");
    }

    #[test]
    fn test_validate() {
        let mut filter = Filter::new();
        filter.add_words(&["apple", "app", "banana"]);

        assert_eq!(filter.validate("I have an apple and a banana"), (false, "apple".to_string()));
        assert_eq!(filter.validate("I have an orange and a banana"), (true, "".to_string()));
    }

    #[test]
    fn test_find_all() {
        let mut filter = Filter::new();
        filter.add_words(&["apple", "app", "banana"]);

        assert_eq!(filter.find_all("I have an apple and a banana"), vec!["apple", "banana"]);
    }

    #[test]
    fn test_remove_noise() {
        let filter = Filter::new();
        assert_eq!(filter.remove_noise("I |have& %an$ @apple*"), "Ihaveanapple");
    }

    #[test]
    fn test_with_default_dict() {
        let filter = Filter::with_default_dict().unwrap();

        // 假设字典文件中包含 "apple" 这个单词
        assert_eq!(filter.find_in("apple"), (true, "apple".to_string()));
    }

    #[test]
    fn test_load_word_dict() {
        let mut filter = Filter::new();
        filter.load_word_dict("dict/dict.txt").unwrap();
        filter.add_word("apple");
        assert_eq!(filter.find_in("apple"), (true, "apple".to_string()));
    }

    #[test]
    #[cfg(feature = "net")]
    fn test_load_net_word_dict() {
        let mut filter = Filter::new();
        filter.load_net_word_dict("https://raw.githubusercontent.com/houseme/sensitive-rs/main/dict/dict.txt").unwrap();
        assert_eq!(filter.find_in("apple"), (true, "apple".to_string()));
    }
}

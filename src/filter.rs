use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead};
use std::time::SystemTime;

use crate::trie::Trie;

pub struct Filter {
    trie: Trie,
    noise: Regex,
    build_ver: SystemTime,
    updated_ver: SystemTime,
}

impl Filter {
    pub fn new() -> Self {
        Filter {
            trie: Trie::new(),
            noise: Regex::new(r"[\|\s&%$@*]+").unwrap(),
            build_ver: SystemTime::UNIX_EPOCH,
            updated_ver: SystemTime::UNIX_EPOCH,
        }
    }

    pub fn update_noise_pattern(&mut self, pattern: &str) {
        self.noise = Regex::new(pattern).unwrap();
    }

    pub fn load_word_dict(&mut self, path: &str) -> io::Result<()> {
        let file = File::open(path)?;
        let reader = io::BufReader::new(file);
        for line in reader.lines() {
            if let Ok(word) = line {
                self.add_word(&word);
            }
        }
        Ok(())
    }

    pub fn add_word(&mut self, word: &str) {
        self.trie.add(&[word]);
        self.updated_ver = SystemTime::now();
    }

    pub fn filter(&mut self, text: &str) -> String {
        self.update_failure_link();
        self.trie.filter(text)
    }

    fn update_failure_link(&mut self) {
        if self.build_ver != self.updated_ver {
            self.trie.build_failure_links();
            self.build_ver = self.updated_ver;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_load_word_dict() {
        let mut filter = Filter::new();
        filter.load_word_dict("test_dict.txt").unwrap();
        // Add assertions to verify loaded words
    }

    #[test]
    fn test_filter_add_word() {
        let mut filter = Filter::new();
        filter.add_word("hello");
        filter.add_word("world");
        // Add assertions to verify added words
    }

    #[test]
    fn test_filter_filter() {
        let mut filter = Filter::new();
        filter.add_word("hello");
        let filtered = filter.filter("hello world");
        assert_eq!(filtered, "***** world");
    }
}

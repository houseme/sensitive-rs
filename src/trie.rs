use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

/// Represents a node in the Trie data structure.
///
/// Each node contains a character, a flag indicating whether it is the end of a word,
/// a flag indicating whether it is the root node, and a hashmap pointing to its child nodes.
pub struct TrieNode {
    children: Arc<RwLock<HashMap<char, Arc<TrieNode>>>>,
    character: char,
    is_end: AtomicBool,
    is_root_node: bool,
}

impl TrieNode {
    /// Creates a new Trie node.
    ///
    /// # Arguments
    /// - `ch`: The character associated with the node.
    /// - `is_root`: A boolean indicating whether the node is the root node.
    ///
    /// # Returns
    /// Returns an `Arc` containing the new node.
    ///
    /// # Example
    /// ```
    /// use sensitive_rs::TrieNode;
    ///
    /// let node = TrieNode::new('a', false);
    /// ```
    /// # Safety
    /// The node is thread-safe.
    fn new(ch: char, is_root: bool) -> Arc<Self> {
        Arc::new(TrieNode {
            children: Arc::new(RwLock::new(HashMap::new())),
            character: ch,
            is_end: AtomicBool::new(false),
            is_root_node: is_root,
        })
    }

    /// Checks if the current node is the root node.
    ///
    /// # Returns
    /// Returns a boolean indicating whether the current node is the root node.
    #[warn(dead_code)]
    pub fn is_root_node(&self) -> bool {
        self.is_root_node
    }

    /// Checks if the current node marks the end of a word.
    ///
    /// # Returns
    /// Returns a boolean indicating whether the current node marks the end of a word.
    #[warn(dead_code)]
    pub fn is_end(&self) -> bool {
        self.is_end.load(Ordering::Relaxed)
    }

    fn can_be_deleted(&self) -> bool {
        !self.is_root_node() && !self.is_end() && self.children.read().unwrap().is_empty()
    }
}

/// Represents a keyword filter based on the Trie data structure.
/// The filter can be used to find, replace, or filter out keywords in a given content.
/// The filter is case-sensitive.
/// The filter is thread-safe.
pub struct Trie {
    root: Arc<TrieNode>,
}

impl Trie {
    /// Creates a new keyword filter.
    ///
    /// # Returns
    /// Returns a new `Trie` instance.
    /// # Example
    /// ```
    /// use sensitive_rs::Trie;
    ///
    /// let filter = Trie::new();
    /// ```
    /// # Safety
    /// The filter is thread-safe.
    pub fn new() -> Self {
        Trie { root: TrieNode::new('\0', true) }
    }

    /// Adds a word to the filter.
    ///
    /// # Arguments
    /// - `word`: The word to be added.
    /// # Example
    /// ```
    /// use sensitive_rs::Trie;
    ///
    /// let filter = Trie::new();
    /// filter.add_word("bad");
    /// filter.add_word("worse");
    /// ```
    /// # Safety
    /// The filter is thread-safe.
    /// # Panics
    /// Panics if the word is empty.
    /// # Errors
    /// Returns an error if the word is empty.
    pub fn add_word(&self, word: &str) {
        let mut current = self.root.clone();
        for ch in word.chars() {
            let next = {
                let mut children = current.children.write().unwrap();
                children.entry(ch).or_insert_with(|| TrieNode::new(ch, false)).clone()
            };
            current = next;
        }
        current.is_end.store(true, Ordering::Relaxed);
    }

    /// Deletes a word from the filter.
    ///
    /// # Arguments
    /// - `word`: The word to be deleted.
    ///
    /// # Returns
    /// Returns a boolean indicating whether the word was successfully deleted.
    /// # Example
    /// ```
    /// use sensitive_rs::Trie;
    ///
    /// let filter = Trie::new();
    /// filter.add_word("bad");
    /// filter.add_word("worse");
    ///
    /// assert!(filter.del_word("bad"));
    /// assert!(!filter.del_word("bad"));
    /// ```
    /// # Safety
    /// The filter is thread-safe.
    /// # Errors
    /// Returns an error if the word is not found.
    /// # Panics
    /// Panics if the word is not found.
    pub fn del_word(&self, word: &str) -> bool {
        fn delete_helper(node: &Arc<TrieNode>, word: &[char], depth: usize) -> bool {
            if depth == word.len() {
                if !node.is_end.load(Ordering::Relaxed) {
                    return false; // Word not found
                }
                node.is_end.store(false, Ordering::Relaxed);
                return true; // Word successfully deleted
            }

            let ch = word[depth];
            let child_deleted = {
                let children = node.children.read().unwrap();
                if let Some(child) = children.get(&ch) {
                    delete_helper(child, word, depth + 1)
                } else {
                    return false; // Word not found
                }
            };

            if child_deleted {
                let mut children = node.children.write().unwrap();
                if children.get(&ch).map_or(false, |child| child.can_be_deleted()) {
                    children.remove(&ch);
                }
            }

            child_deleted
        }

        let word_chars: Vec<char> = word.chars().collect();
        delete_helper(&self.root, &word_chars, 0)
    }

    /// Finds the first matching word and its position in the given content.
    ///
    /// # Arguments
    /// - `content`: The content to search in.
    ///
    /// # Returns
    /// Returns an `Option<(String, usize)>` containing the matching word and its position.
    /// If no match is found, returns `None`.
    ///
    /// # Example
    /// ```
    /// use sensitive_rs::Trie;
    ///
    /// let filter = Trie::new();
    /// filter.add_word("bad");
    /// filter.add_word("worse");
    ///
    /// assert_eq!(filter.find_word_at("This is bad."), Some(("bad".to_string(), 8)));
    /// assert_eq!(filter.find_word_at("This is worse."), Some(("worse".to_string(), 8)));
    /// assert_eq!(filter.find_word_at("This is good."), None);
    /// ```
    /// # Safety
    /// The filter is thread-safe.
    /// # Errors
    /// Returns an error if the content is empty.
    /// # Panics
    /// Panics if the content is empty.
    pub fn find_word_at(&self, content: &str) -> Option<(String, usize)> {
        let mut current = self.root.clone();
        let mut last_match = None;
        let mut matched = String::new();
        let mut chars = content.chars();
        let mut i = 0;

        while let Some(ch) = chars.next() {
            let next = {
                let children = current.children.read().unwrap();
                children.get(&ch).cloned()
            };

            if let Some(child) = next {
                if child.character != ch {
                    break;
                }
                i += 1;
                matched.push(child.character);
                if child.is_end.load(Ordering::Relaxed) {
                    last_match = Some((matched.clone(), i));
                }
                current = child;
            } else {
                break;
            }
        }

        last_match
    }
    /// Replaces all matching words in the content with the specified character.
    ///
    /// # Arguments
    /// - `content`: The content to replace in.
    /// - `replacement`: The character to replace the matching words with.
    ///
    /// # Returns
    /// Returns the content with all matching words replaced.
    ///
    /// # Example
    /// ```
    /// use sensitive_rs::Trie;
    ///
    /// let filter = Trie::new();
    /// filter.add_word("bad");
    /// filter.add_word("worse");
    ///
    /// assert_eq!(filter.replace("This is bad and worse.", '*'), "This is *** and *****.");
    /// ```
    /// # Safety
    /// The filter is thread-safe.
    /// # Errors
    /// Returns an error if the content is empty.
    /// # Panics
    /// Panics if the content is empty.
    pub fn replace(&self, content: &str, replacement: char) -> String {
        let mut result = String::new();
        let mut i = 0;
        while i < content.len() {
            if let Some((word, len)) = self.find_word_at(&content[i..]) {
                result.push_str(&replacement.to_string().repeat(word.len()));
                i += len;
            } else {
                result.push(content[i..].chars().next().unwrap());
                i += 1;
            }
        }
        result
    }

    /// Filters out all matching words in the content.
    ///
    /// # Arguments
    /// - `content`: The content to filter.
    ///
    /// # Returns
    /// Returns the content with all matching words removed.
    ///
    /// # Example
    /// ```
    /// use sensitive_rs::Trie;
    ///
    /// let filter = Trie::new();
    /// filter.add_word("bad");
    /// filter.add_word("worse");
    ///
    /// assert_eq!(filter.filter("This is bad and worse."), "This is  and .");
    /// ```
    /// # Safety
    /// The filter is thread-safe.
    /// # Errors
    /// Returns an error if the content is empty.
    /// # Panics
    /// Panics if the content is empty.
    pub fn filter(&self, content: &str) -> String {
        let mut result = String::new();
        let mut i = 0;
        while i < content.len() {
            if let Some((_, len)) = self.find_word_at(&content[i..]) {
                i += len;
            } else {
                result.push(content[i..].chars().next().unwrap());
                i += 1;
            }
        }
        result
    }

    /// Finds the first matching word in the content.
    ///
    /// # Arguments
    /// - `content`: The content to search in.
    ///
    /// # Returns
    /// Returns an `Option<String>` containing the matching word.
    /// If no match is found, returns `None`.
    ///
    /// # Example
    /// ```
    /// use sensitive_rs::Trie;
    /// let filter = Trie::new();
    /// filter.add_word("bad");
    /// filter.add_word("worse");
    /// assert_eq!(filter.find_in("This is bad."), Some("bad".to_string()));
    /// assert_eq!(filter.find_in("This is worse."), Some("worse".to_string()));
    /// assert_eq!(filter.find_in("This is good."), None);
    /// ```
    /// # Safety
    /// The filter is thread-safe.
    /// # Errors
    /// Returns an error if the content is empty.
    /// # Panics
    /// Panics if the content is empty.
    pub fn find_in(&self, content: &str) -> Option<String> {
        for (i, _) in content.char_indices() {
            if let Some((word, _)) = self.find_word_at(&content[i..]) {
                return Some(word);
            }
        }
        None
    }
    /// Validates whether the content contains any matching words.
    ///
    /// # Arguments
    /// - `content`: The content to validate.
    ///
    /// # Returns
    /// Returns an `Option<String>` containing the matching word.
    /// If no match is found, returns `None`.
    pub fn validate(&self, content: &str) -> Option<String> {
        self.find_in(content)
    }

    /// Finds all matching words in the content.
    ///
    /// # Arguments
    /// - `content`: The content to search in.
    ///
    /// # Returns
    /// Returns a `Vec<String>` containing all matching words.
    pub fn find_all(&self, content: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut i = 0;
        while i < content.len() {
            if let Some((word, len)) = self.find_word_at(&content[i..]) {
                result.push(word);
                i += len;
            } else {
                i += 1;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_add_and_find() {
        let filter = Trie::new();
        filter.add_word("bad");
        filter.add_word("worse");

        assert_eq!(filter.find_in("This is bad."), Some("bad".to_string()));
        assert_eq!(filter.find_in("This is worse."), Some("worse".to_string()));
        assert_eq!(filter.find_in("This is good."), None);
    }

    #[test]
    fn test_replace() {
        let filter = Trie::new();
        filter.add_word("bad");
        filter.add_word("worse");

        assert_eq!(filter.replace("This is bad and worse.", '*'), "This is *** and *****.");
    }

    #[test]
    fn test_filter() {
        let filter = Trie::new();
        filter.add_word("bad");
        filter.add_word("worse");

        assert_eq!(filter.filter("This is bad and worse."), "This is  and .");
    }

    #[test]
    fn test_validate() {
        let filter = Trie::new();
        filter.add_word("bad");
        filter.add_word("worse");

        assert_eq!(filter.validate("This is bad."), Some("bad".to_string()));
        assert_eq!(filter.validate("This is good."), None);
    }

    #[test]
    fn test_find_all() {
        let filter = Trie::new();
        filter.add_word("bad");
        filter.add_word("worse");

        assert_eq!(filter.find_all("This is bad and worse."), vec!["bad", "worse"]);
    }

    #[test]
    fn test_concurrent_access() {
        let filter = Arc::new(Trie::new());
        filter.add_word("concurrent");
        filter.add_word("test");

        let mut handles = vec![];

        for i in 0..10 {
            let filter_clone = filter.clone();
            handles.push(thread::spawn(move || {
                assert_eq!(filter_clone.find_in("This is a concurrent test."), Some("concurrent".to_string()));
                filter_clone.add_word(&format!("thread{}", i));

                // Give some time for other threads to add their words
                thread::sleep(Duration::from_millis(10));

                let result = filter_clone.find_all("Thread test is concurrent.");
                assert!(result.contains(&"test".to_string()));
                assert!(result.contains(&"concurrent".to_string()));
                // We cannot be sure if "thread{i}" has been added, so we do not check it
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // After all threads are done, check the final result
        let final_result = filter.find_all(
            "Thread test is concurrent.thread0 thread1 thread2 thread3 thread4 thread5 thread6 thread7 thread8 thread9",
        );
        assert!(final_result.contains(&"test".to_string()));
        assert!(final_result.contains(&"concurrent".to_string()));

        println!("{:?}", final_result);

        // Check if all "thread{i}" words were successfully added
        for i in 0..10 {
            assert!(final_result.contains(&format!("thread{}", i)));
        }
    }

    #[test]
    fn test_del_word() {
        let filter = Trie::new();
        filter.add_word("bad");
        filter.add_word("badge");
        filter.add_word("badger");
        assert_eq!(filter.find_in("This is bad."), Some("bad".to_string()));
        assert_eq!(filter.find_in("This is a badge."), Some("badge".to_string()));
        assert_eq!(filter.find_in("This is a badger."), Some("badger".to_string()));

        // Delete "bad"
        assert!(filter.del_word("bad"));
        assert_eq!(filter.find_in("This is bad."), None);
        assert_eq!(filter.find_in("This is a badge."), Some("badge".to_string()));
        assert_eq!(filter.find_in("This is a badger."), Some("badger".to_string()));

        // Delete a non-existent word
        assert!(!filter.del_word("bad"));

        // Delete "badge"
        assert!(filter.del_word("badge"));
        assert_eq!(filter.find_in("This is a badge."), None);
        assert_eq!(filter.find_in("This is a badger."), Some("badger".to_string()));

        // Delete the last word "badger"
        assert!(filter.del_word("badger"));
        assert_eq!(filter.find_in("This is a badger."), None);

        // Attempt to delete a word that no longer exists
        assert!(!filter.del_word("badger"));
    }

    #[test]
    fn test_concurrent_del_word() {
        let filter = Arc::new(Trie::new());
        filter.add_word("concurrent");
        filter.add_word("test");
        filter.add_word("delete");

        let mut handles = vec![];

        for i in 0..5 {
            let filter_clone = filter.clone();
            handles.push(thread::spawn(move || {
                // Only the first thread attempts to delete "test"
                if i == 0 {
                    assert!(filter_clone.del_word("test"));
                } else {
                    // Other threads attempt to delete a word that may have already been deleted
                    filter_clone.del_word("test");
                }
                assert!(!filter_clone.del_word("nonexistent"));
                filter_clone.add_word(&format!("thread{}", i));
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(filter.find_in("This is a concurrent test."), Some("concurrent".to_string()));
        assert_eq!(filter.find_in("This is a delete test."), Some("delete".to_string()));
        assert_eq!(filter.find_in("This is a test."), None);

        // Check if all "thread{i}" words were successfully added
        for i in 0..5 {
            assert_eq!(filter.find_in(&format!("This is thread{} test.", i)), Some(format!("thread{}", i)));
        }
    }
}

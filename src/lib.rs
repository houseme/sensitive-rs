//! # Sensitive-rs
//!
//! `sensitive-rs` is a Rust library for finding, validating, filtering, and replacing sensitive words. It provides efficient algorithms to handle sensitive words, suitable for various application scenarios.
//!
//! ## Features
//!
//! - **Find**: Locate all sensitive words in a text.
//! - **Validate**: Check if a text contains any sensitive words.
//! - **Filter**: Remove sensitive words from a text.
//! - **Replace**: Replace sensitive words in a text with specified characters.
//!
//! ## Installation
//!
//! Add the following dependency to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! sensitive-rs = "0.1"
//! ```
//!
//! ## Quick Start
//!
//! ```rust
//! use sensitive_rs::Filter;
//!
//! // Create a new Filter
//! let mut filter = Filter::new();
//! filter.add_word("bad");
//! filter.add_word("worse");
//!
//! // Find sensitive words
//! let result = filter.find_in("This is bad.");
//! assert_eq!(result, (true, "bad".to_string()));
//!
//! // Validate text
//! let result = filter.validate("This is worse.");
//! assert_eq!(result, (true, "worse".to_string()));
//!
//! // Filter sensitive words
//! let filtered_text = filter.filter("This is bad and worse.");
//! assert_eq!(filtered_text, "This is  and .");
//!
//! // Replace sensitive words
//! let replaced_text = filter.replace("This is bad and worse.", '*');
//! assert_eq!(replaced_text, "This is *** and *****.");
//! ```
//!
//! ## Documentation
//!
//! For detailed documentation, please refer to [Documentation](https://docs.rs/sensitive-rs).
//!
//! ## License
//!
//!
//! Licensed under either of
//!
//! * Apache License, Version 2.0, [LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0
//! * MIT license [LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT
mod filter;
mod trie;

/// Sensitive word filter.
/// It provides efficient algorithms to handle sensitive words, suitable for various application scenarios.
/// # Examples
/// ```
/// use sensitive_rs::Filter;
///
/// let mut filter = Filter::new();
/// filter.add_word("bad");
/// filter.add_word("worse");
///
/// let result = filter.find_in("This is bad.");
/// assert_eq!(result, (true, "bad".to_string()));
/// ```
///
/// # Features
/// - **Find**: Locate all sensitive words in a text.
/// - **Validate**: Check if a text contains any sensitive words.
/// - **Filter**: Remove sensitive words from a text.
/// - **Replace**: Replace sensitive words in a text with specified characters.
///
/// # Installation
/// Add the following dependency to your `Cargo.toml`:
/// ```toml
/// [dependencies]
/// sensitive-rs = "0.1"
/// ```
///
/// # Quick Start
/// ```rust
/// use sensitive_rs::Filter;
///
/// let mut filter = Filter::new();
/// filter.add_word("bad");
/// filter.add_word("worse");
///
/// let result = filter.find_in("This is bad.");
/// assert_eq!(result, (true, "bad".to_string()));
///
/// let result = filter.validate("This is worse.");
/// assert_eq!(result, (true, "worse".to_string()));
///
/// let filtered_text = filter.filter("This is bad and worse.");
/// assert_eq!(filtered_text, "This is  and .");
///
/// let replaced_text = filter.replace("This is bad and worse.", '*');
/// assert_eq!(replaced_text, "This is *** and *****.");
/// ```
///
/// # Documentation
/// For detailed documentation, please refer to [Documentation](https://docs.rs/sensitive-rs).
///
/// # License
/// Licensed under either of
/// * Apache License, Version 2.0, [LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0
/// * MIT license [LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT
///
/// # Authors
/// - [houseme](https://github.com/houseme)
/// - [Contributors]
///
/// # Acknowledgments
/// - [Aho-Corasick](https://en.wikipedia.org/wiki/Aho%E2%80%93Corasick_algorithm)
/// - [Trie](https://en.wikipedia.org/wiki/Trie)
/// - [DFA](https://en.wikipedia.org/wiki/Deterministic_finite_automaton)
/// - [NFA](https://en.wikipedia.org/wiki/Nondeterministic_finite_automaton)
/// - [Regex](https://en.wikipedia.org/wiki/Regular_expression)
/// - [KMP](https://en.wikipedia.org/wiki/Knuth%E2%80%93Morris%E2%80%93Pratt_algorithm)
/// - [Boyer-Moore](https://en.wikipedia.org/wiki/Boyer%E2%80%93Moore_string-search_algorithm)
/// - [Rabin-Karp](https://en.wikipedia.org/wiki/Rabin%E2%80%93Karp_algorithm)
/// - [Sunday](https://en.wikipedia.org/wiki/String_searching_algorithm#Sunday's_algorithm)
/// - [Horspool](https://en.wikipedia.org/wiki/String_searching_algorithm#Horspool's_algorithm)
/// - [Comment](https://en.wikipedia.org/wiki/String_searching_algorithm#Comment's_algorithm)
/// - [Shift-And](https://en.wikipedia.org/wiki/Bitap_algorithm)
/// - [Bitap](https://en.wikipedia.org/wiki/Bitap_algorithm)
/// - [Wu-Manber](https://en.wikipedia.org/wiki/Wu%E2%80%93Manber_algorithm)
///
/// # References
/// - [Aho-Corasick](https://en.wikipedia.org/wiki/Aho%E2%80%93Corasick_algorithm)
/// - [Trie](https://en.wikipedia.org/wiki/Trie)
///
/// # See Also
/// - [sensitive-rs](https://github.com/houseme/sensitive-rs)
///
/// # Tags
/// - Sensitive
/// - Filter
/// - Find
/// - Validate
/// - Replace
/// - Trie
///
/// # Categories
/// - Sensitive
/// - Filter
/// - Find
/// - Validate
/// - Replace
/// - Trie
///
/// # Dependencies
/// - [aho-corasick](https://crates.io/crates/aho-corasick)
/// - [regex](https://crates.io/crates/regex)
/// - [trie](https://crates.io/crates/trie)
///
/// # Examples
/// - [sensitive-rs](https://github.com/houseme/sensitive-rs)
pub use filter::Filter;
/// Trie data structure.
/// It is a tree data structure used for efficient retrieval of a key in a large dataset.
/// # Examples
/// ```
/// use sensitive_rs::Trie;
/// use sensitive_rs::TrieNode;
///
/// let mut trie = Trie::new();
/// trie.add_word("bad");
/// trie.add_word("worse");
///
/// let result = trie.find_in("This is bad.");
/// assert_eq!(result, Some("bad".to_string()));
///
/// let result = trie.find_in("This is worse.");
/// assert_eq!(result, Some("worse".to_string()));
///
/// let result = trie.find_in("This is good.");
/// assert_eq!(result, None);
///
/// let result = trie.find_in("This is worse and bad.");
/// assert_eq!(result, Some("worse".to_string()));
///
/// let result = trie.find_in("This is bad and worse.");
/// assert_eq!(result, Some("bad".to_string()));
///
/// let result = trie.find_in("This is good and better.");
/// assert_eq!(result, None);
/// ```
pub use trie::Trie;
/// Trie node.
/// It is a tree data structure used for efficient retrieval of a key in a large dataset.
/// # Examples
/// ```
/// use sensitive_rs::TrieNode;
///
/// let mut node = TrieNode::new('a', false);
/// ```
pub use trie::TrieNode;


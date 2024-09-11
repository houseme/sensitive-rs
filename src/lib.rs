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
//! use sensitive_rs::filter::Filter;
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
pub mod filter;
pub mod trie;

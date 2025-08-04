//! # Sensitive-rs
//!
//! `sensitive-rs` is a Rust library for finding, validating, filtering, and replacing sensitive words.
//! It provides efficient algorithms to handle sensitive words, suitable for various application scenarios.
//!
//! ## Features
//!
//! - **Multi-algorithm support**: Aho-Corasick, Wu-Manber and Regex
//! - **Variant detection**: Handle pinyin and shape variants
//! - **High performance**: Optimized for Chinese text processing
//!
//! ## Quick Start
//!
//! ```rust
//! use sensitive_rs::Filter;
//!
//! let mut filter = Filter::new();
//! filter.add_word("赌博");
//! filter.add_word("色情");
//!
//! // Standard matching
//! assert_eq!(filter.find_in("含有赌博内容"), (true, "赌博".to_string()));
//!
//! // Variant detection
//! assert_eq!(filter.find_in("含有 dubo 内容"), (true, "赌博".to_string()));
//! ```

mod engine;
mod filter;
mod variant;

pub use engine::MultiPatternEngine;
pub use filter::Filter;
pub use variant::VariantDetector;

/// Re-export for backward compatibility
pub use engine::wumanber::WuManber;

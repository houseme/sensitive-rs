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

#![cfg_attr(not(feature = "std"), no_std)]
// On `no_std` some engine/cache/variant surface is exercised only via the `std`
// paths (or tests), so it reads as dead code there. Keep the `std` build fully linted.
#![cfg_attr(not(feature = "std"), allow(dead_code, unused_mut))]

extern crate alloc;

mod engine;
mod filter;
#[cfg(feature = "std")]
mod variant;
#[cfg(feature = "wasm")]
mod wasm;

pub use engine::MatchAlgorithm;
pub use engine::MultiPatternEngine;
pub use filter::Filter;
pub use filter::Match;
#[cfg(feature = "std")]
pub use variant::VariantDetector;

/// Re-export for backward compatibility
#[cfg(feature = "std")]
pub use engine::wumanber::WuManber;

#[cfg(feature = "wasm")]
pub use wasm::WasmFilter;

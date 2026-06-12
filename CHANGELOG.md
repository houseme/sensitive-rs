# CHANGELOG

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [Released]

## [0.8.0] - 2026-06-12

### Fixed

- Cache staleness: clear LRU cache on `add_word`, `add_words`, `del_word`, `del_words`
- Mutex poisoning cascade: recover from poisoned mutex instead of panicking
- `update_noise_pattern` panic on invalid regex: now returns `Result<(), regex::Error>`
- Parallel search missing cross-boundary matches: add overlap between chunks
- `rustfmt.toml` edition mismatch: update from 2021 to 2024

### Changed

- Upgraded crate version to 0.8.0
- `update_noise_pattern` now returns `Result` (breaking change)

## [0.7.0] - 2026-06-11

### Added

- CLI tool (`sensitive-rs` binary) with `check`, `validate`, `replace`, `filter` subcommands
- `cli` feature flag with `clap`, `serde`, `serde_json` dependencies
- JSON output format via `--json` flag
- File and stdin input support
- Colored terminal output with auto TTY detection
- Exported `MatchAlgorithm` enum from library

### Changed

- Upgraded crate version to 0.7.0

## [0.6.0] - 2026-06-11

### Changed

- Migrated from `pinyin` crate to `pinyin-converter` for pinyin conversion
- Updated variant detector to use `Pinyin::chars()` API with toneless pinyin matching
- Upgraded crate version to 0.6.0

## [0.5.0] - 2025-08-06

### Added

- Upgrade Rust edition to 2024
- Introduce parallel search with `rayon` (`find_all_parallel`)
- Add LRU cache for hot query results
- Support batch processing via `find_all_batch`
- Add layered matching with `find_all_layered`
- Support streaming processing with `find_all_streaming`

### Changed

- Upgraded crate version to 0.5.0
- Improved algorithm performance for Chinese text
- Updated documentation with new API examples

### Fixed

- Wu-Manber pattern matching algorithm correctness
- UTF-8 character boundary handling in match results
- Test cases for Chinese character processing

## [0.4.0] - 2025-08-05

### Changed

- Updated `reqwest` dependency to version `0.12.22`.
- Tip will be removed soon `Trie` struct in the next major version.

## [0.3.0] - 2025-02-06

### Changed

- Replaced `native-tls` with `rustls-tls` in reqwest crate features.
- Fixed Clippy warning by using `is_some_and` instead of `map_or`.

## [0.2.2] - 2024-11-28

### Changed

- Removed dev dependencies.
- Modified `net` feature dependency to `net=["dep:reqwest"]`.
- Updated dependency versions.

## [0.2.1] - 2024-09-12

### Changed

- Enhanced `TrieNode` example code in `lib.rs` to demonstrate more functionalities.
- Added assertions to check the character and end status of the node.

## [0.2.0] - 2024-09-12

### Added

- Added `pub use filter::Filter`, `pub use trie::Trie`, and `pub use trie::TrieNode` to `lib.rs`.

### Changed

- Updated documentation comments to reflect the new public exports.
- Synchronized example code in `README.md` to match the changes.

## [0.1.3] - 2024-09-12

### Added

- Implemented `Default` trait for struct `new` methods.
- Enhanced test coverage.

### Changed

- Improved documentation comments.
- Addressed Clippy warnings.

## [0.1.2] - 2024-09-11

### Changed

- Improved documentation comments.

## [0.1.1] - 2024-09-10

### Added

- Implemented `Trie` struct with methods for adding, deleting, finding, validating, filtering, and replacing words.
- Implemented `Filter` struct with methods for adding, deleting, finding, validating, filtering, and replacing words.
- Added support for loading word dictionaries from files and URLs.
- Added support for updating noise patterns using regular expressions.
- Added comprehensive tests for `Trie` and `Filter` functionalities.

## [0.1.0] - 2024-08-16

### Added

- Initial project setup.
- Basic implementation of `Trie` and `Filter` structs.
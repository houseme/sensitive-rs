# CHANGELOG

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.2.0] - 2026-07-13

### Added

- Expanded the shape-confusable character map from 5 to 50+ groups, now loaded from `dict/shape_map.txt`. Each group is a full equivalence class (bidirectional), so evasions are caught in both directions (e.g. a word built from `睹` matches text using `赌`).
- `Filter::find_first_match` and a `Match` type (`{ word, is_variant }`) for richer first-match results. `Filter::find_in` keeps its existing `(bool, String)` signature and now delegates to `find_first_match`.
- `Display` implementation for `MatchAlgorithm` (`Aho-Corasick` / `Wu-Manber` / `Regex`).
- `#[must_use]` on value-returning query methods (`find_in`, `find_all`, `find_first_match`, `replace`, `filter`, `validate`, `find_all_batch`, `find_all_layered`, `current_algorithm`, `remove_noise`, `get_noise_pattern`).
- Rustdoc `# Examples` (runnable doctests) across the public API, plus module-level docs for the `filter`, `engine`, and `variant` modules.
- `Benchmarks` CI workflow (`.github/workflows/bench.yml`) that runs `cargo bench --bench matching` on pull requests and uploads the results.

### Changed

- Dictionary entries that contain whitespace are now loaded with both the original form and a whitespace-folded form, so entries like `A 级` also match `A级`.
- `Filter::validate` is now explicitly documented as an alias of `find_in`.
- README (`README.md` / `README_CN.md`) refreshed with the current API, an algorithm-selection table, and version 1.2.0.

## [1.1.0] - 2026-07-13

### Added

- `criterion` benchmarks (`benches/matching.rs`): `find_all` across vocabulary sizes, algorithm comparison (AhoCorasick/WuManber/Regex), `replace`, cache-hit, and batch.
- `examples/` directory: `basic`, `variant`, `batch`, `custom_dict`.
- `clippy.toml` with MSRV and complexity/line-count thresholds.

### Changed

- `rayon` is now optional behind a new `parallel` feature (enabled in `default` for backward compatibility). With `--no-default-features`, `Filter` falls back to sequential matching - friendlier for WASM/embedded builds.
- `benches/` and `examples/` are now included in the published package so `[[bench]]` resolves on `cargo publish`.
- Performance: `WuManber::find_matches` now drives off the shift/hash tables (sharing one scan core with `search_all`) instead of O(n*m) brute-force per pattern; `WuManber::replace_all`/`remove_all` and `Filter::replace` rebuild the string in a single pass instead of per-pattern `str::replace`. Overlapping dictionary words (e.g. `"赌博"` inside `"赌博机"`) now resolve leftmost-longest on replace/filter - previously undefined.

## [1.0.0] - 2026-07-12

### Added

- Comprehensive test coverage closing the gaps identified in the v1.0.0 audit (no production code changes):
  - `VariantDetector` unit tests (10 tests): pinyin/shape detection, sort+dedup, empty/ASCII/mixed-script/multi-word edge cases
  - `MultiPatternEngine` direct tests (14 tests): `find_first`/`find_all`/`replace_all`/`contains_any`/`find_matches_with_positions`/`stats` across all three algorithms (AhoCorasick, WuManber, Regex)
  - `Filter` advanced method tests: `find_all_batch`, `find_all_layered` (longest-match preference), `find_all_streaming` (multi-line)
  - `Filter` LRU cache behavior tests: cache hit consistency, explicit clear
  - `Filter` edge case tests: empty text, empty dictionary, emoji noise stripping, very long text (100K chars, parallel path), CJK Extension B
  - CLI integration tests (7 tests, `tests/cli_tests.rs`): check/validate/replace/filter subcommands and JSON output, pinned to a fixture dictionary for deterministic results

### Fixed

- `WuManber::find_matches` no longer panics on multi-byte text: the scan cursor now advances by one UTF-8 character instead of one byte, so the next `text[start..]` slice stays on a character boundary. This makes `MultiPatternEngine::find_matches_with_positions` usable under the default WuManber algorithm for Chinese text. Surfaced by the new test coverage and locked in with regression tests in both the `wumanber` and `engine` modules. (`Filter::find_all`/`find_in`/`search` were unaffected — they don't use `find_matches`.)

## [Released]

## [0.9.0] - 2026-06-13

### Fixed

- Wu-Manber `search_all` bypass: use shift/hash tables instead of brute-force for `Strict` mode
- Algorithm recommendation docs: updated enum comments to match actual recommendation behavior
- `WuManber` re-export from private module: changed `pub(crate) mod wumanber` to `pub mod wumanber`

### Changed

- `Filter::filter` now uses engine's optimized `replace_all` path
- Added `Debug` implementations for `Filter`, `MultiPatternEngine`, `VariantDetector`
- Upgraded crate version to 0.9.0

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

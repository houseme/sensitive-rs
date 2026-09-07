# CHANGELOG

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Refreshed `Cargo.lock` to the latest crate versions (`cargo update`): `smallvec` 1.16.0, `lru` 0.18.4, `clap` 4.6.6, `tokio` 1.53.1, `serde` 1.0.229, `wasm-bindgen` 0.2.128 and related transitive bumps. No Cargo.toml requirement changes were needed — every direct dependency was already on the newest release.
- Algorithm auto-selection is now two-tier: **0–100 patterns → Wu-Manber, 100+ → Aho-Corasick**. The old "10,000+ patterns → Regex" tier is gone: on a 27,126-entry dictionary a regex alternation scanned ~60,000× slower than Aho-Corasick (22 ms vs ~0.36 ms per call). Regex remains force-selectable via `rebuild_with_algorithm` / `Filter::with_algorithm` / CLI `--algorithm regex`.
- `VariantDetector` detection is index-driven: word pinyin keys and shape-confusable first characters are indexed at `add_word` time. Pinyin detection scans rolling-hash windows of the text's pinyin (O(text × key_lengths) integer ops) instead of re-deriving every word's pinyin and substring-searching per call; the `original_words` filter set is only built when there is at least one candidate. On the bundled 14k dictionary, `find_first_match` on clean text dropped from 5.48 ms to 31 µs (~175×) and on pinyin-variant text from 5.24 ms to 14 µs (~370×).
- `Filter` now re-syncs the variant detector on every `add_words`/`del_words`, making "registered words == engine patterns" an invariant. This removes the per-call word-set bookkeeping from the hot path and fixes a latent bug where deleted words could still surface as pinyin/shape variants.
- `WuManber` hot-path allocation removal: pattern char decompositions are precomputed once at build time (verify no longer collects `pattern.chars()` per candidate), table building and scanning hash char slices directly (no per-block `String`), text buffers use a stack `SmallVec` for short inputs, and non-strict space-handling searches preprocess the text once instead of per pattern.
- `WasmFilter::load_words` collects all lines and adds them in one batch (the engine is rebuilt once instead of once per word — O(n²) → O(n) build time).
- `Filter::replace` matches via a span-only engine path (`MultiPatternEngine::find_match_spans`) and writes replacement chars directly, avoiding a clone of the matched text per hit; `remove_noise` uses `Cow::into_owned`.

### Added

- `VariantDetector::detect_first` / `Filter::find_first_match`-backed first-match detection that tracks the lexicographic minimum without collecting and sorting all hits.
- `VariantDetector::clear` for dropping all registrations.
- Benchmark coverage for the uncached paths: `find_all` cold-cache groups (the previous `find_all` benchmarks rotated a single warm cache entry and mostly measured LRU hits), cached-vs-cold comparisons, and `find_first_match` (clean / exact hit / variant hit) on the bundled 14k dictionary.
- New tests: substring shape-variant detection, `detect_first` vs `detect` agreement, unregistered-word legacy behavior, blank-line dictionary loading, empty-pattern rejection, and rolling-hash index behavior.

### Fixed

- **Panic with dictionaries containing blank lines**: empty lines became empty patterns; in Aho-Corasick an empty pattern matches at every byte offset (including inside multi-byte characters), crashing `text[start..end]` slicing and polluting results. Blank lines are now skipped by `Filter::load` / `load_word_dict_async` / `WasmFilter::load_words`, and the engine drops empty patterns on rebuild (defense in depth).
- Forced `Regex` on very large vocabularies silently fell back to Wu-Manber because the alternation exceeded the default 10 MB compiled-size limit; the limit is now raised to 64 MB via `RegexBuilder`.
- Shape-variant detection only compared the *whole text* when its length equaled the word's, making it nearly useless inside longer sentences. It now finds confusable words as substrings of any text (anchored on the word's first character and its confusable class).
- Non-strict space handling (`IgnoreSpaces`/`NormalizeSpaces`) re-preprocessed the text once per pattern inside the search loops, and `search_all_with_preprocessing` returned preprocessed pattern forms instead of the original dictionary forms; both fixed.
- `MultiPatternEngine::replace_all` with an all-whitespace pattern under non-strict space handling could match everywhere (the preprocessed pattern becomes empty); empty preprocessed patterns are now skipped.

## [1.3.0] - 2026-07-14

### Added

- WASM support: `WasmFilter` (`wasm-bindgen`) for browsers/Node.js behind the `wasm` feature, exposing `addWord`/`addWords`/`findIn`/`findAll`/`replace`/`filter`/`loadWords`. No filesystem on WASM — use `loadWords` with in-memory text.
- Async dictionary loading: `Filter::load_word_dict_async` (tokio; `async-io` feature) and `Filter::load_net_word_dict_async` (`net-async`). The synchronous API is unchanged.
- `no_std` support: the crate now compiles with `--no-default-features` (verified on `thumbv7em-none-eabihf`). Core exact matching (`find_all`/`find_in`/`replace`/`filter`) works without `std`; the LRU cache, pinyin/shape variant detection, and file/network loaders require the `std` feature (on by default).
- Cross-platform CI: tests run on Linux/macOS/Windows; new `wasm` (wasm32) and `no_std` check jobs.
- "Platform Support" section in README (EN/CN) with WASM/no_std/async usage examples.

### Changed

- `regex` and `aho-corasick` use `default-features = false` (re-enabling their `std` sub-feature via the `std` feature) to permit `no_std`. `lru` and `pinyin-converter` are now optional, pulled only under `std`.
- `WuManber` uses a compact FNV-1a hasher on `no_std` (replacing the std-only `DefaultHasher`), keeping the small-vocabulary algorithm available without `std`.
- The blocking HTTP client is now built lazily per `load_net_word_dict` call (instead of stored on `Filter`), so a `Filter` can be created and dropped inside an async runtime without panicking.

## [1.2.1] - 2026-07-14

### Changed

- Release metadata and the English/Chinese installation examples now reference version 1.2.1. No public API changes.

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

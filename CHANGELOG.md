# CHANGELOG

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
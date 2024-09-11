# Sensitive-rs

English [中文](README_CN.md)

[![Build](https://github.com/houseme/sensitive-rs/workflows/Build/badge.svg)](https://github.com/houseme/sensitive-rs/actions?query=workflow%3ABuild)
[![crates.io](https://img.shields.io/crates/v/sensitive-rs.svg)](https://crates.io/crates/sensitive-rs)
[![docs.rs](https://docs.rs/sensitive-rs/badge.svg)](https://docs.rs/sensitive-rs/)
[![License](https://img.shields.io/crates/l/sensitive-rs)](./LICENSE-APACHE)
[![Crates.io](https://img.shields.io/crates/d/sensitive-rs)](https://crates.io/crates/sensitive-rs)

Sensitive-rs is a Rust library for finding, validating, filtering, and replacing sensitive words. It provides efficient
algorithms to handle sensitive words, suitable for various application scenarios.

## Features

- **Find**: Locate all sensitive words in a text.
- **Validate**: Check if a text contains any sensitive words.
- **Filter**: Remove sensitive words from a text.
- **Replace**: Replace sensitive words in a text with specified characters.

## Installation

Add the following dependency to your `Cargo.toml`:

```toml
[dependencies]
sensitive-rs = "0.1"
```

## Usage Examples

Here are some examples of how to use Sensitive-rs:

Here is an example of how to use the Filter struct

```rust
use sensitive_rs::filter::Filter;

fn main() {
    // Create a new Filter
    let mut filter = Filter::new();
    filter.add_word("bad");
    filter.add_word("worse");

    // Find sensitive words
    let result = filter.find_in("This is bad.");
    assert_eq!(result, (true, "bad".to_string()));

    // Validate text
    let result = filter.validate("This is worse.");
    assert_eq!(result, (true, "worse".to_string()));

    // Filter sensitive words
    let filtered_text = filter.filter("This is bad and worse.");
    assert_eq!(filtered_text, "This is  and .");

    // Replace sensitive words
    let replaced_text = filter.replace("This is bad and worse.", '*');
    assert_eq!(replaced_text, "This is *** and *****.");
}
```

Here is an example of how to use the Trie struct

```rust
use sensitive_rs::trie::Trie;

fn main() {
    // Create a new Trie filter
    let filter = Trie::new();
    filter.add_word("bad");
    filter.add_word("worse");

    // Find sensitive words
    let result = filter.find_in("This is bad.");
    assert_eq!(result, Some("bad".to_string()));

    // Validate text
    let result = filter.validate("This is worse.");
    assert_eq!(result, Some("worse".to_string()));

    // Filter sensitive words
    let filtered_text = filter.filter("This is bad and worse.");
    assert_eq!(filtered_text, "This is  and .");

    // Replace sensitive words
    let replaced_text = filter.replace("This is bad and worse.", '*');
    assert_eq!(replaced_text, "This is *** and *****.");
}
```

## Documentation

For detailed documentation, please refer to [Documentation](https://docs.rs/sensitive-rs).

## License

Licensed under either of

* Apache License, Version 2.0, [LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0
* MIT license [LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as
defined in the Apache-2.0 or MIT license, shall be dual licensed as above, without any additional terms or conditions.
# Sensitive-rs

English | [简体中文](README_CN.md)

[![Build](https://github.com/houseme/sensitive-rs/workflows/Build/badge.svg)](https://github.com/houseme/sensitive-rs/actions?query=workflow%3ABuild)
[![crates.io](https://img.shields.io/crates/v/sensitive-rs.svg)](https://crates.io/crates/sensitive-rs)
[![docs.rs](https://docs.rs/sensitive-rs/badge.svg)](https://docs.rs/sensitive-rs/)
[![License](https://img.shields.io/crates/l/sensitive-rs)](./LICENSE-APACHE)
[![Downloads](https://img.shields.io/crates/d/sensitive-rs)](https://crates.io/crates/sensitive-rs)

A high-performance Rust crate for multi-pattern string matching, validation, filtering, and replacement.

## Features

- Find all sensitive words: `find_all`
- Validate text contains sensitive words: `validate`
- Remove sensitive words: `filter`
- Replace sensitive words with a character: `replace`
- Multi-algorithm engine: Aho-Corasick, Wu-Manber, Regex
- Noise removal via configurable regex
- Variant detection (拼音、形似字)
- Parallel search with `rayon`
- LRU cache for hot queries
- Batch processing: `find_all_batch`
- Layered matching: `find_all_layered`
- Streaming processing: `find_all_streaming`

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sensitive-rs = "0.5.0"
```

## Quick Start

```rust
use sensitive_rs::Filter;

fn main() {
    let mut filter = Filter::new();
    filter.add_words(&["rust", "filter", "敏感词"]);

    let text = "hello rust, this is a filter demo 包含敏感词";
    let found = filter.find_all(text);
    println!("Found: {:?}", found);

    let cleaned = filter.replace(text, '*');
    println!("Cleaned: {}", cleaned);
}
```

## Advanced Usage

Batch processing:

```rust
let texts = vec!["text1", "text2"];
let results = filter.find_all_batch( & texts);
```

Layered matching:

```rust
let layered = filter.find_all_layered("some long text");
```

Streaming large files:

```rust
use std::fs::File;
use std::io::BufReader;

let reader = BufReader::new(File::open("large.txt") ? );
let stream_results = filter.find_all_streaming(reader) ?;
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
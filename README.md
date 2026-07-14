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
- First match with metadata (`Match`): `find_first_match`
- Validate text contains sensitive words: `validate`
- Remove sensitive words: `filter`
- Replace sensitive words with a character: `replace`
- Multi-algorithm engine: Aho-Corasick, Wu-Manber, Regex
- Noise removal via configurable regex
- Variant detection (拼音、形似字) — pinyin plus a 50+ group shape-confusable map
- Parallel search with optional `rayon` support (`parallel` feature, enabled by default)
- LRU cache for hot queries
- Batch processing: `find_all_batch`
- Layered matching: `find_all_layered`
- Streaming processing: `find_all_streaming`
- Criterion benchmarks and runnable examples for release validation

## Algorithm selection

The engine auto-selects based on vocabulary size:

| Patterns   | Algorithm     | Rationale                                  |
|------------|---------------|--------------------------------------------|
| 0–100      | Wu-Manber     | Small tables, quick scan                   |
| 101–10,000 | Aho-Corasick  | O(n) automaton scan regardless of count    |
| 10,000+    | Regex         | Compilation overhead amortized             |

Override with `Filter::with_algorithm(...)` or `--algorithm` on the CLI.

## Platform Support

| Platform | Status | How |
|----------|--------|-----|
| Linux / macOS / Windows | Full support | default features / `--all-features` |
| WASM (browser / Node.js) | Supported | `wasm` feature; no file/network I/O (use `loadWords`) |
| Embedded (`no_std`) | Experimental | `--no-default-features`; core exact matching only |
| Async (tokio) | Supported | `async-io` / `net-async` features |

### WASM

```toml
[dependencies]
sensitive-rs = { version = "1.3", default-features = false, features = ["wasm"] }
```

```javascript
import init, { WasmFilter } from 'sensitive-rs';
await init();
const filter = new WasmFilter();
filter.addWord('赌博');
filter.findAll('含有赌博内容'); // ['赌博']
filter.loadWords('色情\n诈骗'); // bulk-load from in-memory text
```

### `no_std` (embedded)

```toml
[dependencies]
sensitive-rs = { version = "1.3", default-features = false }
```

Core `find_all` / `find_in` / `replace` / `filter` work without `std`. Pinyin/shape variant
detection, the LRU cache, and the file/network loaders require the `std` feature (on by default).

### Async

```toml
[dependencies]
sensitive-rs = { version = "1.3", features = ["async-io"] }
```

```rust,no_run
#[tokio::main]
async fn main() -> std::io::Result<()> {
    use sensitive_rs::Filter;
    let mut filter = Filter::new();
    filter.load_word_dict_async("dict/dict.txt").await?;
    Ok(())
}
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sensitive-rs = "1.3.0"
```

For environments that should avoid `rayon` (for example WASM or embedded targets), disable default features:

```toml
[dependencies]
sensitive-rs = { version = "1.3.0", default-features = false }
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
let results = filter.find_all_batch(&texts);
```

Layered matching:

```rust
let layered = filter.find_all_layered("some long text");
```

Streaming large files:

```rust
use std::fs::File;
use std::io::BufReader;

let reader = BufReader::new(File::open("large.txt")?);
let stream_results = filter.find_all_streaming(reader)?;
```

## CLI Usage

Install with the `cli` feature:

```toml
[dependencies]
sensitive-rs = { version = "1.3.0", features = ["cli"] }
```

Or install directly:

```sh
cargo install sensitive-rs --features cli
```

Both `sensitive` and `sensitive-rs` commands are available after installation.

### Commands

```sh
# Find sensitive words
sensitive check "含有赌博和色情内容"

# Validate (exit 1 if sensitive words found)
sensitive validate "clean text"

# Replace sensitive words
sensitive replace '*' "含有赌博内容"

# Remove sensitive words
sensitive filter "含有赌博内容"

# Read from file
sensitive check --file input.txt

# Pipe from stdin
echo "text" | sensitive check
```

### Options

- `--dict <path>` — custom dictionary file
- `--dict-all` — use extended dictionary (27k words)
- `--algorithm <algo>` — force algorithm: `aho-corasick`, `wumanber`, `regex`
- `--variant` — enable pinyin and shape variant detection
- `--noise-pattern <regex>` — custom noise removal regex
- `--json` — JSON output format
- `--color` — force colored output

## Examples and Benchmarks

```sh
cargo run --example basic
cargo run --example batch
cargo run --example custom_dict
cargo run --example variant
cargo bench
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

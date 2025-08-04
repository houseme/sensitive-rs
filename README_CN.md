# Sensitive-rs

简体中文 | [English](README.md)

[![Build](https://github.com/houseme/sensitive-rs/workflows/Build/badge.svg)](https://github.com/houseme/sensitive-rs/actions?query=workflow%3ABuild)
[![crates.io](https://img.shields.io/crates/v/sensitive-rs.svg)](https://crates.io/crates/sensitive-rs)
[![docs.rs](https://docs.rs/sensitive-rs/badge.svg)](https://docs.rs/sensitive-rs/)
[![License](https://img.shields.io/crates/l/sensitive-rs)](./LICENSE-APACHE)
[![Downloads](https://img.shields.io/crates/d/sensitive-rs)](https://crates.io/crates/sensitive-rs)

一个高性能的 Rust 库，用于多模式字符串查找、验证、过滤和替换。

## 功能

- 查找所有敏感词：`find_all`
- 验证文本是否包含敏感词：`validate`
- 过滤敏感词：`filter`
- 替换敏感词：`replace`
- 多算法引擎：Aho-Corasick、Wu-Manber、Regex
- 正则噪音字符清理
- 拼音与形似字变体检测
- 基于 `rayon` 的并行搜索
- 热点查询 LRU 缓存
- 批量处理：`find_all_batch`
- 分层匹配：`find_all_layered`
- 流式处理：`find_all_streaming`

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
sensitive-rs = "0.5.0"
```

## 快速开始

```rust
use sensitive_rs::Filter;

fn main() {
    let mut filter = Filter::new();
    filter.add_words(&["rust", "filter", "敏感词"]);

    let text = "hello rust, this is a filter demo 包含敏感词";
    let found = filter.find_all(text);
    println!("匹配到：{:?}", found);

    let cleaned = filter.replace(text, '*');
    println!("过滤后：{}", cleaned);
}
```

## 进阶用法

批量处理：

```rust
let texts = vec!["文本 1", "文本 2"];
let results = filter.find_all_batch( & texts);
```

分层匹配：

```rust
let layered = filter.find_all_layered("一些长文本");
```

流式处理大文件：

```rust
use std::fs::File;
use std::io::BufReader;

let reader = BufReader::new(File::open("large.txt") ? );
let stream_results = filter.find_all_streaming(reader) ?;
```

文档
详细文档请参阅 [Documentation](https://docs.rs/sensitive-rs).

## 许可证

可以选择下列任意一种许可证：

* Apache 许可证 2.0，详见 [LICENSE-APACHE](LICENSE-APACHE) 或 http://www.apache.org/licenses/LICENSE-2.0
* MIT 许可证，详见 [LICENSE-MIT](LICENSE-MIT) 或 http://opensource.org/licenses/MIT

## 贡献

除非您明确声明，否则您有意提交的任何贡献将根据 Apache-2.0 或 MIT 许可证的定义，按上述双重许可进行许可，不附加任何其他条款或条件。

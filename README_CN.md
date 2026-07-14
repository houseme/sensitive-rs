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
- 获取首个匹配及元信息（`Match`）：`find_first_match`
- 验证文本是否包含敏感词：`validate`
- 过滤敏感词：`filter`
- 替换敏感词：`replace`
- 多算法引擎：Aho-Corasick、Wu-Manber、Regex
- 正则噪音字符清理
- 拼音与形似字变体检测（含 50+ 组形近字映射）
- 基于可选 `rayon` 的并行搜索（`parallel` feature，默认启用）
- 热点查询 LRU 缓存
- 批量处理：`find_all_batch`
- 分层匹配：`find_all_layered`
- 流式处理：`find_all_streaming`
- Criterion 基准测试和可运行 examples，便于发布验证

## 算法选择

引擎会根据词库规模自动选择算法：

| 词库规模   | 算法          | 说明                                |
|------------|---------------|-------------------------------------|
| 0–100      | Wu-Manber     | 表小，扫描快                        |
| 101–10,000 | Aho-Corasick  | 自动机 O(n) 扫描，与词数无关        |
| 10,000+    | Regex         | 编译开销在大量模式下均摊            |

可通过 `Filter::with_algorithm(...)` 或 CLI 的 `--algorithm` 强制指定。

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
sensitive-rs = "1.2.1"
```

如果目标环境不适合引入 `rayon`（例如 WASM 或嵌入式场景），可以关闭默认功能：

```toml
[dependencies]
sensitive-rs = { version = "1.2.1", default-features = false }
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
let results = filter.find_all_batch(&texts);
```

分层匹配：

```rust
let layered = filter.find_all_layered("一些长文本");
```

流式处理大文件：

```rust
use std::fs::File;
use std::io::BufReader;

let reader = BufReader::new(File::open("large.txt")?);
let stream_results = filter.find_all_streaming(reader)?;
```

## CLI 使用

启用 `cli` 功能安装：

```toml
[dependencies]
sensitive-rs = { version = "1.2.1", features = ["cli"] }
```

或直接安装：

```sh
cargo install sensitive-rs --features cli
```

安装后可使用 `sensitive` 和 `sensitive-rs` 两个命令。

### 命令

```sh
# 查找敏感词
sensitive check "含有赌博和色情内容"

# 验证文本（发现敏感词时 exit 1）
sensitive validate "干净文本"

# 替换敏感词
sensitive replace '*' "含有赌博内容"

# 移除敏感词
sensitive filter "含有赌博内容"

# 从文件读取
sensitive check --file input.txt

# 从 stdin 管道读取
echo "文本" | sensitive check
```

### 选项

- `--dict <path>` — 自定义词典文件
- `--dict-all` — 使用扩展词典（2.7 万词）
- `--algorithm <algo>` — 强制指定算法：`aho-corasick`、`wumanber`、`regex`
- `--variant` — 启用拼音和形近字变体检测
- `--noise-pattern <regex>` — 自定义噪声去除正则
- `--json` — JSON 输出格式
- `--color` — 强制彩色输出

## 示例与基准测试

```sh
cargo run --example basic
cargo run --example batch
cargo run --example custom_dict
cargo run --example variant
cargo bench
```

## 文档
详细文档请参阅 [Documentation](https://docs.rs/sensitive-rs).

## 许可证

可以选择下列任意一种许可证：

* Apache 许可证 2.0，详见 [LICENSE-APACHE](LICENSE-APACHE) 或 http://www.apache.org/licenses/LICENSE-2.0
* MIT 许可证，详见 [LICENSE-MIT](LICENSE-MIT) 或 http://opensource.org/licenses/MIT

## 贡献

除非您明确声明，否则您有意提交的任何贡献将根据 Apache-2.0 或 MIT 许可证的定义，按上述双重许可进行许可，不附加任何其他条款或条件。

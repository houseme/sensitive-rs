# Sensitive-rs

中文 [English](README.md)

[![Build](https://github.com/houseme/sensitive-rs/workflows/Build/badge.svg)](https://github.com/houseme/sensitive-rs/actions?query=workflow%3ABuild)
[![crates.io](https://img.shields.io/crates/v/sensitive-rs.svg)](https://crates.io/crates/sensitive-rs)
[![docs.rs](https://docs.rs/sensitive-rs/badge.svg)](https://docs.rs/sensitive-rs/)
[![License](https://img.shields.io/crates/l/sensitive-rs)](./LICENSE-APACHE)
[![Crates.io](https://img.shields.io/crates/d/sensitive-rs)](https://crates.io/crates/sensitive-rs)

Sensitive-rs 是一个用于敏感词查找、验证、过滤和替换的 Rust 库。它提供了高效的算法来处理敏感词，适用于多种应用场景。

## 功能

- **查找**：在文本中查找所有敏感词。
- **验证**：验证文本中是否包含敏感词。
- **过滤**：过滤掉文本中的敏感词。
- **替换**：将文本中的敏感词替换为指定字符。

## 安装

在 `Cargo.toml` 中添加以下依赖：

Cargo.toml
----------

```toml
[dependencies]
sensitive-rs = "0.1"
```

## 快速开始

```rust
use sensitive_rs::Trie;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    // 创建一个新的 Trie 过滤器
    let filter = Trie::new();
    filter.add_word("bad");
    filter.add_word("worse");

    // 查找敏感词
    let result = filter.find_all("This is bad and worse.");
    assert_eq!(result, vec!["bad", "worse"]);

    // 并发访问示例
    let filter = Arc::new(Trie::new());
    filter.add_word("concurrent");
    filter.add_word("test");

    let mut handles = vec![];

    for i in 0..10 {
        let filter_clone = filter.clone();
        handles.push(thread::spawn(move || {
            assert_eq!(filter_clone.find_in("This is a concurrent test."), Some("concurrent".to_string()));
            filter_clone.add_word(&format!("thread{}", i));

            // 给一些时间让其他线程也能添加它们的词
            thread::sleep(Duration::from_millis(10));

            let result = filter_clone.find_all("Thread test is concurrent.");
            assert!(result.contains(&"test".to_string()));
            assert!(result.contains(&"concurrent".to_string()));
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // 删除敏感词示例
    let filter = Trie::new();
    filter.add_word("bad");
    filter.add_word("badge");
    filter.add_word("badger");

    assert_eq!(filter.find_in("This is bad."), Some("bad".to_string()));
    assert_eq!(filter.find_in("This is a badge."), Some("badge".to_string()));
    assert_eq!(filter.find_in("This is a badger."), Some("badger".to_string()));

    // 删除 "bad"
    assert!(filter.del_word("bad"));
    assert_eq!(filter.find_in("This is bad."), None);
    assert_eq!(filter.find_in("This is a badge."), Some("badge".to_string()));
    assert_eq!(filter.find_in("This is a badger."), Some("badger".to_string()));
}
```

以下是一些使用 Sensitive-rs Filter 的示例代码

```rust
use sensitive_rs::Filter;

fn main() {
    // 创建一个新的 Filter 过滤器
    let mut filter = Filter::new();
    filter.add_word("bad");
    filter.add_word("worse");

    // 查找敏感词
    let result = filter.find_in("This is bad.");
    assert_eq!(result, (true, "bad".to_string()));

    // 验证文本
    let result = filter.validate("This is worse.");
    assert_eq!(result, (true, "worse".to_string()));

    // 过滤敏感词
    let filtered_text = filter.filter("This is bad and worse.");
    assert_eq!(filtered_text, "This is  and .");

    // 替换敏感词
    let replaced_text = filter.replace("This is bad and worse.", '*');
    assert_eq!(replaced_text, "This is *** and *****.");
}
```

文档
详细文档请参阅 [Documentation](https://docs.rs/sensitive-rs).

## 许可证

可以选择下列任意一种许可证：

* Apache 许可证 2.0，详见 [LICENSE-APACHE](LICENSE-APACHE) 或 http://www.apache.org/licenses/LICENSE-2.0
* MIT 许可证，详见 [LICENSE-MIT](LICENSE-MIT) 或 http://opensource.org/licenses/MIT

## 贡献

除非您明确声明，否则您有意提交的任何贡献将根据 Apache-2.0 或 MIT 许可证的定义，按上述双重许可进行许可，不附加任何其他条款或条件。

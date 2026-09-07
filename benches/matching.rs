use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use sensitive_rs::{Filter, MatchAlgorithm};
use std::cell::Cell;

/// Build a filter with `vocab_size` synthetic words.
fn filter_with_vocab(vocab_size: usize) -> Filter {
    let words: Vec<String> = (0..vocab_size).map(|i| format!("敏感词{i}")).collect();
    let word_refs: Vec<&str> = words.iter().map(|s| s.as_str()).collect();
    let mut filter = Filter::new();
    filter.add_words(&word_refs);
    filter
}

/// Rotation size for cold-cache benchmarks: the LRU cache holds 1000 entries, so
/// cycling through more texts than that makes every measured call a cache miss.
const ROTATION: usize = 2048;

/// Benchmark find_all (cold cache) across increasing vocabulary sizes.
fn bench_find_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_all");

    for vocab_size in [10, 100, 1000, 10_000] {
        let filter = filter_with_vocab(vocab_size);
        let texts: Vec<String> = (0..ROTATION)
            .map(|i| format!("这是一段正常的文本，含有敏感词 500 和敏感词 3000 的内容，编号{i}"))
            .collect();
        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let index = Cell::new(0usize);
        group.bench_with_input(BenchmarkId::new("cold_vocab", vocab_size), &text_refs, |b, texts| {
            b.iter_batched(
                || {
                    let i = index.get();
                    index.set((i + 1) % texts.len());
                    texts[i]
                },
                |text| filter.find_all(text),
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

/// Benchmark find_all on a warm LRU cache hit.
fn bench_find_all_cached(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_all");

    for vocab_size in [10, 10_000] {
        let filter = filter_with_vocab(vocab_size);
        let text = "这是一段正常的文本，含有敏感词 500 和敏感词 3000 的内容";
        let _ = filter.find_all(text); // warm the cache
        group.bench_with_input(BenchmarkId::new("cached_vocab", vocab_size), &text, |b, text| {
            b.iter(|| filter.find_all(std::hint::black_box(text)));
        });
    }
    group.finish();
}

/// Benchmark each matching algorithm on the same vocabulary.
fn bench_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("algorithms");
    let words: Vec<String> = (0..50).map(|i| format!("关键词{i}")).collect();
    let word_refs: Vec<&str> = words.iter().map(|s| s.as_str()).collect();

    for algo in [MatchAlgorithm::AhoCorasick, MatchAlgorithm::WuManber, MatchAlgorithm::Regex] {
        let mut filter = Filter::with_algorithm(algo);
        filter.add_words(&word_refs);
        let text = "含有关键词 25 和关键词 30 的文本";
        group.bench_with_input(BenchmarkId::new("algo", format!("{algo:?}")), &text, |b, text| {
            b.iter(|| filter.find_all(std::hint::black_box(text)));
        });
    }
    group.finish();
}

/// Benchmark single-pass replacement.
fn bench_replace(c: &mut Criterion) {
    let mut filter = Filter::new();
    filter.add_words(&["赌博", "色情", "诈骗"]);
    let text = "含有赌博和色情以及诈骗内容的文本";
    c.bench_function("replace", |b| {
        b.iter(|| filter.replace(std::hint::black_box(text), '*'));
    });
}

/// Benchmark first-match validation on a large real dictionary.
fn bench_first_match_large_dict(c: &mut Criterion) {
    if !std::path::Path::new("dict/dict.txt").exists() {
        return; // bench sources ship with dict/, but be defensive
    }
    let mut group = c.benchmark_group("first_match_14k_dict");
    let mut filter = Filter::new();
    filter.load_word_dict("dict/dict.txt").expect("dict/dict.txt must be readable");

    let clean = "这是第七百六十八段正常文本，没有任何需要过滤的内容出现而已，请放心阅读";
    let hit = "这段文本包含赌博相关词汇";
    let variant = "这是关于dubo话题的一段正常长度文本内容";

    group.bench_function("clean", |b| b.iter(|| filter.find_first_match(std::hint::black_box(clean))));
    group.bench_function("exact_hit", |b| b.iter(|| filter.find_first_match(std::hint::black_box(hit))));
    group.bench_function("variant_hit", |b| b.iter(|| filter.find_first_match(std::hint::black_box(variant))));
    group.finish();
}

/// Benchmark batch processing of 100 texts (all cache misses).
fn bench_batch(c: &mut Criterion) {
    let mut filter = Filter::new();
    filter.add_words(&["赌博", "色情"]);
    let texts: Vec<String> = (0..100).map(|i| format!("第{i}段含有赌博内容的文本")).collect();
    let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    c.bench_function("find_all_batch_100", |b| {
        b.iter(|| filter.find_all_batch(std::hint::black_box(&text_refs)));
    });
}

criterion_group!(
    benches,
    bench_find_all,
    bench_find_all_cached,
    bench_algorithms,
    bench_replace,
    bench_first_match_large_dict,
    bench_batch
);
criterion_main!(benches);

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use sensitive_rs::{Filter, MatchAlgorithm};

/// Benchmark find_all across increasing vocabulary sizes.
fn bench_find_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_all");

    for vocab_size in [10, 100, 1000, 10_000] {
        let words: Vec<String> = (0..vocab_size).map(|i| format!("敏感词{i}")).collect();
        let word_refs: Vec<&str> = words.iter().map(|s| s.as_str()).collect();
        let mut filter = Filter::new();
        filter.add_words(&word_refs);

        let text = "这是一段正常的文本，含有敏感词 500 和敏感词 3000 的内容";
        group.bench_with_input(BenchmarkId::new("vocab", vocab_size), &text, |b, text| {
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

/// Benchmark a warm LRU cache hit (second identical call).
fn bench_cache_hit(c: &mut Criterion) {
    let mut filter = Filter::new();
    filter.add_words(&["赌博", "色情"]);
    let text = "含有赌博和色情内容";

    // Warm up the cache so the measured call is a hit.
    let _ = filter.find_all(text);

    c.bench_function("cache_hit", |b| {
        b.iter(|| filter.find_all(std::hint::black_box(text)));
    });
}

/// Benchmark batch processing of 100 texts.
fn bench_batch(c: &mut Criterion) {
    let mut filter = Filter::new();
    filter.add_words(&["赌博", "色情"]);
    let texts: Vec<&str> = (0..100).map(|_| "含有赌博内容的文本").collect();
    c.bench_function("find_all_batch_100", |b| {
        b.iter(|| filter.find_all_batch(std::hint::black_box(&texts)));
    });
}

criterion_group!(benches, bench_find_all, bench_algorithms, bench_replace, bench_cache_hit, bench_batch);
criterion_main!(benches);

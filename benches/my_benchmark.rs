use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use rand::Rng;
use rand::prelude::IndexedRandom;
use redirector::config::AppConfig;
use redirector::resolve;
use std::net::IpAddr;

fn generate_random_query() -> String {
    let bang_commands = [
        "!g",
        "!w",
        "!yt",
        "!gh",
        "!so",
        "!maps",
        "!reddit",
        "!images",
        "!translate",
        "",
    ];
    let words = [
        "rust",
        "cargo",
        "benchmark",
        "performance",
        "async",
        "error",
        "lock",
        "cache",
        "config",
        "update",
        "regex",
        "network",
        "query",
        "thread",
        "sync",
    ];

    let mut rng = rand::rng();
    let include_bang: bool = rng.random_bool(0.5);
    if include_bang {
        let bang = bang_commands.choose(&mut rng).unwrap();
        let num_words = rng.random_range(2..=5);
        let selected_words: Vec<&str> = words
            .choose_multiple(&mut rng, num_words)
            .cloned()
            .collect();
        format!("{} {}", bang, selected_words.join(" "))
    } else {
        let num_words = rng.random_range(2..=5);
        let selected_words: Vec<&str> = words
            .choose_multiple(&mut rng, num_words)
            .cloned()
            .collect();
        selected_words.join(" ")
    }
}

fn benchmark_resolve(c: &mut Criterion) {
    let config = AppConfig {
        port: 3000,
        ip: IpAddr::from([127, 0, 0, 1]),
        bangs_url: "https://duckduckgo.com/bang.js".to_string(),
        default_search: "https://www.startpage.com/do/dsearch?query={}".to_string(),
        bangs: None,
    };

    c.bench_function("resolve plain query", |b| {
        b.iter(|| resolve(&config, "just a regular search query"))
    });
    c.bench_function("resolve query with bang", |b| {
        b.iter(|| resolve(&config, "!gh just a regular search query"))
    });
    c.bench_function("resolve random generated query", |b| {
        b.iter_batched(
            generate_random_query,
            |query| resolve(&config, &query),
            BatchSize::SmallInput,
        )
    });
}

fn custom_criterion() -> Criterion {
    // Increase the sample size to run the benchmarks more times.
    Criterion::default().sample_size(1000)
}

criterion_group! {
    name = benches;
    config = custom_criterion();
    targets = benchmark_resolve
}
criterion_main!(benches);

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use rand::Rng;
use rand::prelude::IndexedRandom;
use redirector::config::AppConfig;
use redirector::{get_bang, resolve, update_bangs};
use std::net::IpAddr;
use tracing::{Level, info};

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
        "!!!!!!!!!!!",
    ];

    let mut rng = rand::rng();
    let include_bang: bool = rng.random_bool(0.5);
    if include_bang {
        // Choose a bang command from the array.
        let bang = bang_commands.choose(&mut rng).unwrap();
        let num_words = rng.random_range(2..=5);
        let mut selected_words: Vec<&str> = words
            .choose_multiple(&mut rng, num_words)
            .cloned()
            .collect();
        // Insert bang into a random position.
        let insert_index = rng.random_range(0..=selected_words.len());
        selected_words.insert(insert_index, bang);
        selected_words.join(" ")
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
    let config = AppConfig::default();
    update_bangs(&config).unwrap();

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

fn benchmark_get_bang(c: &mut Criterion) {
    let config = AppConfig::default();
    update_bangs(&config).unwrap();

    c.bench_function("get bang", |b| {
        b.iter_batched(
            generate_random_query,
            |query| { get_bang(&*query); },
            BatchSize::SmallInput,
        )
    });
}

fn custom_criterion() -> Criterion {
    // Increase the sample size to run the benchmarks more times.
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_writer(std::io::stderr)
        .init();

    Criterion::default().sample_size(10_000)
}

criterion_group! {
    name = benches;
    config = custom_criterion();
    targets = benchmark_resolve, benchmark_get_bang
}
criterion_main!(benches);

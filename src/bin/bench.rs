use std::time::{Duration, Instant};

use bloomy::engine::lsm::LsmEngine;
use bloomy::{KeyRange, KeyValueStore};

const DEFAULT_ITEMS: usize = 100_000;
const SCAN_WIDTH: usize = 100;

fn main() {
    let items = std::env::args()
        .nth(1)
        .map(|value| value.parse::<usize>())
        .transpose()
        .expect("item count must be a positive integer")
        .unwrap_or(DEFAULT_ITEMS);

    if items == 0 {
        eprintln!("item count must be greater than zero");
        std::process::exit(1);
    }

    println!("lsm engine benchmark");
    println!("items: {items}");
    println!();

    bench_writes(items);
    bench_reads(items);
    bench_scans(items);
}

fn bench_writes(items: usize) {
    let mut engine = LsmEngine::new();

    let elapsed = time(|| {
        for index in 0..items {
            engine.put(key(index), value(index)).unwrap();
        }
    });

    print_result("writes", items, elapsed);
}

fn bench_reads(items: usize) {
    let engine = preload(items);

    let elapsed = time(|| {
        for index in 0..items {
            let value = engine.get(&key(index)).unwrap();
            assert!(value.is_some());
        }
    });

    print_result("reads", items, elapsed);
}

fn bench_scans(items: usize) {
    let engine = preload(items);
    let scan_count = items / SCAN_WIDTH;

    if scan_count == 0 {
        println!("scans: skipped; need at least {SCAN_WIDTH} items");
        return;
    }

    let elapsed = time(|| {
        for scan_index in 0..scan_count {
            let start = scan_index * SCAN_WIDTH;
            let end = start + SCAN_WIDTH;
            let rows = engine
                .scan(KeyRange::between(key(start), key(end)))
                .unwrap();
            assert_eq!(rows.len(), SCAN_WIDTH);
        }
    });

    print_result("scans", scan_count, elapsed);
    print_result("scan rows", scan_count * SCAN_WIDTH, elapsed);
}

fn preload(items: usize) -> LsmEngine {
    let mut engine = LsmEngine::new();

    for index in 0..items {
        engine.put(key(index), value(index)).unwrap();
    }

    engine
}

fn key(index: usize) -> Vec<u8> {
    format!("key-{index:012}").into_bytes()
}

fn value(index: usize) -> Vec<u8> {
    format!("value-{index:012}").into_bytes()
}

fn time(work: impl FnOnce()) -> Duration {
    let start = Instant::now();
    work();
    start.elapsed()
}

fn print_result(label: &str, operations: usize, elapsed: Duration) {
    let seconds = elapsed.as_secs_f64();
    let per_second = operations as f64 / seconds;

    println!(
        "{label:>9}: {operations:>10} ops in {:>8.3}s = {:>12.0} ops/sec",
        seconds, per_second
    );
}

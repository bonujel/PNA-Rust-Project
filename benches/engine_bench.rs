use criterion::{criterion_group, criterion_main, Criterion};
use kvs::{KvStore, KvsEngine, SledKvsEngine};
use rand::prelude::*;
use tempfile::TempDir;

fn write_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("write");

    group.bench_function("kvs", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let store = KvStore::open(temp_dir.path()).unwrap();
                (temp_dir, store)
            },
            |(_dir, store)| {
                for i in 0..100 {
                    store
                        .set(format!("key{}", i), "value".to_string())
                        .unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("sled", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let db = sled::open(temp_dir.path()).unwrap();
                let store = SledKvsEngine::new(db);
                (temp_dir, store)
            },
            |(_dir, store)| {
                for i in 0..100 {
                    store
                        .set(format!("key{}", i), "value".to_string())
                        .unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn read_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("read");

    group.bench_function("kvs", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let store = KvStore::open(temp_dir.path()).unwrap();
                for i in 0..100 {
                    store
                        .set(format!("key{}", i), "value".to_string())
                        .unwrap();
                }
                (temp_dir, store)
            },
            |(_dir, store)| {
                let mut rng = thread_rng();
                for _ in 0..100 {
                    let key = format!("key{}", rng.gen_range(0..100));
                    store.get(key).unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("sled", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let db = sled::open(temp_dir.path()).unwrap();
                let store = SledKvsEngine::new(db);
                for i in 0..100 {
                    store
                        .set(format!("key{}", i), "value".to_string())
                        .unwrap();
                }
                (temp_dir, store)
            },
            |(_dir, store)| {
                let mut rng = thread_rng();
                for _ in 0..100 {
                    let key = format!("key{}", rng.gen_range(0..100));
                    store.get(key).unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, write_bench, read_bench);
criterion_main!(benches);

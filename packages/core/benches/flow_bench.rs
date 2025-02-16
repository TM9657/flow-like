use criterion::{criterion_group, criterion_main, Criterion};
use sha2::{Digest, Sha256};
use std::{
    hash::{Hash, Hasher},
    hint::black_box,
};

fn hash_sha(string: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(string);
    let result = hasher.finalize();
    format!("{:x}", result)
}

fn hash_blake(string: String) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(string.as_bytes());
    let result = hasher.finalize();

    let result = result.to_hex();

    result.to_string()
}

fn hash_ahash(string: String) -> String {
    let mut hasher = ahash::AHasher::default();
    string.hash(&mut hasher);
    let result = hasher.finish();
    format!("{:x}", result)
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("sha256_flow", |b| {
        b.iter(|| hash_sha(black_box("hello".to_string())))
    });
    c.bench_function("blake3_flow", |b| {
        b.iter(|| hash_blake(black_box("hello".to_string())))
    });
    c.bench_function("ahash_flow", |b| {
        b.iter(|| hash_ahash(black_box("hello".to_string())))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

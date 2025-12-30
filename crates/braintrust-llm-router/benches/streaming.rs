use criterion::{criterion_group, criterion_main, Criterion};

fn bench_streaming_placeholder(c: &mut Criterion) {
    c.bench_function("streaming_placeholder", |b| b.iter(|| ()));
}

criterion_group!(benches, bench_streaming_placeholder);
criterion_main!(benches);

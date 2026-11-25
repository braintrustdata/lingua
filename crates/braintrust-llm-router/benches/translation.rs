use criterion::{Criterion, criterion_group, criterion_main};

fn bench_translation_placeholder(c: &mut Criterion) {
    c.bench_function("translation_placeholder", |b| b.iter(|| ()));
}

criterion_group!(benches, bench_translation_placeholder);
criterion_main!(benches);

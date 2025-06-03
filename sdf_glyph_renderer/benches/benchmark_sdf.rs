use criterion::{criterion_group, criterion_main, Criterion};
use sdf_glyph_renderer::BitmapGlyph;
use std::hint::black_box;

pub fn benchmark_sdf(c: &mut Criterion) {
    c.bench_function("benchmark standard sdf gen", |b| {
        let alpha = Vec::from(include!("../fixtures/glyph_alpha.json"));
        let bitmap = black_box(BitmapGlyph::new(alpha, 16, 19, 3).unwrap());
        b.iter(|| bitmap.render_sdf(8))
    });
}

criterion_group!(benches, benchmark_sdf);
criterion_main!(benches);

use std::{fs, path::Path};

use criterion::{criterion_group, criterion_main, Criterion};

use strtod::strtod;

fn load_data(filename: impl AsRef<Path>) -> Vec<String> {
    let filename = filename.as_ref();
    fs::read_to_string(&filename)
        .unwrap()
        .trim()
        .lines()
        .map(String::from)
        .collect()
}
fn bench_strtod_mesh(c: &mut Criterion) {
    let data = load_data("ext/data/mesh.txt");
    c.bench_function("bench_strtod_mesh", |b| {
        b.iter(|| {
            std::hint::black_box(for s in &data {
                strtod(s);
            });
        });
    });
}

criterion_group!(benches, bench_strtod_mesh,);
criterion_main!(benches);

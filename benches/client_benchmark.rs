use criterion::{black_box, criterion_group, criterion_main, Criterion};
use reqwest::Client;

fn build_client_benchmark(c: &mut Criterion) {
    c.bench_function("build_client", |b| {
        b.iter(|| {
            let client = Client::builder()
                .build()
                .unwrap();
            black_box(client);
        })
    });
}

fn reuse_client_benchmark(c: &mut Criterion) {
    let client = Client::builder()
        .build()
        .unwrap();
    c.bench_function("reuse_client", |b| {
        b.iter(|| {
            let builder = client.get("http://example.com");
            black_box(builder);
        })
    });
}

criterion_group!(benches, build_client_benchmark, reuse_client_benchmark);
criterion_main!(benches);

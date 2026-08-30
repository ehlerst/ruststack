use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ruststack_sts::StsEngine;

fn sts_benchmark(c: &mut Criterion) {
    let sts = StsEngine::new("000000000000".to_string(), "us-east-1".to_string());

    let mut group = c.benchmark_group("sts_operations");

    group.bench_function("get_caller_identity", |b| {
        b.iter(|| {
            let res = sts.get_caller_identity(black_box(None));
            black_box(res);
        });
    });

    group.bench_function("assume_role", |b| {
        b.iter(|| {
            let res = sts.assume_role(
                black_box("arn:aws:iam::000000000000:role/deploy-role"),
                black_box("test-session"),
                black_box(Some(3600)),
            );
            black_box(res);
        });
    });

    group.finish();
}

criterion_group!(benches, sts_benchmark);
criterion_main!(benches);

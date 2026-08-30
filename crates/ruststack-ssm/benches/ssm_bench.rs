use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ruststack_ssm::{PutParameterRequest, SsmEngine};

fn ssm_benchmark(c: &mut Criterion) {
    let ssm = SsmEngine::new("000000000000".to_string(), "us-east-1".to_string());

    // Pre-populate 100 parameters
    for i in 0..100 {
        ssm.put_parameter(PutParameterRequest {
            name: format!("/app/prod/service/key_{}", i),
            value: format!("secret_value_{}", i),
            parameter_type: Some("SecureString".to_string()),
            description: None,
            overwrite: Some(true),
            key_id: None,
            tier: None,
            data_type: None,
            allowed_pattern: None,
        })
        .unwrap();
    }

    let mut group = c.benchmark_group("ssm_operations");

    group.bench_function("get_parameter", |b| {
        b.iter(|| {
            let res = ssm.get_parameter(black_box("/app/prod/service/key_42"), false);
            black_box(res).unwrap();
        });
    });

    group.bench_function("get_parameters_by_path_recursive", |b| {
        b.iter(|| {
            let res = ssm.get_parameters_by_path(black_box("/app/prod"), true, false, Some(50));
            black_box(res).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, ssm_benchmark);
criterion_main!(benches);

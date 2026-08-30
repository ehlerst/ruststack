use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ruststack_secretsmanager::{CreateSecretRequest, SecretsManagerEngine};

fn secretsmanager_benchmark(c: &mut Criterion) {
    let sm = SecretsManagerEngine::new("000000000000".to_string(), "us-east-1".to_string());

    // Pre-populate secrets
    for i in 0..100 {
        sm.create_secret(CreateSecretRequest {
            name: format!("app/prod/database_{}", i),
            description: Some("Database credentials".to_string()),
            kms_key_id: None,
            secret_string: Some(r#"{"username":"admin","password":"secret_pw_123"}"#.to_string()),
            secret_binary: None,
            client_request_token: None,
        })
        .unwrap();
    }

    let mut group = c.benchmark_group("secretsmanager_operations");

    group.bench_function("get_secret_value", |b| {
        b.iter(|| {
            let res = sm.get_secret_value(black_box("app/prod/database_42"), None, None);
            black_box(res).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, secretsmanager_benchmark);
criterion_main!(benches);

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ruststack_sns::SnsEngine;
use ruststack_sqs::SqsEngine;
use std::sync::Arc;

fn sns_publish_benchmark(c: &mut Criterion) {
    let sqs = Arc::new(SqsEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let sns = SnsEngine::new(
        sqs.clone(),
        "000000000000".to_string(),
        "us-east-1".to_string(),
    );

    let topic_arn = sns.create_topic("bench-topic", None).unwrap();
    let queue_url = sqs.create_queue("bench-queue", None).unwrap();
    sns.subscribe(&topic_arn, "sqs", &queue_url, None).unwrap();

    let mut group = c.benchmark_group("sns_operations");

    group.bench_function("publish_with_sqs_fanout", |b| {
        b.iter(|| {
            let res = sns.publish(
                black_box(&topic_arn),
                black_box("Benchmark Notification Payload".to_string()),
                Some("Test Subject".to_string()),
                None,
                None,
                None,
            );
            black_box(res).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, sns_publish_benchmark);
criterion_main!(benches);

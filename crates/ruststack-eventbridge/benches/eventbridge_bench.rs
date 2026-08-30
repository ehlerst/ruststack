use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ruststack_eventbridge::{EventBridgeEngine, PutEventsRequestEntry, Target};
use ruststack_sns::SnsEngine;
use ruststack_sqs::SqsEngine;
use std::sync::Arc;

fn eventbridge_put_events_benchmark(c: &mut Criterion) {
    let sqs = Arc::new(SqsEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let sns = Arc::new(SnsEngine::new(
        sqs.clone(),
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let eb = EventBridgeEngine::new(
        sqs.clone(),
        sns.clone(),
        "000000000000".to_string(),
        "us-east-1".to_string(),
    );

    let queue_url = sqs.create_queue("target-queue", None).unwrap();
    let queue_attrs = sqs
        .get_queue_attributes(&queue_url, &["QueueArn".to_string()])
        .unwrap();
    let queue_arn = queue_attrs.get("QueueArn").unwrap().clone();

    let pattern = r#"{"source": ["custom.orders"], "detail-type": ["OrderPlaced"]}"#;
    eb.put_rule(
        "bench-rule",
        None,
        Some(pattern.to_string()),
        Some("ENABLED"),
        None,
        None,
    )
    .unwrap();
    eb.put_targets(
        "bench-rule",
        None,
        vec![Target {
            id: "target-1".to_string(),
            arn: queue_arn,
            input: None,
            input_path: None,
            role_arn: None,
        }],
    )
    .unwrap();

    let mut group = c.benchmark_group("eventbridge_operations");

    group.bench_function("put_events_with_rule_and_sqs_dispatch", |b| {
        b.iter(|| {
            let entry = PutEventsRequestEntry {
                time: None,
                source: Some("custom.orders".to_string()),
                resources: None,
                detail_type: Some("OrderPlaced".to_string()),
                detail: Some(r#"{"order_id": "12345", "amount": 99.99}"#.to_string()),
                event_bus_name: None,
                trace_header: None,
            };
            let res = eb.put_events(black_box(vec![entry]));
            black_box(res).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, eventbridge_put_events_benchmark);
criterion_main!(benches);

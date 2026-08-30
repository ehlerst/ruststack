use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use ruststack_sqs::{SendMessageBatchEntry, SqsEngine};
use std::sync::Arc;

fn sqs_send_receive_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("sqs_operations");
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let engine = Arc::new(SqsEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let queue_url = engine.create_queue("bench-queue", None).unwrap();

    // 1. Single SendMessage
    group.throughput(Throughput::Elements(1));
    group.bench_function("send_message_single", |b| {
        let mut i = 0u64;
        b.iter(|| {
            i += 1;
            let body = format!("message payload index {}", i);
            let res = engine
                .send_message(&queue_url, black_box(body), None, None, None, None)
                .unwrap();
            black_box(res);
        });
    });

    // 2. Batch SendMessage (10 messages)
    group.throughput(Throughput::Elements(10));
    group.bench_function("send_message_batch_10", |b| {
        let mut i = 0u64;
        b.iter(|| {
            i += 1;
            let entries: Vec<SendMessageBatchEntry> = (0..10)
                .map(|j| SendMessageBatchEntry {
                    id: format!("msg_{}_{}", i, j),
                    message_body: format!("batch message {} item {}", i, j),
                    delay_seconds: None,
                    message_attributes: None,
                    message_group_id: None,
                    message_deduplication_id: None,
                })
                .collect();
            let res = engine
                .send_message_batch(&queue_url, black_box(entries))
                .unwrap();
            black_box(res);
        });
    });

    // 3. Receive & Delete Cycle
    group.throughput(Throughput::Elements(10));
    group.bench_function("receive_and_delete_10", |b| {
        b.iter(|| {
            rt.block_on(async {
                let msgs = engine
                    .receive_message(&queue_url, 10, Some(30), Some(0))
                    .await
                    .unwrap();
                for m in &msgs {
                    let _ = engine.delete_message(&queue_url, &m.receipt_handle);
                }
                black_box(msgs);
            });
        });
    });

    // 4. FIFO Send with Deduplication
    let fifo_url = engine.create_queue("bench-fifo.fifo", None).unwrap();

    group.throughput(Throughput::Elements(1));
    group.bench_function("send_message_fifo", |b| {
        let mut i = 0u64;
        b.iter(|| {
            i += 1;
            let res = engine
                .send_message(
                    &fifo_url,
                    format!("order_{}", i),
                    None,
                    None,
                    Some("group-1".to_string()),
                    Some(format!("dedup_{}", i)),
                )
                .unwrap();
            black_box(res);
        });
    });

    group.finish();
}

criterion_group!(benches, sqs_send_receive_bench);
criterion_main!(benches);

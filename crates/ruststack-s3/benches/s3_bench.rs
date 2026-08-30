use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ruststack_s3::{InMemoryStorage, S3Storage};
use std::collections::HashMap;

fn s3_put_get_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("s3_put_get");

    for size in [1024, 64 * 1024, 1024 * 1024] {
        group.throughput(Throughput::Bytes(size as u64));

        let data = Bytes::from(vec![b'x'; size]);
        let storage = InMemoryStorage::new();
        storage.create_bucket("bench-bucket", "us-east-1").unwrap();

        group.bench_with_input(BenchmarkId::new("put_object", size), &data, |b, data| {
            let mut i = 0u64;
            b.iter(|| {
                i += 1;
                let key = format!("key-{}", i);
                storage
                    .put_object(
                        "bench-bucket",
                        &key,
                        black_box(data.clone()),
                        Some("application/octet-stream".to_string()),
                        HashMap::new(),
                    )
                    .unwrap();
            });
        });

        // Pre-populate for Get
        storage
            .put_object(
                "bench-bucket",
                "get-bench-key",
                data.clone(),
                Some("application/octet-stream".to_string()),
                HashMap::new(),
            )
            .unwrap();

        group.bench_with_input(BenchmarkId::new("get_object", size), &size, |b, _| {
            b.iter(|| {
                let (meta, bytes, _) = storage
                    .get_object("bench-bucket", "get-bench-key", None)
                    .unwrap();
                black_box((meta, bytes));
            });
        });
    }

    group.finish();
}

fn s3_list_objects_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("s3_list");
    let storage = InMemoryStorage::new();
    storage.create_bucket("list-bucket", "us-east-1").unwrap();

    let data = Bytes::from("bench data");
    for i in 0..1000 {
        let key = format!("prefix/folder_{:02}/item_{:04}.dat", i % 10, i);
        storage
            .put_object(
                "list-bucket",
                &key,
                data.clone(),
                Some("text/plain".to_string()),
                HashMap::new(),
            )
            .unwrap();
    }

    group.bench_function("list_objects_v2_1000", |b| {
        b.iter(|| {
            let res = storage
                .list_objects_v2("list-bucket", Some("prefix/"), None, 1000, None, None)
                .unwrap();
            black_box(res);
        });
    });

    group.bench_function("list_objects_v2_delimiter", |b| {
        b.iter(|| {
            let res = storage
                .list_objects_v2("list-bucket", Some("prefix/"), Some("/"), 1000, None, None)
                .unwrap();
            black_box(res);
        });
    });

    group.finish();
}

fn s3_multipart_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("s3_multipart");
    let storage = InMemoryStorage::new();
    storage
        .create_bucket("mp-bench-bucket", "us-east-1")
        .unwrap();

    let part_data = Bytes::from(vec![b'a'; 5 * 1024 * 1024]); // 5MB part
    group.throughput(Throughput::Bytes(5 * 1024 * 1024));

    group.bench_function("multipart_upload_5mb_part", |b| {
        let upload_id = storage
            .create_multipart_upload("mp-bench-bucket", "large-file.bin", None, HashMap::new())
            .unwrap();
        let mut part_num = 1;

        b.iter(|| {
            part_num += 1;
            let etag = storage
                .upload_part(
                    "mp-bench-bucket",
                    "large-file.bin",
                    &upload_id,
                    part_num,
                    black_box(part_data.clone()),
                )
                .unwrap();
            black_box(etag);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    s3_put_get_bench,
    s3_list_objects_bench,
    s3_multipart_bench
);
criterion_main!(benches);

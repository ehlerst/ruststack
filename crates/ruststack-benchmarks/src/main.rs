use bytes::Bytes;
use clap::Parser;
use ruststack_dynamodb::{
    AttributeDefinition as DdbAttrDef, AttributeValue as DdbAttrVal, DynamoDbEngine,
    KeySchemaElement as DdbKeyElem,
};
use ruststack_eventbridge::{EventBridgeEngine, PutEventsRequestEntry, Target};
use ruststack_s3::{CompletedPart, InMemoryStorage, S3Storage};
use ruststack_secretsmanager::{CreateSecretRequest, SecretsManagerEngine};
use ruststack_sns::SnsEngine;
use ruststack_sqs::{DeleteMessageBatchEntry, SendMessageBatchEntry, SqsEngine};
use ruststack_ssm::{PutParameterRequest, SsmEngine};
use ruststack_sts::StsEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(
    name = "ruststack-benchmarks",
    about = "Performance benchmark runner for RustStack"
)]
pub struct Opts {
    #[arg(short, long, default_value = "all")]
    pub service: String,

    #[arg(short, long, default_value = "5000")]
    pub iterations: usize,

    #[arg(short, long, default_value = "markdown")]
    pub format: String,

    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub service: String,
    pub operation: String,
    pub payload_size: Option<String>,
    pub total_ops: usize,
    pub duration_secs: f64,
    pub ops_per_sec: f64,
    pub throughput_mb_per_sec: Option<f64>,
    pub p50_micros: f64,
    pub p90_micros: f64,
    pub p95_micros: f64,
    pub p99_micros: f64,
    pub rating: String,
}

fn calculate_percentiles(
    mut latencies: Vec<Duration>,
    duration_total: Duration,
    total_ops: usize,
    payload_bytes: Option<usize>,
    service: &str,
    operation: &str,
    payload_desc: Option<&str>,
) -> BenchmarkResult {
    latencies.sort();
    let n = latencies.len();
    let p50 = latencies[n * 50 / 100].as_secs_f64() * 1_000_000.0;
    let p90 = latencies[n * 90 / 100].as_secs_f64() * 1_000_000.0;
    let p95 = latencies[n * 95 / 100].as_secs_f64() * 1_000_000.0;
    let p99 = latencies[n * 99 / 100].as_secs_f64() * 1_000_000.0;

    let duration_secs = duration_total.as_secs_f64();
    let ops_per_sec = total_ops as f64 / duration_secs;
    let throughput_mb =
        payload_bytes.map(|b| (total_ops * b) as f64 / (1024.0 * 1024.0 * duration_secs));

    let rating = if p95 < 20.0 {
        "A+ (Ultra Fast)"
    } else if p95 < 100.0 {
        "A (Excellent)"
    } else if p95 < 500.0 {
        "B+ (Very Good)"
    } else {
        "B (Good)"
    };

    BenchmarkResult {
        service: service.to_string(),
        operation: operation.to_string(),
        payload_size: payload_desc.map(|s| s.to_string()),
        total_ops,
        duration_secs,
        ops_per_sec,
        throughput_mb_per_sec: throughput_mb,
        p50_micros: p50,
        p90_micros: p90,
        p95_micros: p95,
        p99_micros: p99,
        rating: rating.to_string(),
    }
}

pub fn run_s3_benchmarks(iterations: usize) -> Vec<BenchmarkResult> {
    let mut results = Vec::new();
    let storage = InMemoryStorage::new();
    storage.create_bucket("bench-bucket", "us-east-1").unwrap();

    // 1. PutObject 1 KB
    let data_1kb = Bytes::from(vec![b'a'; 1024]);
    let mut latencies = Vec::with_capacity(iterations);
    let start_total = Instant::now();
    for i in 0..iterations {
        let key = format!("1kb_{}", i);
        let start = Instant::now();
        storage
            .put_object("bench-bucket", &key, data_1kb.clone(), None, HashMap::new())
            .unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iterations,
        Some(1024),
        "S3",
        "PutObject",
        Some("1 KB"),
    ));

    // 2. PutObject 64 KB
    let data_64kb = Bytes::from(vec![b'b'; 64 * 1024]);
    let iters_64k = (iterations / 2).max(100);
    let mut latencies = Vec::with_capacity(iters_64k);
    let start_total = Instant::now();
    for i in 0..iters_64k {
        let key = format!("64kb_{}", i);
        let start = Instant::now();
        storage
            .put_object(
                "bench-bucket",
                &key,
                data_64kb.clone(),
                None,
                HashMap::new(),
            )
            .unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iters_64k,
        Some(64 * 1024),
        "S3",
        "PutObject",
        Some("64 KB"),
    ));

    // 3. GetObject 1 KB
    let mut latencies = Vec::with_capacity(iterations);
    let start_total = Instant::now();
    for i in 0..iterations {
        let key = format!("1kb_{}", i);
        let start = Instant::now();
        let _ = storage.get_object("bench-bucket", &key, None).unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iterations,
        Some(1024),
        "S3",
        "GetObject",
        Some("1 KB"),
    ));

    // 4. ListObjectsV2 (1000 items)
    let iters_list = (iterations / 10).max(50);
    let mut latencies = Vec::with_capacity(iters_list);
    let start_total = Instant::now();
    for _ in 0..iters_list {
        let start = Instant::now();
        let _ = storage
            .list_objects_v2("bench-bucket", None, None, 1000, None, None)
            .unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iters_list,
        None,
        "S3",
        "ListObjectsV2",
        Some("1,000 keys"),
    ));

    // 5. Multipart Upload Lifecycle (3 parts of 1MB)
    let iters_mp = (iterations / 20).max(20);
    let part_data = Bytes::from(vec![b'c'; 1024 * 1024]);
    let mut latencies = Vec::with_capacity(iters_mp);
    let start_total = Instant::now();
    for i in 0..iters_mp {
        let start = Instant::now();
        let upload_id = storage
            .create_multipart_upload("bench-bucket", &format!("mp_{}", i), None, HashMap::new())
            .unwrap();
        let etag1 = storage
            .upload_part(
                "bench-bucket",
                &format!("mp_{}", i),
                &upload_id,
                1,
                part_data.clone(),
            )
            .unwrap();
        let etag2 = storage
            .upload_part(
                "bench-bucket",
                &format!("mp_{}", i),
                &upload_id,
                2,
                part_data.clone(),
            )
            .unwrap();
        let etag3 = storage
            .upload_part(
                "bench-bucket",
                &format!("mp_{}", i),
                &upload_id,
                3,
                part_data.clone(),
            )
            .unwrap();
        storage
            .complete_multipart_upload(
                "bench-bucket",
                &format!("mp_{}", i),
                &upload_id,
                vec![
                    CompletedPart {
                        part_number: 1,
                        etag: etag1,
                    },
                    CompletedPart {
                        part_number: 2,
                        etag: etag2,
                    },
                    CompletedPart {
                        part_number: 3,
                        etag: etag3,
                    },
                ],
            )
            .unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iters_mp,
        Some(3 * 1024 * 1024),
        "S3",
        "MultipartUpload",
        Some("3 x 1 MB"),
    ));

    results
}

pub fn run_sqs_benchmarks(iterations: usize) -> Vec<BenchmarkResult> {
    let mut results = Vec::new();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let engine = Arc::new(SqsEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let queue_url = engine.create_queue("bench-queue", None).unwrap();

    // 1. SendMessage Single
    let mut latencies = Vec::with_capacity(iterations);
    let start_total = Instant::now();
    for i in 0..iterations {
        let start = Instant::now();
        engine
            .send_message(&queue_url, format!("payload {}", i), None, None, None, None)
            .unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iterations,
        None,
        "SQS",
        "SendMessage",
        Some("Single"),
    ));

    // 2. SendMessageBatch (10 msgs per call)
    let iters_batch = iterations / 10;
    let mut latencies = Vec::with_capacity(iters_batch);
    let start_total = Instant::now();
    for i in 0..iters_batch {
        let entries: Vec<SendMessageBatchEntry> = (0..10)
            .map(|j| SendMessageBatchEntry {
                id: format!("m_{}_{}", i, j),
                message_body: format!("batch message item {}", j),
                delay_seconds: None,
                message_attributes: None,
                message_group_id: None,
                message_deduplication_id: None,
            })
            .collect();

        let start = Instant::now();
        engine.send_message_batch(&queue_url, entries).unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iters_batch * 10,
        None,
        "SQS",
        "SendMessageBatch",
        Some("10 msgs/batch"),
    ));

    // 3. ReceiveMessage & DeleteMessage Cycle
    let iters_recv = iterations / 5;
    let mut latencies = Vec::with_capacity(iters_recv);
    let start_total = Instant::now();
    for _ in 0..iters_recv {
        let start = Instant::now();
        rt.block_on(async {
            let msgs = engine
                .receive_message(&queue_url, 10, Some(30), Some(0))
                .await
                .unwrap();
            let del_entries: Vec<_> = msgs
                .iter()
                .map(|m| DeleteMessageBatchEntry {
                    id: m.message_id.clone(),
                    receipt_handle: m.receipt_handle.clone(),
                })
                .collect();
            let _ = engine.delete_message_batch(&queue_url, del_entries);
        });
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iters_recv,
        None,
        "SQS",
        "Receive&DeleteBatch",
        Some("10 msgs/batch"),
    ));

    // 4. FIFO Send with Deduplication
    let fifo_url = engine.create_queue("bench-fifo.fifo", None).unwrap();
    let mut latencies = Vec::with_capacity(iterations);
    let start_total = Instant::now();
    for i in 0..iterations {
        let start = Instant::now();
        engine
            .send_message(
                &fifo_url,
                format!("order_{}", i),
                None,
                None,
                Some("group-1".to_string()),
                Some(format!("dedup_{}", i)),
            )
            .unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iterations,
        None,
        "SQS",
        "SendFifoMessage",
        Some("With Dedup"),
    ));

    results
}

pub fn run_sns_benchmarks(iterations: usize) -> Vec<BenchmarkResult> {
    let mut results = Vec::new();
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

    // 1. Publish without subscribers
    let mut latencies = Vec::with_capacity(iterations);
    let start_total = Instant::now();
    for i in 0..iterations {
        let start = Instant::now();
        sns.publish(
            &topic_arn,
            format!("notification body {}", i),
            Some("Subject".to_string()),
            None,
            None,
            None,
        )
        .unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iterations,
        None,
        "SNS",
        "Publish",
        Some("No Subscribers"),
    ));

    // 2. Publish with 5 SQS Queues Fanout
    let fanout_topic = sns.create_topic("fanout-topic", None).unwrap();
    for i in 0..5 {
        let q = sqs.create_queue(&format!("fanout-q-{}", i), None).unwrap();
        sns.subscribe(&fanout_topic, "sqs", &q, None).unwrap();
    }

    let iters_fanout = iterations / 2;
    let mut latencies = Vec::with_capacity(iters_fanout);
    let start_total = Instant::now();
    for i in 0..iters_fanout {
        let start = Instant::now();
        sns.publish(
            &fanout_topic,
            format!("fanout notification {}", i),
            Some("Fanout".to_string()),
            None,
            None,
            None,
        )
        .unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iters_fanout * 5,
        None,
        "SNS",
        "PublishWithFanout",
        Some("5 SQS Subscribers"),
    ));

    results
}

pub fn run_eventbridge_benchmarks(iterations: usize) -> Vec<BenchmarkResult> {
    let mut results = Vec::new();
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

    let queue_url = sqs.create_queue("eb-target-queue", None).unwrap();
    let queue_attrs = sqs
        .get_queue_attributes(&queue_url, &["QueueArn".to_string()])
        .unwrap();
    let queue_arn = queue_attrs.get("QueueArn").unwrap().clone();

    let pattern = r#"{"source": ["ecommerce.orders"], "detail-type": ["OrderPlaced"]}"#;
    eb.put_rule(
        "orders-rule",
        None,
        Some(pattern.to_string()),
        Some("ENABLED"),
        None,
        None,
    )
    .unwrap();
    eb.put_targets(
        "orders-rule",
        None,
        vec![Target {
            id: "sqs-target".to_string(),
            arn: queue_arn,
            input: None,
            input_path: None,
            role_arn: None,
        }],
    )
    .unwrap();

    // 1. PutEvents (Single Event matching rule and dispatching to SQS)
    let mut latencies = Vec::with_capacity(iterations);
    let start_total = Instant::now();
    for i in 0..iterations {
        let entry = PutEventsRequestEntry {
            time: None,
            source: Some("ecommerce.orders".to_string()),
            resources: None,
            detail_type: Some("OrderPlaced".to_string()),
            detail: Some(format!(r#"{{"order_id": {}, "status": "CONFIRMED"}}"#, i)),
            event_bus_name: None,
            trace_header: None,
        };
        let start = Instant::now();
        eb.put_events(vec![entry]).unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iterations,
        None,
        "EventBridge",
        "PutEvents",
        Some("Rule Pattern + SQS Target"),
    ));

    results
}

pub fn run_ssm_benchmarks(iterations: usize) -> Vec<BenchmarkResult> {
    let mut results = Vec::new();
    let ssm = SsmEngine::new("000000000000".to_string(), "us-east-1".to_string());

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

    // 1. GetParameter
    let mut latencies = Vec::with_capacity(iterations);
    let start_total = Instant::now();
    for _ in 0..iterations {
        let start = Instant::now();
        ssm.get_parameter("/app/prod/service/key_42", false)
            .unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iterations,
        None,
        "SSM",
        "GetParameter",
        Some("Exact Key"),
    ));

    // 2. GetParametersByPath (Recursive)
    let iters_path = (iterations / 2).max(100);
    let mut latencies = Vec::with_capacity(iters_path);
    let start_total = Instant::now();
    for _ in 0..iters_path {
        let start = Instant::now();
        ssm.get_parameters_by_path("/app/prod", true, false, Some(50))
            .unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iters_path,
        None,
        "SSM",
        "GetParametersByPath",
        Some("50 Keys Recursive"),
    ));

    results
}

pub fn run_secretsmanager_benchmarks(iterations: usize) -> Vec<BenchmarkResult> {
    let mut results = Vec::new();
    let sm = SecretsManagerEngine::new("000000000000".to_string(), "us-east-1".to_string());

    for i in 0..50 {
        sm.create_secret(CreateSecretRequest {
            name: format!("app/prod/db_{}", i),
            description: Some("DB credentials".to_string()),
            kms_key_id: None,
            secret_string: Some(r#"{"username":"admin","password":"secret_pw_123"}"#.to_string()),
            secret_binary: None,
            client_request_token: None,
        })
        .unwrap();
    }

    // 1. GetSecretValue
    let mut latencies = Vec::with_capacity(iterations);
    let start_total = Instant::now();
    for _ in 0..iterations {
        let start = Instant::now();
        sm.get_secret_value("app/prod/db_25", None, None).unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iterations,
        None,
        "SecretsManager",
        "GetSecretValue",
        Some("JSON Payload"),
    ));

    results
}

pub fn run_sts_benchmarks(iterations: usize) -> Vec<BenchmarkResult> {
    let mut results = Vec::new();
    let sts = StsEngine::new("000000000000".to_string(), "us-east-1".to_string());

    // 1. GetCallerIdentity
    let mut latencies = Vec::with_capacity(iterations);
    let start_total = Instant::now();
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = sts.get_caller_identity(None);
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iterations,
        None,
        "STS",
        "GetCallerIdentity",
        Some("Root Identity"),
    ));

    // 2. AssumeRole
    let iters_role = iterations / 2;
    let mut latencies = Vec::with_capacity(iters_role);
    let start_total = Instant::now();
    for _ in 0..iters_role {
        let start = Instant::now();
        let _ = sts.assume_role(
            "arn:aws:iam::000000000000:role/ci-role",
            "session",
            Some(3600),
        );
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iters_role,
        None,
        "STS",
        "AssumeRole",
        Some("Temporary Credentials"),
    ));

    results
}

pub fn run_dynamodb_benchmarks(iterations: usize) -> Vec<BenchmarkResult> {
    let mut results = Vec::new();
    let ddb = DynamoDbEngine::new("000000000000".to_string(), "us-east-1".to_string());

    ddb.create_table(
        "BenchTable".to_string(),
        vec![
            DdbKeyElem {
                attribute_name: "pk".to_string(),
                key_type: "HASH".to_string(),
            },
            DdbKeyElem {
                attribute_name: "sk".to_string(),
                key_type: "RANGE".to_string(),
            },
        ],
        vec![
            DdbAttrDef {
                attribute_name: "pk".to_string(),
                attribute_type: "S".to_string(),
            },
            DdbAttrDef {
                attribute_name: "sk".to_string(),
                attribute_type: "N".to_string(),
            },
        ],
        Some("PAY_PER_REQUEST".to_string()),
        None,
        None,
    )
    .unwrap();

    // Pre-populate 500 items
    for i in 0..500 {
        let mut item = HashMap::new();
        item.insert("pk".to_string(), DdbAttrVal::S(format!("user_{}", i % 10)));
        item.insert("sk".to_string(), DdbAttrVal::N(i.to_string()));
        item.insert(
            "name".to_string(),
            DdbAttrVal::S(format!("User Name {}", i)),
        );
        ddb.put_item("BenchTable", item, None, None, None).unwrap();
    }

    // 1. PutItem
    let mut latencies = Vec::with_capacity(iterations);
    let start_total = Instant::now();
    for i in 0..iterations {
        let mut item = HashMap::new();
        item.insert("pk".to_string(), DdbAttrVal::S("user_1".to_string()));
        item.insert("sk".to_string(), DdbAttrVal::N(format!("k_{}", i)));
        item.insert("val".to_string(), DdbAttrVal::S("bench_val".to_string()));
        let start = Instant::now();
        ddb.put_item("BenchTable", item, None, None, None).unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iterations,
        None,
        "DynamoDB",
        "PutItem",
        Some("Single Item"),
    ));

    // 2. GetItem
    let mut latencies = Vec::with_capacity(iterations);
    let mut key = HashMap::new();
    key.insert("pk".to_string(), DdbAttrVal::S("user_0".to_string()));
    key.insert("sk".to_string(), DdbAttrVal::N("100".to_string()));

    let start_total = Instant::now();
    for _ in 0..iterations {
        let start = Instant::now();
        ddb.get_item("BenchTable", &key, None, None).unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iterations,
        None,
        "DynamoDB",
        "GetItem",
        Some("Point Read PK+SK"),
    ));

    // 3. Query (Range condition)
    let iters_query = (iterations / 2).max(100);
    let mut latencies = Vec::with_capacity(iters_query);
    let mut attr_values = HashMap::new();
    attr_values.insert(":pk".to_string(), DdbAttrVal::S("user_0".to_string()));
    attr_values.insert(":sk_min".to_string(), DdbAttrVal::N("50".to_string()));
    attr_values.insert(":sk_max".to_string(), DdbAttrVal::N("200".to_string()));

    let start_total = Instant::now();
    for _ in 0..iters_query {
        let start = Instant::now();
        ddb.query(
            "BenchTable",
            None,
            "pk = :pk AND sk BETWEEN :sk_min AND :sk_max",
            None,
            Some(true),
            Some(50),
            None,
            Some(&attr_values),
        )
        .unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iters_query,
        None,
        "DynamoDB",
        "Query",
        Some("BETWEEN Range Scan"),
    ));

    results
}

fn run_kms_benchmarks(iterations: usize) -> Vec<BenchmarkResult> {
    use ruststack_kms::{
        CreateKeyRequest, DecryptRequest, EncryptRequest, GenerateDataKeyRequest, KmsState,
    };
    use base64::Engine;

    let mut results = Vec::new();
    let kms = KmsState::new("000000000000".to_string(), "us-east-1".to_string());

    // 1. CreateKey
    let iters_create = iterations.min(500);
    let mut latencies = Vec::with_capacity(iters_create);
    let start_total = Instant::now();
    for i in 0..iters_create {
        let start = Instant::now();
        let _ = kms.create_key(CreateKeyRequest {
            description: Some(format!("Bench Key {}", i)),
            key_usage: "ENCRYPT_DECRYPT".to_string(),
            key_spec: "SYMMETRIC_DEFAULT".to_string(),
            customer_master_key_spec: "SYMMETRIC_DEFAULT".to_string(),
            tags: None,
        }).unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iters_create,
        None,
        "KMS",
        "CreateKey",
        None,
    ));

    // Master key for crypto benchmarks
    let master_key = kms.create_key(CreateKeyRequest {
        description: Some("Master Bench Key".to_string()),
        key_usage: "ENCRYPT_DECRYPT".to_string(),
        key_spec: "SYMMETRIC_DEFAULT".to_string(),
        customer_master_key_spec: "SYMMETRIC_DEFAULT".to_string(),
        tags: None,
    }).unwrap();

    let raw_payload = vec![0x42u8; 1024]; // 1KB
    let plain_b64 = base64::engine::general_purpose::STANDARD.encode(&raw_payload);

    // 2. Encrypt
    let mut ciphertexts = Vec::with_capacity(iterations);
    let mut latencies = Vec::with_capacity(iterations);
    let start_total = Instant::now();
    for _ in 0..iterations {
        let start = Instant::now();
        let (cipher, _) = kms.encrypt(EncryptRequest {
            key_id: master_key.key_id.clone(),
            plaintext: plain_b64.clone(),
            encryption_algorithm: "SYMMETRIC_DEFAULT".to_string(),
            encryption_context: None,
        }).unwrap();
        latencies.push(start.elapsed());
        ciphertexts.push(cipher);
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iterations,
        Some(1024),
        "KMS",
        "Encrypt",
        Some("1 KB Payload"),
    ));

    // 3. Decrypt
    let mut latencies = Vec::with_capacity(iterations);
    let start_total = Instant::now();
    for i in 0..iterations {
        let start = Instant::now();
        let _ = kms.decrypt(DecryptRequest {
            ciphertext_blob: ciphertexts[i].clone(),
            key_id: None,
            encryption_algorithm: "SYMMETRIC_DEFAULT".to_string(),
            encryption_context: None,
        }).unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iterations,
        Some(1024),
        "KMS",
        "Decrypt",
        Some("1 KB Payload"),
    ));

    // 4. GenerateDataKey
    let mut latencies = Vec::with_capacity(iterations);
    let start_total = Instant::now();
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = kms.generate_data_key(GenerateDataKeyRequest {
            key_id: master_key.key_id.clone(),
            key_spec: "AES_256".to_string(),
            number_of_bytes: Some(32),
            encryption_context: None,
        }).unwrap();
        latencies.push(start.elapsed());
    }
    results.push(calculate_percentiles(
        latencies,
        start_total.elapsed(),
        iterations,
        Some(32),
        "KMS",
        "GenerateDataKey",
        Some("AES-256 (32B)"),
    ));

    results
}

fn format_markdown(results: &[BenchmarkResult]) -> String {
    let mut md = String::new();
    md.push_str("# 🚀 RustStack Performance Benchmark Report\n\n");
    md.push_str("> Automated sub-millisecond local AWS performance rating.\n\n");

    md.push_str("| Service | Operation | Payload / Batch | Throughput (ops/s) | MB/s | p50 (µs) | p95 (µs) | p99 (µs) | Rating |\n");
    md.push_str("|:---|:---|:---|---:|---:|---:|---:|---:|:---|\n");

    for r in results {
        let payload = r.payload_size.as_deref().unwrap_or("-");
        let mb_s = r
            .throughput_mb_per_sec
            .map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "-".to_string());

        md.push_str(&format!(
            "| **{}** | `{}` | {} | {:.0} | {} | {:.1} | {:.1} | {:.1} | **{}** |\n",
            r.service,
            r.operation,
            payload,
            r.ops_per_sec,
            mb_s,
            r.p50_micros,
            r.p95_micros,
            r.p99_micros,
            r.rating
        ));
    }

    md.push_str("\n### Performance Criteria\n");
    md.push_str("- **A+ (Ultra Fast)**: p95 latency < 20 µs (>50k ops/sec)\n");
    md.push_str("- **A (Excellent)**: p95 latency < 100 µs (>10k ops/sec)\n");
    md.push_str("- **B+ (Very Good)**: p95 latency < 500 µs (>2k ops/sec)\n\n");

    md
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let mut all_results = Vec::new();

    if opts.service == "s3" || opts.service == "all" {
        eprintln!(
            "Running S3 performance benchmarks (iterations: {})...",
            opts.iterations
        );
        let s3_res = run_s3_benchmarks(opts.iterations);
        all_results.extend(s3_res);
    }

    if opts.service == "sqs" || opts.service == "all" {
        eprintln!(
            "Running SQS performance benchmarks (iterations: {})...",
            opts.iterations
        );
        let sqs_res = run_sqs_benchmarks(opts.iterations);
        all_results.extend(sqs_res);
    }

    if opts.service == "sns" || opts.service == "all" {
        eprintln!(
            "Running SNS performance benchmarks (iterations: {})...",
            opts.iterations
        );
        let sns_res = run_sns_benchmarks(opts.iterations);
        all_results.extend(sns_res);
    }

    if opts.service == "events" || opts.service == "eventbridge" || opts.service == "all" {
        eprintln!(
            "Running EventBridge performance benchmarks (iterations: {})...",
            opts.iterations
        );
        let eb_res = run_eventbridge_benchmarks(opts.iterations);
        all_results.extend(eb_res);
    }

    if opts.service == "ssm" || opts.service == "all" {
        eprintln!(
            "Running SSM performance benchmarks (iterations: {})...",
            opts.iterations
        );
        let ssm_res = run_ssm_benchmarks(opts.iterations);
        all_results.extend(ssm_res);
    }

    if opts.service == "secretsmanager" || opts.service == "all" {
        eprintln!(
            "Running SecretsManager performance benchmarks (iterations: {})...",
            opts.iterations
        );
        let sm_res = run_secretsmanager_benchmarks(opts.iterations);
        all_results.extend(sm_res);
    }

    if opts.service == "sts" || opts.service == "all" {
        eprintln!(
            "Running STS performance benchmarks (iterations: {})...",
            opts.iterations
        );
        let sts_res = run_sts_benchmarks(opts.iterations);
        all_results.extend(sts_res);
    }

    if opts.service == "dynamodb" || opts.service == "ddb" || opts.service == "all" {
        eprintln!(
            "Running DynamoDB performance benchmarks (iterations: {})...",
            opts.iterations
        );
        let ddb_res = run_dynamodb_benchmarks(opts.iterations);
        all_results.extend(ddb_res);
    }

    if opts.service == "kms" || opts.service == "all" {
        eprintln!(
            "Running KMS performance benchmarks (iterations: {})...",
            opts.iterations
        );
        let kms_res = run_kms_benchmarks(opts.iterations);
        all_results.extend(kms_res);
    }

    let output_str = if opts.format == "json" {
        serde_json::to_string_pretty(&all_results)?
    } else {
        format_markdown(&all_results)
    };

    if let Some(ref path) = opts.output {
        let mut file = File::create(path)?;
        file.write_all(output_str.as_bytes())?;
        eprintln!("Benchmark report saved to: {}", path);
    } else {
        println!("{}", output_str);
    }

    Ok(())
}

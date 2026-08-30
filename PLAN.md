# RustStack Architectural Plan & Implementation Roadmap

> **RustStack**: A blazing-fast, lightweight, single-binary local AWS emulator written in Rust — designed as a high-performance alternative to LocalStack/Ministack.

---

## 1. Executive Summary & Vision

RustStack aims to deliver an ultra-fast, zero-dependency (or minimal dependency) local AWS cloud emulator for testing, local development, and CI/CD pipelines.

### Key Goals
- **Blazing Performance**: Sub-millisecond latency for typical S3/SQS operations, minimal CPU/memory overhead.
- **Drop-in AWS Compatibility**: Native support for standard AWS SDKs (Rust, Python `boto3`, Node/JS, Go, Java), AWS CLI, and Terraform.
- **Single Port Multiplexing**: Seamlessly serves all emulated AWS services over a unified port (default `4566`) using SigV4 headers, `Host` header inspection, `x-amz-target`, and query parameters.
- **Performance Rating & CI Benchmarks**: Every feature is measured with dedicated Criterion and load benchmarks integrated directly into GitHub Actions (GHA) with automated metric tracking and regression detection.

---

## 2. Core Architecture

```
                                  +-----------------------+
                                  | AWS Client / SDK / CLI |
                                  +-----------+-----------+
                                              | HTTP (Port 4566)
                                              v
                              +-------------------------------+
                              |    RustStack HTTP Gateway     |
                              |   (Axum / Hyper / Tokio)      |
                              +---------------+---------------+
                                              |
                                     Service Dispatcher
                      +-----------------------+-----------------------+
                      | (Host / Auth Scope / Target Header / Path)    |
                      v                                               v
        +---------------------------+                   +---------------------------+
        |        ruststack-s3       |                   |       ruststack-sqs       |
        | S3 REST / XML Engine      |                   | SQS Query & JSON Engine   |
        +-------------+-------------+                   +-------------+-------------+
                      |                                               |
         +------------+------------+                     +------------+------------+
         | In-Memory | Filesystem |                     | Standard   |    FIFO     |
         | Storage   | Storage    |                     | Engine     |   Engine    |
         +-------------------------+                     +-------------------------+
```

### 2.1 Service Dispatcher / Protocol Multiplexer
Incoming requests on port `4566` are routed to the appropriate service handler using the standard AWS protocol resolution matrix:
1. **`Authorization` Header (SigV4)**: Extract credential scope `.../us-east-1/<service>/aws4_request`.
2. **`x-amz-target` Header**: For JSON 1.0/1.1 protocols (e.g. `AmazonSQS.SendMessage`, `DynamoDB_20120810.*`).
3. **`Host` Header (Virtual Hosted Style)**: `<bucket>.localhost:4566` or `<bucket>.s3.localhost` $\to$ **S3**.
4. **URL Path & Query Parameters**:
   - `Action=...&Version=2012-11-05` or `/000000000000/<queue_name>` $\to$ **SQS**.
   - Path-style `/bucket/key` $\to$ **S3**.

---

## 3. Initial Features Specification

### 3.1 Amazon S3 (`ruststack-s3`)
Supports both Path-Style (`http://localhost:4566/<bucket>/<key>`) and Virtual-Hosted Style (`http://<bucket>.localhost:4566/<key>`).

#### Operations Supported:
- **Bucket Operations**:
  - `CreateBucket` (`PUT /<bucket>`)
  - `DeleteBucket` (`DELETE /<bucket>`)
  - `ListBuckets` (`GET /`)
  - `HeadBucket` (`HEAD /<bucket>`)
  - `GetBucketLocation` (`GET /<bucket>?location`)
- **Object Operations**:
  - `PutObject` (`PUT /<bucket>/<key>`) with metadata (`x-amz-meta-*`), Content-Type, MD5 / ETag calculation.
  - `GetObject` (`GET /<bucket>/<key>`) with byte-range requests (`Range: bytes=start-end`), conditional headers (`If-Match`, `If-None-Match`).
  - `HeadObject` (`HEAD /<bucket>/<key>`)
  - `DeleteObject` (`DELETE /<bucket>/<key>`)
  - `DeleteObjects` (`POST /<bucket>?delete`) multi-object deletion.
  - `ListObjectsV2` (`GET /<bucket>?list-type=2`) & `ListObjects` (prefix, delimiter, continuation token, max-keys).
  - `CopyObject` (`PUT /<bucket>/<key>` with `x-amz-copy-source`).
- **Multipart Uploads**:
  - `CreateMultipartUpload` (`POST /<bucket>/<key>?uploads`)
  - `UploadPart` (`PUT /<bucket>/<key>?uploadId=...&partNumber=...`)
  - `CompleteMultipartUpload` (`POST /<bucket>/<key>?uploadId=...`)
  - `AbortMultipartUpload` (`DELETE /<bucket>/<key>?uploadId=...`)
  - `ListParts` / `ListMultipartUploads`
- **Storage Backends**:
  - **In-Memory Backend**: Fast concurrent in-memory map for testing and zero-IO benchmark performance.
  - **Filesystem Backend**: Local directory persistence.

---

### 3.2 Amazon SQS (`ruststack-sqs`)
Supports both **SQS Query Protocol** (`Action=SendMessage&...`) and modern AWS SDK **JSON 1.0 Protocol** (`x-amz-target: AmazonSQS.*`).

#### Operations Supported:
- **Queue Operations**:
  - `CreateQueue` (`Action=CreateQueue` or JSON `CreateQueue`) - Standard and FIFO queues.
  - `GetQueueUrl`, `ListQueues`, `GetQueueAttributes`, `SetQueueAttributes`, `DeleteQueue`, `PurgeQueue`.
- **Message Operations**:
  - `SendMessage` with message bodies, delay (`DelaySeconds`), message attributes, and FIFO attributes (`MessageGroupId`, `MessageDeduplicationId`).
  - `SendMessageBatch` (up to 10 messages per batch).
  - `ReceiveMessage`:
    - `MaxNumberOfMessages` (1-10)
    - `VisibilityTimeout` management (in-flight tracking and timeout expiration)
    - `WaitTimeSeconds` (async long-polling without blocking threads)
  - `DeleteMessage` & `DeleteMessageBatch` (via receipt handles).
  - `ChangeMessageVisibility` & `ChangeMessageVisibilityBatch`.
- **Queue Engine**:
  - Tokio-native asynchronous timer wheel for visibility timeout expiration.
  - FIFO deduplication window (5 minutes) and group sequence ordering.
  - Dead Letter Queue (DLQ) support via `RedrivePolicy` and `maxReceiveCount`.

---

## 4. Performance & Benchmarking Architecture

Every feature in RustStack is treated with a **performance-first** mindset. We provide both micro-benchmarks (in-process engine benchmarks) and end-to-end HTTP load benchmarks.

### 4.1 Benchmark Suites
1. **S3 Benchmarks (`benches/s3_bench.rs`)**:
   - `PutObject` throughput (1 KB, 64 KB, 1 MB, 10 MB payloads).
   - `GetObject` read latency and streaming throughput.
   - `ListObjectsV2` latency on flat and hierarchical bucket keys (1k to 10k objects).
   - Multipart Upload concurrency and part assembly throughput.
2. **SQS Benchmarks (`benches/sqs_bench.rs`)**:
   - `SendMessage` single and batch (10 items) throughput (messages/sec).
   - `ReceiveMessage` + `DeleteMessage` round-trip consumer latency under high concurrency.
   - Long-polling efficiency under high idle connection counts (1,000+ waiting consumers).
   - FIFO queue in-order processing rate under multiple message groups.
3. **End-to-End Server Load Benchmarks (`benches/e2e_bench.rs` or load runner)**:
   - Measures raw requests per second (RPS), p50 / p90 / p99 / p99.9 latency, and memory footprint.

### 4.2 GitHub Actions (GHA) Performance Workflow (`.github/workflows/benchmarks.yml`)
To ensure continuous performance tracking and prevent regressions:
- **Per-Feature Matrix**: Individual benchmark jobs run for S3 and SQS independently on every push and pull request.
- **Criterion Output Capture**: Generates JSON & markdown benchmark reports.
- **Step Summary & PR Comments**: Posts clean performance rating tables with:
  - Throughput (ops/sec or MB/s)
  - Latency (p50, p95, p99 in $\mu$s/ms)
  - Memory consumption (peak RSS)
  - Comparison vs previous baseline / main branch.
- **Fail-on-Regression**: Configurable threshold (e.g. warn if regression > 10%, fail if > 25%).

---

## 5. Workspace Project Structure

```
ruststack/
├── .github/
│   └── workflows/
│       ├── ci.yml                 # Build, test, format, clippy
│       └── benchmarks.yml         # Individual S3 & SQS GHA benchmark runner
├── Cargo.toml                     # Cargo workspace definition
├── crates/
│   ├── ruststack-core/            # Shared types, SigV4 parser, error models, router
│   ├── ruststack-s3/              # S3 service implementation, XML codecs, storage
│   ├── ruststack-sqs/             # SQS service implementation, queue engine, codecs
│   ├── ruststack-server/          # Main binary, CLI arguments, Axum server integration
│   └── ruststack-benchmarks/      # Reusable load testing & benchmark harnesses
├── benches/
│   ├── s3_bench.rs                # S3 Criterion benchmark suite
│   ├── sqs_bench.rs               # SQS Criterion benchmark suite
│   └── e2e_bench.rs               # End-to-End HTTP load benchmark suite
├── tests/
│   ├── s3_integration.rs          # S3 integration tests (AWS SDK & HTTP)
│   └── sqs_integration.rs         # SQS integration tests (AWS SDK & HTTP)
├── Dockerfile                     # Multi-stage lightweight scratch/alpine container
└── PLAN.md                        # This document
```

---

## 6. Phased Implementation Plan

### Phase 1: Workspace & Core Foundation
- Initialize Cargo workspace with optimal compiler flags (`lto = "thin"`, `codegen-units = 1`, `opt-level = 3`).
- Implement `ruststack-core`:
  - Request dispatcher & service classifier (SigV4, Host, x-amz-target, path).
  - Common AWS XML/JSON error serializers (`NoSuchBucket`, `NoSuchKey`, `QueueDoesNotExist`, etc.).
  - Unified HTTP middleware (logging, metrics, tracing, CORS).

### Phase 2: S3 Implementation (`ruststack-s3`)
- Fast in-memory storage layer using `dashmap` / `parking_lot` / `bytes::Bytes`.
- Quick-xml based serialization for `ListBucketsResult`, `ListBucketResult`, `DeleteResult`, `CopyObjectResult`.
- Full HTTP routing for S3 REST API (Path-style & Virtual-hosted style).
- Range requests, multipart uploads, and ETag calculation.
- Unit and integration tests with realistic S3 payloads.

### Phase 3: SQS Implementation (`ruststack-sqs`)
- In-memory queue manager supporting Standard and FIFO queues.
- Asynchronous message visibility tracker & long-polling wait queue (`tokio::sync::Notify`).
- Protocol parsers for both SQS Query Form-URL-Encoded and JSON 1.0.
- XML and JSON response serializers matching AWS SQS specs.
- Unit and integration tests covering message lifecycle, batch operations, and visibility expiration.

### Phase 4: Server Integration & Binary (`ruststack-server`)
- Axum router combining S3 and SQS endpoints on port `4566`.
- Graceful shutdown, CLI configuration flags (`--port`, `--host`, `--data-dir`, `--services`).
- Health check endpoints (`/_ruststack/health`, `/_ruststack/info`).

### Phase 5: Individual Benchmark Suite & Performance Rating
- Criterion benchmarks for:
  - S3 `PutObject`, `GetObject`, `ListObjectsV2` at various sizes.
  - SQS `SendMessage`, `ReceiveMessage`, `DeleteMessage`, batch operations.
- Dedicated standalone benchmark CLI / harness to calculate RPS, latency percentiles, and memory usage.

### Phase 6: GitHub Actions Workflows
- `.github/workflows/ci.yml`: Format, Clippy, Multi-platform builds, Unit & Integration tests.
- `.github/workflows/benchmarks.yml`: Individual benchmark runs for S3 and SQS, Markdown summary report generation to GitHub Step Summary.

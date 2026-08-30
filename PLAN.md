# RustStack Architectural Plan & Implementation Roadmap

> **RustStack**: A blazing-fast, lightweight, single-binary local AWS emulator written in Rust — designed as a high-performance alternative to LocalStack/Ministack.

---

## 1. Executive Summary & Vision

RustStack aims to deliver an ultra-fast, zero-dependency local AWS cloud emulator for testing, local development, and CI/CD pipelines.

### Key Goals
- **Blazing Performance**: Sub-millisecond latency for typical S3/SQS/SNS/EventBridge operations, minimal CPU/memory overhead.
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
                  +-------------------+-------+-------------------+
                  | (Host / Auth Scope / Target Header / Path / Query) |
                  v                   v                   v       v
        +-------------------+ +-------------------+ +-------+ +--------------------+
        |   ruststack-s3    | |   ruststack-sqs   | |  SNS  | |   EventBridge      |
        |  REST / XML Engine| | Query & JSON Engine| |Fanout | | Buses & Rule Match|
        +---------+---------+ +---------+---------+ +---+---+ +---------+----------+
                  |                     |               |               |
             In-Memory             Standard/FIFO   SQS Delivery   SQS/SNS Targets
             Storage               + DLQ Redrive
```

---

## 3. Services Implemented

### 3.1 Amazon S3 (`ruststack-s3`)
- **Bucket Operations**: `CreateBucket`, `DeleteBucket`, `ListBuckets`, `HeadBucket`, `GetBucketLocation`.
- **Object Operations**: `PutObject`, `GetObject` (with byte ranges `Range: bytes=start-end`), `HeadObject`, `DeleteObject`, `DeleteObjects`, `CopyObject`.
- **Object Listing**: `ListObjectsV2` & `ListObjects` with prefix, delimiter, and continuation tokens.
- **Multipart Uploads**: `CreateMultipartUpload`, `UploadPart`, `CompleteMultipartUpload`, `AbortMultipartUpload`, `ListParts`.
- **Routing**: Full support for both path-style (`http://localhost:4566/bucket/key`) and virtual-hosted style (`http://bucket.localhost:4566/key`).

### 3.2 Amazon SQS (`ruststack-sqs`)
- **Dual Protocols**: Query Protocol (form-urlencoded & query params) and AWS JSON 1.0 Protocol (`x-amz-target: AmazonSQS.*`).
- **Dead-Letter Queues (DLQ)**: Automatic redrive policy execution when message receive counts exceed `maxReceiveCount`, and `ListDeadLetterSourceQueues`.
- **Queue Operations**: `CreateQueue`, `DeleteQueue`, `GetQueueUrl`, `ListQueues`, `GetQueueAttributes`, `SetQueueAttributes`, `PurgeQueue`.
- **Message Operations**: `SendMessage`, `SendMessageBatch`, `ReceiveMessage` (with long polling & visibility timeout), `DeleteMessage`, `DeleteMessageBatch`, `ChangeMessageVisibility`.
- **Queue Types**: Standard Queues and FIFO Queues (`.fifo` suffix, `MessageGroupId`, `MessageDeduplicationId`, 5-minute deduplication window).

### 3.3 Amazon SNS (`ruststack-sns`)
- **Dual Protocols**: Query Protocol and AWS JSON Protocol (`x-amz-target: AmazonSNS.*`).
- **Topic Management**: `CreateTopic`, `DeleteTopic`, `ListTopics`, `GetTopicAttributes`, `SetTopicAttributes`.
- **Subscriptions**: `Subscribe`, `Unsubscribe`, `ListSubscriptions`, `ListSubscriptionsByTopic`, `GetSubscriptionAttributes`, `SetSubscriptionAttributes`.
- **SQS Fanout**: Automatic zero-latency in-memory fanout from SNS topics to multiple subscribed SQS queues.
- **Delivery & Filtering**: Standard AWS JSON Notification envelope or `RawMessageDelivery`, with attribute `FilterPolicy` matching.

### 3.4 Amazon EventBridge / CloudWatch Events (`ruststack-eventbridge`)
- **Event Buses**: `CreateEventBus`, `DeleteEventBus`, `ListEventBuses`, `DescribeEventBus` (with pre-provisioned `default` bus).
- **Rule Engine**: `PutRule`, `DeleteRule`, `ListRules`, `DescribeRule`, `EnableRule`, `DisableRule`.
- **Pattern Matching**: Content-based JSON rule matching (`source`, `detail-type`, nested `detail`, prefix matching, anything-but, exists).
- **Target Dispatching**: `PutTargets`, `RemoveTargets`, `ListTargetsByRule` with automated dispatching to SQS queues and SNS topics on `PutEvents`.

---

## 4. Performance Rating System & CI Integration

### 4.1 Rating Tiers
- **Grade A+ (Ultra Fast)**: Latency $p95 < 20\,\mu\text{s}$, Throughput $> 50,000\,\text{ops/s}$.
- **Grade A (Excellent)**: Latency $p95 < 100\,\mu\text{s}$, Throughput $> 10,000\,\text{ops/s}$.
- **Grade B+ (Very Good)**: Latency $p95 < 500\,\mu\text{s}$, Throughput $> 2,000\,\text{ops/s}$.

### 4.2 GitHub Actions Benchmark Workflows
The `benchmarks.yml` workflow tests each service independently across four parallel jobs:
1. `s3-performance`: S3 micro-benchmarks and rating report.
2. `sqs-performance`: SQS micro-benchmarks and rating report.
3. `sns-performance`: SNS publish and SQS fanout rating report.
4. `eventbridge-performance`: EventBridge rule pattern matching and target dispatch rating report.

Reports are automatically posted to `$GITHUB_STEP_SUMMARY` and uploaded as workflow artifacts (`.md` and `.json`).

---

## 5. Implementation Roadmap

```
  [Phase 1] (Completed)  --->  [Phase 2] (Completed)     --->  [Phase 3] (Next)
  - S3 (Buckets, Objects)       - SNS Engine & Topics          - DynamoDB Engine
  - SQS (Query & JSON)          - SQS Fanout                   - SSM & SecretsManager
  - Unified Gateway             - DLQ Redrive Policies         - STS Identity Mock
  - GHA Release & Benchmarks    - EventBridge Buses & Rules    - State Reset & Snapshots
```

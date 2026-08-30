# ⚡ RustStack

> **LocalStack / Ministack alternative in Rust — built for ultra-low latency, blazing throughput, and sub-millisecond execution.**

[![CI](https://github.com/ruststack/ruststack/actions/workflows/ci.yml/badge.svg)](https://github.com/ruststack/ruststack/actions/workflows/ci.yml)
[![Performance Benchmarks](https://github.com/ruststack/ruststack/actions/workflows/benchmarks.yml/badge.svg)](https://github.com/ruststack/ruststack/actions/workflows/benchmarks.yml)
[![Release](https://github.com/ruststack/ruststack/actions/workflows/release.yml/badge.svg)](https://github.com/ruststack/ruststack/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

RustStack is a high-performance local AWS emulator designed for instant local development, automated integration testing, and rapid CI/CD test runners. It is delivered as a single lightweight binary with zero runtime dependencies.

---

## ⚡ Performance Highlights

Automated in-memory performance rating on standard developer hardware:

| Service | Operation | Payload / Batch | Throughput | p50 Latency | p95 Latency | Grade |
|:---|:---|:---|---:|---:|---:|:---|
| **S3** | `GetObject` | 1 KB | **422,000+ ops/s** | 1.4 µs | 1.5 µs | **A+ (Ultra Fast)** |
| **S3** | `PutObject` | 1 KB | **171,000+ ops/s** | 4.3 µs | 6.6 µs | **A+ (Ultra Fast)** |
| **SNS** | `Publish` | Single Topic | **227,000+ ops/s** | 3.4 µs | 3.8 µs | **A+ (Ultra Fast)** |
| **SNS** | `PublishWithFanout` | 5 SQS Queues | **122,000+ msgs/s** | 38.6 µs | 44.5 µs | **A (Excellent)** |
| **EventBridge** | `PutEvents` | Pattern Match + SQS Target | **87,000+ ops/s** | 10.0 µs | 13.0 µs | **A+ (Ultra Fast)** |
| **SQS** | `SendMessageBatch` | 10 msgs/batch | **211,000+ msgs/s** | 43.0 µs | 50.8 µs | **A (Excellent)** |
| **SQS** | `SendMessage` | Single | **148,000+ ops/s** | 5.2 µs | 7.1 µs | **A+ (Ultra Fast)** |
| **SQS** | `Receive&DeleteBatch`| 10 msgs/batch | **24,000+ batches/s** | 39.0 µs | 44.8 µs | **A (Excellent)** |

---

## 📦 Features Implemented

### 1. Amazon S3 (`ruststack-s3`)
- **Bucket Operations**: `CreateBucket`, `DeleteBucket`, `ListBuckets`, `HeadBucket`, `GetBucketLocation`.
- **Object Operations**: `PutObject`, `GetObject` (with byte-range `Range: bytes=start-end`), `HeadObject`, `DeleteObject`, `DeleteObjects` (multi-delete), `CopyObject`.
- **Object Listing**: `ListObjectsV2` & `ListObjects` with prefix, delimiter, start-after, and continuation tokens.
- **Multipart Uploads**: `CreateMultipartUpload`, `UploadPart`, `CompleteMultipartUpload`, `AbortMultipartUpload`, `ListParts`.
- **Routing**: Full support for both path-style (`http://localhost:4566/bucket/key`) and virtual-hosted style (`http://bucket.localhost:4566/key`).

### 2. Amazon SQS (`ruststack-sqs`)
- **Protocols**: Dual protocol support — **SQS Query Protocol** (form-urlencoded & query params) and modern AWS SDK **JSON 1.0 Protocol** (`x-amz-target: AmazonSQS.*`).
- **Dead-Letter Queues (DLQ)**: Automatic redrive policy execution when message receive counts exceed `maxReceiveCount`, plus `ListDeadLetterSourceQueues`.
- **Queue Operations**: `CreateQueue`, `DeleteQueue`, `GetQueueUrl`, `ListQueues`, `GetQueueAttributes`, `SetQueueAttributes`, `PurgeQueue`.
- **Message Operations**: `SendMessage`, `SendMessageBatch` (up to 10 messages), `ReceiveMessage` (with `MaxNumberOfMessages`, `VisibilityTimeout`, async long polling), `DeleteMessage`, `DeleteMessageBatch`, `ChangeMessageVisibility`.
- **Queue Types**: Standard Queues and FIFO Queues (`.fifo` suffix, `MessageGroupId`, `MessageDeduplicationId`, 5-minute deduplication window).

### 3. Amazon SNS (`ruststack-sns`)
- **Protocols**: Query Protocol and modern JSON Protocol (`x-amz-target: AmazonSNS.*`).
- **Topic Management**: `CreateTopic`, `DeleteTopic`, `ListTopics`, `GetTopicAttributes`, `SetTopicAttributes`.
- **Subscriptions**: `Subscribe`, `Unsubscribe`, `ListSubscriptions`, `ListSubscriptionsByTopic`, `GetSubscriptionAttributes`, `SetSubscriptionAttributes`.
- **SQS Fanout**: Automatic zero-latency in-memory fanout from SNS topics to multiple subscribed SQS queues.
- **Message Delivery & Filtering**: Standard AWS JSON Notification envelope or `RawMessageDelivery`, with attribute `FilterPolicy` matching.

### 4. Amazon EventBridge / CloudWatch Events (`ruststack-eventbridge`)
- **Event Buses**: `CreateEventBus`, `DeleteEventBus`, `ListEventBuses`, `DescribeEventBus` (with pre-provisioned `default` bus).
- **Rule Engine**: `PutRule`, `DeleteRule`, `ListRules`, `DescribeRule`, `EnableRule`, `DisableRule`.
- **Pattern Matching**: Content-based JSON rule matching (`source`, `detail-type`, nested `detail`, prefix matching, anything-but, exists).
- **Target Dispatching**: `PutTargets`, `RemoveTargets`, `ListTargetsByRule` with automated dispatching to SQS queues and SNS topics on `PutEvents`.

---

## 🚀 Quick Start

### Run on Linux & macOS
```bash
# Build and run directly with cargo (port 4566)
cargo run --release -p ruststack-server -- --port 4566
```

### Windows Users: Use WSL2
Native Windows targets are not supported directly. Windows users should use **WSL2 (Windows Subsystem for Linux)**:
1. Open PowerShell and install WSL if not already installed:
   ```powershell
   wsl --install
   ```
2. In your WSL terminal (e.g. Ubuntu):
   ```bash
   cargo run --release -p ruststack-server -- --port 4566
   ```
3. RustStack is immediately accessible from both WSL and Windows host at `http://localhost:4566`.

---

## 💻 AWS CLI Examples

Set standard dummy credentials:
```bash
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
export AWS_DEFAULT_REGION=us-east-1
```

### S3 Examples
```bash
# Create a bucket
aws --endpoint-url=http://localhost:4566 s3 mb s3://my-bucket

# Upload an object
aws --endpoint-url=http://localhost:4566 s3 cp myfile.txt s3://my-bucket/myfile.txt

# List objects
aws --endpoint-url=http://localhost:4566 s3 ls s3://my-bucket
```

### SQS with Dead-Letter Queue (DLQ)
```bash
# 1. Create DLQ
aws --endpoint-url=http://localhost:4566 sqs create-queue --queue-name orders-dlq

# 2. Create Source Queue with DLQ Redrive Policy
aws --endpoint-url=http://localhost:4566 sqs create-queue \
  --queue-name orders-queue \
  --attributes '{"RedrivePolicy": "{\"deadLetterTargetArn\":\"arn:aws:sqs:us-east-1:000000000000:orders-dlq\",\"maxReceiveCount\":3}"}'
```

### SNS to SQS Fanout
```bash
# 1. Create SNS topic
aws --endpoint-url=http://localhost:4566 sns create-topic --name order-events

# 2. Subscribe SQS queue to topic
aws --endpoint-url=http://localhost:4566 sns subscribe \
  --topic-arn arn:aws:sns:us-east-1:000000000000:order-events \
  --protocol sqs \
  --notification-endpoint http://localhost:4566/000000000000/orders-queue

# 3. Publish to topic (automatically fans out to SQS queue)
aws --endpoint-url=http://localhost:4566 sns publish \
  --topic-arn arn:aws:sns:us-east-1:000000000000:order-events \
  --message '{"order_id": 1234, "status": "COMPLETED"}'
```

### EventBridge Rules & PutEvents
```bash
# 1. Create a rule
aws --endpoint-url=http://localhost:4566 events put-rule \
  --name payment-alerts \
  --event-pattern '{"source": ["payment.gateway"], "detail-type": ["PaymentCaptured"]}'

# 2. Add SQS queue target
aws --endpoint-url=http://localhost:4566 events put-targets \
  --rule payment-alerts \
  --targets '{"Id": "1", "Arn": "arn:aws:sqs:us-east-1:000000000000:orders-queue"}'

# 3. Send event to EventBridge
aws --endpoint-url=http://localhost:4566 events put-events \
  --entries '[{"Source": "payment.gateway", "DetailType": "PaymentCaptured", "Detail": "{\"amount\": 49.99}"}]'
```

---

## 📊 Performance Testing & Benchmarking

Every feature in RustStack comes with dedicated benchmarks and GitHub Actions CI rating workflows.

```bash
# Rate all services
cargo run --release -p ruststack-benchmarks -- --iterations 10000

# Rate only specific services
cargo run --release -p ruststack-benchmarks -- --service s3 --iterations 10000
cargo run --release -p ruststack-benchmarks -- --service sqs --iterations 10000
cargo run --release -p ruststack-benchmarks -- --service sns --iterations 10000
cargo run --release -p ruststack-benchmarks -- --service eventbridge --iterations 10000
```

### Run Criterion Micro-benchmarks
```bash
cargo bench -p ruststack-s3
cargo bench -p ruststack-sqs
cargo bench -p ruststack-sns
cargo bench -p ruststack-eventbridge
```

# ⚡ RustStack

> **LocalStack / Ministack alternative in Rust — built for ultra-low latency, blazing throughput, and sub-millisecond execution.**

[![CI](https://github.com/ruststack/ruststack/actions/workflows/ci.yml/badge.svg)](https://github.com/ruststack/ruststack/actions/workflows/ci.yml)
[![Performance Benchmarks](https://github.com/ruststack/ruststack/actions/workflows/benchmarks.yml/badge.svg)](https://github.com/ruststack/ruststack/actions/workflows/benchmarks.yml)
[![Resource & Startup Benchmark](https://github.com/ruststack/ruststack/actions/workflows/compare-alternatives.yml/badge.svg)](https://github.com/ruststack/ruststack/actions/workflows/compare-alternatives.yml)
[![Release](https://github.com/ruststack/ruststack/actions/workflows/release.yml/badge.svg)](https://github.com/ruststack/ruststack/actions/workflows/release.yml)
[![Docker Image](https://img.shields.io/badge/docker-ehlers320%2Fruststack-blue.svg)](https://hub.docker.com/r/ehlers320/ruststack)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

RustStack is a high-performance local AWS emulator designed for instant local development, automated integration testing, and rapid CI/CD test runners. It is delivered as a single lightweight binary with zero runtime dependencies and official multi-arch Docker images.

---

## ⚡ Resource & Startup Benchmark (RustStack vs MiniStack vs LocalStack)

Automated container startup time, idle memory footprint, and image size measured via [`Testcontainers/rust-measure-alternatives/`](file:///home/tim/git/ruststack/Testcontainers/rust-measure-alternatives/):

| Local Cloud Stack | Docker Image | Image Size | Avg Startup Time | Min Startup Time | Idle Memory (RSS) | Idle CPU |
|:---|:---|---:|---:|---:|---:|---:|
| **⚡ RustStack (Winner)** | `ehlers320/ruststack:latest` | **101.4 MB** | **240.2 ms** | **226.4 ms** | **2.0 MiB** | **0.00%** |
| **MiniStack** | `ministackorg/ministack:latest` | 175.9 MB | 1,607.6 ms | 1,586.5 ms | 33.6 MiB | 0.00% |
| **LocalStack** | `localstack/localstack:3.8.1` | 1,204.1 MB | 3,990.0 ms | 3,972.2 ms | 419.2 MiB | 25.80% |

### 🚀 RustStack Advantages
- **Startup Speed**: **6.7x faster** than MiniStack (240 ms vs 1,608 ms) and **16.6x faster** than LocalStack (3,990 ms).
- **Memory Footprint**: **16.8x less memory** than MiniStack (2.0 MiB vs 33.6 MiB) and **209.6x less memory** than LocalStack (419.2 MiB).
- **Docker Image Size**: **1.7x smaller** than MiniStack and **11.9x smaller** than LocalStack.

---

## ⚡ Performance Highlights

Automated in-memory performance rating on standard developer hardware:

| Service | Operation | Payload / Batch | Throughput | p50 Latency | p95 Latency | Grade |
|:---|:---|:---|---:|---:|---:|:---|
| **SSM** | `GetParameter` | Exact Key | **450,000+ ops/s** | **1.0 µs** | 1.5 µs | **A+ (Ultra Fast)** |
| **S3** | `GetObject` | 1 KB | **422,000+ ops/s** | **1.4 µs** | 1.5 µs | **A+ (Ultra Fast)** |
| **STS** | `GetCallerIdentity` | Root Identity | **421,000+ ops/s** | **1.0 µs** | 1.5 µs | **A+ (Ultra Fast)** |
| **DynamoDB** | `GetItem` | Point Read PK+SK | **389,000+ ops/s** | **1.4 µs** | 1.9 µs | **A+ (Ultra Fast)** |
| **DynamoDB** | `PutItem` | Single Item | **364,000+ ops/s** | **1.4 µs** | 2.0 µs | **A+ (Ultra Fast)** |
| **SecretsManager** | `GetSecretValue` | JSON Payload | **293,000+ ops/s** | **2.4 µs** | 2.8 µs | **A+ (Ultra Fast)** |
| **SNS** | `Publish` | Single Topic | **227,000+ ops/s** | **3.4 µs** | 3.8 µs | **A+ (Ultra Fast)** |
| **STS** | `AssumeRole` | Temporary Creds | **223,000+ ops/s** | **3.4 µs** | 3.8 µs | **A+ (Ultra Fast)** |
| **SQS** | `SendMessageBatch` | 10 msgs/batch | **211,000+ msgs/s** | **42.8 µs** | 55.4 µs | **A (Excellent)** |
| **S3** | `PutObject` | 1 KB | **175,000+ ops/s** | **4.3 µs** | 5.7 µs | **A+ (Ultra Fast)** |
| **SQS** | `SendMessage` | Single | **156,000+ ops/s** | **5.2 µs** | 5.7 µs | **A+ (Ultra Fast)** |
| **SNS** | `PublishWithFanout` | 5 SQS Queues | **122,000+ msgs/s** | **39.1 µs** | 44.6 µs | **A (Excellent)** |
| **SSM** | `GetParametersByPath` | 50 Keys Recursive | **99,000+ ops/s** | **9.0 µs** | 9.2 µs | **A+ (Ultra Fast)** |
| **EventBridge** | `PutEvents` | Pattern Match + SQS Target | **85,000+ ops/s** | **10.0 µs** | 14.7 µs | **A+ (Ultra Fast)** |

---

## 📦 Features Implemented

### 1. Amazon S3 (`ruststack-s3`)
- **Bucket Operations**: `CreateBucket`, `DeleteBucket`, `ListBuckets`, `HeadBucket`, `GetBucketLocation`.
- **Object Operations**: `PutObject`, `GetObject` (with byte-range `Range: bytes=start-end`), `HeadObject`, `DeleteObject`, `DeleteObjects` (multi-delete), `CopyObject`.
- **Object Listing**: `ListObjectsV2` & `ListObjects` with prefix, delimiter, start-after, and continuation tokens.
- **Multipart Uploads**: `CreateMultipartUpload`, `UploadPart`, `CompleteMultipartUpload`, `AbortMultipartUpload`, `ListParts`.
- **Bucket Notifications**: `PutBucketNotificationConfiguration` and `GetBucketNotificationConfiguration` with instant in-memory event dispatching to SQS queues, SNS topics, and EventBridge default bus on `s3:ObjectCreated:*`, `s3:ObjectCreated:Put`, `s3:ObjectCreated:Copy`, `s3:ObjectCreated:CompleteMultipartUpload`, and `s3:ObjectRemoved:Delete` with key prefix/suffix filtering.
- **Routing**: Full support for both path-style (`http://localhost:4566/bucket/key`) and virtual-hosted style (`http://bucket.localhost:4566/key`).

### 2. Amazon DynamoDB (`ruststack-dynamodb`)
- **Protocols**: AWS JSON 1.0 protocol (`x-amz-target: DynamoDB_20120810.*`).
- **Table Operations**: `CreateTable`, `DeleteTable`, `DescribeTable`, `ListTables` (supporting `String`, `Number`, `Binary` Partition & Sort Keys, `PAY_PER_REQUEST` billing mode, GSIs and LSIs).
- **Item Operations**: `PutItem` (with `ConditionExpression`), `GetItem` (with `ProjectionExpression`), `DeleteItem`, `UpdateItem` (with `SET` / `REMOVE` expressions).
- **Batch Operations**: `BatchGetItem`, `BatchWriteItem` across multiple tables.
- **Query & Scan Engine**: `Query` with `KeyConditionExpression` (`=`, `<`, `<=`, `>`, `>=`, `BETWEEN`, `begins_with`), `Scan` with `FilterExpression`.

### 3. Amazon SQS (`ruststack-sqs`)
- **Protocols**: Dual protocol support — **SQS Query Protocol** (form-urlencoded & query params) and modern AWS SDK **JSON 1.0 Protocol** (`x-amz-target: AmazonSQS.*`).
- **Dead-Letter Queues (DLQ)**: Automatic redrive policy execution when message receive counts exceed `maxReceiveCount`, plus `ListDeadLetterSourceQueues`.
- **Queue Operations**: `CreateQueue`, `DeleteQueue`, `GetQueueUrl`, `ListQueues`, `GetQueueAttributes`, `SetQueueAttributes`, `PurgeQueue`.
- **Message Operations**: `SendMessage`, `SendMessageBatch` (up to 10 messages), `ReceiveMessage` (with `MaxNumberOfMessages`, `VisibilityTimeout`, async long polling), `DeleteMessage`, `DeleteMessageBatch`, `ChangeMessageVisibility`.
- **Queue Types**: Standard Queues and FIFO Queues (`.fifo` suffix, `MessageGroupId`, `MessageDeduplicationId`, 5-minute deduplication window).

### 4. Amazon SNS (`ruststack-sns`)
- **Protocols**: Query Protocol and modern JSON Protocol (`x-amz-target: AmazonSNS.*`).
- **Topic Management**: `CreateTopic`, `DeleteTopic`, `ListTopics`, `GetTopicAttributes`, `SetTopicAttributes`.
- **Subscriptions**: `Subscribe`, `Unsubscribe`, `ListSubscriptions`, `ListSubscriptionsByTopic`, `GetSubscriptionAttributes`, `SetSubscriptionAttributes`.
- **SQS Fanout**: Automatic zero-latency in-memory fanout from SNS topics to multiple subscribed SQS queues.
- **Message Delivery & Filtering**: Standard AWS JSON Notification envelope or `RawMessageDelivery`, with attribute `FilterPolicy` matching.

### 5. Amazon EventBridge / CloudWatch Events (`ruststack-eventbridge`)
- **Event Buses**: `CreateEventBus`, `DeleteEventBus`, `ListEventBuses`, `DescribeEventBus` (with pre-provisioned `default` bus).
- **Rule Engine**: `PutRule`, `DeleteRule`, `ListRules`, `DescribeRule`, `EnableRule`, `DisableRule`.
- **Pattern Matching**: Content-based JSON rule matching (`source`, `detail-type`, nested `detail`, prefix matching, anything-but, exists).
- **Target Dispatching**: `PutTargets`, `RemoveTargets`, `ListTargetsByRule` with automated dispatching to SQS queues and SNS topics on `PutEvents`.

### 6. Amazon SSM Parameter Store (`ruststack-ssm`)
- **Parameters**: `PutParameter` (with automatic versioning & overwrite support), `GetParameter` (exact name or version suffix `/key:1`), `GetParameters` (multi-key batch), `DeleteParameter`, `DeleteParameters`, `DescribeParameters`.
- **Hierarchical Paths**: `GetParametersByPath` supporting prefix scans, recursive and single-level sub-tree traversal.
- **Types**: `String`, `StringList`, and `SecureString`.

### 7. Amazon Secrets Manager (`ruststack-secretsmanager`)
- **Secret Lifecycle**: `CreateSecret`, `GetSecretValue`, `PutSecretValue`, `UpdateSecret`, `DeleteSecret` (with force delete), `DescribeSecret`, `ListSecrets`, `GetRandomPassword`.
- **Versioning & Rotation**: Multi-version tracking with staging labels (`AWSCURRENT`, `AWSPREVIOUS`).

### 8. Amazon Security Token Service Mock (`ruststack-sts`)
- **Protocols**: Query / Form-urlencoded and JSON 1.1 protocols (`x-amz-target: AWSSecurityTokenServiceV20110615.*`).
- **Operations**: `GetCallerIdentity` (essential for Terraform & AWS SDK init), `AssumeRole`, `GetSessionToken`.

---

## 🌪️ Chaos Engineering & Fault Injection Engine

Test client resiliency, retry backoff algorithms, and disaster recovery locally:

### 1. Inject DynamoDB Throttling
```bash
curl -X POST http://localhost:4566/_ruststack/chaos/rules \
  -H "Content-Type: application/json" \
  -d '{
    "service": "dynamodb",
    "action": "PutItem",
    "probability": 0.5,
    "error_status": 400,
    "error_code": "ProvisionedThroughputExceededException",
    "error_message": "Rate of requests exceeds current throughput capacity."
  }'
```

### 2. Inject S3 SlowDown / Service Unavailable with Auto-Expiration
```bash
# Inject 503 error for the first 3 requests then automatically recover
curl -X POST http://localhost:4566/_ruststack/chaos/rules \
  -H "Content-Type: application/json" \
  -d '{
    "service": "s3",
    "error_status": 503,
    "error_code": "SlowDown",
    "limit_times": 3
  }'
```

### 3. Inject Network Latency & Jitter
```bash
# Injects 150ms ± 50ms latency on SQS operations
curl -X POST http://localhost:4566/_ruststack/chaos/rules \
  -H "Content-Type: application/json" \
  -d '{
    "service": "sqs",
    "latency_ms": 150,
    "latency_jitter_ms": 50
  }'
```

### 4. Manage Chaos Rules
```bash
# List all active rules and trigger counters
curl http://localhost:4566/_ruststack/chaos/rules

# Clear all rules
curl -X POST http://localhost:4566/_ruststack/chaos/reset

# Toggle chaos globally
curl -X POST http://localhost:4566/_ruststack/chaos/disable
curl -X POST http://localhost:4566/_ruststack/chaos/enable
```

---

## 💾 State Management & Deterministic Testing API

RustStack provides instant state control plane endpoints for integration test isolation, snapshotting, and CI/CD deterministic runs:

### 1. Atomic State Reset (`POST /_ruststack/state/reset`)
```bash
# Wipe all 8 services completely
curl -X POST http://localhost:4566/_ruststack/state/reset

# Selectively reset only S3 and SQS
curl -X POST http://localhost:4566/_ruststack/state/reset \
  -H "Content-Type: application/json" \
  -d '{"services": ["s3", "sqs"]}'
```

### 2. Full State Dump (`GET` or `POST /_ruststack/state/dump`)
```bash
# Export cluster snapshot as JSON payload
curl http://localhost:4566/_ruststack/state/dump > my-state.json

# Or tell RustStack to write directly to a file
curl -X POST http://localhost:4566/_ruststack/state/dump \
  -H "Content-Type: application/json" \
  -d '{"file_path": "/tmp/cluster-state.json"}'
```

### 3. State Restoration (`POST /_ruststack/state/load`)
```bash
# Load state directly from JSON payload
curl -X POST http://localhost:4566/_ruststack/state/load \
  -H "Content-Type: application/json" \
  -d @my-state.json

# Or load from an existing file path
curl -X POST http://localhost:4566/_ruststack/state/load \
  -H "Content-Type: application/json" \
  -d '{"file_path": "/tmp/cluster-state.json"}'
```

---

## 🚀 Quick Start

### Run with Docker
```bash
# Pull official multi-arch image from DockerHub
docker pull ehlers320/ruststack:latest

# Run container on port 4566
docker run -d --name ruststack -p 4566:4566 ehlers320/ruststack:latest
```

### Run Native Binary (Linux & macOS)
```bash
# Build and run directly with cargo
cargo run --release -p ruststack-server -- --port 4566
```

### Windows Users: Use WSL2
Native Windows targets are not supported directly. Windows users should use **WSL2 (Windows Subsystem for Linux)** or Docker:
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

## 🖥️ Embedded Web Admin UI & `ruststack` CLI

RustStack comes out of the box with an embedded, zero-dependency dark-mode visual administration console and a powerful companion CLI:

### 1. Web Admin UI
Open `http://localhost:4566/_ruststack/ui/` in any browser (or navigate directly to `http://localhost:4566/`):
- **Overview & Ratings**: Live dashboard displaying cluster health, region, account ID, and in-memory engine benchmark ratings.
- **Resource Explorers**: Visual inspection tables for S3 Buckets, DynamoDB Tables & Items, SQS Queues, SNS Topics, SSM Parameters, and SecretsManager Secrets.
- **Interactive Chaos Studio**: Create, list, delete, and toggle fault injection rules on the fly with live execution counters.
- **State Snapshot Management**: 1-click JSON snapshot export/copy, import/restore, and atomic selective/full cluster resets.

### 2. `ruststack` CLI Companion Commands
The `ruststack` binary doubles as a local management CLI:
```bash
# Query status and health of running RustStack cluster
ruststack status

# Export full cluster snapshot to a file or stdout
ruststack state dump --output snapshot.json

# Restore cluster state from a snapshot file
ruststack state load snapshot.json

# Atomically reset state (all services or selective)
ruststack state reset --services s3,dynamodb

# Manage Chaos Engineering rules
ruststack chaos list
ruststack chaos add --service dynamodb --action PutItem --probability 0.5 --status 400 --error-code ProvisionedThroughputExceededException
ruststack chaos reset
```

---

## 💻 AWS CLI Examples

Set standard dummy credentials:
```bash
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
export AWS_DEFAULT_REGION=us-east-1
```

### S3 & Bucket Notifications
```bash
# Create bucket and upload
aws --endpoint-url=http://localhost:4566 s3 mb s3://my-bucket
aws --endpoint-url=http://localhost:4566 s3 cp myfile.txt s3://my-bucket/uploads/myfile.txt

# Configure S3 bucket notification to SQS
aws --endpoint-url=http://localhost:4566 s3api put-bucket-notification-configuration \
  --bucket my-bucket \
  --notification-configuration '{
    "QueueConfigurations": [
      {
        "Id": "uploads-notif",
        "QueueArn": "arn:aws:sqs:us-east-1:000000000000:orders-queue",
        "Events": ["s3:ObjectCreated:*"],
        "Filter": {
          "Key": {
            "FilterRules": [
              { "Name": "prefix", "Value": "uploads/" }
            ]
          }
        }
      }
    ]
  }'
```

### DynamoDB
```bash
# Create Table
aws --endpoint-url=http://localhost:4566 dynamodb create-table \
  --table-name Users \
  --attribute-definitions AttributeName=userId,AttributeType=S AttributeName=timestamp,AttributeType=N \
  --key-schema AttributeName=userId,KeyType=HASH AttributeName=timestamp,KeyType=RANGE \
  --billing-mode PAY_PER_REQUEST

# Put Item
aws --endpoint-url=http://localhost:4566 dynamodb put-item \
  --table-name Users \
  --item '{"userId": {"S": "u100"}, "timestamp": {"N": "1700000000"}, "email": {"S": "user@example.com"}}'

# Query
aws --endpoint-url=http://localhost:4566 dynamodb query \
  --table-name Users \
  --key-condition-expression "userId = :uid" \
  --expression-attribute-values '{":uid": {"S": "u100"}}'
```

---

## 🧪 Testcontainers Integration (Go & Rust)

### Go Testcontainers Example
Explore [`Testcontainers/go-testcontainers/`](file:///home/tim/git/ruststack/Testcontainers/go-testcontainers/):
```bash
cd Testcontainers/go-testcontainers
go test -v .
```

### Rust Testcontainers Example
Explore [`Testcontainers/rust-testcontainers/`](file:///home/tim/git/ruststack/Testcontainers/rust-testcontainers/):
```bash
cd Testcontainers/rust-testcontainers
cargo test -- --ignored
```

---

## 📊 Pure Rust AWS Compatibility Test Suite

RustStack features a 100% pure Rust integration and compatibility test suite (`crates/ruststack-compat-tests`) providing full feature parity testing without any Python runtime dependencies:

```bash
# Run all workspace tests (unit tests + full integration suite)
cargo test --workspace

# Run only the AWS Compatibility test suite
cargo test -p ruststack-compat-tests
```

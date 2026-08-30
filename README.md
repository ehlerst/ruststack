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

### 1. Amazon DynamoDB (`ruststack-dynamodb`)
- **Protocols**: AWS JSON 1.0 protocol (`x-amz-target: DynamoDB_20120810.*`).
- **Table Operations**: `CreateTable`, `DeleteTable`, `DescribeTable`, `ListTables` (supporting `String`, `Number`, `Binary` Partition & Sort Keys, `PAY_PER_REQUEST` billing mode, GSIs and LSIs).
- **Item Operations**: `PutItem` (with `ConditionExpression`), `GetItem` (with `ProjectionExpression`), `DeleteItem`, `UpdateItem` (with `SET` / `REMOVE` expressions).
- **Batch Operations**: `BatchGetItem`, `BatchWriteItem` across multiple tables.
- **Query & Scan Engine**: `Query` with `KeyConditionExpression` (`=`, `<`, `<=`, `>`, `>=`, `BETWEEN`, `begins_with`), `Scan` with `FilterExpression`.

### 2. Amazon S3 (`ruststack-s3`)
- **Bucket Operations**: `CreateBucket`, `DeleteBucket`, `ListBuckets`, `HeadBucket`, `GetBucketLocation`.
- **Object Operations**: `PutObject`, `GetObject` (with byte-range `Range: bytes=start-end`), `HeadObject`, `DeleteObject`, `DeleteObjects` (multi-delete), `CopyObject`.
- **Object Listing**: `ListObjectsV2` & `ListObjects` with prefix, delimiter, start-after, and continuation tokens.
- **Multipart Uploads**: `CreateMultipartUpload`, `UploadPart`, `CompleteMultipartUpload`, `AbortMultipartUpload`, `ListParts`.
- **Routing**: Full support for both path-style (`http://localhost:4566/bucket/key`) and virtual-hosted style (`http://bucket.localhost:4566/key`).

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

# Get Item
aws --endpoint-url=http://localhost:4566 dynamodb get-item \
  --table-name Users \
  --key '{"userId": {"S": "u100"}, "timestamp": {"N": "1700000000"}}'

# Query
aws --endpoint-url=http://localhost:4566 dynamodb query \
  --table-name Users \
  --key-condition-expression "userId = :uid" \
  --expression-attribute-values '{":uid": {"S": "u100"}}'
```

### STS & Identity
```bash
# Verify credentials & caller identity
aws --endpoint-url=http://localhost:4566 sts get-caller-identity
```

### SSM Parameter Store
```bash
# Put parameter
aws --endpoint-url=http://localhost:4566 ssm put-parameter \
  --name "/app/prod/database/url" \
  --value "postgres://user:pass@localhost:5432/app" \
  --type "SecureString"

# Get parameter
aws --endpoint-url=http://localhost:4566 ssm get-parameter \
  --name "/app/prod/database/url"
```

### Secrets Manager
```bash
# Create secret
aws --endpoint-url=http://localhost:4566 secretsmanager create-secret \
  --name "prod/api/keys" \
  --secret-string '{"api_key": "sk_live_12345"}'
```

### S3 & SQS
```bash
# S3
aws --endpoint-url=http://localhost:4566 s3 mb s3://my-bucket
aws --endpoint-url=http://localhost:4566 s3 cp myfile.txt s3://my-bucket/myfile.txt

# SQS
aws --endpoint-url=http://localhost:4566 sqs create-queue --queue-name orders-queue
```

---

## 🧪 Pure Rust Native AWS Compatibility Test Suite

RustStack features a 100% pure Rust integration and compatibility test suite (`crates/ruststack-compat-tests`) providing full feature parity testing without any Python runtime dependencies:

```bash
# Run all workspace tests (unit tests + full integration suite)
cargo test --workspace

# Run only the AWS Compatibility test suite
cargo test -p ruststack-compat-tests
```

---

## 📊 Performance Testing & Benchmarking

Every feature in RustStack comes with dedicated benchmarks and GitHub Actions CI rating workflows.

```bash
# Rate all 8 services
cargo run --release -p ruststack-benchmarks -- --iterations 10000

# Rate only specific services
cargo run --release -p ruststack-benchmarks -- --service dynamodb --iterations 10000
cargo run --release -p ruststack-benchmarks -- --service ssm --iterations 10000
```

### Run Criterion Micro-benchmarks
```bash
cargo bench -p ruststack-dynamodb
cargo bench -p ruststack-s3
cargo bench -p ruststack-sqs
cargo bench -p ruststack-sns
cargo bench -p ruststack-eventbridge
cargo bench -p ruststack-ssm
cargo bench -p ruststack-secretsmanager
cargo bench -p ruststack-sts
```

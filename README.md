# ⚡ RustStack

> **LocalStack / Ministack alternative in Rust — built for ultra-low latency, blazing throughput, and sub-millisecond execution.**

[![CI](https://github.com/ruststack/ruststack/actions/workflows/ci.yml/badge.svg)](https://github.com/ruststack/ruststack/actions/workflows/ci.yml)
[![Performance Benchmarks](https://github.com/ruststack/ruststack/actions/workflows/benchmarks.yml/badge.svg)](https://github.com/ruststack/ruststack/actions/workflows/benchmarks.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

RustStack is a high-performance local AWS emulator designed for instant local development, automated integration testing, and rapid CI/CD test runners. It is delivered as a single lightweight binary with zero runtime dependencies.

---

## ⚡ Performance Highlights

Automated in-memory performance rating on standard developer hardware:

| Service | Operation | Payload / Batch | Throughput | p50 Latency | p95 Latency | Grade |
|:---|:---|:---|---:|---:|---:|:---|
| **S3** | `GetObject` | 1 KB | **415,000+ ops/s** | 1.4 µs | 1.5 µs | **A+ (Ultra Fast)** |
| **S3** | `PutObject` | 1 KB | **170,000+ ops/s** | 4.3 µs | 5.8 µs | **A+ (Ultra Fast)** |
| **SQS** | `SendMessageBatch` | 10 msgs/batch | **218,000+ msgs/s** | 42.5 µs | 48.3 µs | **A (Excellent)** |
| **SQS** | `SendMessage` | Single | **158,000+ ops/s** | 5.2 µs | 5.7 µs | **A+ (Ultra Fast)** |
| **SQS** | `Receive&DeleteBatch`| 10 msgs/batch | **23,000+ batches/s** | 39.6 µs | 54.3 µs | **A (Excellent)** |

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
- **Queue Operations**: `CreateQueue`, `DeleteQueue`, `GetQueueUrl`, `ListQueues`, `GetQueueAttributes`, `SetQueueAttributes`, `PurgeQueue`.
- **Message Operations**: `SendMessage`, `SendMessageBatch` (up to 10 messages), `ReceiveMessage` (with `MaxNumberOfMessages`, `VisibilityTimeout`, async long polling), `DeleteMessage`, `DeleteMessageBatch`, `ChangeMessageVisibility`.
- **Queue Types**: Standard Queues and FIFO Queues (`.fifo` suffix, `MessageGroupId`, `MessageDeduplicationId`, 5-minute deduplication window).

---

## 🚀 Quick Start

### Run via Cargo
```bash
cargo run --release -p ruststack-server -- --port 4566
```

### Run via Docker
```bash
docker build -t ruststack .
docker run -p 4566:4566 ruststack
```

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

# Download object
aws --endpoint-url=http://localhost:4566 s3 cp s3://my-bucket/myfile.txt downloaded.txt
```

### SQS Examples
```bash
# Create a standard queue
aws --endpoint-url=http://localhost:4566 sqs create-queue --queue-name test-queue

# Send a message
aws --endpoint-url=http://localhost:4566 sqs send-message \
  --queue-url http://localhost:4566/000000000000/test-queue \
  --message-body "Hello from RustStack!"

# Receive a message
aws --endpoint-url=http://localhost:4566 sqs receive-message \
  --queue-url http://localhost:4566/000000000000/test-queue

# Create a FIFO queue
aws --endpoint-url=http://localhost:4566 sqs create-queue \
  --queue-name orders.fifo \
  --attributes FifoQueue=true,ContentBasedDeduplication=true
```

---

## 📊 Performance Testing & Benchmarking

Every feature in RustStack comes with dedicated benchmarks and GitHub Actions CI rating workflows.

### Run Performance Rating Tool
```bash
# Rate all services
cargo run --release -p ruststack-benchmarks -- --iterations 10000

# Rate only S3
cargo run --release -p ruststack-benchmarks -- --service s3 --iterations 10000

# Rate only SQS
cargo run --release -p ruststack-benchmarks -- --service sqs --iterations 10000
```

### Run Criterion Micro-benchmarks
```bash
# S3 micro-benchmarks
cargo bench -p ruststack-s3

# SQS micro-benchmarks
cargo bench -p ruststack-sqs
```

---

## 🏗️ Architecture

See [PLAN.md](PLAN.md) for the detailed architecture, roadmap, and design specifications.

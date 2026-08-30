# RustStack Architectural Plan & Implementation Roadmap

> **RustStack**: A blazing-fast, lightweight, single-binary local AWS emulator written in Rust — designed as a high-performance alternative to LocalStack/Ministack.

---

## 1. Executive Summary & Vision

RustStack delivers an ultra-fast, zero-dependency local AWS cloud emulator for testing, local development, and CI/CD pipelines.

### Key Goals
- **Blazing Performance**: Sub-microsecond to single-digit microsecond latency across S3, SQS, SNS, EventBridge, SSM, SecretsManager, STS, and DynamoDB.
- **Drop-in AWS Compatibility**: Native support for standard AWS SDKs (Rust, Python `boto3`, Node/JS, Go, Java), AWS CLI, and Terraform.
- **Pure Rust Native Architecture**: Zero external dependencies, no Python runtime requirement in the codebase.
- **Single Port Multiplexing**: Seamlessly serves all 8 emulated AWS services over a unified port (default `4566`) using SigV4 headers, `Host` header inspection, `x-amz-target`, query parameters, and URL path patterns.
- **State Management & Snapshots**: Instant atomic cluster resets (`POST /_ruststack/state/reset`), full cluster state serialization (`GET/POST /_ruststack/state/dump`), and instant state restoration (`POST /_ruststack/state/load`).
- **Chaos Engineering & Fault Injection Engine**: Highly configurable latency jitter, probabilistic error rate simulation, rule expiration limits, and service/action-level failure injection (`/_ruststack/chaos/rules`).
- **Performance Rating & CI Benchmarks**: Every feature is measured with dedicated Criterion and load benchmarks integrated directly into GitHub Actions (GHA) with automated metric tracking and regression detection.
- **Multi-Platform & Container Delivery**: Multi-arch Linux & macOS binaries and official multi-arch Docker images on DockerHub (`ehlers320/ruststack`).

---

## 2. Core Architecture & State Management

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
     +----------------------------------------+----------------------------------------+
     |                                        |                                        |
     v                                        v                                        v
Chaos Engineering & Fault Injection   Service Dispatcher                      State Control Plane
 (/_ruststack/chaos/rules/enable)    (Host / Auth Scope / Target / Path)   (/_ruststack/state/reset/dump/load)
     |                                v    v    v    v    v    v    v    v                 |
     |                              +---+ +---+ +--+ +--+ +--+ +--+ ++ +---+               v
     +==> [Latency / Jitter / Err]  |S3 | |SQS| |SN| |Ev| |SS| |Se| |S| |DD| <==== Snapshots / Reset
                                    +---+ +---+ +--+ +--+ +--+ +--+ ++ +---+
```

---

## 3. Chaos Engineering & Fault Injection Engine

Configurable dynamic fault injection engine with near-zero nanosecond overhead when idle:

### 3.1 Registering Fault Injection Rules (`POST /_ruststack/chaos/rules`)
```json
{
  "service": "dynamodb",
  "action": "PutItem",
  "probability": 0.5,
  "error_status": 400,
  "error_code": "ProvisionedThroughputExceededException",
  "error_message": "Rate of requests exceeds the configured throughput.",
  "latency_ms": 50,
  "latency_jitter_ms": 10,
  "limit_times": 5
}
```

### 3.2 Chaos Endpoints
- `POST /_ruststack/chaos/rules`: Register new chaos rule (returns `rule_id`).
- `GET /_ruststack/chaos/rules`: List all active chaos rules and trigger statistics.
- `DELETE /_ruststack/chaos/rules/{id}`: Delete specific chaos rule.
- `DELETE /_ruststack/chaos/rules` or `POST /_ruststack/chaos/reset`: Clear all rules.
- `POST /_ruststack/chaos/enable` / `POST /_ruststack/chaos/disable`: Global killswitch.

---

## 4. State Management & Deterministic Testing API

### 4.1 `POST /_ruststack/state/reset`
Atomically clears in-memory state. Supports full cluster wipe or selective service resets:
```json
{
  "services": ["s3", "dynamodb"]
}
```

### 4.2 `GET` or `POST /_ruststack/state/dump`
Serializes the complete cluster snapshot across all 8 services into JSON (or writes to a file path):
```json
{
  "file_path": "/tmp/test-snapshot.json"
}
```

### 4.3 `POST /_ruststack/state/load`
Restores the complete cluster state directly from a JSON snapshot payload or file path:
```json
{
  "file_path": "/tmp/test-snapshot.json"
}
```

---

## 5. Services Implemented

### 5.1 Amazon DynamoDB (`ruststack-dynamodb`)
- **JSON Protocol**: Modern AWS JSON 1.0 protocol (`x-amz-target: DynamoDB_20120810.*`).
- **Table Operations**: `CreateTable`, `DeleteTable`, `DescribeTable`, `ListTables`, `UpdateTable` (supporting String, Number, Binary Partition Keys & Sort Keys, `PAY_PER_REQUEST` billing mode, GSIs and LSIs).
- **Item Operations**: `PutItem` (with `ConditionExpression`), `GetItem` (with `ProjectionExpression`), `DeleteItem`, `UpdateItem` (with `SET` / `REMOVE` expressions), `BatchGetItem`, `BatchWriteItem`.
- **Query & Scan Engine**: `Query` with `KeyConditionExpression` (`=`, `<`, `<=`, `>`, `>=`, `BETWEEN`, `begins_with`), `Scan` with `FilterExpression`.
- **State Management**: Full table definition and item collection snapshotting & restoration.

### 5.2 Amazon S3 (`ruststack-s3`)
- **Bucket Operations**: `CreateBucket`, `DeleteBucket`, `ListBuckets`, `HeadBucket`, `GetBucketLocation`.
- **Object Operations**: `PutObject`, `GetObject` (with byte ranges `Range: bytes=start-end`), `HeadObject`, `DeleteObject`, `DeleteObjects`, `CopyObject`.
- **Object Listing**: `ListObjectsV2` & `ListObjects` with prefix, delimiter, and continuation tokens.
- **Multipart Uploads**: `CreateMultipartUpload`, `UploadPart`, `CompleteMultipartUpload`, `AbortMultipartUpload`, `ListParts`.
- **Bucket Notifications**: `PutBucketNotificationConfiguration` and `GetBucketNotificationConfiguration` with instant in-memory event dispatching to SQS queues, SNS topics, and EventBridge default bus on `s3:ObjectCreated:*`, `s3:ObjectCreated:Put`, `s3:ObjectCreated:Copy`, `s3:ObjectCreated:CompleteMultipartUpload`, and `s3:ObjectRemoved:Delete` with key prefix/suffix filtering.
- **State Management**: Base64 encoded binary object and notification snapshotting & restoration.

### 5.3 Amazon SQS (`ruststack-sqs`)
- **Dual Protocols**: Query Protocol (form-urlencoded & query params) and AWS JSON 1.0 Protocol (`x-amz-target: AmazonSQS.*`).
- **Dead-Letter Queues (DLQ)**: Automatic redrive policy execution when message receive counts exceed `maxReceiveCount`, and `ListDeadLetterSourceQueues`.
- **Message Operations**: `SendMessage`, `SendMessageBatch`, `ReceiveMessage` (with long polling & visibility timeout), `DeleteMessage`, `DeleteMessageBatch`, `ChangeMessageVisibility`.
- **Queue Types**: Standard Queues and FIFO Queues (`.fifo` suffix, `MessageGroupId`, `MessageDeduplicationId`, 5-minute deduplication window).
- **State Management**: Queue attributes and in-flight/pending message queue snapshotting & restoration.

### 5.4 Amazon SNS (`ruststack-sns`)
- **Dual Protocols**: Query Protocol and AWS JSON Protocol (`x-amz-target: AmazonSNS.*`).
- **Topic Management**: `CreateTopic`, `DeleteTopic`, `ListTopics`, `GetTopicAttributes`, `SetTopicAttributes`.
- **Subscriptions & Fanout**: `Subscribe`, `Unsubscribe`, `ListSubscriptions`, `ListSubscriptionsByTopic`, automatic in-memory SQS fanout.
- **Delivery & Filtering**: Standard AWS JSON Notification envelope or `RawMessageDelivery`, with attribute `FilterPolicy` matching.
- **State Management**: Topic definitions and subscriptions snapshotting & restoration.

### 5.5 Amazon EventBridge / CloudWatch Events (`ruststack-eventbridge`)
- **Event Buses**: `CreateEventBus`, `DeleteEventBus`, `ListEventBuses`, `DescribeEventBus` (with default bus).
- **Rule Engine**: `PutRule`, `DeleteRule`, `ListRules`, `DescribeRule`, `EnableRule`, `DisableRule`.
- **Pattern Matching**: Content-based JSON rule matching (`source`, `detail-type`, nested `detail`, prefix matching, anything-but, exists).
- **Target Dispatching**: Automated dispatching to SQS queues and SNS topics on `PutEvents`.
- **State Management**: Rule definitions and target bindings snapshotting & restoration.

### 5.6 Amazon SSM Parameter Store (`ruststack-ssm`)
- **Parameters**: `PutParameter` (with versioning), `GetParameter`, `GetParameters`, `GetParametersByPath` (hierarchical recursive scans), `DeleteParameter`, `DeleteParameters`, `DescribeParameters`.
- **State Management**: Parameter values, histories, and types snapshotting & restoration.

### 5.7 Amazon Secrets Manager (`ruststack-secretsmanager`)
- **Secret Lifecycle**: `CreateSecret`, `GetSecretValue`, `PutSecretValue`, `UpdateSecret`, `DeleteSecret` (with force delete), `DescribeSecret`, `ListSecrets`, `GetRandomPassword`.
- **Versioning**: Version stages (`AWSCURRENT`, `AWSPREVIOUS`) and rotation tracking.
- **State Management**: Secret definitions, versions, and staging labels snapshotting & restoration.

### 5.8 Amazon STS Mock (`ruststack-sts`)
- **Identity & Session**: `GetCallerIdentity`, `AssumeRole`, `GetSessionToken` supporting Query and JSON 1.1 protocols.

---

## 6. Implementation Roadmap Sequence

```
  [Step 1] (Completed)  --->  [Step 2] (Completed)  --->  [Step 3] (Completed)
  - SSM Parameter Store        - DynamoDB Engine           - S3 Bucket Notifications
  - Secrets Manager            - KeyCondition & Scan       - Pure Rust Compat Tests
  - STS Identity Mock          - 8 GHA Benchmarks Jobs     - DockerHub Publishing
                                                                    |
                                                                    v
  [Step 6] (Completed)  <---  [Step 5] (Completed)  <---  [Step 4] (Completed)
  - Embedded Web Admin UI      - Chaos Engineering         - State Management & Snapshots
  - ruststack-cli Commands      (Latency/Fault Injection)   (/_ruststack/state/dump/load)
```

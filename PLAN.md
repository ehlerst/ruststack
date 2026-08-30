# RustStack Architectural Plan & Implementation Roadmap

> **RustStack**: A blazing-fast, lightweight, single-binary local AWS emulator written in Rust — designed as a high-performance alternative to LocalStack/Ministack.

---

## 1. Executive Summary & Vision

RustStack aims to deliver an ultra-fast, zero-dependency local AWS cloud emulator for testing, local development, and CI/CD pipelines.

### Key Goals
- **Blazing Performance**: Sub-microsecond to single-digit microsecond latency across S3, SQS, SNS, EventBridge, SSM, SecretsManager, STS, and DynamoDB.
- **Drop-in AWS Compatibility**: Native support for standard AWS SDKs (Rust, Python `boto3`, Node/JS, Go, Java), AWS CLI, and Terraform.
- **Pure Rust Native Architecture**: Zero external dependencies, no Python runtime requirement in the codebase.
- **Single Port Multiplexing**: Seamlessly serves all 8 emulated AWS services over a unified port (default `4566`) using SigV4 headers, `Host` header inspection, `x-amz-target`, query parameters, and URL path patterns.
- **Performance Rating & CI Benchmarks**: Every feature is measured with dedicated Criterion and load benchmarks integrated directly into GitHub Actions (GHA) with automated metric tracking and regression detection.
- **Multi-Platform & Container Delivery**: Multi-arch Linux & macOS binaries and official multi-arch Docker images on DockerHub (`ehlers320/ruststack`).

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
                  v        v       v        v       v        v       v        v
            +-------+ +------+ +------+ +-------+ +------+ +------+ +----+ +------+
            |Dynamo | |  S3  | | SQS  | |  SNS  | |Events| | SSM  | |Secr| | STS  |
            |  DB   | | REST | |Query/| |Fanout | | Rules| |Store | |Mgr | | Mock |
            | Engine| |Engine| | JSON | | Engine| |Engine| |BTree | |Ver | | Dual |
            +---+---+ +--+---+ +--+---+ +---+---+ +--+---+ +--+---+ +-+--+ +--+---+
                |        |        |         |        |        |       |       |
              Items/  In-Mem   Std/FIFO   SQS      Targets  Prefix  DashMap Caller
              Indexes Storage    +DLQ   Delivery  (SQS/SNS) Ranges  Labels  Id/Role
                         |        ^         ^        ^
                         +--------+---------+--------+
                               S3 Bucket Notifications
                     (s3:ObjectCreated:* / s3:ObjectRemoved:*)
```

---

## 3. Services Implemented

### 3.1 Amazon DynamoDB (`ruststack-dynamodb`)
- **JSON Protocol**: Modern AWS JSON 1.0 protocol (`x-amz-target: DynamoDB_20120810.*`).
- **Table Operations**: `CreateTable`, `DeleteTable`, `DescribeTable`, `ListTables`, `UpdateTable` (supporting String, Number, Binary Partition Keys & Sort Keys, `PAY_PER_REQUEST` billing mode, GSIs and LSIs).
- **Item Operations**: `PutItem` (with `ConditionExpression`), `GetItem` (with `ProjectionExpression`), `DeleteItem`, `UpdateItem` (with `SET` / `REMOVE` expressions), `BatchGetItem`, `BatchWriteItem`.
- **Query & Scan Engine**: `Query` with `KeyConditionExpression` (`=`, `<`, `<=`, `>`, `>=`, `BETWEEN`, `begins_with`), `Scan` with `FilterExpression`.

### 3.2 Amazon S3 (`ruststack-s3`)
- **Bucket Operations**: `CreateBucket`, `DeleteBucket`, `ListBuckets`, `HeadBucket`, `GetBucketLocation`.
- **Object Operations**: `PutObject`, `GetObject` (with byte ranges `Range: bytes=start-end`), `HeadObject`, `DeleteObject`, `DeleteObjects`, `CopyObject`.
- **Object Listing**: `ListObjectsV2` & `ListObjects` with prefix, delimiter, and continuation tokens.
- **Multipart Uploads**: `CreateMultipartUpload`, `UploadPart`, `CompleteMultipartUpload`, `AbortMultipartUpload`, `ListParts`.
- **Bucket Notifications**: `PutBucketNotificationConfiguration` and `GetBucketNotificationConfiguration` with instant in-memory event dispatching to SQS queues, SNS topics, and EventBridge default bus on `s3:ObjectCreated:*`, `s3:ObjectCreated:Put`, `s3:ObjectCreated:Copy`, `s3:ObjectCreated:CompleteMultipartUpload`, and `s3:ObjectRemoved:Delete` with key prefix/suffix filtering.

### 3.3 Amazon SQS (`ruststack-sqs`)
- **Dual Protocols**: Query Protocol (form-urlencoded & query params) and AWS JSON 1.0 Protocol (`x-amz-target: AmazonSQS.*`).
- **Dead-Letter Queues (DLQ)**: Automatic redrive policy execution when message receive counts exceed `maxReceiveCount`, and `ListDeadLetterSourceQueues`.
- **Message Operations**: `SendMessage`, `SendMessageBatch`, `ReceiveMessage` (with long polling & visibility timeout), `DeleteMessage`, `DeleteMessageBatch`, `ChangeMessageVisibility`.
- **Queue Types**: Standard Queues and FIFO Queues (`.fifo` suffix, `MessageGroupId`, `MessageDeduplicationId`, 5-minute deduplication window).

### 3.4 Amazon SNS (`ruststack-sns`)
- **Dual Protocols**: Query Protocol and AWS JSON Protocol (`x-amz-target: AmazonSNS.*`).
- **Topic Management**: `CreateTopic`, `DeleteTopic`, `ListTopics`, `GetTopicAttributes`, `SetTopicAttributes`.
- **Subscriptions & Fanout**: `Subscribe`, `Unsubscribe`, `ListSubscriptions`, `ListSubscriptionsByTopic`, automatic in-memory SQS fanout.
- **Delivery & Filtering**: Standard AWS JSON Notification envelope or `RawMessageDelivery`, with attribute `FilterPolicy` matching.

### 3.5 Amazon EventBridge / CloudWatch Events (`ruststack-eventbridge`)
- **Event Buses**: `CreateEventBus`, `DeleteEventBus`, `ListEventBuses`, `DescribeEventBus` (with default bus).
- **Rule Engine**: `PutRule`, `DeleteRule`, `ListRules`, `DescribeRule`, `EnableRule`, `DisableRule`.
- **Pattern Matching**: Content-based JSON rule matching (`source`, `detail-type`, nested `detail`, prefix matching, anything-but, exists).
- **Target Dispatching**: Automated dispatching to SQS queues and SNS topics on `PutEvents`.

### 3.6 Amazon SSM Parameter Store (`ruststack-ssm`)
- **Parameters**: `PutParameter` (with versioning), `GetParameter`, `GetParameters`, `GetParametersByPath` (hierarchical recursive scans), `DeleteParameter`, `DeleteParameters`, `DescribeParameters`.

### 3.7 Amazon Secrets Manager (`ruststack-secretsmanager`)
- **Secret Lifecycle**: `CreateSecret`, `GetSecretValue`, `PutSecretValue`, `UpdateSecret`, `DeleteSecret`, `DescribeSecret`, `ListSecrets`, `GetRandomPassword`.
- **Versioning**: Version stages (`AWSCURRENT`, `AWSPREVIOUS`) and rotation tracking.

### 3.8 Amazon STS Mock (`ruststack-sts`)
- **Identity & Session**: `GetCallerIdentity`, `AssumeRole`, `GetSessionToken` supporting Query and JSON 1.1 protocols.

---

## 4. Pure Rust Native AWS Compatibility Test Suite (`crates/ruststack-compat-tests`)

Translating the comprehensive test scenarios from Ministack into a 100% pure Rust test harness:

- `test_dynamodb_compat.rs`: Full table and item CRUD, batch operations, query condition expressions, filter expressions.
- `test_s3_compat.rs`: Bucket CRUD, object CRUD, byte-range downloads, `ListObjectsV2` prefix pagination, virtual-hosted routing.
- `test_s3_notifications_compat.rs`: Bucket notification configuration to SQS and SNS on `s3:ObjectCreated:*` with prefix filter rules.
- `test_sqs_compat.rs`: SQS JSON 1.0 protocol, DLQ redrive policies, and Query protocol.
- `test_sns_compat.rs`: Topic CRUD, SQS subscriptions, in-memory fanout, raw delivery, filter policy.
- `test_eventbridge_compat.rs`: Event bus CRUD, rule patterns, SQS target dispatching on `PutEvents`.
- `test_ssm_compat.rs`: Parameter CRUD, overwrite versioning, hierarchical path tree scans.
- `test_secretsmanager_compat.rs`: Secret lifecycle, multi-version rotation, staging labels.
- `test_sts_compat.rs`: `GetCallerIdentity`, `AssumeRole`, `GetSessionToken` across Query and JSON protocols.

---

## 5. Implementation Roadmap Sequence

```
  [Step 1] (Completed)  --->  [Step 2] (Completed)  --->  [Step 3] (Completed)
  - SSM Parameter Store        - DynamoDB Engine           - S3 Bucket Notifications
  - Secrets Manager            - KeyCondition & Scan       - Pure Rust Compat Tests
  - STS Identity Mock          - 8 GHA Benchmarks Jobs     - DockerHub Publishing
                                                                    |
                                                                    v
  [Step 6]              <---  [Step 5]              <---  [Step 4] (Next)
  - Embedded Web Admin UI      - Chaos Engineering         - State Management & Snapshots
  - ruststack-cli               (Latency/Fault Injection)   (/_ruststack/state/dump/load)
```

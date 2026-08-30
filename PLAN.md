# RustStack Architectural Plan & Implementation Roadmap

> **RustStack**: A blazing-fast, lightweight, single-binary local AWS emulator written in Rust — designed as a high-performance alternative to LocalStack/Ministack.

---

## 1. Executive Summary & Vision

RustStack aims to deliver an ultra-fast, zero-dependency local AWS cloud emulator for testing, local development, and CI/CD pipelines.

### Key Goals
- **Blazing Performance**: Sub-millisecond latency for typical S3, SQS, SNS, EventBridge, SSM, SecretsManager, STS operations.
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
                  v           v           v          v         v           v           v
        +-----------+ +-----------+ +-------+ +----------+ +-------+ +-----------+ +-------+
        |    S3     | |    SQS    | |  SNS  | |  Events  | |  SSM  | |  Secrets  | |  STS  |
        |REST Engine| |Query/JSON | | Fanout| |Rule Match| | Store | |  Manager  | | Mock  |
        +-----+-----+ +-----+-----+ +---+---+ +----+-----+ +---+---+ +-----+-----+ +---+---+
              |             |           |          |           |           |           |
          In-Memory    Std/FIFO+DLQ    SQS      SQS/SNS     BTreeMap    DashMap+     Caller
           Storage      Redrive     Delivery    Targets    Hierarchies  Versions    Identity
```

---

## 3. Services Implemented

### 3.1 Amazon S3 (`ruststack-s3`)
- **Bucket Operations**: `CreateBucket`, `DeleteBucket`, `ListBuckets`, `HeadBucket`, `GetBucketLocation`.
- **Object Operations**: `PutObject`, `GetObject` (with byte ranges `Range: bytes=start-end`), `HeadObject`, `DeleteObject`, `DeleteObjects`, `CopyObject`.
- **Object Listing**: `ListObjectsV2` & `ListObjects` with prefix, delimiter, and continuation tokens.
- **Multipart Uploads**: `CreateMultipartUpload`, `UploadPart`, `CompleteMultipartUpload`, `AbortMultipartUpload`, `ListParts`.

### 3.2 Amazon SQS (`ruststack-sqs`)
- **Dual Protocols**: Query Protocol (form-urlencoded & query params) and AWS JSON 1.0 Protocol (`x-amz-target: AmazonSQS.*`).
- **Dead-Letter Queues (DLQ)**: Automatic redrive policy execution when message receive counts exceed `maxReceiveCount`, and `ListDeadLetterSourceQueues`.
- **Message Operations**: `SendMessage`, `SendMessageBatch`, `ReceiveMessage` (with long polling & visibility timeout), `DeleteMessage`, `DeleteMessageBatch`, `ChangeMessageVisibility`.
- **Queue Types**: Standard Queues and FIFO Queues (`.fifo` suffix, `MessageGroupId`, `MessageDeduplicationId`, 5-minute deduplication window).

### 3.3 Amazon SNS (`ruststack-sns`)
- **Dual Protocols**: Query Protocol and AWS JSON Protocol (`x-amz-target: AmazonSNS.*`).
- **Topic Management**: `CreateTopic`, `DeleteTopic`, `ListTopics`, `GetTopicAttributes`, `SetTopicAttributes`.
- **Subscriptions & Fanout**: `Subscribe`, `Unsubscribe`, `ListSubscriptions`, `ListSubscriptionsByTopic`, automatic in-memory SQS fanout.
- **Delivery & Filtering**: Standard AWS JSON Notification envelope or `RawMessageDelivery`, with attribute `FilterPolicy` matching.

### 3.4 Amazon EventBridge / CloudWatch Events (`ruststack-eventbridge`)
- **Event Buses**: `CreateEventBus`, `DeleteEventBus`, `ListEventBuses`, `DescribeEventBus` (with default bus).
- **Rule Engine**: `PutRule`, `DeleteRule`, `ListRules`, `DescribeRule`, `EnableRule`, `DisableRule`.
- **Pattern Matching**: Content-based JSON rule matching (`source`, `detail-type`, nested `detail`, prefix matching, anything-but, exists).
- **Target Dispatching**: Automated dispatching to SQS queues and SNS topics on `PutEvents`.

### 3.5 Amazon SSM Parameter Store (`ruststack-ssm`)
- **Parameters**: `PutParameter` (with versioning), `GetParameter`, `GetParameters`, `GetParametersByPath` (hierarchical recursive scans), `DeleteParameter`, `DeleteParameters`, `DescribeParameters`.

### 3.6 Amazon Secrets Manager (`ruststack-secretsmanager`)
- **Secret Lifecycle**: `CreateSecret`, `GetSecretValue`, `PutSecretValue`, `UpdateSecret`, `DeleteSecret`, `DescribeSecret`, `ListSecrets`, `GetRandomPassword`.
- **Versioning**: Version stages (`AWSCURRENT`, `AWSPREVIOUS`) and rotation tracking.

### 3.7 Amazon STS Mock (`ruststack-sts`)
- **Identity & Session**: `GetCallerIdentity`, `AssumeRole`, `GetSessionToken` supporting Query and JSON 1.1 protocols.

---

## 4. Implementation Roadmap

```
  [Phase 1] (Completed)  --->  [Phase 2] (Completed)      --->  [Phase 3] (Next)
  - S3 (Buckets, Objects)       - SNS & SQS Fanout             - DynamoDB Ultra-Fast Engine
  - SQS (Query & JSON)          - SQS DLQ Redrive              - S3 Bucket Notifications
  - SSM Parameter Store         - EventBridge Buses & Rules    - State Reset & Snapshots (/_ruststack)
  - Secrets Manager             - STS Identity Mock            - Chaos & Fault Injection
  - GHA Matrix Benchmarks       - 7 Independent GHA Benchmarks - Embedded Web Admin UI
```

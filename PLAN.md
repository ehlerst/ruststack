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

Translating the comprehensive test scenarios from Ministack (`https://github.com/ministackorg/ministack/tree/main/tests`) into a 100% pure Rust test harness:

### Service Test Modules:
1. `tests/test_dynamodb_compat.rs`:
   - Table lifecycle (Hash key, Hash+Range key, GSI, LSI)
   - Full attribute types (`S`, `N`, `B`, `SS`, `NS`, `BS`, `M`, `L`, `NULL`, `BOOL`)
   - KeyCondition expressions (`=`, `begins_with`, `BETWEEN`, `>`, `<`, `>=`, `<=`)
   - Filter expressions with expression attribute names (`#n`) & values (`:v`)
   - `ConditionalCheckFailedException` testing
   - `BatchGetItem` & `BatchWriteItem`
2. `tests/test_s3_compat.rs`:
   - Bucket creation, deletion, location, head bucket
   - Object put, get, head, delete, multi-delete
   - Byte-range downloads (`bytes=0-10`, `bytes=5-`, `bytes=-10`)
   - `ListObjectsV2` & `ListObjects` pagination (delimiter, prefix, start-after, continuation-token)
   - Multipart upload complete lifecycle & aborted multipart uploads
   - Virtual hosted bucket routing vs path-style routing
3. `tests/test_sqs_compat.rs`:
   - Standard and FIFO queues
   - `SendMessage` and `SendMessageBatch` (up to 10 items)
   - `ReceiveMessage` with visibility timeout, long polling, and receipt handles
   - `DeleteMessage` and `DeleteMessageBatch`
   - Dead-letter queue redrive (`maxReceiveCount`) & `ListDeadLetterSourceQueues`
   - Query protocol & JSON 1.0 protocol
4. `tests/test_sns_compat.rs`:
   - Topic CRUD & attributes
   - Subscriptions to SQS queues
   - Direct message publish and SQS fanout
   - `RawMessageDelivery` vs standard JSON notification payload
   - Attribute `FilterPolicy`
5. `tests/test_eventbridge_compat.rs`:
   - Event bus CRUD
   - Rule creation, patterns (source, detail-type, nested detail, prefix, exists)
   - Target binding (SQS queues & SNS topics)
   - `PutEvents` matching and target execution
6. `tests/test_ssm_compat.rs`:
   - Parameter CRUD (String, StringList, SecureString)
   - Overwrite & version tracking
   - `GetParametersByPath` recursive & single-level tree scans
   - `GetParameters` multi-key query
7. `tests/test_secretsmanager_compat.rs`:
   - Secret lifecycle (create, get, put value, update, delete)
   - Version stages (`AWSCURRENT`, `AWSPREVIOUS`)
   - String and binary secrets
8. `tests/test_sts_compat.rs`:
   - `GetCallerIdentity`, `AssumeRole`, `GetSessionToken` across Query and JSON protocols

---

## 5. Implementation Roadmap Sequence

```
  [Step 1] (Completed)  --->  [Step 2] (Completed)  --->  [Step 3] (Current)
  - SSM Parameter Store        - DynamoDB Engine           - Pure Rust Native Ministack
  - Secrets Manager            - KeyCondition & Scan         Compatibility Test Suite
  - STS Identity Mock          - 8 GHA Benchmarks Jobs       (Full Parity Suite)
                                                                    |
                                                                    v
  [Step 6]              <---  [Step 5]              <---  [Step 4]
  - Embedded Web Admin UI      - Chaos Engineering         - State Management & Snapshots
  - ruststack-cli               (Latency/Fault Injection)   (/_ruststack/state/dump/load)
```

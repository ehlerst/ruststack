# ⚡ RustStack

> **Blazing-fast local emulator for 37 AWS services in Rust: 1.9 MB idle memory footprint, <1ms native cold start & up to 200x lighter than LocalStack.**

[![CI](https://github.com/ruststack/ruststack/actions/workflows/ci.yml/badge.svg)](https://github.com/ruststack/ruststack/actions/workflows/ci.yml)
[![Performance Benchmarks](https://github.com/ruststack/ruststack/actions/workflows/benchmarks.yml/badge.svg)](https://github.com/ruststack/ruststack/actions/workflows/benchmarks.yml)
[![Resource & Startup Benchmark](https://github.com/ruststack/ruststack/actions/workflows/compare-alternatives.yml/badge.svg)](https://github.com/ruststack/ruststack/actions/workflows/compare-alternatives.yml)
[![Release](https://github.com/ruststack/ruststack/actions/workflows/release.yml/badge.svg)](https://github.com/ruststack/ruststack/actions/workflows/release.yml)
[![Docker Image](https://img.shields.io/badge/docker-ehlers320%2Fruststack-blue.svg)](https://hub.docker.com/r/ehlers320/ruststack)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

RustStack is an ultra-fast, lightweight local AWS cloud emulator designed for instant local development, automated integration testing, and sub-second CI/CD test runners. It is delivered as a single zero-dependency static binary with official multi-arch Docker images.

---

## ⚡ Resource & Startup Benchmark (RustStack vs MiniStack vs LocalStack)

Continuous automated container startup time, idle memory footprint, and image size measured on GitHub Actions runners via [`Testcontainers/rust-measure-alternatives/`](file:///home/tim/git/ruststack/Testcontainers/rust-measure-alternatives/):

| Local Cloud Stack | Docker Image | Image Size | Avg Startup Time | Min Startup Time | Idle Memory (RSS) | Idle CPU |
|:---|:---|---:|---:|---:|---:|---:|
| **⚡ RustStack (Winner)** | `ehlers320/ruststack:latest` | **30.7 MB** | **216.2 ms** | **199.3 ms** | **1.9 MiB** | **0.00%** |
| **MiniStack** | `ministackorg/ministack:latest` | 175.9 MB | 1,685.8 ms | 1,567.2 ms | 33.7 MiB | 0.00% |
| **LocalStack** | `localstack/localstack:3.8.1` | 1,204.1 MB | 2,770.1 ms | 2,668.0 ms | 398.5 MiB | 0.04% |

### 🚀 RustStack Benchmark Findings
- **Memory Footprint**: **17.7x less memory** than MiniStack (1.9 MiB vs 33.7 MiB) and **209.7x less memory** than LocalStack (398.5 MiB).
- **Cold-Start Startup Speed**: **7.8x faster** than MiniStack (216.2 ms vs 1,685.8 ms) and **12.8x faster** than LocalStack (2,770.1 ms).
- **Docker Image Size**: **5.7x smaller** than MiniStack (30.7 MB vs 175.9 MB) and **39.2x smaller** than LocalStack (1,204.1 MB).

---

## ⚡ Performance Highlights

Automated in-memory performance rating on standard developer hardware:

| Service | Operation | Payload / Batch | Throughput | p50 Latency | p95 Latency | Grade |
|:---|:---|:---|---:|---:|---:|:---|
| **STS** | `GetCallerIdentity` | Root Identity | **457,000+ ops/s** | **1.0 µs** | 1.5 µs | **A+ (Ultra Fast)** |
| **SSM** | `GetParameter` | Exact Key | **425,000+ ops/s** | **1.4 µs** | 1.5 µs | **A+ (Ultra Fast)** |
| **IAM** | `GetRole` | Direct Lookup | **413,000+ ops/s** | **1.4 µs** | 1.5 µs | **A+ (Ultra Fast)** |
| **DynamoDB** | `GetItem` | Point Read PK+SK | **410,000+ ops/s** | **1.4 µs** | 1.9 µs | **A+ (Ultra Fast)** |
| **Lambda** | `InvokeFunction` | RequestResponse | **408,000+ ops/s** | **1.4 µs** | 1.5 µs | **A+ (Ultra Fast)** |
| **Lambda** | `GetFunction` | Direct Lookup | **397,000+ ops/s** | **1.4 µs** | 1.8 µs | **A+ (Ultra Fast)** |
| **S3** | `GetObject` | 1 KB | **393,000+ ops/s** | **1.4 µs** | 1.5 µs | **A+ (Ultra Fast)** |
| **KMS** | `GenerateDataKey` | AES-256 (32B) | **353,000+ ops/s** | **1.8 µs** | 2.0 µs | **A+ (Ultra Fast)** |
| **DynamoDB** | `PutItem` | Single Item | **332,000+ ops/s** | **1.9 µs** | 2.0 µs | **A+ (Ultra Fast)** |
| **SecretsManager** | `GetSecretValue` | JSON Payload | **325,000+ ops/s** | **2.0 µs** | 2.4 µs | **A+ (Ultra Fast)** |
| **SES** | `ListIdentities` | Direct Lookup | **324,000+ ops/s** | **1.9 µs** | 2.4 µs | **A+ (Ultra Fast)** |
| **CloudWatch** | `PutMetricData` | Single Datum | **276,000+ ops/s** | **2.4 µs** | 3.4 µs | **A+ (Ultra Fast)** |
| **Logs** | `PutLogEvents` | Single Event | **262,000+ ops/s** | **2.4 µs** | 3.4 µs | **A+ (Ultra Fast)** |
| **Kinesis** | `PutRecord` | 128 B (31.7 MB/s) | **259,000+ ops/s** | **2.9 µs** | 3.4 µs | **A+ (Ultra Fast)** |
| **SES** | `SendEmail` | Outbox Record | **242,000+ ops/s** | **2.9 µs** | 3.4 µs | **A+ (Ultra Fast)** |
| **DynamoDB Streams** | `PutItem (Capture)` | Stream CDC Event | **240,000+ ops/s** | **2.9 µs** | 3.8 µs | **A+ (Ultra Fast)** |
| **KMS** | `CreateKey` | Customer Key | **233,000+ ops/s** | **3.3 µs** | 4.2 µs | **A+ (Ultra Fast)** |
| **STS** | `AssumeRole` | Temporary Creds | **225,000+ ops/s** | **3.4 µs** | 3.8 µs | **A+ (Ultra Fast)** |
| **SQS** | `SendMessageBatch` | 10 msgs/batch | **224,000+ msgs/s** | **41.4 µs** | 50.1 µs | **A (Excellent)** |
| **SNS** | `Publish` | Single Topic | **212,000+ ops/s** | **3.4 µs** | 4.8 µs | **A+ (Ultra Fast)** |
| **IAM** | `CreateRole` | Role Definition | **204,000+ ops/s** | **3.4 µs** | 5.2 µs | **A+ (Ultra Fast)** |
| **KMS** | `Decrypt` | 1 KB Payload | **182,000+ ops/s** | **4.3 µs** | 5.2 µs | **A+ (Ultra Fast)** |
| **KMS** | `Encrypt` | 1 KB Payload | **179,000+ ops/s** | **4.3 µs** | 4.8 µs | **A+ (Ultra Fast)** |
| **S3** | `PutObject` | 1 KB | **168,000+ ops/s** | **4.3 µs** | 7.3 µs | **A+ (Ultra Fast)** |
| **SQS** | `SendMessage` | Single | **154,000+ ops/s** | **5.2 µs** | 5.7 µs | **A+ (Ultra Fast)** |
| **Kinesis** | `GetRecords` | Limit 25 | **152,000+ ops/s** | **5.2 µs** | 6.6 µs | **A+ (Ultra Fast)** |
| **SNS** | `PublishWithFanout` | 5 SQS Queues | **111,000+ msgs/s** | **40.5 µs** | 55.7 µs | **A (Excellent)** |
| **CloudWatch** | `ListMetrics` | Namespace Query | **98,000+ ops/s** | **8.9 µs** | 9.7 µs | **A+ (Ultra Fast)** |
| **EventBridge** | `PutEvents` | Pattern Match + SQS Target | **86,000+ ops/s** | **10.1 µs** | 12.4 µs | **A+ (Ultra Fast)** |
| **SQS** | `SendFifoMessage` | With Dedup Window | **86,000+ ops/s** | **10.1 µs** | 13.9 µs | **A+ (Ultra Fast)** |
| **SSM** | `GetParametersByPath` | 50 Keys Recursive | **72,000+ ops/s** | **13.7 µs** | 16.6 µs | **A+ (Ultra Fast)** |

---

## 📦 Features Implemented (37 AWS Services)

RustStack implements 37 AWS services natively in pure in-memory Rust, organized into high-performance domains:

### 🗄️ Core Storage, Databases & Analytics
1. **Amazon S3 (`ruststack-s3`)**:
   - Bucket lifecycle: `CreateBucket`, `DeleteBucket`, `ListBuckets`, `HeadBucket`, `GetBucketLocation`.
   - Object lifecycle: `PutObject`, `GetObject` (with byte ranges `Range: bytes=start-end`), `HeadObject`, `DeleteObject`, `DeleteObjects` (multi-delete), `CopyObject`.
   - Object listing: `ListObjectsV2` & `ListObjects` with prefix, delimiter, start-after, and continuation tokens.
   - Multipart uploads: `CreateMultipartUpload`, `UploadPart`, `CompleteMultipartUpload`, `AbortMultipartUpload`, `ListParts`.
   - Bucket notifications: `PutBucketNotificationConfiguration` and `GetBucketNotificationConfiguration` with instant in-memory event dispatching to SQS queues, SNS topics, and EventBridge default bus on `s3:ObjectCreated:*` and `s3:ObjectRemoved:*`.
   - Dual routing: Full support for both path-style (`http://localhost:4566/bucket/key`) and virtual-hosted style (`http://bucket.localhost:4566/key`).

2. **Amazon DynamoDB & Streams (`ruststack-dynamodb`)**:
   - Table management: `CreateTable`, `DeleteTable`, `DescribeTable`, `ListTables`, `UpdateTable` (Hash & Range keys, `PAY_PER_REQUEST`, GSIs, LSIs).
   - Item CRUD: `PutItem` (with `ConditionExpression`), `GetItem` (with `ProjectionExpression`), `DeleteItem`, `UpdateItem` (`SET`, `REMOVE`).
   - Batch operations: `BatchGetItem`, `BatchWriteItem` across tables.
   - Query & Scan: `Query` with `KeyConditionExpression` (`=`, `<`, `<=`, `>`, `>=`, `BETWEEN`, `begins_with`), `Scan` with `FilterExpression`.
   - DynamoDB Streams CDC: Automatic stream provisioning (`NEW_IMAGE`, `OLD_IMAGE`, `NEW_AND_OLD_IMAGES`, `KEYS_ONLY`), `ListStreams`, `DescribeStream`, `GetShardIterator`, `GetRecords` capturing real-time `INSERT`, `MODIFY`, `REMOVE` events.

3. **Amazon Relational Database Service - RDS (`ruststack-rds`)**:
   - DB instances: `CreateDBInstance`, `DescribeDBInstances`, `DeleteDBInstance`, `ModifyDBInstance`, `RebootDBInstance`.
   - Subnets & parameters: `CreateDBSubnetGroup`, `DescribeDBSubnetGroups`, `DeleteDBSubnetGroup`, `CreateDBParameterGroup`, `DescribeDBParameterGroups`.
   - Engine emulation: Realistic connection endpoints and configuration for PostgreSQL, MySQL, MariaDB, and Aurora.

4. **Amazon ElastiCache (`ruststack-elasticache`)**:
   - Cache clusters: `CreateCacheCluster`, `DescribeCacheClusters`, `DeleteCacheCluster`, `ModifyCacheCluster`.
   - Replication groups: `CreateReplicationGroup`, `DescribeReplicationGroups`, `DeleteReplicationGroup`.
   - Engine support: Redis and Memcached cluster topologies with node endpoints and parameter group management.

5. **Amazon Redshift Data Warehouse (`ruststack-redshift`)**:
   - Cluster lifecycle: `CreateCluster`, `DescribeClusters`, `DeleteCluster`, `ModifyCluster`, `RebootCluster`.
   - Cluster configuration: Parameter groups, subnet groups, security groups, and simulated database endpoints.

6. **Amazon OpenSearch Service / Elasticsearch (`ruststack-opensearch`)**:
   - Domain lifecycle: `CreateDomain`, `DescribeDomain`, `DescribeDomainConfig`, `ListDomainNames`, `DeleteDomain`, `UpdateDomainConfig`.
   - Cluster state: Simulated search engine endpoints, cluster health status, and node configurations.

7. **Amazon Athena Interactive SQL (`ruststack-athena`)**:
   - Query execution: `StartQueryExecution`, `GetQueryExecution`, `StopQueryExecution`, `GetQueryResults` (with column metadata and row arrays).
   - Workgroups & queries: `CreateWorkGroup`, `GetWorkGroup`, `ListWorkGroups`, `CreateNamedQuery`, `GetNamedQuery`, `ListNamedQueries`.

---

### 📬 Messaging, Streaming & Event-Driven Architecture
8. **Amazon SQS (`ruststack-sqs`)**:
   - Dual protocol: SQS Query Protocol (form-urlencoded & query params) and modern AWS SDK JSON 1.0 (`AmazonSQS.*`).
   - Dead-Letter Queues (DLQ): Automatic redrive policy execution when receive counts exceed `maxReceiveCount`, plus `ListDeadLetterSourceQueues`.
   - Queue management: `CreateQueue`, `DeleteQueue`, `GetQueueUrl`, `ListQueues`, `GetQueueAttributes`, `SetQueueAttributes`, `PurgeQueue`.
   - Message operations: `SendMessage`, `SendMessageBatch` (up to 10 messages), `ReceiveMessage` (with `MaxNumberOfMessages`, `VisibilityTimeout`, async long polling), `DeleteMessage`, `DeleteMessageBatch`, `ChangeMessageVisibility`.
   - FIFO queues: `.fifo` suffix support, `MessageGroupId`, `MessageDeduplicationId`, 5-minute deduplication window.

9. **Amazon SNS (`ruststack-sns`)**:
   - Dual protocol: Query Protocol and modern JSON Protocol (`AmazonSNS.*`).
   - Topic management: `CreateTopic`, `DeleteTopic`, `ListTopics`, `GetTopicAttributes`, `SetTopicAttributes`.
   - Subscriptions & fanout: `Subscribe`, `Unsubscribe`, `ListSubscriptions`, `ListSubscriptionsByTopic`, `GetSubscriptionAttributes`, `SetSubscriptionAttributes`. Zero-latency in-memory fanout to subscribed SQS queues with `FilterPolicy` matching and raw message delivery.

10. **Amazon EventBridge / CloudWatch Events (`ruststack-eventbridge`)**:
    - Event buses: `CreateEventBus`, `DeleteEventBus`, `ListEventBuses`, `DescribeEventBus` (with pre-provisioned `default` bus).
    - Rule engine: `PutRule`, `DeleteRule`, `ListRules`, `DescribeRule`, `EnableRule`, `DisableRule`.
    - Pattern matching & targets: Content-based JSON rule matching (`source`, `detail-type`, nested `detail`, prefix matching, anything-but, exists) with automated target dispatching to SQS and SNS on `PutEvents`.

11. **Amazon EventBridge Pipes (`ruststack-pipes`)**:
    - Pipe management: `CreatePipe`, `DescribePipe`, `ListPipes`, `DeletePipe`, `StartPipe`, `StopPipe`, `UpdatePipe`.
    - Streaming & queue integration: Point-to-point event pipe routing between sources (DynamoDB Streams, Kinesis, SQS) and targets (SQS, SNS, EventBridge, CloudWatch Logs).

12. **Amazon Managed Streaming for Apache Kafka - MSK (`ruststack-msk`)**:
    - Cluster lifecycle: `CreateCluster`, `DescribeCluster`, `ListClusters`, `DeleteCluster`, `GetBootstrapBrokers`.
    - Configurations: Multi-broker cluster topologies, plaintext and TLS broker endpoints.

13. **Amazon Kinesis Data Streams (`ruststack-kinesis`)**:
    - Stream lifecycle: `CreateStream`, `DeleteStream`, `DescribeStream`, `DescribeStreamSummary`, `ListStreams`, `AddTagsToStream`, `ListTagsForStream`.
    - Record ingestion & consumption: `PutRecord`, `PutRecords` (multi-record batching) with deterministic MD5 partition hashing across active shards, `GetShardIterator` (`TRIM_HORIZON`, `LATEST`, `AT_SEQUENCE_NUMBER`), and `GetRecords`.

---

### ⚡ Compute, Containers & Workflow Services
14. **AWS Lambda Mock & Registry (`ruststack-lambda`)**:
    - Function management: `CreateFunction`, `GetFunction`, `GetFunctionConfiguration`, `ListFunctions`, `DeleteFunction`, `UpdateFunctionCode`, `UpdateFunctionConfiguration`.
    - Invocation engine: Synchronous (`RequestResponse`), asynchronous (`Event`), and `DryRun` invocations with mock response payload generation.
    - Event source mappings: `CreateEventSourceMapping`, `ListEventSourceMappings`, `DeleteEventSourceMapping` for DynamoDB Streams and Kinesis stream trigger bindings.

15. **Amazon API Gateway REST API v1 (`ruststack-apigateway`)**:
    - API lifecycle: `CreateRestApi`, `GetRestApi`, `GetRestApis`, `DeleteRestApi`.
    - Resources & methods: `CreateResource`, `GetResource`, `GetResources`, `PutMethod`, `GetMethod`, `PutIntegration`, `PutMethodResponse`, `PutIntegrationResponse`.
    - Deployments & stages: `CreateDeployment`, `GetDeployments`, `CreateStage`, `GetStage`, `GetStages`.

16. **Amazon API Gateway HTTP & WebSocket API v2 (`ruststack-apigatewayv2`)**:
    - API lifecycle: `CreateApi`, `GetApi`, `GetApis`, `DeleteApi`, `UpdateApi` (HTTP & WEBSOCKET protocols).
    - Routes & integrations: `CreateRoute`, `GetRoute`, `GetRoutes`, `CreateIntegration`, `GetIntegration`, `GetIntegrations`.
    - Deployments & stages: `CreateDeployment`, `GetDeployments`, `CreateStage`, `GetStage`, `GetStages`.

17. **AWS AppSync GraphQL API (`ruststack-appsync`)**:
    - GraphQL APIs: `CreateGraphqlApi`, `GetGraphqlApi`, `ListGraphqlApis`, `DeleteGraphqlApi`.
    - API keys & schemas: `CreateApiKey`, `ListApiKeys`, `DeleteApiKey`, `StartSchemaCreation`, `GetSchemaCreationStatus`.
    - Data sources & resolvers: `CreateDataSource`, `GetDataSource`, `ListDataSources`, and built-in interactive GraphQL execution engine.

18. **AWS Step Functions (`ruststack-stepfunctions`)**:
    - State machines: `CreateStateMachine`, `DescribeStateMachine`, `ListStateMachines`, `DeleteStateMachine`, `UpdateStateMachine`.
    - Executions: `StartExecution`, `DescribeExecution`, `GetExecutionHistory`, `StopExecution`, `ListExecutions` (Standard and Express workflows).

19. **AWS CloudFormation (`ruststack-cloudformation`)**:
    - Stack lifecycle: `CreateStack`, `DescribeStacks`, `ListStacks`, `DeleteStack`, `UpdateStack`, `GetTemplate`.
    - Resources & events: `DescribeStackResources`, `ListStackResources`, `DescribeStackEvents`.

20. **AWS Elastic Beanstalk (`ruststack-elasticbeanstalk`)**:
    - Application lifecycle: `CreateApplication`, `DescribeApplications`, `DeleteApplication`, `UpdateApplication`.
    - Environments: `CreateEnvironment`, `DescribeEnvironments`, `TerminateEnvironment`, `RestartAppServer`.

21. **Amazon Elastic Container Registry - ECR (`ruststack-ecr`)**:
    - Repositories: `CreateRepository`, `DescribeRepositories`, `DeleteRepository`, `GetRepositoryPolicy`, `SetRepositoryPolicy`.
    - Image management: `PutImage`, `BatchGetImage`, `BatchDeleteImage`, `ListImages`, `DescribeImages`.

22. **Amazon Elastic Container Service - ECS (`ruststack-ecs`)**:
    - Clusters & tasks: `CreateCluster`, `DescribeClusters`, `ListClusters`, `DeleteCluster`.
    - Task definitions: `RegisterTaskDefinition`, `DescribeTaskDefinition`, `DeregisterTaskDefinition`, `ListTaskDefinitions`.
    - Services & running: `CreateService`, `DescribeServices`, `RunTask`, `ListTasks`.

23. **Amazon Elastic Compute Cloud - EC2 (`ruststack-ec2`)**:
    - Instances: `RunInstances`, `DescribeInstances`, `StartInstances`, `StopInstances`, `TerminateInstances`.
    - VPC networking: `CreateVpc`, `DescribeVpcs`, `DeleteVpc`, `CreateSubnet`, `DescribeSubnets`, `CreateSecurityGroup`, `DescribeSecurityGroups`.

24. **Elastic Load Balancing v2 - ALB / NLB (`ruststack-elbv2`)**:
    - Load balancers: `CreateLoadBalancer`, `DescribeLoadBalancers`, `DeleteLoadBalancer`.
    - Target groups & listeners: `CreateTargetGroup`, `DescribeTargetGroups`, `RegisterTargets`, `DeregisterTargets`, `CreateListener`, `DescribeListeners`.

---

### 🛡️ Security, Identity & Cryptography
25. **AWS Key Management Service - KMS (`ruststack-kms`)**:
    - Key management: `CreateKey`, `DescribeKey`, `ListKeys`, `EnableKey`, `DisableKey`, `ScheduleKeyDeletion`.
    - Aliases: `CreateAlias`, `DeleteAlias`, `ListAliases` with automatic key/alias identifier resolution.
    - Cryptographic operations: `Encrypt`, `Decrypt` with authenticated self-describing ciphertext envelopes, and `GenerateDataKey` (AES-128 & AES-256). Pre-configured default managed keys (`alias/aws/s3`, `alias/aws/dynamodb`).

26. **AWS Identity and Access Management - IAM (`ruststack-iam`)**:
    - Roles & policies: `CreateRole`, `GetRole`, `DeleteRole`, `ListRoles`, `CreatePolicy`, `GetPolicy`, `DeletePolicy`, `ListPolicies`.
    - Attachments: `AttachRolePolicy`, `DetachRolePolicy`, `ListAttachedRolePolicies`, inline policies (`PutRolePolicy`, `GetRolePolicy`, `DeleteRolePolicy`).
    - Users & access keys: `CreateUser`, `GetUser`, `DeleteUser`, `ListUsers`, `CreateAccessKey`, `ListAccessKeys`, `DeleteAccessKey`.
    - Pre-seeded AWS managed policies: Instant availability of `AdministratorAccess`, `AmazonS3FullAccess`, `AmazonDynamoDBFullAccess`, `AmazonSQSFullAccess`, `AWSLambdaBasicExecutionRole`.

27. **Amazon Security Token Service - STS (`ruststack-sts`)**:
    - Identity & credentials: `GetCallerIdentity` (essential for Terraform & AWS SDK init), `AssumeRole`, `GetSessionToken`.

28. **Amazon Cognito Identity Provider (`ruststack-cognito`)**:
    - User pools: `CreateUserPool`, `DescribeUserPool`, `ListUserPools`, `DeleteUserPool`, `CreateUserPoolClient`, `DescribeUserPoolClient`.
    - Users & auth: `AdminCreateUser`, `AdminGetUser`, `AdminInitiateAuth`, `AdminDeleteUser`, `ListUsers`.

29. **AWS Certificate Manager - ACM (`ruststack-acm`)**:
    - Certificates: `RequestCertificate`, `DescribeCertificate`, `ListCertificates`, `DeleteCertificate`, `GetCertificate`, `AddTagsToCertificate`.
    - Validation: Automated DNS and Email validation record generation.

30. **AWS WAFv2 Web Application Firewall (`ruststack-wafv2`)**:
    - Web ACLs: `CreateWebACL`, `GetWebACL`, `ListWebACLs`, `DeleteWebACL`, `UpdateWebACL`.
    - Rule & IP sets: `CreateIPSet`, `GetIPSet`, `ListIPSets`, `CreateRuleGroup`, `GetRuleGroup`.

31. **AWS Organizations (`ruststack-organizations`)**:
    - Organization hierarchy: `CreateOrganization`, `DescribeOrganization`, `ListRoots`, `CreateOrganizationalUnit`, `ListOrganizationalUnitsForParent`.
    - Accounts & SCPs: `CreateAccount`, `ListAccounts`, `CreatePolicy`, `AttachPolicy`, `ListPolicies`.

---

### ⚙️ Systems Management, Secrets & AI
32. **Amazon SSM Parameter Store (`ruststack-ssm`)**:
    - Parameter operations: `PutParameter` (with automatic versioning & overwrite), `GetParameter`, `GetParameters` (multi-key batch), `DeleteParameter`, `DeleteParameters`, `DescribeParameters`.
    - Hierarchical paths: `GetParametersByPath` supporting prefix scans, recursive and single-level sub-tree traversal.
    - Data types: `String`, `StringList`, and `SecureString`.

33. **Amazon Secrets Manager (`ruststack-secretsmanager`)**:
    - Secret lifecycle: `CreateSecret`, `GetSecretValue`, `PutSecretValue`, `UpdateSecret`, `DeleteSecret` (with force delete), `DescribeSecret`, `ListSecrets`, `GetRandomPassword`.
    - Versioning & staging: Multi-version tracking with staging labels (`AWSCURRENT`, `AWSPREVIOUS`).

34. **Amazon Bedrock Generative AI (`ruststack-bedrock`)**:
    - Foundation models: `ListFoundationModels`, `GetFoundationModel`.
    - Model invocation: `InvokeModel` supporting Claude, Titan, and Llama mock inference responses for rapid local AI agent development.

---

### 📈 Observability, DNS & Communications
35. **Amazon CloudWatch Metrics & Alarms (`ruststack-cloudwatch`)**:
    - Metric ingestion: `PutMetricData` (with high-resolution timestamps, custom dimensions, units, and values/counts).
    - Statistics calculation: `ListMetrics`, `GetMetricData`, `GetMetricStatistics` (bucketing by `Average`, `Sum`, `SampleCount`, `Minimum`, `Maximum`).
    - Metric alarms: `PutMetricAlarm`, `DescribeAlarms`, `DescribeAlarmsForMetric`, `DeleteAlarms`, `SetAlarmState`.

36. **Amazon CloudWatch Logs (`ruststack-logs`)**:
    - Groups & streams: `CreateLogGroup`, `DeleteLogGroup`, `DescribeLogGroups`, `CreateLogStream`, `DeleteLogStream`, `DescribeLogStreams`.
    - Events & search: `PutLogEvents` (with sequence token chaining), `GetLogEvents`, and `FilterLogEvents` with pattern matching.

37. **Amazon Route 53 DNS (`ruststack-route53`)**:
    - Hosted zones: `CreateHostedZone`, `GetHostedZone`, `ListHostedZones`, `DeleteHostedZone` (public & private hosted zones).
    - Record sets: `ChangeResourceRecordSets`, `ListResourceRecordSets` supporting `A`, `AAAA`, `CNAME`, `TXT`, `MX`, `SRV` records.

38. **Amazon Simple Email Service - SES (`ruststack-ses`)**:
    - Identity verification: `VerifyEmailIdentity`, `VerifyDomainIdentity`, `ListIdentities`, `DeleteIdentity`, `GetIdentityVerificationAttributes`.
    - Email dispatching: `SendEmail`, `SendRawEmail` with automatic base64 MIME payload decoding, HTML & Text body capture, and subject extraction.
    - Local outbox: Integrated inspection outbox for testing transactional emails without external network dependencies.
    - Quota & statistics: `GetSendQuota`, `GetSendStatistics`.

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
# Wipe all 37 services completely
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

### 4. Auto-Disk Persistence (`RUSTSTACK_DATA_DIR` / `--data-dir`)
RustStack supports automatic state restoration and persistence between container restarts:
```bash
# Docker volume mount for automatic state persistence
docker run -d -p 4566:4566 \
  -v $(pwd)/ruststack-data:/data \
  -e RUSTSTACK_DATA_DIR=/data \
  ehlers320/ruststack:latest

# Or with CLI binary
ruststack start --data-dir ./data
```
When `--data-dir` is configured, RustStack automatically loads `<data-dir>/state.json` on startup and saves cluster state to disk on graceful shutdown.

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

RustStack comes out of the box with an embedded, zero-dependency dark-mode visual administration console and a companion management CLI:

### 1. Web Admin UI
Open `http://localhost:4566/_ruststack/ui/` in any browser (or navigate directly to `http://localhost:4566/`):
- **📊 Overview**: Cluster health status, active region, simulated account ID, in-memory performance grade, and metrics cards covering all 37 active AWS services.
- **🗄️ DynamoDB**: Interactive table list, partition & sort keys, item inspector, and real-time CDC streams.
- **🪣 S3**: Bucket creation, object keys, sizes, multipart uploads, and notification targets.
- **📬 SQS & SNS**: Message counts, FIFO deduplication, DLQs, SNS topics, and subscriber fanout topologies.
- **🔐 SSM & Secrets**: Hierarchical parameter viewer, secret value reveal, version history, and rotation staging labels.
- **🔑 KMS**: Master encryption keys, alias routing, and algorithm descriptors.
- **🪵 CloudWatch Logs**: Log groups, streams, and live structured event log viewers.
- **👥 IAM**: Roles, policies, users, access keys, and managed policy bindings.
- **📧 SES**: Domain/email verification identities and full transactional email outbox viewer.
- **🌊 Kinesis**: Active shards, partition key hashing, and stream iterators.
- **λ Lambda**: Function registry, runtime configurations, and event source mappings.
- **📈 CloudWatch**: Ingested metric time series and alarm state monitors.
- **🗄️ Relational & Cache**: Real-time instance tables for RDS databases, ElastiCache clusters, and Redshift data warehouses.
- **🛡️ Security & Org**: Certificate Manager (ACM), WAFv2 Web ACL firewalls, and AWS Organizations account hierarchy.
- **⚡ Streaming & APIs**: EventBridge Pipes point-to-point routes, Kafka MSK brokers, and AppSync GraphQL schemas.
- **🌪️ Chaos Studio**: Fault injection rule creator, latency/jitter simulator, error status overrides, and live trigger counters.
- **💾 State Snapshots**: 1-click JSON cluster state export, disk persistence management, and atomic multi-service state wipe.

### 2. `ruststack` CLI Companion Commands
The `ruststack` binary doubles as a local management CLI:
```bash
# Query status and health of running RustStack cluster
ruststack status

# Export full cluster snapshot to a file or stdout
ruststack state dump --output snapshot.json

# Restore cluster state from a snapshot file
ruststack state load snapshot.json

# Atomically reset state (all 37 services or selective)
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

### 1. S3 & Bucket Notifications
```bash
# Create bucket and upload an object
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

### 2. DynamoDB & Querying
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

# Query Table
aws --endpoint-url=http://localhost:4566 dynamodb query \
  --table-name Users \
  --key-condition-expression "userId = :uid" \
  --expression-attribute-values '{":uid": {"S": "u100"}}'
```

### 3. KMS Cryptography
```bash
# Create Customer Master Key
KEY_ID=$(aws --endpoint-url=http://localhost:4566 kms create-key \
  --description "app-encryption-key" \
  --query "KeyMetadata.KeyId" --output text)

# Encrypt plaintext
CIPHERTEXT=$(aws --endpoint-url=http://localhost:4566 kms encrypt \
  --key-id "$KEY_ID" \
  --plaintext $(echo -n "secret-token" | base64) \
  --query "CiphertextBlob" --output text)

# Decrypt ciphertext
aws --endpoint-url=http://localhost:4566 kms decrypt \
  --ciphertext-blob "$CIPHERTEXT" \
  --query "Plaintext" --output text | base64 -d
```

### 4. SSM Parameter Store & Secrets Manager
```bash
# Store hierarchical parameter
aws --endpoint-url=http://localhost:4566 ssm put-parameter \
  --name "/production/database/host" \
  --value "db.internal.local" \
  --type String

# Retrieve parameters by path prefix
aws --endpoint-url=http://localhost:4566 ssm get-parameters-by-path \
  --path "/production/database" --recursive

# Create JSON secret in Secrets Manager
aws --endpoint-url=http://localhost:4566 secretsmanager create-secret \
  --name "prod/api/credentials" \
  --secret-string '{"api_key": "sec_12345", "token": "tok_abcdef"}'
```

### 5. Relational & Cache (RDS & ElastiCache)
```bash
# Create PostgreSQL RDS instance
aws --endpoint-url=http://localhost:4566 rds create-db-instance \
  --db-instance-identifier "users-db" \
  --db-instance-class "db.t4g.micro" \
  --engine "postgres" \
  --master-username "postgres" \
  --master-user-password "mypassword"

# Create Redis Cache Cluster
aws --endpoint-url=http://localhost:4566 elasticache create-cache-cluster \
  --cache-cluster-id "session-cache" \
  --cache-node-type "cache.t4g.micro" \
  --engine "redis" \
  --num-cache-nodes 1
```

### 6. Streaming & EventBridge Pipes
```bash
# Create EventBridge Pipe routing SQS to SNS
aws --endpoint-url=http://localhost:4566 pipes create-pipe \
  --name "queue-to-topic-pipe" \
  --source "arn:aws:sqs:us-east-1:000000000000:orders-queue" \
  --target "arn:aws:sns:us-east-1:000000000000:orders-topic"

# List pipes
aws --endpoint-url=http://localhost:4566 pipes list-pipes
```

---

## 🧪 Testcontainers Integration (Go & Rust)

RustStack is built from the ground up for sub-second, zero-dependency containerized integration tests using [Testcontainers](https://testcontainers.com):

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
cargo test -- --ignored --nocapture
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

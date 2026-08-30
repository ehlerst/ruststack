# ⚡ RustStack

> **Ultra-fast, lightweight local AWS cloud emulator written in 100% pure Rust.**  
> Drop-in replacement for LocalStack and MiniStack with a **1.6 MB memory footprint**, **sub-millisecond latency**, and **500ms cold-start startup**.

[![GitHub](https://img.shields.io/badge/github-ehlerst%2Fruststack-blue.svg?logo=github)](https://github.com/ehlerst/ruststack)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/ehlerst/ruststack/blob/main/LICENSE)
[![Docker Pulls](https://img.shields.io/docker/pulls/ehlers320/ruststack.svg)](https://hub.docker.com/r/ehlers320/ruststack)
[![Multi-Arch](https://img.shields.io/badge/arch-amd64%20%7C%20arm64-green.svg)](https://hub.docker.com/r/ehlers320/ruststack)

---

## ⚡ Resource & Startup Benchmark

Measured on standard GitHub Actions runners (`ubuntu-latest`):

| Local Cloud Stack | Docker Image | Image Size | Avg Startup Time | Min Startup Time | Idle Memory (RSS) | Idle CPU |
|:---|:---|---:|---:|---:|---:|---:|
| **⚡ RustStack (Winner)** | `ehlers320/ruststack:latest` | **30.7 MB** | **216.2 ms** | **199.3 ms** | **1.9 MiB** | **0.00%** |
| **MiniStack** | `ministackorg/ministack:latest` | 175.9 MB | 1,685.8 ms | 1,567.2 ms | 33.7 MiB | 0.00% |
| **LocalStack** | `localstack/localstack:3.8.1` | 1,204.1 MB | 2,770.1 ms | 2,668.0 ms | 398.5 MiB | 0.04% |

### 🚀 Key Advantages
- **Memory Footprint**: **17.7x lighter** than MiniStack (`1.9 MiB` vs `33.7 MiB`) and **209.7x lighter** than LocalStack (`398.5 MiB`).
- **Cold-Start Readiness**: **7.8x faster** startup (`216.2 ms` vs `1,685.8 ms`).
- **Image Size**: **30.7 MB** (down from 92.9 MB, **5.7x smaller** than MiniStack and **39.2x smaller** than LocalStack).

---

## 🚀 Quick Start

### 1. Run with Docker CLI
```bash
docker run -d --name ruststack -p 4566:4566 ehlers320/ruststack:latest
```

### 2. Docker Compose
```yaml
services:
  ruststack:
    image: ehlers320/ruststack:latest
    container_name: ruststack
    ports:
      - "4566:4566"
    volumes:
      - ./ruststack-data:/data
    environment:
      - SERVICES=s3,sqs,sns,events,ssm,secretsmanager,sts,dynamodb,kms
      - DEFAULT_REGION=us-east-1
      - ACCOUNT_ID=000000000000
      - RUSTSTACK_DATA_DIR=/data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:4566/_ruststack/health"]
      interval: 5s
      timeout: 2s
      retries: 3
```

---

## 🖥️ Embedded Web Admin UI

RustStack includes a zero-dependency, dark-mode visual administration dashboard served directly on port `4566`:

👉 Open **`http://localhost:4566/_ruststack/ui/`** in your browser to:
- Browse **S3 Buckets & Objects**
- Inspect **DynamoDB Tables, Key Schemas & Items**
- View **SQS Queues, Messages & DLQ Redrive Policies**
- Explore **SNS Topics & Subscriptions**
- Inspect **SSM Parameters & Secrets Manager Secrets**
- Manage **KMS Customer & AWS Managed Master Keys & Aliases**
- Interactively build **Chaos Engineering** rules & manage **State Snapshots**

---

## 🧪 Testcontainers Integration

### Go
```go
req := testcontainers.ContainerRequest{
    Image:        "ehlers320/ruststack:latest",
    ExposedPorts: []string{"4566/tcp"},
    WaitingFor:   wait.ForHTTP("/_ruststack/health").WithPort("4566/tcp"),
}
container, err := testcontainers.GenericContainer(ctx, testcontainers.GenericContainerRequest{
    ContainerRequest: req,
    Started:          true,
})
```

### Python
```python
from testcontainers.core.container import DockerContainer
from testcontainers.core.waiting_utils import wait_for_logs

with DockerContainer("ehlers320/ruststack:latest").with_exposed_ports(4566) as ruststack:
    endpoint = f"http://{ruststack.get_container_host_ip()}:{ruststack.get_exposed_port(4566)}"
    # Pass endpoint to boto3 clients
```

### Node / TypeScript
```typescript
import { GenericContainer, Wait } from "testcontainers";

const container = await new GenericContainer("ehlers320/ruststack:latest")
  .withExposedPorts(4566)
  .withWaitStrategy(Wait.forHttp("/_ruststack/health", 4566))
  .start();
```

---

## 📦 Supported AWS Services

All services multiplex across the single port `4566`:

| Service | Protocol | Key Features Supported |
|:---|:---|:---|
| **Amazon S3** | REST XML / Path & Virtual-Host | Buckets, Objects, Multipart Uploads, Range Requests, S3 Bucket Notifications -> SQS/SNS/EventBridge |
| **Amazon DynamoDB** | AWS JSON 1.0 | Tables, `PAY_PER_REQUEST`, Put/Get/Delete/UpdateItem, ConditionExpressions, Query (KeyConditions), Scan (FilterExpressions), Batch Operations |
| **Amazon SQS** | Query & JSON 1.0 | Standard & FIFO Queues, DLQ Redrive Policies, Long Polling, Visibility Timeouts, Batch Operations |
| **Amazon SNS** | Query & JSON | Topics, Subscriptions, Automatic Zero-Latency SQS Fanout, Filter Policies, RawMessageDelivery |
| **Amazon EventBridge** | AWS JSON 1.1 | Custom & Default Buses, JSON Pattern Matching Rules, SQS & SNS Target Dispatching |
| **Amazon SSM** | AWS JSON 1.1 | Parameter Store (String, StringList, SecureString), Automatic Versioning, `GetParametersByPath` Recursive Trees |
| **Secrets Manager** | AWS JSON 1.1 | Secret Lifecycle, Version Staging (`AWSCURRENT`, `AWSPREVIOUS`), GetSecretValue |
| **Amazon STS** | Query & JSON 1.1 | `GetCallerIdentity`, `AssumeRole`, `GetSessionToken` |
| **AWS KMS** | AWS JSON 1.1 | `CreateKey`, `DescribeKey`, `ListKeys`, `CreateAlias`, `ListAliases`, `Encrypt`, `Decrypt`, `GenerateDataKey` (AES-128/256) |

---

## 🌪️ Chaos Engineering & Fault Injection

Inject simulated network delays, error rates, and failure conditions on the fly:

```bash
# Inject DynamoDB ProvisionedThroughputExceededException
curl -X POST http://localhost:4566/_ruststack/chaos/rules \
  -H "Content-Type: application/json" \
  -d '{
    "service": "dynamodb",
    "action": "PutItem",
    "probability": 0.5,
    "error_status": 400,
    "error_code": "ProvisionedThroughputExceededException"
  }'

# Inject 150ms latency with 50ms jitter on SQS
curl -X POST http://localhost:4566/_ruststack/chaos/rules \
  -H "Content-Type: application/json" \
  -d '{
    "service": "sqs",
    "latency_ms": 150,
    "latency_jitter_ms": 50
  }'

# Clear all chaos rules
curl -X POST http://localhost:4566/_ruststack/chaos/reset
```

---

## 💾 State Snapshots, Reset & Auto-Disk Persistence

```bash
# Export cluster snapshot JSON
curl http://localhost:4566/_ruststack/state/dump > state.json

# Restore cluster state from snapshot
curl -X POST http://localhost:4566/_ruststack/state/load \
  -H "Content-Type: application/json" \
  -d @state.json

# Atomically reset cluster state (all services or selective)
curl -X POST http://localhost:4566/_ruststack/state/reset
```

### Auto-Disk Persistence
Mount a local directory to preserve cluster state across container restarts:
```bash
docker run -d -p 4566:4566 \
  -v $(pwd)/ruststack-data:/data \
  -e RUSTSTACK_DATA_DIR=/data \
  ehlers320/ruststack:latest
```

---

## ⚙️ Environment Variables

| Variable | Default | Description |
|:---|:---|:---|
| `PORT` | `4566` | Listening HTTP port |
| `HOST` | `0.0.0.0` | Bind host address |
| `SERVICES` | `s3,sqs,sns,events,ssm,secretsmanager,sts,dynamodb,kms` | Comma-separated list of enabled services |
| `DEFAULT_REGION` | `us-east-1` | Default AWS region |
| `ACCOUNT_ID` | `000000000000` | Default AWS Account ID |
| `RUSTSTACK_DATA_DIR` | `None` | Directory path for auto-saving & auto-loading cluster state |
| `PERSISTENCE` | `false` | Enable automatic disk persistence |

---

## 📄 License

Licensed under either of [Apache License, Version 2.0](https://github.com/ehlerst/ruststack/blob/main/LICENSE) or [MIT license](https://github.com/ehlerst/ruststack/blob/main/LICENSE) at your option.

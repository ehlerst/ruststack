use ruststack_testcontainers_example::ruststack_image;
use serde_json::json;
use testcontainers::runners::AsyncRunner;

struct ContainerContext {
    base_url: String,
    http: reqwest::Client,
}

impl ContainerContext {
    fn new(host_port: u16) -> Self {
        Self {
            base_url: format!("http://127.0.0.1:{}", host_port),
            http: reqwest::Client::new(),
        }
    }

    async fn call_json(
        &self,
        target: &str,
        body: serde_json::Value,
    ) -> (reqwest::StatusCode, serde_json::Value) {
        let resp = self
            .http
            .post(format!("{}/", self.base_url))
            .header("x-amz-target", target)
            .header("content-type", "application/x-amz-json-1.0")
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = resp.status();
        let val = resp.json().await.unwrap_or(serde_json::Value::Null);
        (status, val)
    }

    async fn call_rest_json(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> (reqwest::StatusCode, serde_json::Value) {
        let mut builder = self
            .http
            .request(method, format!("{}{}", self.base_url, path));
        if let Some(b) = body {
            builder = builder.json(&b);
        }
        let resp = builder.send().await.unwrap();
        let status = resp.status();
        let val = resp.json().await.unwrap_or(serde_json::Value::Null);
        (status, val)
    }

    async fn call_query(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> (reqwest::StatusCode, String) {
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .form(&params)
            .send()
            .await
            .unwrap();
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        (status, text)
    }

    async fn call_s3(
        &self,
        method: reqwest::Method,
        path: &str,
        headers: reqwest::header::HeaderMap,
        body: bytes::Bytes,
    ) -> (reqwest::StatusCode, reqwest::header::HeaderMap, String) {
        let resp = self
            .http
            .request(method, format!("{}{}", self.base_url, path))
            .headers(headers)
            .body(body)
            .send()
            .await
            .unwrap();
        let status = resp.status();
        let resp_headers = resp.headers().clone();
        let text = resp.text().await.unwrap_or_default();
        (status, resp_headers, text)
    }
}

#[tokio::test]
#[ignore = "requires running docker daemon and pulled ehlers320/ruststack:latest"]
async fn test_ruststack_container_s3_sqs_dynamodb() {
    let image = ruststack_image();
    let container = image
        .start()
        .await
        .expect("failed to start ruststack container");
    let host_port = container
        .get_host_port_ipv4(4566)
        .await
        .expect("failed to get host port");

    let base_url = format!("http://127.0.0.1:{}", host_port);
    let http = reqwest::Client::new();

    // 1. Health check
    let resp = http
        .get(format!("{}/_ruststack/health", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // 2. S3 CreateBucket & PutObject
    let resp = http
        .put(format!("{}/container-test-bucket", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = http
        .put(format!("{}/container-test-bucket/test.txt", base_url))
        .body("hello from rust testcontainers")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = http
        .get(format!("{}/container-test-bucket/test.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let text = resp.text().await.unwrap();
    assert_eq!(text, "hello from rust testcontainers");

    // 3. SQS CreateQueue & Send/Receive
    let resp = http
        .post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSQS.CreateQueue")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({ "QueueName": "container-sqs-q" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let val: serde_json::Value = resp.json().await.unwrap();
    let q_url = val["QueueUrl"].as_str().unwrap().to_string();

    let resp = http
        .post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSQS.SendMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "QueueUrl": q_url,
            "MessageBody": "container-msg-123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = http
        .post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSQS.ReceiveMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "QueueUrl": q_url,
            "MaxNumberOfMessages": 1
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let val: serde_json::Value = resp.json().await.unwrap();
    let msgs = val["Messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["Body"].as_str().unwrap(), "container-msg-123");

    // 4. DynamoDB CreateTable & PutItem
    let resp = http
        .post(format!("{}/", base_url))
        .header("x-amz-target", "DynamoDB_20120810.CreateTable")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "TableName": "ContainerUsers",
            "KeySchema": [{ "AttributeName": "id", "KeyType": "HASH" }],
            "AttributeDefinitions": [{ "AttributeName": "id", "AttributeType": "S" }],
            "BillingMode": "PAY_PER_REQUEST"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = http
        .post(format!("{}/", base_url))
        .header("x-amz-target", "DynamoDB_20120810.PutItem")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "TableName": "ContainerUsers",
            "Item": {
                "id": { "S": "u-456" },
                "name": { "S": "Alice Rust Testcontainers" }
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = http
        .post(format!("{}/", base_url))
        .header("x-amz-target", "DynamoDB_20120810.GetItem")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "TableName": "ContainerUsers",
            "Key": { "id": { "S": "u-456" } }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let val: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        val["Item"]["name"]["S"].as_str().unwrap(),
        "Alice Rust Testcontainers"
    );
}

#[tokio::test]
#[ignore = "requires running docker daemon and pulled ehlers320/ruststack:latest"]
async fn test_ruststack_container_sns_fanout_to_sqs() {
    let image = ruststack_image();
    let container = image
        .start()
        .await
        .expect("failed to start ruststack container");
    let host_port = container
        .get_host_port_ipv4(4566)
        .await
        .expect("failed to get host port");
    let ctx = ContainerContext::new(host_port);

    // 1. Create SQS Queue
    let (status, q_val) = ctx
        .call_json(
            "AmazonSQS.CreateQueue",
            json!({ "QueueName": "fanout-target-q" }),
        )
        .await;
    assert_eq!(status, 200);
    let q_url = q_val["QueueUrl"].as_str().unwrap().to_string();

    // 2. Create SNS Topic
    let (status, t_val) = ctx
        .call_json("AmazonSNS.CreateTopic", json!({ "Name": "fanout-topic" }))
        .await;
    assert_eq!(status, 200);
    let topic_arn = t_val["TopicArn"].as_str().unwrap().to_string();

    // 3. Subscribe SQS queue to SNS topic
    let (status, sub_val) = ctx
        .call_json(
            "AmazonSNS.Subscribe",
            json!({
                "TopicArn": topic_arn,
                "Protocol": "sqs",
                "Endpoint": q_url,
                "Attributes": {
                    "RawMessageDelivery": "true"
                }
            }),
        )
        .await;
    assert_eq!(status, 200);
    assert!(sub_val["SubscriptionArn"].is_string());

    // 4. Publish message to SNS topic
    let (status, pub_val) = ctx
        .call_json(
            "AmazonSNS.Publish",
            json!({
                "TopicArn": topic_arn,
                "Message": "event-fanout-payload-999"
            }),
        )
        .await;
    assert_eq!(status, 200);
    assert!(pub_val["MessageId"].is_string());

    // 5. Receive message from SQS queue
    let (status, recv_val) = ctx
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({
                "QueueUrl": q_url,
                "MaxNumberOfMessages": 1
            }),
        )
        .await;
    assert_eq!(status, 200);
    let msgs = recv_val["Messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(
        msgs[0]["Body"].as_str().unwrap(),
        "event-fanout-payload-999"
    );
}

#[tokio::test]
#[ignore = "requires running docker daemon and pulled ehlers320/ruststack:latest"]
async fn test_ruststack_container_ssm_and_secrets_manager() {
    let image = ruststack_image();
    let container = image
        .start()
        .await
        .expect("failed to start ruststack container");
    let host_port = container
        .get_host_port_ipv4(4566)
        .await
        .expect("failed to get host port");
    let ctx = ContainerContext::new(host_port);

    // 1. SSM PutParameter & GetParameter
    let (status, put_val) = ctx
        .call_json(
            "AmazonSSM.PutParameter",
            json!({
                "Name": "/config/db/host",
                "Value": "postgres.internal",
                "Type": "String"
            }),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(put_val["Version"].as_i64().unwrap(), 1);

    let (status, get_val) = ctx
        .call_json(
            "AmazonSSM.GetParameter",
            json!({
                "Name": "/config/db/host"
            }),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(
        get_val["Parameter"]["Value"].as_str().unwrap(),
        "postgres.internal"
    );

    // 2. Secrets Manager CreateSecret & GetSecretValue
    let (status, sec_val) = ctx
        .call_json(
            "secretsmanager.CreateSecret",
            json!({
                "Name": "app/credentials",
                "SecretString": "{\"api_key\": \"sec_test_token_123\"}"
            }),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(sec_val["Name"].as_str().unwrap(), "app/credentials");

    let (status, get_sec) = ctx
        .call_json(
            "secretsmanager.GetSecretValue",
            json!({
                "SecretId": "app/credentials"
            }),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(
        get_sec["SecretString"].as_str().unwrap(),
        "{\"api_key\": \"sec_test_token_123\"}"
    );
}

#[tokio::test]
#[ignore = "requires running docker daemon and pulled ehlers320/ruststack:latest"]
async fn test_ruststack_container_kms_cryptography() {
    let image = ruststack_image();
    let container = image
        .start()
        .await
        .expect("failed to start ruststack container");
    let host_port = container
        .get_host_port_ipv4(4566)
        .await
        .expect("failed to get host port");
    let ctx = ContainerContext::new(host_port);

    // 1. Create KMS Key
    let (status, key_val) = ctx
        .call_json(
            "TrentService.CreateKey",
            json!({
                "Description": "Testcontainers Customer Key",
                "KeyUsage": "ENCRYPT_DECRYPT"
            }),
        )
        .await;
    assert_eq!(status, 200);
    let key_id = key_val["KeyMetadata"]["KeyId"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. Encrypt plaintext (base64 of "hello-testcontainers" is "aGVsbG8tdGVzdGNvbnRhaW5lcnM=")
    let (status, enc_val) = ctx
        .call_json(
            "TrentService.Encrypt",
            json!({
                "KeyId": key_id,
                "Plaintext": "aGVsbG8tdGVzdGNvbnRhaW5lcnM="
            }),
        )
        .await;
    assert_eq!(status, 200);
    let ciphertext = enc_val["CiphertextBlob"].as_str().unwrap().to_string();

    // 3. Decrypt ciphertext
    let (status, dec_val) = ctx
        .call_json(
            "TrentService.Decrypt",
            json!({
                "CiphertextBlob": ciphertext
            }),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(
        dec_val["Plaintext"].as_str().unwrap(),
        "aGVsbG8tdGVzdGNvbnRhaW5lcnM="
    );
}

#[tokio::test]
#[ignore = "requires running docker daemon and pulled ehlers320/ruststack:latest"]
async fn test_ruststack_container_eventbridge_rule_to_sqs() {
    let image = ruststack_image();
    let container = image
        .start()
        .await
        .expect("failed to start ruststack container");
    let host_port = container
        .get_host_port_ipv4(4566)
        .await
        .expect("failed to get host port");
    let ctx = ContainerContext::new(host_port);

    // 1. Create SQS target queue
    let (status, q_val) = ctx
        .call_json(
            "AmazonSQS.CreateQueue",
            json!({ "QueueName": "eb-orders-target" }),
        )
        .await;
    assert_eq!(status, 200);
    let q_url = q_val["QueueUrl"].as_str().unwrap().to_string();
    let q_arn = "arn:aws:sqs:us-east-1:000000000000:eb-orders-target";

    // 2. Create EventBridge Rule
    let pattern = json!({
        "source": ["ecommerce.checkout"],
        "detail-type": ["OrderPlaced"]
    })
    .to_string();
    let (status, rule_val) = ctx
        .call_json(
            "AWSEvents.PutRule",
            json!({
                "Name": "order-placed-rule",
                "EventPattern": pattern,
                "State": "ENABLED"
            }),
        )
        .await;
    assert_eq!(status, 200);
    assert!(rule_val["RuleArn"].is_string());

    // 3. PutTargets
    let (status, target_val) = ctx
        .call_json(
            "AWSEvents.PutTargets",
            json!({
                "Rule": "order-placed-rule",
                "Targets": [{
                    "Id": "target-1",
                    "Arn": q_arn
                }]
            }),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(target_val["FailedEntryCount"].as_i64().unwrap(), 0);

    // 4. PutEvents
    let (status, evt_val) = ctx
        .call_json(
            "AWSEvents.PutEvents",
            json!({
                "Entries": [{
                    "Source": "ecommerce.checkout",
                    "DetailType": "OrderPlaced",
                    "Detail": "{\"orderId\": \"order-987\"}"
                }]
            }),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(evt_val["FailedEntryCount"].as_i64().unwrap(), 0);

    // 5. Receive message from target SQS queue
    let (status, recv_val) = ctx
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({
                "QueueUrl": q_url,
                "MaxNumberOfMessages": 1
            }),
        )
        .await;
    assert_eq!(status, 200);
    let msgs = recv_val["Messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 1);
    assert!(msgs[0]["Body"].as_str().unwrap().contains("order-987"));
}

#[tokio::test]
#[ignore = "requires running docker daemon and pulled ehlers320/ruststack:latest"]
async fn test_ruststack_container_rds_and_elasticache() {
    let image = ruststack_image();
    let container = image
        .start()
        .await
        .expect("failed to start ruststack container");
    let host_port = container
        .get_host_port_ipv4(4566)
        .await
        .expect("failed to get host port");
    let ctx = ContainerContext::new(host_port);

    // 1. Create RDS DB Instance
    let (status, rds_body) = ctx
        .call_query(
            "/",
            &[
                ("Action", "CreateDBInstance"),
                ("DBInstanceIdentifier", "container-pg"),
                ("Engine", "postgres"),
                ("DBInstanceClass", "db.t4g.micro"),
                ("MasterUsername", "dbuser"),
            ],
        )
        .await;
    assert_eq!(status, 200);
    assert!(rds_body.contains("<DBInstanceIdentifier>container-pg</DBInstanceIdentifier>"));

    // 2. Describe RDS DB Instances
    let (status, desc_body) = ctx
        .call_query(
            "/",
            &[
                ("Action", "DescribeDBInstances"),
                ("DBInstanceIdentifier", "container-pg"),
            ],
        )
        .await;
    assert_eq!(status, 200);
    assert!(desc_body.contains("<Port>5432</Port>"));

    // 3. Create ElastiCache Cluster
    let (status, cache_body) = ctx
        .call_query(
            "/",
            &[
                ("Action", "CreateCacheCluster"),
                ("CacheClusterIdentifier", "container-redis"),
                ("Engine", "redis"),
                ("CacheNodeType", "cache.t4g.micro"),
                ("NumCacheNodes", "1"),
            ],
        )
        .await;
    assert_eq!(status, 200);
    assert!(cache_body.contains("<CacheClusterIdentifier>container-redis</CacheClusterIdentifier>"));
}

#[tokio::test]
#[ignore = "requires running docker daemon and pulled ehlers320/ruststack:latest"]
async fn test_ruststack_container_pipes_and_msk() {
    let image = ruststack_image();
    let container = image
        .start()
        .await
        .expect("failed to start ruststack container");
    let host_port = container
        .get_host_port_ipv4(4566)
        .await
        .expect("failed to get host port");
    let ctx = ContainerContext::new(host_port);

    // 1. Create MSK Cluster
    let (status, msk_body) = ctx
        .call_rest_json(
            reqwest::Method::POST,
            "/v1/clusters",
            Some(json!({
                "ClusterName": "container-kafka",
                "NumberOfBrokerNodes": 2,
                "KafkaVersion": "3.5.1"
            })),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(msk_body["ClusterName"], "container-kafka");
    let cluster_arn = msk_body["ClusterArn"].as_str().unwrap();

    // 2. Get Bootstrap Brokers
    let (status, broker_body) = ctx
        .call_rest_json(
            reqwest::Method::GET,
            &format!("/v1/clusters/{}/bootstrap-brokers", cluster_arn),
            None,
        )
        .await;
    assert_eq!(status, 200);
    assert!(broker_body["BootstrapBrokerString"]
        .as_str()
        .unwrap()
        .contains("9092"));

    // 3. Create EventBridge Pipe
    let (status, pipe_body) = ctx
        .call_rest_json(
            reqwest::Method::POST,
            "/v1/pipes/container-pipe",
            Some(json!({
                "Source": "arn:aws:sqs:us-east-1:000000000000:source-q",
                "Target": "arn:aws:sns:us-east-1:000000000000:target-topic",
                "RoleArn": "arn:aws:iam::000000000000:role/PipesRole",
                "DesiredState": "RUNNING"
            })),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(pipe_body["Name"], "container-pipe");
    assert_eq!(pipe_body["CurrentState"], "RUNNING");
}

#[tokio::test]
#[ignore = "requires running docker daemon and pulled ehlers320/ruststack:latest"]
async fn test_ruststack_container_chaos_and_state_lifecycle() {
    let image = ruststack_image();
    let container = image
        .start()
        .await
        .expect("failed to start ruststack container");
    let host_port = container
        .get_host_port_ipv4(4566)
        .await
        .expect("failed to get host port");
    let ctx = ContainerContext::new(host_port);

    // 1. Add Chaos Rule injecting 503 on SQS
    let (status, _) = ctx
        .call_rest_json(
            reqwest::Method::POST,
            "/_ruststack/chaos/rules",
            Some(json!({
                "service": "sqs",
                "error_status": 503,
                "error_code": "SlowDown",
                "error_message": "Injected chaos slowdown"
            })),
        )
        .await;
    assert_eq!(status, 201);

    // 2. Verify chaos rule intercepts SQS request
    let (status, _) = ctx.call_json("AmazonSQS.ListQueues", json!({})).await;
    assert_eq!(status, 503);

    // 3. Reset chaos rules
    let (status, _) = ctx
        .call_rest_json(reqwest::Method::POST, "/_ruststack/chaos/reset", None)
        .await;
    assert_eq!(status, 200);

    // 4. Verify request succeeds after reset
    let (status, _) = ctx.call_json("AmazonSQS.ListQueues", json!({})).await;
    assert_eq!(status, 200);

    // 5. State Dump
    let (status, dump_val) = ctx
        .call_rest_json(reqwest::Method::GET, "/_ruststack/state/dump", None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(dump_val["version"].as_i64().unwrap(), 1);
    assert!(dump_val["s3"].is_object());
    assert!(dump_val["dynamodb"].is_object());

    // 6. State Reset
    let (status, reset_val) = ctx
        .call_rest_json(reqwest::Method::POST, "/_ruststack/state/reset", None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(reset_val["status"].as_str().unwrap(), "ok");
}

#[tokio::test]
#[ignore = "requires running docker daemon and pulled ehlers320/ruststack:latest"]
async fn test_ruststack_container_s3_bucket_notifications_to_sqs() {
    let image = ruststack_image();
    let container = image
        .start()
        .await
        .expect("failed to start ruststack container");
    let host_port = container
        .get_host_port_ipv4(4566)
        .await
        .expect("failed to get host port");
    let ctx = ContainerContext::new(host_port);

    // 1. Create SQS target queue
    let (status, q_val) = ctx
        .call_json(
            "AmazonSQS.CreateQueue",
            json!({ "QueueName": "s3-event-target-q" }),
        )
        .await;
    assert_eq!(status, 200);
    let q_url = q_val["QueueUrl"].as_str().unwrap().to_string();
    let q_arn = "arn:aws:sqs:us-east-1:000000000000:s3-event-target-q";

    // 2. Create S3 Bucket
    let (status, _, _) = ctx
        .call_s3(
            reqwest::Method::PUT,
            "/event-test-bucket",
            reqwest::header::HeaderMap::new(),
            bytes::Bytes::new(),
        )
        .await;
    assert_eq!(status, 200);

    // 3. Configure Bucket Notification for prefix "uploads/"
    let notif_xml = format!(
        r#"<NotificationConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
    <QueueConfiguration>
        <Id>sqs-uploads-rule</Id>
        <Queue>{}</Queue>
        <Event>s3:ObjectCreated:*</Event>
        <Filter>
            <S3Key>
                <FilterRule>
                    <Name>prefix</Name>
                    <Value>uploads/</Value>
                </FilterRule>
            </S3Key>
        </Filter>
    </QueueConfiguration>
</NotificationConfiguration>"#,
        q_arn
    );
    let (status, _, _) = ctx
        .call_s3(
            reqwest::Method::PUT,
            "/event-test-bucket?notification",
            reqwest::header::HeaderMap::new(),
            bytes::Bytes::from(notif_xml),
        )
        .await;
    assert_eq!(status, 200);

    // 4. PutObject outside prefix -> Should NOT notify
    let (status, _, _) = ctx
        .call_s3(
            reqwest::Method::PUT,
            "/event-test-bucket/temp/draft.txt",
            reqwest::header::HeaderMap::new(),
            bytes::Bytes::from("draft data"),
        )
        .await;
    assert_eq!(status, 200);

    let (status, recv_none) = ctx
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({ "QueueUrl": q_url, "MaxNumberOfMessages": 10 }),
        )
        .await;
    assert_eq!(status, 200);
    assert!(
        recv_none.get("Messages").is_none() || recv_none["Messages"].as_array().unwrap().is_empty()
    );

    // 5. PutObject matching prefix -> MUST deliver event!
    let (status, _, _) = ctx
        .call_s3(
            reqwest::Method::PUT,
            "/event-test-bucket/uploads/order_999.json",
            reqwest::header::HeaderMap::new(),
            bytes::Bytes::from(r#"{"orderId": 999}"#),
        )
        .await;
    assert_eq!(status, 200);

    let (status, recv_event) = ctx
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({ "QueueUrl": q_url, "MaxNumberOfMessages": 10 }),
        )
        .await;
    assert_eq!(status, 200);
    let msgs = recv_event["Messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 1);
    let body_str = msgs[0]["Body"].as_str().unwrap();
    assert!(body_str.contains("event-test-bucket"));
    assert!(body_str.contains("uploads/order_999.json"));
}

#[tokio::test]
#[ignore = "requires running docker daemon and pulled ehlers320/ruststack:latest"]
async fn test_ruststack_container_dynamodb_batch_and_query_expressions() {
    let image = ruststack_image();
    let container = image
        .start()
        .await
        .expect("failed to start ruststack container");
    let host_port = container
        .get_host_port_ipv4(4566)
        .await
        .expect("failed to get host port");
    let ctx = ContainerContext::new(host_port);

    // 1. CreateTable
    let (status, _) = ctx
        .call_json(
            "DynamoDB_20120810.CreateTable",
            json!({
                "TableName": "StoreProducts",
                "KeySchema": [
                    { "AttributeName": "category", "KeyType": "HASH" },
                    { "AttributeName": "sku", "KeyType": "RANGE" }
                ],
                "AttributeDefinitions": [
                    { "AttributeName": "category", "AttributeType": "S" },
                    { "AttributeName": "sku", "AttributeType": "S" }
                ],
                "BillingMode": "PAY_PER_REQUEST"
            }),
        )
        .await;
    assert_eq!(status, 200);

    // 2. BatchWriteItem
    let (status, bw_val) = ctx
        .call_json(
            "DynamoDB_20120810.BatchWriteItem",
            json!({
                "RequestItems": {
                    "StoreProducts": [
                        {
                            "PutRequest": {
                                "Item": {
                                    "category": { "S": "hardware" },
                                    "sku": { "S": "hw-001" },
                                    "name": { "S": "SSD 1TB" },
                                    "price": { "N": "100" }
                                }
                            }
                        },
                        {
                            "PutRequest": {
                                "Item": {
                                    "category": { "S": "hardware" },
                                    "sku": { "S": "hw-002" },
                                    "name": { "S": "RAM 32GB" },
                                    "price": { "N": "150" }
                                }
                            }
                        }
                    ]
                }
            }),
        )
        .await;
    assert_eq!(status, 200);
    assert!(bw_val["UnprocessedItems"].as_object().unwrap().is_empty());

    // 3. BatchGetItem
    let (status, bg_val) = ctx
        .call_json(
            "DynamoDB_20120810.BatchGetItem",
            json!({
                "RequestItems": {
                    "StoreProducts": {
                        "Keys": [
                            { "category": { "S": "hardware" }, "sku": { "S": "hw-001" } },
                            { "category": { "S": "hardware" }, "sku": { "S": "hw-002" } }
                        ]
                    }
                }
            }),
        )
        .await;
    assert_eq!(status, 200);
    let items = bg_val["Responses"]["StoreProducts"].as_array().unwrap();
    assert_eq!(items.len(), 2);

    // 4. Query with KeyConditionExpression and FilterExpression
    let (status, q_res) = ctx
        .call_json(
            "DynamoDB_20120810.Query",
            json!({
                "TableName": "StoreProducts",
                "KeyConditionExpression": "category = :c AND begins_with(sku, :pfx)",
                "FilterExpression": "price > :min_p",
                "ExpressionAttributeValues": {
                    ":c": { "S": "hardware" },
                    ":pfx": { "S": "hw-" },
                    ":min_p": { "N": "120" }
                }
            }),
        )
        .await;
    assert_eq!(status, 200);
    let matches = q_res["Items"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["name"]["S"].as_str().unwrap(), "RAM 32GB");
}

#[tokio::test]
#[ignore = "requires running docker daemon and pulled ehlers320/ruststack:latest"]
async fn test_ruststack_container_cloudwatch_metrics_and_logs() {
    let image = ruststack_image();
    let container = image
        .start()
        .await
        .expect("failed to start ruststack container");
    let host_port = container
        .get_host_port_ipv4(4566)
        .await
        .expect("failed to get host port");
    let ctx = ContainerContext::new(host_port);

    // 1. CreateLogGroup & CreateLogStream
    let (status, _) = ctx
        .call_json(
            "Logs_20140328.CreateLogGroup",
            json!({ "logGroupName": "/aws/container/prod-app" }),
        )
        .await;
    assert_eq!(status, 200);

    let (status, _) = ctx
        .call_json(
            "Logs_20140328.CreateLogStream",
            json!({
                "logGroupName": "/aws/container/prod-app",
                "logStreamName": "instance-001"
            }),
        )
        .await;
    assert_eq!(status, 200);

    // 2. PutLogEvents
    let (status, _) = ctx
        .call_json(
            "Logs_20140328.PutLogEvents",
            json!({
                "logGroupName": "/aws/container/prod-app",
                "logStreamName": "instance-001",
                "logEvents": [
                    {
                        "timestamp": 1700000000000_i64,
                        "message": "Application booted successfully"
                    }
                ]
            }),
        )
        .await;
    assert_eq!(status, 200);

    // 3. CloudWatch PutMetricData & ListMetrics
    let (status, put_metric) = ctx
        .call_query(
            "/",
            &[
                ("Action", "PutMetricData"),
                ("Namespace", "CustomApp"),
                ("MetricData.member.1.MetricName", "OrderLatency"),
                ("MetricData.member.1.Value", "45.5"),
                ("MetricData.member.1.Unit", "Milliseconds"),
            ],
        )
        .await;
    assert_eq!(status, 200);
    assert!(put_metric.contains("PutMetricDataResponse"));

    let (status, list_metric) = ctx
        .call_query(
            "/",
            &[("Action", "ListMetrics"), ("Namespace", "CustomApp")],
        )
        .await;
    assert_eq!(status, 200);
    assert!(list_metric.contains("OrderLatency"));
}

#[tokio::test]
#[ignore = "requires running docker daemon and pulled ehlers320/ruststack:latest"]
async fn test_ruststack_container_security_acm_wafv2_and_sts() {
    let image = ruststack_image();
    let container = image
        .start()
        .await
        .expect("failed to start ruststack container");
    let host_port = container
        .get_host_port_ipv4(4566)
        .await
        .expect("failed to get host port");
    let ctx = ContainerContext::new(host_port);

    // 1. STS GetCallerIdentity
    let (status, sts_val) = ctx
        .call_json(
            "AWSSecurityTokenServiceV20110615.GetCallerIdentity",
            json!({}),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(sts_val["Account"].as_str().unwrap(), "000000000000");
    assert!(sts_val["Arn"]
        .as_str()
        .unwrap()
        .contains("arn:aws:iam::000000000000:root"));

    // 2. ACM RequestCertificate & DescribeCertificate
    let (status, acm_val) = ctx
        .call_json(
            "CertificateManager.RequestCertificate",
            json!({
                "DomainName": "api.ruststack.local",
                "ValidationMethod": "DNS"
            }),
        )
        .await;
    assert_eq!(status, 200);
    let cert_arn = acm_val["CertificateArn"].as_str().unwrap();

    let (status, desc_val) = ctx
        .call_json(
            "CertificateManager.DescribeCertificate",
            json!({ "CertificateArn": cert_arn }),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(
        desc_val["Certificate"]["DomainName"].as_str().unwrap(),
        "api.ruststack.local"
    );
    assert_eq!(
        desc_val["Certificate"]["Status"].as_str().unwrap(),
        "ISSUED"
    );

    // 3. WAFv2 CreateWebACL & GetWebACL
    let (status, waf_val) = ctx
        .call_json(
            "AWSWAF_20190729.CreateWebACL",
            json!({
                "Name": "container-waf",
                "Scope": "REGIONAL",
                "DefaultAction": { "Allow": {} },
                "VisibilityConfig": {
                    "SampledRequestsEnabled": true,
                    "CloudWatchMetricsEnabled": true,
                    "MetricName": "ContainerWafMetric"
                }
            }),
        )
        .await;
    assert_eq!(status, 200);
    let waf_id = waf_val["Summary"]["Id"].as_str().unwrap();

    let (status, get_waf) = ctx
        .call_json(
            "AWSWAF_20190729.GetWebACL",
            json!({
                "Name": "container-waf",
                "Id": waf_id,
                "Scope": "REGIONAL"
            }),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(get_waf["WebACL"]["Name"].as_str().unwrap(), "container-waf");
}

#[tokio::test]
#[ignore = "requires running docker daemon and pulled ehlers320/ruststack:latest"]
async fn test_ruststack_container_full_state_snapshot_roundtrip() {
    let image = ruststack_image();
    let container = image
        .start()
        .await
        .expect("failed to start ruststack container");
    let host_port = container
        .get_host_port_ipv4(4566)
        .await
        .expect("failed to get host port");
    let ctx = ContainerContext::new(host_port);

    // 1. Provision S3 bucket and SSM parameter
    let (status, _, _) = ctx
        .call_s3(
            reqwest::Method::PUT,
            "/persist-test-bucket",
            reqwest::header::HeaderMap::new(),
            bytes::Bytes::new(),
        )
        .await;
    assert_eq!(status, 200);

    let (status, _) = ctx
        .call_json(
            "AmazonSSM.PutParameter",
            json!({
                "Name": "/persist/test_token",
                "Value": "snapshotted-secret-val",
                "Type": "String"
            }),
        )
        .await;
    assert_eq!(status, 200);

    // 2. Dump State Snapshot
    let (status, dump_val) = ctx
        .call_rest_json(reqwest::Method::GET, "/_ruststack/state/dump", None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(dump_val["version"].as_i64().unwrap(), 1);

    // 3. Reset State (wipes everything)
    let (status, reset_val) = ctx
        .call_rest_json(reqwest::Method::POST, "/_ruststack/state/reset", None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(reset_val["status"].as_str().unwrap(), "ok");

    // 4. Verify bucket is gone (HEAD returns 404)
    let (status, _, _) = ctx
        .call_s3(
            reqwest::Method::HEAD,
            "/persist-test-bucket",
            reqwest::header::HeaderMap::new(),
            bytes::Bytes::new(),
        )
        .await;
    assert_eq!(status, 404);

    // 5. Restore State from Snapshot
    let (status, load_val) = ctx
        .call_rest_json(
            reqwest::Method::POST,
            "/_ruststack/state/load",
            Some(json!({ "snapshot": dump_val })),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(load_val["status"].as_str().unwrap(), "ok");

    // 6. Verify bucket and SSM parameter are restored!
    let (status, _, _) = ctx
        .call_s3(
            reqwest::Method::HEAD,
            "/persist-test-bucket",
            reqwest::header::HeaderMap::new(),
            bytes::Bytes::new(),
        )
        .await;
    assert_eq!(status, 200);

    let (status, ssm_val) = ctx
        .call_json(
            "AmazonSSM.GetParameter",
            json!({ "Name": "/persist/test_token" }),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(
        ssm_val["Parameter"]["Value"].as_str().unwrap(),
        "snapshotted-secret-val"
    );
}

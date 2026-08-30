use base64::Engine;
use serde_json::json;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url = "http://localhost:4566";
    let client = reqwest::Client::new();

    println!("==========================================================================");
    println!("🧪 LIVE END-TO-END VERIFICATION: RustStack @ {}", base_url);
    println!("==========================================================================");

    // 0. Health Check & Clean Slate
    let t0 = Instant::now();
    let resp = client.get(format!("{}/_ruststack/health", base_url)).send().await?;
    assert_eq!(resp.status(), 200);
    client.post(format!("{}/_ruststack/chaos/reset", base_url)).send().await?;
    client.post(format!("{}/_ruststack/state/reset", base_url)).send().await?;
    println!("✅ 0. Health Check & Cluster Init: OK in {:.2} ms ({})", t0.elapsed().as_secs_f64() * 1000.0, resp.text().await?);

    // 1. Amazon S3 Lifecycle
    println!("\n📦 1. Testing Amazon S3...");
    let t = Instant::now();
    let resp = client.put(format!("{}/live-bucket", base_url)).send().await?;
    assert_eq!(resp.status(), 200, "CreateBucket failed");

    let resp = client.put(format!("{}/live-bucket/data.json", base_url))
        .header("content-type", "application/json")
        .body(r#"{"message": "RustStack is 100% real and blazing fast!"}"#)
        .send().await?;
    assert_eq!(resp.status(), 200, "PutObject failed");

    let resp = client.get(format!("{}/live-bucket/data.json", base_url)).send().await?;
    assert_eq!(resp.status(), 200, "GetObject failed");
    let content = resp.text().await?;
    assert!(content.contains("RustStack is 100% real"));
    println!("   ✓ CreateBucket + PutObject + GetObject verified in {:.2} ms: content = '{}'", t.elapsed().as_secs_f64() * 1000.0, content);

    // 2. Amazon DynamoDB Lifecycle
    println!("\n🗄️ 2. Testing Amazon DynamoDB...");
    let t = Instant::now();
    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "DynamoDB_20120810.CreateTable")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "TableName": "LiveUsers",
            "KeySchema": [{ "AttributeName": "id", "KeyType": "HASH" }],
            "AttributeDefinitions": [{ "AttributeName": "id", "AttributeType": "S" }],
            "BillingMode": "PAY_PER_REQUEST"
        })).send().await?;
    assert_eq!(resp.status(), 200, "CreateTable failed");

    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "DynamoDB_20120810.PutItem")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "TableName": "LiveUsers",
            "Item": {
                "id": { "S": "user-42" },
                "name": { "S": "Ada Lovelace" },
                "role": { "S": "Chief Scientist" }
            }
        })).send().await?;
    assert_eq!(resp.status(), 200, "PutItem failed");

    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "DynamoDB_20120810.GetItem")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "TableName": "LiveUsers",
            "Key": { "id": { "S": "user-42" } }
        })).send().await?;
    assert_eq!(resp.status(), 200, "GetItem failed");
    let item_val: serde_json::Value = resp.json().await?;
    assert_eq!(item_val["Item"]["name"]["S"].as_str().unwrap(), "Ada Lovelace");
    println!("   ✓ CreateTable + PutItem + GetItem verified in {:.2} ms: name = {}", t.elapsed().as_secs_f64() * 1000.0, item_val["Item"]["name"]["S"]);

    // 3. Amazon SQS Lifecycle
    println!("\n📬 3. Testing Amazon SQS...");
    let t = Instant::now();
    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSQS.CreateQueue")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({ "QueueName": "live-orders" })).send().await?;
    assert_eq!(resp.status(), 200);
    let q_url = resp.json::<serde_json::Value>().await?["QueueUrl"].as_str().unwrap().to_string();

    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSQS.SendMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "QueueUrl": q_url,
            "MessageBody": "order-id-998877"
        })).send().await?;
    assert_eq!(resp.status(), 200);

    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSQS.ReceiveMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "QueueUrl": q_url,
            "MaxNumberOfMessages": 1
        })).send().await?;
    assert_eq!(resp.status(), 200);
    let rcv_body: serde_json::Value = resp.json().await?;
    let msg_body = rcv_body["Messages"][0]["Body"].as_str().unwrap();
    assert_eq!(msg_body, "order-id-998877");
    println!("   ✓ CreateQueue + SendMessage + ReceiveMessage verified in {:.2} ms: msg = '{}'", t.elapsed().as_secs_f64() * 1000.0, msg_body);

    // 4. Amazon SNS -> SQS Fanout
    println!("\n📢 4. Testing Amazon SNS -> SQS Fanout...");
    let t = Instant::now();
    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSNS.CreateTopic")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({ "Name": "live-notifications" })).send().await?;
    assert_eq!(resp.status(), 200);
    let topic_arn = resp.json::<serde_json::Value>().await?["TopicArn"].as_str().unwrap().to_string();

    let q_arn = "arn:aws:sqs:us-east-1:000000000000:live-orders";
    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSNS.Subscribe")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "TopicArn": topic_arn,
            "Protocol": "sqs",
            "Endpoint": q_arn
        })).send().await?;
    assert_eq!(resp.status(), 200);

    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSNS.Publish")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "TopicArn": topic_arn,
            "Message": "CRITICAL_SYSTEM_ALERT_FANOUT"
        })).send().await?;
    assert_eq!(resp.status(), 200);

    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSQS.ReceiveMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "QueueUrl": q_url,
            "MaxNumberOfMessages": 1
        })).send().await?;
    assert_eq!(resp.status(), 200);
    let rcv_body: serde_json::Value = resp.json().await?;
    let raw_env = rcv_body["Messages"][0]["Body"].as_str().unwrap();
    let parsed_env: serde_json::Value = serde_json::from_str(raw_env)?;
    assert_eq!(parsed_env["Message"].as_str().unwrap(), "CRITICAL_SYSTEM_ALERT_FANOUT");
    println!("   ✓ CreateTopic + Subscribe + Publish + SQS Fanout Delivery verified in {:.2} ms!", t.elapsed().as_secs_f64() * 1000.0);

    // 5. Amazon SSM Parameter Store
    println!("\n🔐 5. Testing Amazon SSM Parameter Store...");
    let t = Instant::now();
    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSSM.PutParameter")
        .header("content-type", "application/x-amz-json-1.1")
        .json(&json!({
            "Name": "/prod/db/connection_string",
            "Value": "postgresql://admin:supersecret@10.0.0.1:5432/main",
            "Type": "SecureString",
            "Overwrite": true
        })).send().await?;
    assert_eq!(resp.status(), 200);

    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSSM.GetParameter")
        .header("content-type", "application/x-amz-json-1.1")
        .json(&json!({ "Name": "/prod/db/connection_string" })).send().await?;
    assert_eq!(resp.status(), 200);
    let ssm_val: serde_json::Value = resp.json().await?;
    assert_eq!(ssm_val["Parameter"]["Value"].as_str().unwrap(), "postgresql://admin:supersecret@10.0.0.1:5432/main");
    println!("   ✓ PutParameter + GetParameter verified in {:.2} ms!", t.elapsed().as_secs_f64() * 1000.0);

    // 6. Amazon Secrets Manager
    println!("\n🗝️ 6. Testing Amazon Secrets Manager...");
    let t = Instant::now();
    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "secretsmanager.CreateSecret")
        .header("content-type", "application/x-amz-json-1.1")
        .json(&json!({
            "Name": "live/jwt/private-key",
            "SecretString": "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0..."
        })).send().await?;
    assert_eq!(resp.status(), 200);

    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "secretsmanager.GetSecretValue")
        .header("content-type", "application/x-amz-json-1.1")
        .json(&json!({ "SecretId": "live/jwt/private-key" })).send().await?;
    assert_eq!(resp.status(), 200);
    let sec_val: serde_json::Value = resp.json().await?;
    assert!(sec_val["SecretString"].as_str().unwrap().starts_with("-----BEGIN"));
    println!("   ✓ CreateSecret + GetSecretValue verified in {:.2} ms!", t.elapsed().as_secs_f64() * 1000.0);

    // 7. Amazon STS Identity
    println!("\n🆔 7. Testing Amazon STS...");
    let t = Instant::now();
    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "AWSSecurityTokenServiceV20110615.GetCallerIdentity")
        .header("content-type", "application/x-amz-json-1.1")
        .json(&json!({})).send().await?;
    assert_eq!(resp.status(), 200);
    let sts_val: serde_json::Value = resp.json().await?;
    assert_eq!(sts_val["Account"].as_str().unwrap(), "000000000000");
    println!("   ✓ GetCallerIdentity verified in {:.2} ms: Account = {}", t.elapsed().as_secs_f64() * 1000.0, sts_val["Account"]);

    // 8. AWS Key Management Service (KMS)
    println!("\n🔐 8. Testing AWS Key Management Service (KMS)...");
    let t = Instant::now();
    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "TrentService.CreateKey")
        .header("content-type", "application/x-amz-json-1.1")
        .json(&json!({
            "Description": "Live Test Key",
            "KeyUsage": "ENCRYPT_DECRYPT"
        })).send().await?;
    assert_eq!(resp.status(), 200);
    let kms_val: serde_json::Value = resp.json().await?;
    let key_id = kms_val["KeyMetadata"]["KeyId"].as_str().unwrap().to_string();

    // Create Alias
    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "TrentService.CreateAlias")
        .header("content-type", "application/x-amz-json-1.1")
        .json(&json!({
            "AliasName": "alias/live-key",
            "TargetKeyId": key_id
        })).send().await?;
    assert_eq!(resp.status(), 200);

    // Encrypt
    let plain_b64 = base64::engine::general_purpose::STANDARD.encode("LiveEncryptedPayload123!");
    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "TrentService.Encrypt")
        .header("content-type", "application/x-amz-json-1.1")
        .json(&json!({
            "KeyId": "alias/live-key",
            "Plaintext": plain_b64
        })).send().await?;
    assert_eq!(resp.status(), 200);
    let cipher_blob = resp.json::<serde_json::Value>().await?["CiphertextBlob"].as_str().unwrap().to_string();

    // Decrypt
    let resp = client.post(format!("{}/", base_url))
        .header("x-amz-target", "TrentService.Decrypt")
        .header("content-type", "application/x-amz-json-1.1")
        .json(&json!({
            "CiphertextBlob": cipher_blob
        })).send().await?;
    assert_eq!(resp.status(), 200);
    let decrypted_b64 = resp.json::<serde_json::Value>().await?["Plaintext"].as_str().unwrap().to_string();
    assert_eq!(decrypted_b64, plain_b64);
    println!("   ✓ CreateKey + CreateAlias + Encrypt + Decrypt verified in {:.2} ms!", t.elapsed().as_secs_f64() * 1000.0);

    // 9. Chaos Engineering Fault Injection & Auto-Healing
    println!("\n🌪️ 9. Testing Chaos Engineering & Self-Healing...");
    let t = Instant::now();
    let resp = client.post(format!("{}/_ruststack/chaos/rules", base_url))
        .json(&json!({
            "service": "s3",
            "error_status": 503,
            "error_code": "SlowDown",
            "limit_times": 1
        })).send().await?;
    assert!(resp.status().is_success(), "Register chaos rule failed");

    // Request 1: Should be throttled with 503 SlowDown
    let resp = client.put(format!("{}/live-bucket/chaos-test.txt", base_url)).body("test").send().await?;
    assert_eq!(resp.status(), 503, "Expected 503 SlowDown injected");
    let err_xml = resp.text().await?;
    assert!(err_xml.contains("SlowDown"));

    // Request 2: Rule auto-expired! Should succeed with 200 OK!
    let resp = client.put(format!("{}/live-bucket/chaos-test.txt", base_url)).body("test").send().await?;
    assert_eq!(resp.status(), 200, "Expected 200 OK after auto-heal");
    println!("   ✓ Injected 503 SlowDown -> Triggered -> Auto-Healed on next request in {:.2} ms!", t.elapsed().as_secs_f64() * 1000.0);

    // 9. State Dump & Selective Reset
    println!("\n💾 9. Testing State Dump & Restoration...");
    let t = Instant::now();
    let resp = client.get(format!("{}/_ruststack/state/dump", base_url)).send().await?;
    assert_eq!(resp.status(), 200);
    let snapshot_json = resp.text().await?;
    assert!(snapshot_json.contains("LiveUsers"));
    assert!(snapshot_json.contains("live-orders"));

    // Reset SQS only
    let resp = client.post(format!("{}/_ruststack/state/reset", base_url))
        .json(&json!({ "services": ["sqs"] })).send().await?;
    assert_eq!(resp.status(), 200);

    // Verify SQS wiped while DynamoDB still intact
    let resp = client.get(format!("{}/_ruststack/state/dump", base_url)).send().await?;
    let partial_dump: serde_json::Value = resp.json().await?;
    assert_eq!(partial_dump["sqs"]["queues"].as_array().unwrap().len(), 0);
    assert_eq!(partial_dump["dynamodb"]["tables"].as_array().unwrap().len(), 1);

    // Restore full state
    let resp = client.post(format!("{}/_ruststack/state/load", base_url))
        .header("content-type", "application/json")
        .body(snapshot_json).send().await?;
    assert_eq!(resp.status(), 200);

    // Verify SQS restored
    let resp = client.get(format!("{}/_ruststack/state/dump", base_url)).send().await?;
    let restored_dump: serde_json::Value = resp.json().await?;
    assert_eq!(restored_dump["sqs"]["queues"].as_array().unwrap().len(), 1);
    println!("   ✓ Full State Snapshot Export + Selective Reset + Snapshot Restore verified in {:.2} ms!", t.elapsed().as_secs_f64() * 1000.0);

    // 10. Web Admin UI
    println!("\n🖥️ 10. Testing Embedded Web Admin UI...");
    let t = Instant::now();
    let resp = client.get(format!("{}/_ruststack/ui/", base_url)).send().await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers().get("content-type").unwrap(), "text/html; charset=utf-8");
    let html = resp.text().await?;
    assert!(html.contains("RustStack Cloud Console"));
    println!("   ✓ Web Console delivered in {:.2} ms ({} bytes)", t.elapsed().as_secs_f64() * 1000.0, html.len());

    println!("\n==========================================================================");
    println!("🎉 ALL 10 COMPREHENSIVE LIVE TESTS PASSED 100% AGAINST RUNNING CONTAINER!");
    println!("==========================================================================");

    Ok(())
}

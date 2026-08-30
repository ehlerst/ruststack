use axum::http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_sts_caller_identity_and_assume_role() {
    let client = RustStackTestClient::new();

    // 1. GetCallerIdentity JSON protocol
    let (status, val) = client
        .call_json(
            "AWSSecurityTokenServiceV20110615.GetCallerIdentity",
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(val["Account"].as_str().unwrap(), client.account_id);
    assert_eq!(val["UserId"].as_str().unwrap(), client.account_id);
    assert!(val["Arn"].as_str().unwrap().contains("root"));

    // 2. GetCallerIdentity Query protocol
    let (status, text) = client
        .call_query(
            "/",
            &[("Action", "GetCallerIdentity"), ("Version", "2011-06-15")],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(text.contains("<Account>000000000000</Account>"));
    assert!(text.contains("<UserId>000000000000</UserId>"));

    // 3. AssumeRole JSON protocol
    let (status, val) = client
        .call_json(
            "AWSSecurityTokenServiceV20110615.AssumeRole",
            json!({
                "RoleArn": "arn:aws:iam::000000000000:role/deploy-role",
                "RoleSessionName": "deploy-session-123"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let creds = &val["Credentials"];
    assert!(creds["AccessKeyId"].as_str().unwrap().starts_with("ASIA"));
    assert!(creds["SecretAccessKey"].is_string());
    assert!(creds["SessionToken"].is_string());
    assert!(val["AssumedRoleUser"]["Arn"]
        .as_str()
        .unwrap()
        .contains("deploy-session-123"));
}

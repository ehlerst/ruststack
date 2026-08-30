use axum::http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_secretsmanager_versioning_and_staging_labels() {
    let client = RustStackTestClient::new();

    // 1. CreateSecret
    let (status, val) = client
        .call_json(
            "secretsmanager.CreateSecret",
            json!({
                "Name": "app/prod/api_key",
                "Description": "Production API keys",
                "SecretString": "{\"api_key\": \"key_v1\"}"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(val["Name"].as_str().unwrap(), "app/prod/api_key");
    assert!(val["VersionId"].is_string());

    // 2. GetSecretValue (AWSCURRENT)
    let (status, val) = client
        .call_json(
            "secretsmanager.GetSecretValue",
            json!({
                "SecretId": "app/prod/api_key"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        val["SecretString"].as_str().unwrap(),
        "{\"api_key\": \"key_v1\"}"
    );
    let stages = val["VersionStages"].as_array().unwrap();
    assert!(stages.iter().any(|s| s.as_str().unwrap() == "AWSCURRENT"));

    // 3. PutSecretValue (Version 2)
    let (status, val) = client
        .call_json(
            "secretsmanager.PutSecretValue",
            json!({
                "SecretId": "app/prod/api_key",
                "SecretString": "{\"api_key\": \"key_v2_rotated\"}"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let v2_id = val["VersionId"].as_str().unwrap().to_string();

    // 4. GetSecretValue returns v2 by default
    let (status, val) = client
        .call_json(
            "secretsmanager.GetSecretValue",
            json!({
                "SecretId": "app/prod/api_key"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(val["VersionId"].as_str().unwrap(), v2_id);
    assert_eq!(
        val["SecretString"].as_str().unwrap(),
        "{\"api_key\": \"key_v2_rotated\"}"
    );

    // 5. GetSecretValue by stage AWSPREVIOUS returns v1
    let (status, val) = client
        .call_json(
            "secretsmanager.GetSecretValue",
            json!({
                "SecretId": "app/prod/api_key",
                "VersionStage": "AWSPREVIOUS"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        val["SecretString"].as_str().unwrap(),
        "{\"api_key\": \"key_v1\"}"
    );
}

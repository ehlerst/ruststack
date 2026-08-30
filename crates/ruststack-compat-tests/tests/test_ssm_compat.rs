use axum::http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_ssm_parameter_versioning_and_path_traversal() {
    let client = RustStackTestClient::new();

    // 1. PutParameter version 1
    let (status, val) = client
        .call_json(
            "AmazonSSM.PutParameter",
            json!({
                "Name": "/prod/db/host",
                "Value": "db1.internal",
                "Type": "String"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(val["Version"].as_i64().unwrap(), 1);

    // 2. PutParameter overwrite version 2
    let (status, val) = client
        .call_json(
            "AmazonSSM.PutParameter",
            json!({
                "Name": "/prod/db/host",
                "Value": "db2.internal",
                "Type": "String",
                "Overwrite": true
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(val["Version"].as_i64().unwrap(), 2);

    // 3. Put additional parameters under /prod
    let params = vec![
        ("/prod/db/port", "5432", "String"),
        ("/prod/db/password", "secret_db_pass", "SecureString"),
        ("/prod/redis/url", "redis://cache:6379", "String"),
        ("/dev/db/host", "localhost", "String"),
    ];

    for (name, val_str, p_type) in params {
        let (status, _) = client
            .call_json(
                "AmazonSSM.PutParameter",
                json!({
                    "Name": name,
                    "Value": val_str,
                    "Type": p_type
                }),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
    }

    // 4. GetParametersByPath with recursive=true
    let (status, val) = client
        .call_json(
            "AmazonSSM.GetParametersByPath",
            json!({
                "Path": "/prod",
                "Recursive": true
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let items = val["Parameters"].as_array().unwrap();
    assert_eq!(items.len(), 4);

    // 5. GetParametersByPath under /prod/db (single level)
    let (status, val) = client
        .call_json(
            "AmazonSSM.GetParametersByPath",
            json!({
                "Path": "/prod/db",
                "Recursive": false
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let db_items = val["Parameters"].as_array().unwrap();
    assert_eq!(db_items.len(), 3);
}

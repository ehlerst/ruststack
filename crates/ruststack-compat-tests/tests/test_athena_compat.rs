use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_athena_query_compat() {
    let client = RustStackTestClient::new();

    // 1. StartQueryExecution
    let (status, body) = client
        .call_json(
            "AmazonAthena.StartQueryExecution",
            json!({
                "QueryString": "SELECT COUNT(*) FROM access_logs",
                "WorkGroup": "primary"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let qid = body["QueryExecutionId"].as_str().unwrap();

    // 2. GetQueryResults
    let (status, body) = client
        .call_json(
            "AmazonAthena.GetQueryResults",
            json!({
                "QueryExecutionId": qid
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["ResultSet"]["Rows"].as_array().unwrap();
    assert!(rows.len() >= 2);
}

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_athena::{handle_athena_request, AthenaState};

#[tokio::test]
async fn test_athena_query_and_results_lifecycle() {
    let state = AthenaState::new("000000000000".to_string(), "us-east-1".to_string());
    let mut headers = HeaderMap::new();
    headers.insert("x-amz-target", "AmazonAthena.StartQueryExecution".parse().unwrap());

    // 1. StartQueryExecution
    let body = Bytes::from(serde_json::json!({
        "QueryString": "SELECT COUNT(*) FROM cloudtrail_logs",
        "WorkGroup": "primary"
    }).to_string());

    let resp = handle_athena_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let qid = json["QueryExecutionId"].as_str().unwrap();

    // 2. GetQueryExecution
    let mut headers = HeaderMap::new();
    headers.insert("x-amz-target", "AmazonAthena.GetQueryExecution".parse().unwrap());
    let body = Bytes::from(serde_json::json!({
        "QueryExecutionId": qid
    }).to_string());

    let resp = handle_athena_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["QueryExecution"]["Status"]["State"], "SUCCEEDED");

    // 3. GetQueryResults
    let mut headers = HeaderMap::new();
    headers.insert("x-amz-target", "AmazonAthena.GetQueryResults".parse().unwrap());
    let body = Bytes::from(serde_json::json!({
        "QueryExecutionId": qid
    }).to_string());

    let resp = handle_athena_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["ResultSet"]["Rows"].as_array().unwrap().len() >= 2);
}

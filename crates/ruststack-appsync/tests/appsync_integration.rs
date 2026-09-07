use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_appsync::{handle_appsync_request, AppSyncState};
use serde_json::json;

#[tokio::test]
async fn test_appsync_api_and_graphql_lifecycle() {
    let state = AppSyncState::new("000000000000".to_string(), "us-east-1".to_string());
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());

    // 1. Create GraphqlApi
    let body = Bytes::from(
        json!({
            "name": "ecommerce-api",
            "authenticationType": "API_KEY"
        })
        .to_string(),
    );
    let uri: Uri = "/v1/apis".parse().unwrap();
    let resp = handle_appsync_request(
        State(state.clone()),
        Method::POST,
        uri,
        headers.clone(),
        body,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let api_id = val["graphqlApi"]["apiId"].as_str().unwrap();
    assert_eq!(val["graphqlApi"]["name"], "ecommerce-api");

    // 2. Get GraphqlApi
    let uri: Uri = format!("/v1/apis/{}", api_id).parse().unwrap();
    let resp = handle_appsync_request(
        State(state.clone()),
        Method::GET,
        uri,
        headers.clone(),
        Bytes::new(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["graphqlApi"]["apiId"], api_id);

    // 3. Create ApiKey
    let uri: Uri = format!("/v1/apis/{}/apikeys", api_id).parse().unwrap();
    let body = Bytes::from(json!({ "description": "Test Key" }).to_string());
    let resp = handle_appsync_request(
        State(state.clone()),
        Method::POST,
        uri,
        headers.clone(),
        body,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(val["apiKey"]["id"].as_str().unwrap().starts_with("da2-"));

    // 4. Create DataSource
    let uri: Uri = format!("/v1/apis/{}/datasources", api_id).parse().unwrap();
    let body = Bytes::from(json!({ "name": "NoneDS", "type": "NONE" }).to_string());
    let resp = handle_appsync_request(
        State(state.clone()),
        Method::POST,
        uri,
        headers.clone(),
        body,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. Execute GraphQL query
    let uri: Uri = format!("/v1/apis/{}/graphql", api_id).parse().unwrap();
    let body = Bytes::from(json!({ "query": "query { hello }" }).to_string());
    let resp = handle_appsync_request(
        State(state.clone()),
        Method::POST,
        uri,
        headers.clone(),
        body,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["data"]["hello"], "world");

    // 6. Delete GraphqlApi
    let uri: Uri = format!("/v1/apis/{}", api_id).parse().unwrap();
    let resp = handle_appsync_request(
        State(state.clone()),
        Method::DELETE,
        uri,
        headers.clone(),
        Bytes::new(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

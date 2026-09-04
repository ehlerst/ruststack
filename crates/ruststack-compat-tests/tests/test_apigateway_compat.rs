use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_apigateway_rest_api_and_mock_routing_compat() {
    let client = RustStackTestClient::new();

    // 1. CreateRestApi
    let req = Request::builder()
        .method(Method::POST)
        .uri("/restapis")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "EcommerceApi",
                "description": "API Gateway for Ecommerce"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = client.send_request(req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let api_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let api_id = api_json["id"].as_str().unwrap();
    let root_res_id = api_json["rootResourceId"].as_str().unwrap();

    // 2. CreateResource /items
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/restapis/{}/resources/{}", api_id, root_res_id))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "pathPart": "items"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = client.send_request(req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let res_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let items_res_id = res_json["id"].as_str().unwrap();

    // 3. PutMethod GET
    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!(
            "/restapis/{}/resources/{}/methods/GET",
            api_id, items_res_id
        ))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "authorizationType": "NONE"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = client.send_request(req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 4. PutIntegration Mock
    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!(
            "/restapis/{}/resources/{}/methods/GET/integration",
            api_id, items_res_id
        ))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "type": "MOCK",
                "requestTemplates": {
                    "application/json": "{\"items\": [\"item1\", \"item2\"]}"
                }
            })
            .to_string(),
        ))
        .unwrap();
    let resp = client.send_request(req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 5. CreateDeployment for stage "v1"
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/restapis/{}/deployments", api_id))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "stageName": "v1"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = client.send_request(req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 6. Test Invocation routing /{api_id}/v1/items
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/{}/v1/items", api_id))
        .body(Body::empty())
        .unwrap();
    let resp = client.send_request(req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let out: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(out["items"][0].as_str().unwrap(), "item1");
}

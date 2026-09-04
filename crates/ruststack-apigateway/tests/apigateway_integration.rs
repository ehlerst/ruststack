use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use bytes::Bytes;
use ruststack_apigateway::handle_apigateway_request;
use ruststack_apigateway::ApiGatewayState;
use ruststack_apigateway::types::*;

#[tokio::test]
async fn test_apigateway_lifecycle_and_mock_invocation() {
    let state = ApiGatewayState::new("000000000000", "us-east-1");

    // 1. Create REST API
    let api = state
        .create_rest_api(CreateRestApiRequest {
            name: "PetStore".to_string(),
            description: Some("My Pet Store API".to_string()),
        })
        .unwrap();
    let api_id = api.id;
    let root_id = api.root_resource_id;

    // 2. Create Resource /pets
    let pet_resource = state
        .create_resource(&api_id, &root_id, "pets")
        .unwrap();
    let pet_res_id = pet_resource.id;
    assert_eq!(pet_resource.path, "/pets");

    // 3. Put Method GET /pets
    state
        .put_method(
            &api_id,
            &pet_res_id,
            "GET",
            PutMethodRequest {
                authorization_type: Some("NONE".to_string()),
                request_parameters: None,
            },
        )
        .unwrap();

    // 4. Put Mock Integration
    let mut tpls = std::collections::HashMap::new();
    tpls.insert(
        "application/json".to_string(),
        r#"{"pets": [{"id": 1, "name": "Dog"}]}"#.to_string(),
    );

    state
        .put_integration(
            &api_id,
            &pet_res_id,
            "GET",
            PutIntegrationRequest {
                integration_type: IntegrationType::Mock,
                http_method: Some("GET".to_string()),
                uri: None,
                request_templates: Some(tpls),
            },
        )
        .unwrap();

    // 5. Create Deployment & Stage prod
    state
        .create_deployment(
            &api_id,
            CreateDeploymentRequest {
                stage_name: Some("prod".to_string()),
                description: Some("Production deployment".to_string()),
                stage_description: None,
                variables: None,
            },
        )
        .unwrap();

    // 6. Invoke GET /{api_id}/prod/pets via HTTP Handler
    let uri: Uri = format!("/{}/prod/pets", api_id).parse().unwrap();
    let resp = handle_apigateway_request(
        State(state.clone()),
        Method::GET,
        uri,
        HeaderMap::new(),
        Bytes::new(),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(resp.into_body()).await.unwrap().to_bytes();
    let json_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json_resp["pets"][0]["name"].as_str().unwrap(), "Dog");
}

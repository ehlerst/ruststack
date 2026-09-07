use http::{Method, StatusCode};
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_appsync_lifecycle_compat() {
    let client = RustStackTestClient::new();

    // 1. Create GraphQL API
    let (status, body) = client
        .call_rest_json(
            Method::POST,
            "/v1/apis",
            Some(json!({
                "name": "supergraph-api",
                "authenticationType": "API_KEY"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let api_id = body["graphqlApi"]["apiId"].as_str().unwrap();
    assert_eq!(body["graphqlApi"]["name"], "supergraph-api");
    assert!(body["graphqlApi"]["uris"]["GRAPHQL"]
        .as_str()
        .unwrap()
        .contains(api_id));

    // 2. Get GraphQL API
    let api_uri = format!("/v1/apis/{}", api_id);
    let (status, body) = client.call_rest_json(Method::GET, &api_uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["graphqlApi"]["apiId"], api_id);

    // 3. Create API Key
    let api_keys_uri = format!("/v1/apis/{}/apikeys", api_id);
    let (status, body) = client
        .call_rest_json(
            Method::POST,
            &api_keys_uri,
            Some(json!({ "description": "Production client key" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let key_id = body["apiKey"]["id"].as_str().unwrap();
    assert!(key_id.starts_with("da2-"));

    // 4. List API Keys
    let (status, body) = client
        .call_rest_json(Method::GET, &api_keys_uri, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let keys = body["apiKeys"].as_array().unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0]["id"], key_id);

    // 5. Start Schema Creation
    let schema_uri = format!("/v1/apis/{}/schemacreation", api_id);
    let (status, body) = client
        .call_rest_json(
            Method::POST,
            &schema_uri,
            Some(json!({
                "definition": "type Query { ping: String }"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "PROCESSING");

    // 6. Get Schema Creation Status
    let (status, body) = client.call_rest_json(Method::GET, &schema_uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "SUCCESS");

    // 7. Create Data Source
    let ds_uri = format!("/v1/apis/{}/datasources", api_id);
    let (status, body) = client
        .call_rest_json(
            Method::POST,
            &ds_uri,
            Some(json!({
                "name": "DynamoDB_Users",
                "type": "AMAZON_DYNAMODB"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["dataSource"]["name"], "DynamoDB_Users");

    // 8. List Data Sources
    let (status, body) = client.call_rest_json(Method::GET, &ds_uri, None).await;
    assert_eq!(status, StatusCode::OK);
    let ds_list = body["dataSources"].as_array().unwrap();
    assert_eq!(ds_list.len(), 1);
    assert_eq!(ds_list[0]["name"], "DynamoDB_Users");

    // 9. Execute GraphQL Query
    let gql_uri = format!("/v1/apis/{}/graphql", api_id);
    let (status, body) = client
        .call_rest_json(
            Method::POST,
            &gql_uri,
            Some(json!({
                "query": "query { ping }"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["data"]["status"],
        "RustStack AppSync Simulated Response"
    );

    // 10. Delete GraphQL API
    let (status, _) = client.call_rest_json(Method::DELETE, &api_uri, None).await;
    assert_eq!(status, StatusCode::OK);

    // 11. Verify List GraphQL APIs is empty
    let (status, body) = client.call_rest_json(Method::GET, "/v1/apis", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["graphqlApis"].as_array().unwrap().len(), 0);
}

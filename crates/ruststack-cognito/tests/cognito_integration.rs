use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use bytes::Bytes;
use ruststack_cognito::handle_cognito_request;
use ruststack_cognito::CognitoState;
use ruststack_cognito::types::*;

#[tokio::test]
async fn test_cognito_user_pool_and_auth_lifecycle() {
    let state = CognitoState::new("000000000000", "us-east-1");

    // 1. Create User Pool
    let pool_resp = state
        .create_user_pool(CreateUserPoolRequest {
            pool_name: "test-pool".to_string(),
        })
        .unwrap();
    let pool_id = pool_resp.user_pool.id;
    assert_eq!(pool_resp.user_pool.name, "test-pool");

    // 2. Create User Pool Client
    let client_resp = state
        .create_user_pool_client(CreateUserPoolClientRequest {
            user_pool_id: pool_id.clone(),
            client_name: "web-client".to_string(),
            generate_secret: Some(false),
        })
        .unwrap();
    let client_id = client_resp.user_pool_client.client_id;
    assert_eq!(client_resp.user_pool_client.client_name, "web-client");

    // 3. Sign Up User
    let signup_resp = state
        .sign_up(SignUpRequest {
            client_id: client_id.clone(),
            username: "alice".to_string(),
            password: "MyPassword123!".to_string(),
            user_attributes: Some(vec![AttributeType {
                name: "email".to_string(),
                value: Some("alice@example.com".to_string()),
            }]),
        })
        .unwrap();
    assert!(signup_resp.user_confirmed);
    assert!(!signup_resp.user_sub.is_empty());

    // 4. Admin Get User
    let get_user_resp = state
        .admin_get_user(AdminGetUserRequest {
            user_pool_id: pool_id.clone(),
            username: "alice".to_string(),
        })
        .unwrap();
    assert_eq!(get_user_resp.username, "alice");
    assert_eq!(get_user_resp.user_status, UserStatusType::Confirmed);

    // 5. Initiate Auth -> Login with password
    let mut auth_params = std::collections::HashMap::new();
    auth_params.insert("USERNAME".to_string(), "alice".to_string());
    auth_params.insert("PASSWORD".to_string(), "MyPassword123!".to_string());

    let auth_resp = state
        .initiate_auth(InitiateAuthRequest {
            auth_flow: "USER_PASSWORD_AUTH".to_string(),
            client_id: client_id.clone(),
            auth_parameters: Some(auth_params),
        })
        .unwrap();

    let auth_result = auth_resp.authentication_result.unwrap();
    let access_token = auth_result.access_token.unwrap();
    let id_token = auth_result.id_token.unwrap();
    assert!(!access_token.is_empty());
    assert!(!id_token.is_empty());

    // 6. Test HTTP Gateway and JWKS endpoint
    let headers = HeaderMap::new();
    let uri: Uri = format!("/{}/.well-known/jwks.json", pool_id).parse().unwrap();
    let resp = handle_cognito_request(State(state.clone()), uri, headers.clone(), Bytes::new()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(resp.into_body()).await.unwrap().to_bytes();
    let jwks: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(jwks.get("keys").is_some());
}

use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_cognito_user_pools_and_token_flow_compat() {
    let client = RustStackTestClient::new();

    // 1. CreateUserPool
    let (status, body) = client
        .call_json(
            "AWSCognitoIdentityProviderService.CreateUserPool",
            json!({
                "PoolName": "ProdUsers"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let pool_id = body["UserPool"]["Id"].as_str().unwrap();
    assert_eq!(body["UserPool"]["Name"].as_str().unwrap(), "ProdUsers");

    // 2. CreateUserPoolClient
    let (status, body) = client
        .call_json(
            "AWSCognitoIdentityProviderService.CreateUserPoolClient",
            json!({
                "UserPoolId": pool_id,
                "ClientName": "MobileApp"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let client_id = body["UserPoolClient"]["ClientId"].as_str().unwrap();

    // 3. AdminCreateUser
    let (status, body) = client
        .call_json(
            "AWSCognitoIdentityProviderService.AdminCreateUser",
            json!({
                "UserPoolId": pool_id,
                "Username": "bob",
                "TemporaryPassword": "TempPassword99!",
                "UserAttributes": [
                    {"Name": "email", "Value": "bob@example.com"}
                ]
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["User"]["Username"].as_str().unwrap(), "bob");

    // 4. AdminSetUserPassword (Permanent)
    let (status, _) = client
        .call_json(
            "AWSCognitoIdentityProviderService.AdminSetUserPassword",
            json!({
                "UserPoolId": pool_id,
                "Username": "bob",
                "Password": "PermanentPassword123!",
                "Permanent": true
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 5. InitiateAuth
    let (status, body) = client
        .call_json(
            "AWSCognitoIdentityProviderService.InitiateAuth",
            json!({
                "AuthFlow": "USER_PASSWORD_AUTH",
                "ClientId": client_id,
                "AuthParameters": {
                    "USERNAME": "bob",
                    "PASSWORD": "PermanentPassword123!"
                }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let auth_res = &body["AuthenticationResult"];
    assert!(auth_res["AccessToken"].is_string());
    assert!(auth_res["IdToken"].is_string());
}

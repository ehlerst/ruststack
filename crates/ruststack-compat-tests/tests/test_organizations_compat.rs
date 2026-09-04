use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_organizations_lifecycle_compat() {
    let client = RustStackTestClient::new();

    // 1. Create Organization
    let (status, body) = client
        .call_json(
            "AWSOrganizationsV20161128.CreateOrganization",
            json!({
                "FeatureSet": "ALL"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let org_id = body["Organization"]["Id"].as_str().unwrap();
    assert!(org_id.starts_with("o-"));

    // 2. Describe Organization
    let (status, body) = client
        .call_json(
            "AWSOrganizationsV20161128.DescribeOrganization",
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["Organization"]["Id"], org_id);

    // 3. Create Account
    let (status, body) = client
        .call_json(
            "AWSOrganizationsV20161128.CreateAccount",
            json!({
                "AccountName": "Staging",
                "Email": "staging@example.com"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["CreateAccountStatus"]["State"], "SUCCEEDED");

    // 4. List Accounts
    let (status, body) = client
        .call_json(
            "AWSOrganizationsV20161128.ListAccounts",
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["Accounts"].as_array().unwrap().len(), 2);
}

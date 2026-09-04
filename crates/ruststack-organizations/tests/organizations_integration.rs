use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_organizations::{handle_organizations_request, OrganizationsState};
use serde_json::json;

#[tokio::test]
async fn test_organizations_lifecycle() {
    let state = OrganizationsState::new("000000000000".to_string(), "us-east-1".to_string());
    let mut headers = HeaderMap::new();

    // 1. Create Organization
    headers.insert("x-amz-target", "AWSOrganizationsV20161128.CreateOrganization".parse().unwrap());
    let resp = handle_organizations_request(State(state.clone()), headers.clone(), Bytes::from("{}")).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let org_id = json["Organization"]["Id"].as_str().unwrap();
    assert!(org_id.starts_with("o-"));

    // 2. Describe Organization
    headers.insert("x-amz-target", "AWSOrganizationsV20161128.DescribeOrganization".parse().unwrap());
    let resp = handle_organizations_request(State(state.clone()), headers.clone(), Bytes::from("{}")).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Create Account
    headers.insert("x-amz-target", "AWSOrganizationsV20161128.CreateAccount".parse().unwrap());
    let body = Bytes::from(
        json!({
            "AccountName": "Production",
            "Email": "prod@example.com"
        })
        .to_string(),
    );
    let resp = handle_organizations_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["CreateAccountStatus"]["State"], "SUCCEEDED");

    // 4. List Accounts
    headers.insert("x-amz-target", "AWSOrganizationsV20161128.ListAccounts".parse().unwrap());
    let resp = handle_organizations_request(State(state.clone()), headers.clone(), Bytes::from("{}")).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["Accounts"].as_array().unwrap().len(), 2);
}

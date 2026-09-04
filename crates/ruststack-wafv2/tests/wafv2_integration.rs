use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_wafv2::{handle_wafv2_request, Wafv2State};
use serde_json::json;

#[tokio::test]
async fn test_wafv2_web_acl_and_ipset_lifecycle() {
    let state = Wafv2State::new("000000000000".to_string(), "us-east-1".to_string());
    let mut headers = HeaderMap::new();

    // 1. Create WebACL
    headers.insert("x-amz-target", "AWSWAF_20190729.CreateWebACL".parse().unwrap());
    let body = Bytes::from(
        json!({
            "Name": "test-web-acl",
            "Scope": "REGIONAL",
            "DefaultAction": { "Allow": {} },
            "VisibilityConfig": {
                "SampledRequestsEnabled": true,
                "CloudWatchMetricsEnabled": true,
                "MetricName": "TestWebAclMetric"
            }
        })
        .to_string(),
    );
    let resp = handle_wafv2_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = json["Summary"]["Id"].as_str().unwrap();
    assert_eq!(json["Summary"]["Name"], "test-web-acl");

    // 2. Get WebACL
    headers.insert("x-amz-target", "AWSWAF_20190729.GetWebACL".parse().unwrap());
    let body = Bytes::from(json!({ "Name": "test-web-acl", "Id": id, "Scope": "REGIONAL" }).to_string());
    let resp = handle_wafv2_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Create IPSet
    headers.insert("x-amz-target", "AWSWAF_20190729.CreateIPSet".parse().unwrap());
    let body = Bytes::from(
        json!({
            "Name": "blocked-ips",
            "Scope": "REGIONAL",
            "IPAddressVersion": "IPV4",
            "Addresses": ["192.0.2.0/24", "198.51.100.0/24"]
        })
        .to_string(),
    );
    let resp = handle_wafv2_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["Summary"]["Name"], "blocked-ips");
}

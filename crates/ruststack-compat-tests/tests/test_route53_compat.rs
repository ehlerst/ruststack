use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_route53_hosted_zone_and_dns_records_compat() {
    let client = RustStackTestClient::new();

    // 1. CreateHostedZone
    let req = Request::builder()
        .method(Method::POST)
        .uri("/2013-04-01/hostedzone")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "Name": "app.internal.",
                "CallerReference": "compat-ref-456",
                "HostedZoneConfig": {
                    "Comment": "Internal DNS",
                    "PrivateZone": true
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = client.send_request(req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let zone_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let zone_id = zone_json["HostedZone"]["Id"].as_str().unwrap();

    // 2. ChangeResourceRecordSets (UPSERT A Record)
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/2013-04-01{}/rrset", zone_id))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "ChangeBatch": {
                    "Comment": "Add app DNS",
                    "Changes": [
                        {
                            "Action": "UPSERT",
                            "ResourceRecordSet": {
                                "Name": "web.app.internal.",
                                "Type": "A",
                                "TTL": 60,
                                "ResourceRecords": [
                                    {"Value": "10.0.0.5"}
                                ]
                            }
                        }
                    ]
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = client.send_request(req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. ListResourceRecordSets
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/2013-04-01{}/rrset", zone_id))
        .body(Body::empty())
        .unwrap();

    let resp = client.send_request(req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let rr_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let sets = rr_json["ResourceRecordSets"].as_array().unwrap();
    assert!(sets.iter().any(|s| s["Name"].as_str() == Some("web.app.internal.")));
}

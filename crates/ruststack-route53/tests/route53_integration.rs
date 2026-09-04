use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use bytes::Bytes;
use ruststack_route53::handle_route53_request;
use ruststack_route53::Route53State;
use ruststack_route53::types::*;

#[tokio::test]
async fn test_route53_hosted_zone_and_rrset_lifecycle() {
    let state = Route53State::new("000000000000", "us-east-1");

    // 1. Create Hosted Zone
    let zone_resp = state
        .create_hosted_zone(CreateHostedZoneRequest {
            name: "example.com".to_string(),
            caller_reference: "ref-123".to_string(),
            hosted_zone_config: Some(HostedZoneConfig {
                comment: Some("Test Zone".to_string()),
                private_zone: Some(false),
            }),
        })
        .unwrap();

    let zone_id = zone_resp.hosted_zone.id.clone();
    assert_eq!(zone_resp.hosted_zone.name, "example.com.");
    assert!(!zone_resp.delegation_set.name_servers.is_empty());

    // 2. Change Resource Record Sets (UPSERT A and CNAME)
    let change_resp = state
        .change_resource_record_sets(
            &zone_id,
            ChangeResourceRecordSetsRequest {
                change_batch: ChangeBatch {
                    comment: Some("Add A record".to_string()),
                    changes: vec![
                        Change {
                            action: "UPSERT".to_string(),
                            resource_record_set: ResourceRecordSet {
                                name: "api.example.com".to_string(),
                                record_type: "A".to_string(),
                                ttl: Some(300),
                                resource_records: Some(vec![ResourceRecord {
                                    value: "1.2.3.4".to_string(),
                                }]),
                                set_identifier: None,
                                weight: None,
                            },
                        },
                        Change {
                            action: "UPSERT".to_string(),
                            resource_record_set: ResourceRecordSet {
                                name: "www.example.com".to_string(),
                                record_type: "CNAME".to_string(),
                                ttl: Some(300),
                                resource_records: Some(vec![ResourceRecord {
                                    value: "example.com.".to_string(),
                                }]),
                                set_identifier: None,
                                weight: None,
                            },
                        },
                    ],
                },
            },
        )
        .unwrap();

    assert_eq!(change_resp.change_info.status, "INSYNC");

    // 3. List Resource Record Sets
    let list_rrsets = state.list_resource_record_sets(&zone_id).unwrap();
    assert_eq!(list_rrsets.resource_record_sets.len(), 3); // NS + A + CNAME

    // 4. Test HTTP Gateway Handler
    let uri: Uri = format!("/2013-04-01{}/rrset", zone_id).parse().unwrap();
    let resp = handle_route53_request(
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
    assert!(json_resp.get("ResourceRecordSets").is_some());
}

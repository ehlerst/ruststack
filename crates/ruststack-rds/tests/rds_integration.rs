use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_rds::{handle_rds_request, RdsState};

#[tokio::test]
async fn test_rds_instance_and_cluster_lifecycle() {
    let state = RdsState::new("000000000000".to_string(), "us-east-1".to_string());
    let uri: Uri = "/".parse().unwrap();
    let headers = HeaderMap::new();

    // 1. Create DB Instance
    let body = Bytes::from("Action=CreateDBInstance&DBInstanceIdentifier=test-postgres&Engine=postgres&DBInstanceClass=db.t3.micro&MasterUsername=root");
    let resp = handle_rds_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(xml.contains("<DBInstanceIdentifier>test-postgres</DBInstanceIdentifier>"));
    assert!(xml.contains("<Address>test-postgres.us-east-1.rds.localhost.localstack.cloud</Address>"));

    // 2. Describe DB Instances
    let body = Bytes::from("Action=DescribeDBInstances&DBInstanceIdentifier=test-postgres");
    let resp = handle_rds_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Create DB Cluster
    let body = Bytes::from("Action=CreateDBCluster&DBClusterIdentifier=aurora-pg-cluster&Engine=aurora-postgresql&MasterUsername=postgres");
    let resp = handle_rds_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(xml.contains("<DBClusterIdentifier>aurora-pg-cluster</DBClusterIdentifier>"));
}

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_redshift::{handle_redshift_request, RedshiftState};

#[tokio::test]
async fn test_redshift_cluster_and_snapshot_lifecycle() {
    let state = RedshiftState::new("000000000000".to_string(), "us-east-1".to_string());
    let uri: Uri = "/".parse().unwrap();
    let headers = HeaderMap::new();

    // 1. Create Cluster
    let body = Bytes::from("Action=CreateCluster&ClusterIdentifier=analytics-dw&NodeType=dc2.large&MasterUsername=admin&DBName=analytics&NumberOfNodes=2");
    let resp = handle_redshift_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(xml.contains("<ClusterIdentifier>analytics-dw</ClusterIdentifier>"));
    assert!(xml.contains("<Address>analytics-dw.us-east-1.redshift.localhost.localstack.cloud</Address>"));

    // 2. Describe Clusters
    let body = Bytes::from("Action=DescribeClusters&ClusterIdentifier=analytics-dw");
    let resp = handle_redshift_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Create Cluster Snapshot
    let body = Bytes::from("Action=CreateClusterSnapshot&SnapshotIdentifier=dw-backup-01&ClusterIdentifier=analytics-dw");
    let resp = handle_redshift_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(xml.contains("<SnapshotIdentifier>dw-backup-01</SnapshotIdentifier>"));
}

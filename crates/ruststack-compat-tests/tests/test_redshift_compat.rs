use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;

#[tokio::test]
async fn test_redshift_cluster_compat() {
    let client = RustStackTestClient::new();

    // 1. Create Cluster
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateCluster"),
                ("ClusterIdentifier", "compat-dw"),
                ("NodeType", "dc2.large"),
                ("MasterUsername", "admin"),
                ("DBName", "analytics"),
                ("NumberOfNodes", "2"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<ClusterIdentifier>compat-dw</ClusterIdentifier>"));

    // 2. Describe Clusters
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "DescribeClusters"),
                ("ClusterIdentifier", "compat-dw"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<ClusterIdentifier>compat-dw</ClusterIdentifier>"));

    // 3. Create Cluster Snapshot
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateClusterSnapshot"),
                ("SnapshotIdentifier", "compat-dw-snap"),
                ("ClusterIdentifier", "compat-dw"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<SnapshotIdentifier>compat-dw-snap</SnapshotIdentifier>"));
}

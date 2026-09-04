use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;

#[tokio::test]
async fn test_rds_instance_and_cluster_compat() {
    let client = RustStackTestClient::new();

    // 1. Create DB Instance
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateDBInstance"),
                ("DBInstanceIdentifier", "compat-postgres"),
                ("Engine", "postgres"),
                ("DBInstanceClass", "db.t3.micro"),
                ("MasterUsername", "root"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<DBInstanceIdentifier>compat-postgres</DBInstanceIdentifier>"));

    // 2. Describe DB Instances
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "DescribeDBInstances"),
                ("DBInstanceIdentifier", "compat-postgres"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<DBInstanceIdentifier>compat-postgres</DBInstanceIdentifier>"));

    // 3. Create DB Cluster
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateDBCluster"),
                ("DBClusterIdentifier", "compat-aurora-pg"),
                ("Engine", "aurora-postgresql"),
                ("MasterUsername", "postgres"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<DBClusterIdentifier>compat-aurora-pg</DBClusterIdentifier>"));
}

use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;

#[tokio::test]
async fn test_elasticache_cluster_and_replication_compat() {
    let client = RustStackTestClient::new();

    // 1. Create Cache Cluster
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateCacheCluster"),
                ("CacheClusterIdentifier", "compat-redis"),
                ("Engine", "redis"),
                ("CacheNodeType", "cache.t3.micro"),
                ("NumCacheNodes", "1"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<CacheClusterIdentifier>compat-redis</CacheClusterIdentifier>"));

    // 2. Describe Cache Clusters
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "DescribeCacheClusters"),
                ("CacheClusterIdentifier", "compat-redis"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<CacheClusterIdentifier>compat-redis</CacheClusterIdentifier>"));

    // 3. Create Replication Group
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateReplicationGroup"),
                ("ReplicationGroupId", "compat-redis-rg"),
                ("ReplicationGroupDescription", "Test Replication Group"),
                ("NumCacheClusters", "2"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<ReplicationGroupId>compat-redis-rg</ReplicationGroupId>"));
}

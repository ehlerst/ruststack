use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_elasticache::{handle_elasticache_request, ElastiCacheState};

#[tokio::test]
async fn test_elasticache_cluster_and_replication_group_lifecycle() {
    let state = ElastiCacheState::new("000000000000".to_string(), "us-east-1".to_string());
    let uri: Uri = "/".parse().unwrap();
    let headers = HeaderMap::new();

    // 1. Create Cache Cluster
    let body = Bytes::from("Action=CreateCacheCluster&CacheClusterIdentifier=test-redis-cluster&Engine=redis&CacheNodeType=cache.t3.micro&NumCacheNodes=1");
    let resp = handle_elasticache_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(xml.contains("<CacheClusterIdentifier>test-redis-cluster</CacheClusterIdentifier>"));

    // 2. Describe Cache Clusters
    let body = Bytes::from("Action=DescribeCacheClusters&CacheClusterIdentifier=test-redis-cluster");
    let resp = handle_elasticache_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Create Replication Group
    let body = Bytes::from("Action=CreateReplicationGroup&ReplicationGroupId=redis-ha-group&ReplicationGroupDescription=High+Availability+Redis&NumCacheClusters=2");
    let resp = handle_elasticache_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(xml.contains("<ReplicationGroupId>redis-ha-group</ReplicationGroupId>"));
}

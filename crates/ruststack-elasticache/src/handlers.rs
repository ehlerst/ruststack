use crate::state::ElastiCacheState;
use crate::types::*;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode, Uri};
use bytes::Bytes;
use std::collections::HashMap;
use uuid::Uuid;

const ELASTICACHE_XMLNS: &str = "http://elasticache.amazonaws.com/doc/2015-02-02/";

pub async fn handle_elasticache_request(
    State(state): State<ElastiCacheState>,
    uri: Uri,
    _headers: HeaderMap,
    body_bytes: Bytes,
) -> Response<Body> {
    let mut params = HashMap::new();

    if let Some(query) = uri.query() {
        for (k, v) in form_urlencoded::parse(query.as_bytes()) {
            params.insert(k.to_string(), v.to_string());
        }
    }

    if !body_bytes.is_empty() {
        for (k, v) in form_urlencoded::parse(&body_bytes) {
            params.insert(k.to_string(), v.to_string());
        }
    }

    let action = params.get("Action").cloned().unwrap_or_default();
    let request_id = Uuid::new_v4().to_string();

    match action.as_str() {
        "CreateCacheCluster" => {
            let identifier = params.get("CacheClusterIdentifier").cloned().unwrap_or_default();
            let cache_node_type = params.get("CacheNodeType").cloned();
            let engine = params.get("Engine").cloned();
            let engine_version = params.get("EngineVersion").cloned();
            let num_cache_nodes = params.get("NumCacheNodes").and_then(|s| s.parse::<i32>().ok());
            let replication_group_id = params.get("ReplicationGroupId").cloned();

            match state.create_cache_cluster(
                identifier,
                cache_node_type,
                engine,
                engine_version,
                num_cache_nodes,
                replication_group_id,
            ) {
                Ok(cl) => {
                    let xml = format!(
                        r#"<CreateCacheClusterResponse xmlns="{}">
  <CreateCacheClusterResult>
    {}
  </CreateCacheClusterResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</CreateCacheClusterResponse>"#,
                        ELASTICACHE_XMLNS,
                        render_cache_cluster(&cl),
                        request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_elasticache_error(&e.to_string(), StatusCode::BAD_REQUEST, &request_id),
            }
        }
        "DescribeCacheClusters" => {
            let identifier = params.get("CacheClusterIdentifier").map(|s| s.as_str());
            match state.describe_cache_clusters(identifier) {
                Ok(clusters) => {
                    let items_xml: String = clusters.iter().map(render_cache_cluster).collect();
                    let xml = format!(
                        r#"<DescribeCacheClustersResponse xmlns="{}">
  <DescribeCacheClustersResult>
    <CacheClusters>
      {}
    </CacheClusters>
  </DescribeCacheClustersResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</DescribeCacheClustersResponse>"#,
                        ELASTICACHE_XMLNS, items_xml, request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_elasticache_error(&e.to_string(), StatusCode::NOT_FOUND, &request_id),
            }
        }
        "DeleteCacheCluster" => {
            let identifier = params.get("CacheClusterIdentifier").cloned().unwrap_or_default();
            match state.delete_cache_cluster(&identifier) {
                Ok(cl) => {
                    let xml = format!(
                        r#"<DeleteCacheClusterResponse xmlns="{}">
  <DeleteCacheClusterResult>
    {}
  </DeleteCacheClusterResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</DeleteCacheClusterResponse>"#,
                        ELASTICACHE_XMLNS,
                        render_cache_cluster(&cl),
                        request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_elasticache_error(&e.to_string(), StatusCode::NOT_FOUND, &request_id),
            }
        }
        "CreateReplicationGroup" => {
            let identifier = params.get("ReplicationGroupId").cloned().unwrap_or_default();
            let description = params.get("ReplicationGroupDescription").cloned().unwrap_or_else(|| "Redis replication group".to_string());
            let num_clusters = params.get("NumCacheClusters").and_then(|s| s.parse::<i32>().ok());

            match state.create_replication_group(identifier, description, num_clusters) {
                Ok(rg) => {
                    let xml = format!(
                        r#"<CreateReplicationGroupResponse xmlns="{}">
  <CreateReplicationGroupResult>
    {}
  </CreateReplicationGroupResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</CreateReplicationGroupResponse>"#,
                        ELASTICACHE_XMLNS,
                        render_replication_group(&rg),
                        request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_elasticache_error(&e.to_string(), StatusCode::BAD_REQUEST, &request_id),
            }
        }
        "DescribeReplicationGroups" => {
            let identifier = params.get("ReplicationGroupId").map(|s| s.as_str());
            match state.describe_replication_groups(identifier) {
                Ok(rgs) => {
                    let items_xml: String = rgs.iter().map(render_replication_group).collect();
                    let xml = format!(
                        r#"<DescribeReplicationGroupsResponse xmlns="{}">
  <DescribeReplicationGroupsResult>
    <ReplicationGroups>
      {}
    </ReplicationGroups>
  </DescribeReplicationGroupsResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</DescribeReplicationGroupsResponse>"#,
                        ELASTICACHE_XMLNS, items_xml, request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_elasticache_error(&e.to_string(), StatusCode::NOT_FOUND, &request_id),
            }
        }
        _ => render_elasticache_error(&format!("Unknown Action: {}", action), StatusCode::BAD_REQUEST, &request_id),
    }
}

fn render_cache_cluster(cl: &CacheCluster) -> String {
    let nodes_xml: String = cl
        .cache_nodes
        .iter()
        .map(|node| {
            let ep_xml = if let Some(ref ep) = node.endpoint {
                format!("<Endpoint><Address>{}</Address><Port>{}</Port></Endpoint>", ep.address, ep.port)
            } else {
                "".to_string()
            };
            format!(
                "<CacheNode><CacheNodeId>{}</CacheNodeId><CacheNodeStatus>{}</CacheNodeStatus>{}</CacheNode>",
                node.cache_node_id, node.cache_node_status, ep_xml
            )
        })
        .collect();

    format!(
        r#"<CacheCluster>
  <CacheClusterIdentifier>{}</CacheClusterIdentifier>
  <CacheNodeType>{}</CacheNodeType>
  <Engine>{}</Engine>
  <EngineVersion>{}</EngineVersion>
  <CacheClusterStatus>{}</CacheClusterStatus>
  <NumCacheNodes>{}</NumCacheNodes>
  <PreferredAvailabilityZone>{}</PreferredAvailabilityZone>
  <CacheNodes>
    {}
  </CacheNodes>
</CacheCluster>"#,
        cl.cache_cluster_identifier,
        cl.cache_node_type,
        cl.engine,
        cl.engine_version,
        cl.cache_cluster_status,
        cl.num_cache_nodes,
        cl.preferred_availability_zone,
        nodes_xml
    )
}

fn render_replication_group(rg: &ReplicationGroup) -> String {
    let ep_xml = if let Some(ref ep) = rg.primary_endpoint {
        format!("<PrimaryEndpoint><Address>{}</Address><Port>{}</Port></PrimaryEndpoint>", ep.address, ep.port)
    } else {
        "".to_string()
    };

    let member_xml: String = rg.member_clusters.iter().map(|m| format!("<member>{}</member>", m)).collect();

    format!(
        r#"<ReplicationGroup>
  <ReplicationGroupId>{}</ReplicationGroupId>
  <Description>{}</Description>
  <Status>{}</Status>
  {}
  <MemberClusters>
    {}
  </MemberClusters>
  <MultiAZ>{}</MultiAZ>
  <AutomaticFailover>{}</AutomaticFailover>
</ReplicationGroup>"#,
        rg.replication_group_id,
        rg.description,
        rg.status,
        ep_xml,
        member_xml,
        rg.multi_az,
        rg.automatic_failover
    )
}

fn render_elasticache_error(msg: &str, status: StatusCode, request_id: &str) -> Response<Body> {
    let xml = format!(
        r#"<ErrorResponse xmlns="{}">
  <Error>
    <Type>Sender</Type>
    <Code>InvalidParameterValue</Code>
    <Message>{}</Message>
  </Error>
  <RequestId>{}</RequestId>
</ErrorResponse>"#,
        ELASTICACHE_XMLNS, msg, request_id
    );
    Response::builder()
        .status(status)
        .header("content-type", "text/xml")
        .body(Body::from(xml))
        .unwrap()
}

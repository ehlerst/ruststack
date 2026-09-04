use crate::state::RedshiftState;
use crate::types::*;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode, Uri};
use bytes::Bytes;
use std::collections::HashMap;
use uuid::Uuid;

const REDSHIFT_XMLNS: &str = "http://redshift.amazonaws.com/doc/2012-12-01/";

pub async fn handle_redshift_request(
    State(state): State<RedshiftState>,
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
        "CreateCluster" => {
            let identifier = params.get("ClusterIdentifier").cloned().unwrap_or_default();
            let node_type = params.get("NodeType").cloned();
            let master_username = params.get("MasterUsername").cloned();
            let db_name = params.get("DBName").cloned();
            let number_of_nodes = params.get("NumberOfNodes").and_then(|s| s.parse::<i32>().ok());
            let encrypted = params.get("Encrypted").map(|s| s == "true");

            match state.create_cluster(
                identifier,
                node_type,
                master_username,
                db_name,
                number_of_nodes,
                encrypted,
            ) {
                Ok(cl) => {
                    let xml = format!(
                        r#"<CreateClusterResponse xmlns="{}">
  <CreateClusterResult>
    {}
  </CreateClusterResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</CreateClusterResponse>"#,
                        REDSHIFT_XMLNS,
                        render_cluster(&cl),
                        request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_redshift_error(&e.to_string(), StatusCode::BAD_REQUEST, &request_id),
            }
        }
        "DescribeClusters" => {
            let identifier = params.get("ClusterIdentifier").map(|s| s.as_str());
            match state.describe_clusters(identifier) {
                Ok(clusters) => {
                    let items_xml: String = clusters.iter().map(render_cluster).collect();
                    let xml = format!(
                        r#"<DescribeClustersResponse xmlns="{}">
  <DescribeClustersResult>
    <Clusters>
      {}
    </Clusters>
  </DescribeClustersResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</DescribeClustersResponse>"#,
                        REDSHIFT_XMLNS, items_xml, request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_redshift_error(&e.to_string(), StatusCode::NOT_FOUND, &request_id),
            }
        }
        "DeleteCluster" => {
            let identifier = params.get("ClusterIdentifier").cloned().unwrap_or_default();
            match state.delete_cluster(&identifier) {
                Ok(cl) => {
                    let xml = format!(
                        r#"<DeleteClusterResponse xmlns="{}">
  <DeleteClusterResult>
    {}
  </DeleteClusterResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</DeleteClusterResponse>"#,
                        REDSHIFT_XMLNS,
                        render_cluster(&cl),
                        request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_redshift_error(&e.to_string(), StatusCode::NOT_FOUND, &request_id),
            }
        }
        "CreateClusterSnapshot" => {
            let snapshot_id = params.get("SnapshotIdentifier").cloned().unwrap_or_default();
            let cluster_id = params.get("ClusterIdentifier").cloned().unwrap_or_default();
            match state.create_cluster_snapshot(snapshot_id, cluster_id) {
                Ok(sn) => {
                    let xml = format!(
                        r#"<CreateClusterSnapshotResponse xmlns="{}">
  <CreateClusterSnapshotResult>
    <Snapshot>
      <SnapshotIdentifier>{}</SnapshotIdentifier>
      <ClusterIdentifier>{}</ClusterIdentifier>
      <Status>{}</Status>
      <NodeType>{}</NodeType>
      <NumberOfNodes>{}</NumberOfNodes>
      <DBName>{}</DBName>
      <MasterUsername>{}</MasterUsername>
      <Encrypted>{}</Encrypted>
    </Snapshot>
  </CreateClusterSnapshotResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</CreateClusterSnapshotResponse>"#,
                        REDSHIFT_XMLNS, sn.snapshot_identifier, sn.cluster_identifier, sn.status, sn.node_type, sn.number_of_nodes, sn.db_name, sn.master_username, sn.encrypted, request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_redshift_error(&e.to_string(), StatusCode::BAD_REQUEST, &request_id),
            }
        }
        "DescribeClusterSnapshots" => {
            let snapshot_id = params.get("SnapshotIdentifier").map(|s| s.as_str());
            match state.describe_cluster_snapshots(snapshot_id) {
                Ok(snapshots) => {
                    let items_xml: String = snapshots.iter().map(|sn| format!(
                        r#"<Snapshot>
  <SnapshotIdentifier>{}</SnapshotIdentifier>
  <ClusterIdentifier>{}</ClusterIdentifier>
  <Status>{}</Status>
  <NodeType>{}</NodeType>
  <NumberOfNodes>{}</NumberOfNodes>
  <DBName>{}</DBName>
  <MasterUsername>{}</MasterUsername>
  <Encrypted>{}</Encrypted>
</Snapshot>"#,
                        sn.snapshot_identifier, sn.cluster_identifier, sn.status, sn.node_type, sn.number_of_nodes, sn.db_name, sn.master_username, sn.encrypted
                    )).collect();

                    let xml = format!(
                        r#"<DescribeClusterSnapshotsResponse xmlns="{}">
  <DescribeClusterSnapshotsResult>
    <Snapshots>
      {}
    </Snapshots>
  </DescribeClusterSnapshotsResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</DescribeClusterSnapshotsResponse>"#,
                        REDSHIFT_XMLNS, items_xml, request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_redshift_error(&e.to_string(), StatusCode::NOT_FOUND, &request_id),
            }
        }
        _ => render_redshift_error(&format!("Unknown Action: {}", action), StatusCode::BAD_REQUEST, &request_id),
    }
}

fn render_cluster(cl: &RedshiftCluster) -> String {
    let ep_xml = if let Some(ref ep) = cl.endpoint {
        format!("<Endpoint><Address>{}</Address><Port>{}</Port></Endpoint>", ep.address, ep.port)
    } else {
        "".to_string()
    };

    let nodes_xml: String = cl
        .cluster_nodes
        .iter()
        .map(|node| {
            format!(
                "<ClusterNode><NodeRole>{}</NodeRole><PrivateIPAddress>{}</PrivateIPAddress><PublicIPAddress>{}</PublicIPAddress></ClusterNode>",
                node.node_role, node.private_ip_address, node.public_ip_address
            )
        })
        .collect();

    format!(
        r#"<Cluster>
  <ClusterIdentifier>{}</ClusterIdentifier>
  <NodeType>{}</NodeType>
  <ClusterStatus>{}</ClusterStatus>
  <ClusterAvailabilityStatus>{}</ClusterAvailabilityStatus>
  <MasterUsername>{}</MasterUsername>
  <DBName>{}</DBName>
  {}
  <NumberOfNodes>{}</NumberOfNodes>
  <ClusterNodes>
    {}
  </ClusterNodes>
  <Encrypted>{}</Encrypted>
</Cluster>"#,
        cl.cluster_identifier,
        cl.node_type,
        cl.cluster_status,
        cl.cluster_availability_status,
        cl.master_username,
        cl.db_name,
        ep_xml,
        cl.number_of_nodes,
        nodes_xml,
        cl.encrypted
    )
}

fn render_redshift_error(msg: &str, status: StatusCode, request_id: &str) -> Response<Body> {
    let xml = format!(
        r#"<ErrorResponse xmlns="{}">
  <Error>
    <Type>Sender</Type>
    <Code>InvalidParameterValue</Code>
    <Message>{}</Message>
  </Error>
  <RequestId>{}</RequestId>
</ErrorResponse>"#,
        REDSHIFT_XMLNS, msg, request_id
    );
    Response::builder()
        .status(status)
        .header("content-type", "text/xml")
        .body(Body::from(xml))
        .unwrap()
}

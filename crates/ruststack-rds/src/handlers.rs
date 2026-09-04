use crate::state::RdsState;
use crate::types::*;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode, Uri};
use bytes::Bytes;
use std::collections::HashMap;
use uuid::Uuid;

const RDS_XMLNS: &str = "http://rds.amazonaws.com/doc/2014-10-31/";

pub async fn handle_rds_request(
    State(state): State<RdsState>,
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
        "CreateDBInstance" => {
            let identifier = params.get("DBInstanceIdentifier").cloned().unwrap_or_default();
            let db_instance_class = params.get("DBInstanceClass").cloned().unwrap_or_else(|| "db.t3.micro".to_string());
            let engine = params.get("Engine").cloned().unwrap_or_else(|| "postgres".to_string());
            let engine_version = params.get("EngineVersion").cloned();
            let master_username = params.get("MasterUsername").cloned().unwrap_or_else(|| "postgres".to_string());
            let db_name = params.get("DBName").cloned();
            let allocated_storage = params.get("AllocatedStorage").and_then(|s| s.parse::<i32>().ok());
            let db_cluster_identifier = params.get("DBClusterIdentifier").cloned();

            match state.create_db_instance(
                identifier,
                db_instance_class,
                engine,
                engine_version,
                master_username,
                db_name,
                allocated_storage,
                db_cluster_identifier,
            ) {
                Ok(inst) => {
                    let xml = format!(
                        r#"<CreateDBInstanceResponse xmlns="{}">
  <CreateDBInstanceResult>
    {}
  </CreateDBInstanceResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</CreateDBInstanceResponse>"#,
                        RDS_XMLNS,
                        render_db_instance(&inst),
                        request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_rds_error(&e.to_string(), StatusCode::BAD_REQUEST, &request_id),
            }
        }
        "DescribeDBInstances" => {
            let identifier = params.get("DBInstanceIdentifier").map(|s| s.as_str());
            match state.describe_db_instances(identifier) {
                Ok(instances) => {
                    let items_xml: String = instances.iter().map(render_db_instance).collect();
                    let xml = format!(
                        r#"<DescribeDBInstancesResponse xmlns="{}">
  <DescribeDBInstancesResult>
    <DBInstances>
      {}
    </DBInstances>
  </DescribeDBInstancesResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</DescribeDBInstancesResponse>"#,
                        RDS_XMLNS, items_xml, request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_rds_error(&e.to_string(), StatusCode::NOT_FOUND, &request_id),
            }
        }
        "DeleteDBInstance" => {
            let identifier = params.get("DBInstanceIdentifier").cloned().unwrap_or_default();
            match state.delete_db_instance(&identifier) {
                Ok(inst) => {
                    let xml = format!(
                        r#"<DeleteDBInstanceResponse xmlns="{}">
  <DeleteDBInstanceResult>
    {}
  </DeleteDBInstanceResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</DeleteDBInstanceResponse>"#,
                        RDS_XMLNS,
                        render_db_instance(&inst),
                        request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_rds_error(&e.to_string(), StatusCode::NOT_FOUND, &request_id),
            }
        }
        "CreateDBCluster" => {
            let identifier = params.get("DBClusterIdentifier").cloned().unwrap_or_default();
            let engine = params.get("Engine").cloned().unwrap_or_else(|| "aurora-postgresql".to_string());
            let engine_version = params.get("EngineVersion").cloned();
            let master_username = params.get("MasterUsername").cloned().unwrap_or_else(|| "postgres".to_string());
            let database_name = params.get("DatabaseName").cloned();

            match state.create_db_cluster(identifier, engine, engine_version, master_username, database_name) {
                Ok(cl) => {
                    let xml = format!(
                        r#"<CreateDBClusterResponse xmlns="{}">
  <CreateDBClusterResult>
    {}
  </CreateDBClusterResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</CreateDBClusterResponse>"#,
                        RDS_XMLNS,
                        render_db_cluster(&cl),
                        request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_rds_error(&e.to_string(), StatusCode::BAD_REQUEST, &request_id),
            }
        }
        "DescribeDBClusters" => {
            let identifier = params.get("DBClusterIdentifier").map(|s| s.as_str());
            match state.describe_db_clusters(identifier) {
                Ok(clusters) => {
                    let items_xml: String = clusters.iter().map(render_db_cluster).collect();
                    let xml = format!(
                        r#"<DescribeDBClustersResponse xmlns="{}">
  <DescribeDBClustersResult>
    <DBClusters>
      {}
    </DBClusters>
  </DescribeDBClustersResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</DescribeDBClustersResponse>"#,
                        RDS_XMLNS, items_xml, request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_rds_error(&e.to_string(), StatusCode::NOT_FOUND, &request_id),
            }
        }
        "CreateDBSnapshot" => {
            let snapshot_id = params.get("DBSnapshotIdentifier").cloned().unwrap_or_default();
            let instance_id = params.get("DBInstanceIdentifier").cloned().unwrap_or_default();
            match state.create_db_snapshot(snapshot_id, instance_id) {
                Ok(sn) => {
                    let xml = format!(
                        r#"<CreateDBSnapshotResponse xmlns="{}">
  <CreateDBSnapshotResult>
    <DBSnapshot>
      <DBSnapshotIdentifier>{}</DBSnapshotIdentifier>
      <DBInstanceIdentifier>{}</DBInstanceIdentifier>
      <Engine>{}</Engine>
      <AllocatedStorage>{}</AllocatedStorage>
      <Status>{}</Status>
      <MasterUsername>{}</MasterUsername>
    </DBSnapshot>
  </CreateDBSnapshotResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</CreateDBSnapshotResponse>"#,
                        RDS_XMLNS, sn.db_snapshot_identifier, sn.db_instance_identifier, sn.engine, sn.allocated_storage, sn.status, sn.master_username, request_id
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/xml")
                        .body(Body::from(xml))
                        .unwrap()
                }
                Err(e) => render_rds_error(&e.to_string(), StatusCode::BAD_REQUEST, &request_id),
            }
        }
        _ => render_rds_error(&format!("Unknown Action: {}", action), StatusCode::BAD_REQUEST, &request_id),
    }
}

fn render_db_instance(inst: &DBInstance) -> String {
    let endpoint_xml = if let Some(ref ep) = inst.endpoint {
        format!(
            "<Endpoint><Address>{}</Address><Port>{}</Port><HostedZoneId>{}</HostedZoneId></Endpoint>",
            ep.address, ep.port, ep.hosted_zone_id
        )
    } else {
        "".to_string()
    };

    format!(
        r#"<DBInstance>
  <DBInstanceIdentifier>{}</DBInstanceIdentifier>
  <DBInstanceClass>{}</DBInstanceClass>
  <Engine>{}</Engine>
  <EngineVersion>{}</EngineVersion>
  <DBInstanceStatus>{}</DBInstanceStatus>
  <MasterUsername>{}</MasterUsername>
  {}
  <AllocatedStorage>{}</AllocatedStorage>
  <MultiAZ>{}</MultiAZ>
  <StorageType>{}</StorageType>
</DBInstance>"#,
        inst.db_instance_identifier,
        inst.db_instance_class,
        inst.engine,
        inst.engine_version,
        inst.db_instance_status,
        inst.master_username,
        endpoint_xml,
        inst.allocated_storage,
        inst.multi_az,
        inst.storage_type
    )
}

fn render_db_cluster(cl: &DBCluster) -> String {
    format!(
        r#"<DBCluster>
  <DBClusterIdentifier>{}</DBClusterIdentifier>
  <Engine>{}</Engine>
  <EngineVersion>{}</EngineVersion>
  <Status>{}</Status>
  <MasterUsername>{}</MasterUsername>
  <Endpoint>{}</Endpoint>
  <ReaderEndpoint>{}</ReaderEndpoint>
  <Port>{}</Port>
  <MultiAZ>{}</MultiAZ>
</DBCluster>"#,
        cl.db_cluster_identifier,
        cl.engine,
        cl.engine_version,
        cl.status,
        cl.master_username,
        cl.endpoint.as_deref().unwrap_or(""),
        cl.reader_endpoint.as_deref().unwrap_or(""),
        cl.port,
        cl.multi_az
    )
}

fn render_rds_error(msg: &str, status: StatusCode, request_id: &str) -> Response<Body> {
    let xml = format!(
        r#"<ErrorResponse xmlns="{}">
  <Error>
    <Type>Sender</Type>
    <Code>InvalidParameterValue</Code>
    <Message>{}</Message>
  </Error>
  <RequestId>{}</RequestId>
</ErrorResponse>"#,
        RDS_XMLNS, msg, request_id
    );
    Response::builder()
        .status(status)
        .header("content-type", "text/xml")
        .body(Body::from(xml))
        .unwrap()
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HostedZoneConfig {
    pub comment: Option<String>,
    pub private_zone: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HostedZone {
    pub id: String,
    pub name: String,
    pub caller_reference: String,
    pub config: Option<HostedZoneConfig>,
    pub resource_record_set_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DelegationSet {
    pub name_servers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChangeInfo {
    pub id: String,
    pub status: String, // "PENDING" | "INSYNC"
    pub submitted_at: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ResourceRecord {
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ResourceRecordSet {
    pub name: String,
    #[serde(rename = "Type")]
    pub record_type: String,
    #[serde(rename = "TTL")]
    pub ttl: Option<i64>,
    pub resource_records: Option<Vec<ResourceRecord>>,
    pub set_identifier: Option<String>,
    pub weight: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Change {
    pub action: String, // "CREATE" | "DELETE" | "UPSERT"
    pub resource_record_set: ResourceRecordSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChangeBatch {
    pub comment: Option<String>,
    pub changes: Vec<Change>,
}

// ----------------------------------------------------------------------------
// Requests / Responses
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateHostedZoneRequest {
    pub name: String,
    pub caller_reference: String,
    pub hosted_zone_config: Option<HostedZoneConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateHostedZoneResponse {
    pub hosted_zone: HostedZone,
    pub change_info: ChangeInfo,
    pub delegation_set: DelegationSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetHostedZoneResponse {
    pub hosted_zone: HostedZone,
    pub delegation_set: Option<DelegationSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ListHostedZonesResponse {
    pub hosted_zones: Vec<HostedZone>,
    pub marker: Option<String>,
    pub is_truncated: bool,
    pub next_marker: Option<String>,
    pub max_items: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChangeResourceRecordSetsRequest {
    pub change_batch: ChangeBatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChangeResourceRecordSetsResponse {
    pub change_info: ChangeInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ListResourceRecordSetsResponse {
    pub resource_record_sets: Vec<ResourceRecordSet>,
    pub is_truncated: bool,
    pub max_items: Option<i32>,
}

// ----------------------------------------------------------------------------
// Storage & Snapshot
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredHostedZone {
    pub id: String,
    pub name: String,
    pub caller_reference: String,
    pub comment: Option<String>,
    pub private_zone: bool,
    pub record_sets: HashMap<(String, String), ResourceRecordSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Route53StateSnapshot {
    pub hosted_zones: HashMap<String, StoredHostedZone>,
}

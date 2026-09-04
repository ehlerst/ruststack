use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterConfig {
    pub instance_type: Option<String>,
    pub instance_count: Option<i32>,
    pub dedicated_master_enabled: Option<bool>,
    pub zone_awareness_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EBSOptions {
    pub ebs_enabled: bool,
    pub volume_type: Option<String>,
    pub volume_size: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeToNodeEncryptionOptions {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainStatus {
    pub domain_id: String,
    pub domain_name: String,
    pub arn: String,
    pub created: bool,
    pub deleted: bool,
    pub endpoint: Option<String>,
    pub engine_version: String,
    pub cluster_config: Option<ClusterConfig>,
    pub ebs_options: Option<EBSOptions>,
    pub node_to_node_encryption_options: Option<NodeToNodeEncryptionOptions>,
    pub processing: bool,
    pub upgrade_processing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainInfo {
    pub domain_name: String,
    pub engine_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDomainRequest {
    #[serde(alias = "DomainName")]
    pub domain_name: String,
    #[serde(alias = "EngineVersion")]
    pub engine_version: Option<String>,
    #[serde(alias = "ClusterConfig")]
    pub cluster_config: Option<ClusterConfig>,
    #[serde(alias = "EBSOptions")]
    pub ebs_options: Option<EBSOptions>,
    #[serde(alias = "NodeToNodeEncryptionOptions")]
    pub node_to_node_encryption_options: Option<NodeToNodeEncryptionOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDomainResponse {
    pub domain_status: DomainStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescribeDomainResponse {
    pub domain_status: DomainStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDomainNamesResponse {
    pub domain_names: Vec<DomainInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteDomainResponse {
    pub domain_status: DomainStatus,
}

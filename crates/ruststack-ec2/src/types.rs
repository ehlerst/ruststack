use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vpc {
    pub vpc_id: String,
    pub cidr_block: String,
    pub is_default: bool,
    pub state: String,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subnet {
    pub subnet_id: String,
    pub vpc_id: String,
    pub cidr_block: String,
    pub availability_zone: String,
    pub default_for_az: bool,
    pub state: String,
    pub available_ip_address_count: i32,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpRange {
    pub cidr_ip: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpPermission {
    pub ip_protocol: String,
    pub from_port: Option<i32>,
    pub to_port: Option<i32>,
    pub ip_ranges: Vec<IpRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityGroup {
    pub group_id: String,
    pub group_name: String,
    pub description: String,
    pub vpc_id: String,
    pub ip_permissions: Vec<IpPermission>,
    pub ip_permissions_egress: Vec<IpPermission>,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    pub key_name: String,
    pub key_pair_id: String,
    pub key_fingerprint: String,
    pub key_material: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub instance_id: String,
    pub image_id: String,
    pub instance_type: String,
    pub key_name: Option<String>,
    pub subnet_id: Option<String>,
    pub vpc_id: Option<String>,
    pub private_ip_address: String,
    pub public_ip_address: Option<String>,
    pub state: String, // "running", "stopped", "terminated"
    pub state_code: i32,
    pub launch_time: String,
    pub security_groups: Vec<String>, // group IDs
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Ec2StateSnapshot {
    pub vpcs: Vec<Vpc>,
    pub subnets: Vec<Subnet>,
    pub security_groups: Vec<SecurityGroup>,
    pub key_pairs: Vec<KeyPair>,
    pub instances: Vec<Instance>,
}

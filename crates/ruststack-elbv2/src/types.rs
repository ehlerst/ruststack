use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancer {
    pub load_balancer_arn: String,
    pub dns_name: String,
    pub load_balancer_name: String,
    pub scheme: String, // "internet-facing" or "internal"
    pub vpc_id: Option<String>,
    pub state: String,  // "active", "provisioning", "failed"
    pub lb_type: String, // "application" or "network"
    pub availability_zones: Vec<String>,
    pub security_groups: Vec<String>,
    pub created_time: String,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetDescription {
    pub id: String, // instance ID or IP
    pub port: Option<i32>,
    pub availability_zone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetHealthDescription {
    pub target: TargetDescription,
    pub target_health: String, // "healthy", "unhealthy", "unused", "initial"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetGroup {
    pub target_group_arn: String,
    pub target_group_name: String,
    pub protocol: Option<String>,
    pub port: Option<i32>,
    pub vpc_id: Option<String>,
    pub health_check_protocol: Option<String>,
    pub health_check_port: Option<String>,
    pub health_check_path: Option<String>,
    pub target_type: String, // "instance", "ip", "lambda"
    pub load_balancer_arns: Vec<String>,
    pub targets: Vec<TargetDescription>,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenerAction {
    pub action_type: String, // "forward", "fixed-response", "redirect"
    pub target_group_arn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listener {
    pub listener_arn: String,
    pub load_balancer_arn: String,
    pub port: i32,
    pub protocol: String,
    pub default_actions: Vec<ListenerAction>,
    pub ssl_policy: Option<String>,
    pub certificates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Elbv2StateSnapshot {
    pub load_balancers: Vec<LoadBalancer>,
    pub target_groups: Vec<TargetGroup>,
    pub listeners: Vec<Listener>,
}

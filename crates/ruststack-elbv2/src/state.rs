use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

use crate::types::*;

#[derive(Debug, Error)]
pub enum Elbv2Error {
    #[error("LoadBalancerNotFoundException: The specified load balancer does not exist.")]
    LoadBalancerNotFound(String),
    #[error("TargetGroupNotFoundException: The specified target group does not exist.")]
    TargetGroupNotFound(String),
    #[error("ListenerNotFoundException: The specified listener does not exist.")]
    ListenerNotFound(String),
    #[error("DuplicateLoadBalancerNameException: A load balancer with the specified name already exists.")]
    DuplicateLoadBalancerName(String),
    #[error("DuplicateTargetGroupNameException: A target group with the specified name already exists.")]
    DuplicateTargetGroupName(String),
    #[error("InvalidConfigurationRequest: {0}")]
    InvalidConfiguration(String),
}

#[derive(Clone)]
pub struct Elbv2State {
    account_id: String,
    region: String,
    load_balancers: Arc<DashMap<String, LoadBalancer>>, // ARN -> LoadBalancer
    target_groups: Arc<DashMap<String, TargetGroup>>,   // ARN -> TargetGroup
    listeners: Arc<DashMap<String, Listener>>,           // ARN -> Listener
}

impl Elbv2State {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            load_balancers: Arc::new(DashMap::new()),
            target_groups: Arc::new(DashMap::new()),
            listeners: Arc::new(DashMap::new()),
        }
    }

    pub fn reset(&self) {
        self.load_balancers.clear();
        self.target_groups.clear();
        self.listeners.clear();
    }

    pub fn export_snapshot(&self) -> Elbv2StateSnapshot {
        Elbv2StateSnapshot {
            load_balancers: self.load_balancers.iter().map(|kv| kv.value().clone()).collect(),
            target_groups: self.target_groups.iter().map(|kv| kv.value().clone()).collect(),
            listeners: self.listeners.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: Elbv2StateSnapshot) {
        self.reset();
        for lb in snapshot.load_balancers {
            self.load_balancers.insert(lb.load_balancer_arn.clone(), lb);
        }
        for tg in snapshot.target_groups {
            self.target_groups.insert(tg.target_group_arn.clone(), tg);
        }
        for l in snapshot.listeners {
            self.listeners.insert(l.listener_arn.clone(), l);
        }
    }

    // Load Balancer APIs
    pub fn create_load_balancer(
        &self,
        name: String,
        subnets: Vec<String>,
        security_groups: Vec<String>,
        scheme: Option<String>,
        lb_type: Option<String>,
        tags: Vec<Tag>,
    ) -> Result<LoadBalancer, Elbv2Error> {
        for item in self.load_balancers.iter() {
            if item.value().load_balancer_name == name {
                return Err(Elbv2Error::DuplicateLoadBalancerName(name));
            }
        }

        let lb_type_val = lb_type.unwrap_or_else(|| "application".to_string());
        let short_type = if lb_type_val == "network" { "net" } else { "app" };
        let hex_id = &Uuid::new_v4().to_string().replace('-', "")[..16];
        let arn = format!(
            "arn:aws:elasticloadbalancing:{}:{}:loadbalancer/{}/{}/{}",
            self.region, self.account_id, short_type, name, hex_id
        );
        let dns_name = format!("{}-{}.{}.elb.localhost.localstack.cloud", name, hex_id, self.region);

        let lb = LoadBalancer {
            load_balancer_arn: arn.clone(),
            dns_name,
            load_balancer_name: name,
            scheme: scheme.unwrap_or_else(|| "internet-facing".to_string()),
            vpc_id: Some("vpc-default00000001".to_string()),
            state: "active".to_string(),
            lb_type: lb_type_val,
            availability_zones: subnets,
            security_groups,
            created_time: Utc::now().to_rfc3339(),
            tags,
        };

        self.load_balancers.insert(arn, lb.clone());
        Ok(lb)
    }

    pub fn describe_load_balancers(
        &self,
        names: Option<Vec<String>>,
        arns: Option<Vec<String>>,
    ) -> Vec<LoadBalancer> {
        let mut results = Vec::new();
        for item in self.load_balancers.iter() {
            let lb = item.value();
            if let Some(ref n_list) = names {
                if !n_list.is_empty() && !n_list.contains(&lb.load_balancer_name) {
                    continue;
                }
            }
            if let Some(ref a_list) = arns {
                if !a_list.is_empty() && !a_list.contains(&lb.load_balancer_arn) {
                    continue;
                }
            }
            results.push(lb.clone());
        }
        results
    }

    pub fn delete_load_balancer(&self, lb_arn: &str) -> Result<(), Elbv2Error> {
        self.load_balancers
            .remove(lb_arn)
            .ok_or_else(|| Elbv2Error::LoadBalancerNotFound(lb_arn.to_string()))?;
        Ok(())
    }

    // Target Group APIs
    pub fn create_target_group(
        &self,
        name: String,
        protocol: Option<String>,
        port: Option<i32>,
        vpc_id: Option<String>,
        target_type: Option<String>,
        tags: Vec<Tag>,
    ) -> Result<TargetGroup, Elbv2Error> {
        for item in self.target_groups.iter() {
            if item.value().target_group_name == name {
                return Err(Elbv2Error::DuplicateTargetGroupName(name));
            }
        }

        let hex_id = &Uuid::new_v4().to_string().replace('-', "")[..16];
        let arn = format!(
            "arn:aws:elasticloadbalancing:{}:{}:targetgroup/{}/{}",
            self.region, self.account_id, name, hex_id
        );

        let tg = TargetGroup {
            target_group_arn: arn.clone(),
            target_group_name: name,
            protocol: protocol.or_else(|| Some("HTTP".to_string())),
            port: port.or(Some(80)),
            vpc_id: vpc_id.or_else(|| Some("vpc-default00000001".to_string())),
            health_check_protocol: Some("HTTP".to_string()),
            health_check_port: Some("traffic-port".to_string()),
            health_check_path: Some("/".to_string()),
            target_type: target_type.unwrap_or_else(|| "instance".to_string()),
            load_balancer_arns: vec![],
            targets: vec![],
            tags,
        };

        self.target_groups.insert(arn, tg.clone());
        Ok(tg)
    }

    pub fn describe_target_groups(
        &self,
        names: Option<Vec<String>>,
        arns: Option<Vec<String>>,
        lb_arn: Option<String>,
    ) -> Vec<TargetGroup> {
        let mut results = Vec::new();
        for item in self.target_groups.iter() {
            let tg = item.value();
            if let Some(ref n_list) = names {
                if !n_list.is_empty() && !n_list.contains(&tg.target_group_name) {
                    continue;
                }
            }
            if let Some(ref a_list) = arns {
                if !a_list.is_empty() && !a_list.contains(&tg.target_group_arn) {
                    continue;
                }
            }
            if let Some(ref l_arn) = lb_arn {
                if !tg.load_balancer_arns.contains(l_arn) {
                    continue;
                }
            }
            results.push(tg.clone());
        }
        results
    }

    pub fn delete_target_group(&self, tg_arn: &str) -> Result<(), Elbv2Error> {
        self.target_groups
            .remove(tg_arn)
            .ok_or_else(|| Elbv2Error::TargetGroupNotFound(tg_arn.to_string()))?;
        Ok(())
    }

    pub fn register_targets(&self, tg_arn: &str, targets: Vec<TargetDescription>) -> Result<(), Elbv2Error> {
        let mut tg = self.target_groups.get_mut(tg_arn).ok_or_else(|| Elbv2Error::TargetGroupNotFound(tg_arn.to_string()))?;
        for t in targets {
            if !tg.targets.iter().any(|existing| existing.id == t.id) {
                tg.targets.push(t);
            }
        }
        Ok(())
    }

    pub fn deregister_targets(&self, tg_arn: &str, targets: Vec<TargetDescription>) -> Result<(), Elbv2Error> {
        let mut tg = self.target_groups.get_mut(tg_arn).ok_or_else(|| Elbv2Error::TargetGroupNotFound(tg_arn.to_string()))?;
        let remove_ids: Vec<String> = targets.into_iter().map(|t| t.id).collect();
        tg.targets.retain(|t| !remove_ids.contains(&t.id));
        Ok(())
    }

    pub fn describe_target_health(&self, tg_arn: &str) -> Result<Vec<TargetHealthDescription>, Elbv2Error> {
        let tg = self.target_groups.get(tg_arn).ok_or_else(|| Elbv2Error::TargetGroupNotFound(tg_arn.to_string()))?;
        let descs = tg.targets.iter().map(|t| TargetHealthDescription {
            target: t.clone(),
            target_health: "healthy".to_string(),
        }).collect();
        Ok(descs)
    }

    // Listener APIs
    pub fn create_listener(
        &self,
        lb_arn: String,
        port: i32,
        protocol: String,
        default_actions: Vec<ListenerAction>,
    ) -> Result<Listener, Elbv2Error> {
        let lb = self.load_balancers.get(&lb_arn).ok_or_else(|| Elbv2Error::LoadBalancerNotFound(lb_arn.clone()))?;
        
        let hex_id = &Uuid::new_v4().to_string().replace('-', "")[..16];
        let listener_arn = format!("{}/{}", lb_arn.replace(":loadbalancer/", ":listener/"), hex_id);

        for action in &default_actions {
            if let Some(ref tg_arn) = action.target_group_arn {
                if let Some(mut tg) = self.target_groups.get_mut(tg_arn) {
                    if !tg.load_balancer_arns.contains(&lb_arn) {
                        tg.load_balancer_arns.push(lb_arn.clone());
                    }
                }
            }
        }

        let listener = Listener {
            listener_arn: listener_arn.clone(),
            load_balancer_arn: lb.load_balancer_arn.clone(),
            port,
            protocol,
            default_actions,
            ssl_policy: None,
            certificates: vec![],
        };

        self.listeners.insert(listener_arn, listener.clone());
        Ok(listener)
    }

    pub fn describe_listeners(&self, lb_arn: Option<String>, listener_arns: Option<Vec<String>>) -> Vec<Listener> {
        let mut results = Vec::new();
        for item in self.listeners.iter() {
            let l = item.value();
            if let Some(ref l_arn) = lb_arn {
                if l.load_balancer_arn != *l_arn {
                    continue;
                }
            }
            if let Some(ref a_list) = listener_arns {
                if !a_list.is_empty() && !a_list.contains(&l.listener_arn) {
                    continue;
                }
            }
            results.push(l.clone());
        }
        results
    }

    pub fn delete_listener(&self, listener_arn: &str) -> Result<(), Elbv2Error> {
        self.listeners
            .remove(listener_arn)
            .ok_or_else(|| Elbv2Error::ListenerNotFound(listener_arn.to_string()))?;
        Ok(())
    }
}

use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

use crate::types::*;

#[derive(Debug, Error)]
pub enum Ec2Error {
    #[error("VpcNotFoundException: The specified VPC does not exist.")]
    VpcNotFound(String),
    #[error("SubnetNotFoundException: The specified subnet does not exist.")]
    SubnetNotFound(String),
    #[error("SecurityGroupNotFoundException: The specified security group does not exist.")]
    SecurityGroupNotFound(String),
    #[error("KeyPairNotFoundException: The specified key pair does not exist.")]
    KeyPairNotFound(String),
    #[error("InstanceNotFoundException: The specified instance does not exist.")]
    InstanceNotFound(String),
    #[error("InvalidParameterValue: {0}")]
    InvalidParameter(String),
}

#[derive(Clone)]
pub struct Ec2State {
    account_id: String,
    region: String,
    vpcs: Arc<DashMap<String, Vpc>>,
    subnets: Arc<DashMap<String, Subnet>>,
    security_groups: Arc<DashMap<String, SecurityGroup>>,
    key_pairs: Arc<DashMap<String, KeyPair>>,
    instances: Arc<DashMap<String, Instance>>,
}

impl Ec2State {
    pub fn new(account_id: String, region: String) -> Self {
        let state = Self {
            account_id,
            region: region.clone(),
            vpcs: Arc::new(DashMap::new()),
            subnets: Arc::new(DashMap::new()),
            security_groups: Arc::new(DashMap::new()),
            key_pairs: Arc::new(DashMap::new()),
            instances: Arc::new(DashMap::new()),
        };

        state.init_default_resources();
        state
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    fn init_default_resources(&self) {
        let default_vpc_id = "vpc-default00000001".to_string();
        let default_vpc = Vpc {
            vpc_id: default_vpc_id.clone(),
            cidr_block: "172.31.0.0/16".to_string(),
            is_default: true,
            state: "available".to_string(),
            tags: vec![Tag {
                key: "Name".to_string(),
                value: "Default VPC".to_string(),
            }],
        };
        self.vpcs.insert(default_vpc_id.clone(), default_vpc);

        let default_subnet_id = "subnet-default000001".to_string();
        let default_subnet = Subnet {
            subnet_id: default_subnet_id.clone(),
            vpc_id: default_vpc_id.clone(),
            cidr_block: "172.31.0.0/20".to_string(),
            availability_zone: format!("{}a", self.region),
            default_for_az: true,
            state: "available".to_string(),
            available_ip_address_count: 4091,
            tags: vec![Tag {
                key: "Name".to_string(),
                value: "Default Subnet".to_string(),
            }],
        };
        self.subnets.insert(default_subnet_id, default_subnet);

        let default_sg_id = "sg-default000000001".to_string();
        let default_sg = SecurityGroup {
            group_id: default_sg_id.clone(),
            group_name: "default".to_string(),
            description: "default VPC security group".to_string(),
            vpc_id: default_vpc_id,
            ip_permissions: vec![IpPermission {
                ip_protocol: "-1".to_string(),
                from_port: None,
                to_port: None,
                ip_ranges: vec![IpRange {
                    cidr_ip: "0.0.0.0/0".to_string(),
                    description: None,
                }],
            }],
            ip_permissions_egress: vec![IpPermission {
                ip_protocol: "-1".to_string(),
                from_port: None,
                to_port: None,
                ip_ranges: vec![IpRange {
                    cidr_ip: "0.0.0.0/0".to_string(),
                    description: None,
                }],
            }],
            tags: vec![],
        };
        self.security_groups.insert(default_sg_id, default_sg);
    }

    pub fn reset(&self) {
        self.vpcs.clear();
        self.subnets.clear();
        self.security_groups.clear();
        self.key_pairs.clear();
        self.instances.clear();
        self.init_default_resources();
    }

    pub fn export_snapshot(&self) -> Ec2StateSnapshot {
        Ec2StateSnapshot {
            vpcs: self.vpcs.iter().map(|kv| kv.value().clone()).collect(),
            subnets: self.subnets.iter().map(|kv| kv.value().clone()).collect(),
            security_groups: self.security_groups.iter().map(|kv| kv.value().clone()).collect(),
            key_pairs: self.key_pairs.iter().map(|kv| kv.value().clone()).collect(),
            instances: self.instances.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: Ec2StateSnapshot) {
        self.reset();
        self.vpcs.clear();
        self.subnets.clear();
        self.security_groups.clear();

        for v in snapshot.vpcs {
            self.vpcs.insert(v.vpc_id.clone(), v);
        }
        for s in snapshot.subnets {
            self.subnets.insert(s.subnet_id.clone(), s);
        }
        for g in snapshot.security_groups {
            self.security_groups.insert(g.group_id.clone(), g);
        }
        for k in snapshot.key_pairs {
            self.key_pairs.insert(k.key_name.clone(), k);
        }
        for i in snapshot.instances {
            self.instances.insert(i.instance_id.clone(), i);
        }
    }

    // VPC APIs
    pub fn create_vpc(&self, cidr_block: String, tags: Vec<Tag>) -> Vpc {
        let vpc_id = format!("vpc-{}", &Uuid::new_v4().to_string().replace('-', "")[..17]);
        let vpc = Vpc {
            vpc_id: vpc_id.clone(),
            cidr_block,
            is_default: false,
            state: "available".to_string(),
            tags,
        };
        self.vpcs.insert(vpc_id, vpc.clone());
        vpc
    }

    pub fn describe_vpcs(&self, vpc_ids: Option<Vec<String>>) -> Vec<Vpc> {
        match vpc_ids {
            Some(ids) if !ids.is_empty() => ids
                .iter()
                .filter_map(|id| self.vpcs.get(id).map(|v| v.clone()))
                .collect(),
            _ => self.vpcs.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn delete_vpc(&self, vpc_id: &str) -> Result<(), Ec2Error> {
        self.vpcs
            .remove(vpc_id)
            .ok_or_else(|| Ec2Error::VpcNotFound(vpc_id.to_string()))?;
        Ok(())
    }

    // Subnet APIs
    pub fn create_subnet(&self, vpc_id: String, cidr_block: String, az: Option<String>, tags: Vec<Tag>) -> Result<Subnet, Ec2Error> {
        if !self.vpcs.contains_key(&vpc_id) {
            return Err(Ec2Error::VpcNotFound(vpc_id));
        }

        let subnet_id = format!("subnet-{}", &Uuid::new_v4().to_string().replace('-', "")[..17]);
        let subnet = Subnet {
            subnet_id: subnet_id.clone(),
            vpc_id,
            cidr_block,
            availability_zone: az.unwrap_or_else(|| format!("{}a", self.region)),
            default_for_az: false,
            state: "available".to_string(),
            available_ip_address_count: 251,
            tags,
        };
        self.subnets.insert(subnet_id, subnet.clone());
        Ok(subnet)
    }

    pub fn describe_subnets(&self, subnet_ids: Option<Vec<String>>) -> Vec<Subnet> {
        match subnet_ids {
            Some(ids) if !ids.is_empty() => ids
                .iter()
                .filter_map(|id| self.subnets.get(id).map(|s| s.clone()))
                .collect(),
            _ => self.subnets.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn delete_subnet(&self, subnet_id: &str) -> Result<(), Ec2Error> {
        self.subnets
            .remove(subnet_id)
            .ok_or_else(|| Ec2Error::SubnetNotFound(subnet_id.to_string()))?;
        Ok(())
    }

    // Security Group APIs
    pub fn create_security_group(&self, name: String, description: String, vpc_id: Option<String>, tags: Vec<Tag>) -> SecurityGroup {
        let actual_vpc_id = vpc_id.unwrap_or_else(|| {
            self.vpcs
                .iter()
                .find(|kv| kv.value().is_default)
                .map(|kv| kv.key().clone())
                .unwrap_or_else(|| "vpc-default00000001".to_string())
        });

        let sg_id = format!("sg-{}", &Uuid::new_v4().to_string().replace('-', "")[..17]);
        let sg = SecurityGroup {
            group_id: sg_id.clone(),
            group_name: name,
            description,
            vpc_id: actual_vpc_id,
            ip_permissions: vec![],
            ip_permissions_egress: vec![IpPermission {
                ip_protocol: "-1".to_string(),
                from_port: None,
                to_port: None,
                ip_ranges: vec![IpRange {
                    cidr_ip: "0.0.0.0/0".to_string(),
                    description: None,
                }],
            }],
            tags,
        };

        self.security_groups.insert(sg_id, sg.clone());
        sg
    }

    pub fn describe_security_groups(&self, group_ids: Option<Vec<String>>, group_names: Option<Vec<String>>) -> Vec<SecurityGroup> {
        if let Some(ids) = group_ids {
            if !ids.is_empty() {
                return ids.iter().filter_map(|id| self.security_groups.get(id).map(|g| g.clone())).collect();
            }
        }
        if let Some(names) = group_names {
            if !names.is_empty() {
                return self.security_groups.iter().filter(|kv| names.contains(&kv.value().group_name)).map(|kv| kv.value().clone()).collect();
            }
        }
        self.security_groups.iter().map(|kv| kv.value().clone()).collect()
    }

    pub fn authorize_security_group_ingress(&self, group_id: &str, permissions: Vec<IpPermission>) -> Result<(), Ec2Error> {
        let mut sg = self.security_groups.get_mut(group_id).ok_or_else(|| Ec2Error::SecurityGroupNotFound(group_id.to_string()))?;
        sg.ip_permissions.extend(permissions);
        Ok(())
    }

    pub fn delete_security_group(&self, group_id: &str) -> Result<(), Ec2Error> {
        self.security_groups
            .remove(group_id)
            .ok_or_else(|| Ec2Error::SecurityGroupNotFound(group_id.to_string()))?;
        Ok(())
    }

    // Key Pair APIs
    pub fn create_key_pair(&self, key_name: String) -> KeyPair {
        let key_pair_id = format!("key-{}", &Uuid::new_v4().to_string().replace('-', "")[..17]);
        let kp = KeyPair {
            key_name: key_name.clone(),
            key_pair_id,
            key_fingerprint: "11:22:33:44:55:66:77:88:99:aa:bb:cc:dd:ee:ff:00:11:22:33:44".to_string(),
            key_material: Some("-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0...MOCK...\n-----END RSA PRIVATE KEY-----".to_string()),
        };
        self.key_pairs.insert(key_name, kp.clone());
        kp
    }

    pub fn describe_key_pairs(&self, key_names: Option<Vec<String>>) -> Vec<KeyPair> {
        match key_names {
            Some(names) if !names.is_empty() => names
                .iter()
                .filter_map(|n| self.key_pairs.get(n).map(|k| k.clone()))
                .collect(),
            _ => self.key_pairs.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn delete_key_pair(&self, key_name: &str) -> Result<(), Ec2Error> {
        self.key_pairs
            .remove(key_name)
            .ok_or_else(|| Ec2Error::KeyPairNotFound(key_name.to_string()))?;
        Ok(())
    }

    // Instance Lifecycle APIs
    pub fn run_instances(
        &self,
        image_id: String,
        instance_type: Option<String>,
        key_name: Option<String>,
        subnet_id: Option<String>,
        security_group_ids: Vec<String>,
        count: Option<i32>,
        tags: Vec<Tag>,
    ) -> Vec<Instance> {
        let num = count.unwrap_or(1).max(1);
        let now_str = Utc::now().to_rfc3339();
        let mut created = Vec::new();

        let vpc_id = if let Some(ref s_id) = subnet_id {
            self.subnets.get(s_id).map(|s| s.vpc_id.clone())
        } else {
            None
        };

        for i in 0..num {
            let instance_id = format!("i-{}", &Uuid::new_v4().to_string().replace('-', "")[..17]);
            let inst = Instance {
                instance_id: instance_id.clone(),
                image_id: image_id.clone(),
                instance_type: instance_type.clone().unwrap_or_else(|| "t3.micro".to_string()),
                key_name: key_name.clone(),
                subnet_id: subnet_id.clone(),
                vpc_id: vpc_id.clone(),
                private_ip_address: format!("172.31.{}.{}", 10 + i, 50 + i),
                public_ip_address: Some(format!("54.210.{}.{}", 10 + i, 50 + i)),
                state: "running".to_string(),
                state_code: 16,
                launch_time: now_str.clone(),
                security_groups: security_group_ids.clone(),
                tags: tags.clone(),
            };

            self.instances.insert(instance_id, inst.clone());
            created.push(inst);
        }

        created
    }

    pub fn describe_instances(&self, instance_ids: Option<Vec<String>>) -> Vec<Instance> {
        match instance_ids {
            Some(ids) if !ids.is_empty() => ids
                .iter()
                .filter_map(|id| self.instances.get(id).map(|i| i.clone()))
                .collect(),
            _ => self.instances.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn terminate_instances(&self, instance_ids: Vec<String>) -> Vec<Instance> {
        let mut terminated = Vec::new();
        for id in instance_ids {
            if let Some(mut inst) = self.instances.get_mut(&id) {
                inst.state = "terminated".to_string();
                inst.state_code = 48;
                terminated.push(inst.clone());
            }
        }
        terminated
    }
}

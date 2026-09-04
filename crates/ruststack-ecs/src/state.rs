use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

use crate::types::*;

#[derive(Debug, Error)]
pub enum EcsError {
    #[error("ClusterNotFoundException: The specified cluster was not found.")]
    ClusterNotFound(String),
    #[error("ClientException: Cluster already exists.")]
    ClusterAlreadyExists(String),
    #[error("ClientException: The specified task definition does not exist.")]
    TaskDefinitionNotFound(String),
    #[error("ServiceNotFoundException: The specified service was not found.")]
    ServiceNotFound(String),
    #[error("InvalidParameterException: {0}")]
    InvalidParameter(String),
}

#[derive(Clone)]
pub struct EcsState {
    account_id: String,
    region: String,
    clusters: Arc<DashMap<String, Cluster>>,
    task_definitions: Arc<DashMap<String, Vec<TaskDefinition>>>, // family -> revisions
    tasks: Arc<DashMap<String, Task>>,
    services: Arc<DashMap<String, Service>>,
}

impl EcsState {
    pub fn new(account_id: String, region: String) -> Self {
        let state = Self {
            account_id: account_id.clone(),
            region: region.clone(),
            clusters: Arc::new(DashMap::new()),
            task_definitions: Arc::new(DashMap::new()),
            tasks: Arc::new(DashMap::new()),
            services: Arc::new(DashMap::new()),
        };

        // Create default cluster
        let default_name = "default".to_string();
        let default_arn = format!("arn:aws:ecs:{}:{}:cluster/{}", region, account_id, default_name);
        state.clusters.insert(
            default_name.clone(),
            Cluster {
                cluster_arn: default_arn,
                cluster_name: default_name,
                status: "ACTIVE".to_string(),
                registered_container_instances_count: 0,
                running_tasks_count: 0,
                pending_tasks_count: 0,
                active_services_count: 0,
            },
        );

        state
    }

    pub fn reset(&self) {
        self.clusters.clear();
        self.task_definitions.clear();
        self.tasks.clear();
        self.services.clear();

        // Restore default cluster
        let default_name = "default".to_string();
        let default_arn = format!("arn:aws:ecs:{}:{}:cluster/{}", self.region, self.account_id, default_name);
        self.clusters.insert(
            default_name.clone(),
            Cluster {
                cluster_arn: default_arn,
                cluster_name: default_name,
                status: "ACTIVE".to_string(),
                registered_container_instances_count: 0,
                running_tasks_count: 0,
                pending_tasks_count: 0,
                active_services_count: 0,
            },
        );
    }

    pub fn export_snapshot(&self) -> EcsStateSnapshot {
        let mut defs = Vec::new();
        for item in self.task_definitions.iter() {
            defs.extend(item.value().clone());
        }

        EcsStateSnapshot {
            clusters: self.clusters.iter().map(|kv| kv.value().clone()).collect(),
            task_definitions: defs,
            tasks: self.tasks.iter().map(|kv| kv.value().clone()).collect(),
            services: self.services.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: EcsStateSnapshot) {
        self.reset();
        self.clusters.clear();

        for c in snapshot.clusters {
            self.clusters.insert(c.cluster_name.clone(), c);
        }
        for td in snapshot.task_definitions {
            self.task_definitions
                .entry(td.family.clone())
                .or_default()
                .push(td);
        }
        for t in snapshot.tasks {
            self.tasks.insert(t.task_arn.clone(), t);
        }
        for s in snapshot.services {
            self.services.insert(s.service_name.clone(), s);
        }
    }

    // Cluster APIs
    pub fn create_cluster(&self, req: CreateClusterRequest) -> Result<CreateClusterResponse, EcsError> {
        let name = req.cluster_name.unwrap_or_else(|| "default".to_string());
        if self.clusters.contains_key(&name) {
            let cluster = self.clusters.get(&name).unwrap().clone();
            return Ok(CreateClusterResponse { cluster });
        }

        let cluster_arn = format!("arn:aws:ecs:{}:{}:cluster/{}", self.region, self.account_id, name);
        let cluster = Cluster {
            cluster_arn,
            cluster_name: name.clone(),
            status: "ACTIVE".to_string(),
            registered_container_instances_count: 0,
            running_tasks_count: 0,
            pending_tasks_count: 0,
            active_services_count: 0,
        };

        self.clusters.insert(name, cluster.clone());
        Ok(CreateClusterResponse { cluster })
    }

    pub fn describe_clusters(&self, req: DescribeClustersRequest) -> DescribeClustersResponse {
        let mut clusters = Vec::new();
        let mut failures = Vec::new();

        let cluster_names = req.clusters.unwrap_or_else(|| vec!["default".to_string()]);
        for name in cluster_names {
            let clean_name = name.rsplit('/').next().unwrap_or(&name);
            if let Some(c) = self.clusters.get(clean_name) {
                clusters.push(c.clone());
            } else {
                failures.push(serde_json::json!({
                    "arn": name,
                    "reason": "MISSING"
                }));
            }
        }

        DescribeClustersResponse { clusters, failures }
    }

    pub fn list_clusters(&self) -> ListClustersResponse {
        let cluster_arns = self.clusters.iter().map(|kv| kv.value().cluster_arn.clone()).collect();
        ListClustersResponse { cluster_arns }
    }

    pub fn delete_cluster(&self, req: DeleteClusterRequest) -> Result<DeleteClusterResponse, EcsError> {
        let clean_name = req.cluster.rsplit('/').next().unwrap_or(&req.cluster);
        let (_, cluster) = self.clusters.remove(clean_name).ok_or_else(|| EcsError::ClusterNotFound(req.cluster))?;
        Ok(DeleteClusterResponse { cluster })
    }

    // Task Definition APIs
    pub fn register_task_definition(&self, req: RegisterTaskDefinitionRequest) -> Result<RegisterTaskDefinitionResponse, EcsError> {
        let mut entry = self.task_definitions.entry(req.family.clone()).or_default();
        let revision = (entry.len() + 1) as i32;
        let arn = format!(
            "arn:aws:ecs:{}:{}:task-definition/{}:{}",
            self.region, self.account_id, req.family, revision
        );

        let td = TaskDefinition {
            task_definition_arn: arn,
            family: req.family,
            revision,
            container_definitions: req.container_definitions,
            status: "ACTIVE".to_string(),
            cpu: req.cpu,
            memory: req.memory,
            network_mode: req.network_mode,
            requires_compatibilities: req.requires_compatibilities,
            execution_role_arn: req.execution_role_arn,
            task_role_arn: req.task_role_arn,
        };

        entry.push(td.clone());
        Ok(RegisterTaskDefinitionResponse { task_definition: td })
    }

    pub fn describe_task_definition(&self, req: DescribeTaskDefinitionRequest) -> Result<DescribeTaskDefinitionResponse, EcsError> {
        let target = req.task_definition.rsplit('/').next().unwrap_or(&req.task_definition);
        let (family, rev) = if target.contains(':') {
            let mut parts = target.split(':');
            let f = parts.next().unwrap();
            let r = parts.next().and_then(|s| s.parse::<i32>().ok());
            (f, r)
        } else {
            (target, None)
        };

        let entry = self.task_definitions.get(family).ok_or_else(|| EcsError::TaskDefinitionNotFound(req.task_definition.clone()))?;
        let td = if let Some(r) = rev {
            entry.iter().find(|t| t.revision == r).ok_or_else(|| EcsError::TaskDefinitionNotFound(req.task_definition.clone()))?
        } else {
            entry.last().ok_or_else(|| EcsError::TaskDefinitionNotFound(req.task_definition.clone()))?
        };

        Ok(DescribeTaskDefinitionResponse { task_definition: td.clone() })
    }

    pub fn deregister_task_definition(&self, req: DeregisterTaskDefinitionRequest) -> Result<DeregisterTaskDefinitionResponse, EcsError> {
        let mut resp = self.describe_task_definition(DescribeTaskDefinitionRequest {
            task_definition: req.task_definition,
        })?;
        resp.task_definition.status = "INACTIVE".to_string();
        Ok(DeregisterTaskDefinitionResponse { task_definition: resp.task_definition })
    }

    pub fn list_task_definitions(&self, req: ListTaskDefinitionsRequest) -> ListTaskDefinitionsResponse {
        let mut arns = Vec::new();
        for item in self.task_definitions.iter() {
            if let Some(ref prefix) = req.family_prefix {
                if !item.key().starts_with(prefix) {
                    continue;
                }
            }
            for td in item.value() {
                if let Some(ref status) = req.status {
                    if td.status != *status {
                        continue;
                    }
                }
                arns.push(td.task_definition_arn.clone());
            }
        }
        ListTaskDefinitionsResponse { task_definition_arns: arns }
    }

    // Task Execution APIs
    pub fn run_task(&self, req: RunTaskRequest) -> Result<RunTaskResponse, EcsError> {
        let cluster_name = req.cluster.unwrap_or_else(|| "default".to_string());
        let cluster_clean = cluster_name.rsplit('/').next().unwrap_or(&cluster_name);
        let cluster = self.clusters.get(cluster_clean).ok_or_else(|| EcsError::ClusterNotFound(cluster_clean.to_string()))?;

        let td_resp = self.describe_task_definition(DescribeTaskDefinitionRequest {
            task_definition: req.task_definition,
        })?;

        let count = req.count.unwrap_or(1).max(1);
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;
        let mut tasks = Vec::new();

        for _ in 0..count {
            let task_id = Uuid::new_v4().to_string();
            let task_arn = format!("arn:aws:ecs:{}:{}:task/{}/{}", self.region, self.account_id, cluster_clean, task_id);
            
            let containers = td_resp.task_definition.container_definitions.iter().map(|cd| {
                Container {
                    container_arn: format!("{}/{}", task_arn, cd.name),
                    task_arn: task_arn.clone(),
                    name: cd.name.clone(),
                    last_status: "RUNNING".to_string(),
                    exit_code: None,
                    image: cd.image.clone(),
                }
            }).collect();

            let task = Task {
                task_arn: task_arn.clone(),
                cluster_arn: cluster.cluster_arn.clone(),
                task_definition_arn: td_resp.task_definition.task_definition_arn.clone(),
                last_status: "RUNNING".to_string(),
                desired_status: "RUNNING".to_string(),
                containers,
                started_at: Some(now),
                created_at: now,
                launch_type: req.launch_type.clone().or_else(|| Some("FARGATE".to_string())),
            };

            self.tasks.insert(task_arn, task.clone());
            tasks.push(task);
        }

        Ok(RunTaskResponse { tasks, failures: vec![] })
    }

    pub fn describe_tasks(&self, req: DescribeTasksRequest) -> DescribeTasksResponse {
        let mut tasks = Vec::new();
        let mut failures = Vec::new();

        for arn in req.tasks {
            if let Some(t) = self.tasks.get(&arn) {
                tasks.push(t.clone());
            } else {
                failures.push(serde_json::json!({
                    "arn": arn,
                    "reason": "MISSING"
                }));
            }
        }

        DescribeTasksResponse { tasks, failures }
    }

    pub fn list_tasks(&self, _req: ListTasksRequest) -> ListTasksResponse {
        let task_arns = self.tasks.iter().map(|kv| kv.key().clone()).collect();
        ListTasksResponse { task_arns }
    }

    pub fn stop_task(&self, req: StopTaskRequest) -> Result<StopTaskResponse, EcsError> {
        let mut task = self.tasks.get_mut(&req.task).ok_or_else(|| EcsError::InvalidParameter("Task not found".to_string()))?;
        task.last_status = "STOPPED".to_string();
        task.desired_status = "STOPPED".to_string();
        for c in &mut task.containers {
            c.last_status = "STOPPED".to_string();
            c.exit_code = Some(0);
        }
        Ok(StopTaskResponse { task: task.clone() })
    }

    // Service APIs
    pub fn create_service(&self, req: CreateServiceRequest) -> Result<CreateServiceResponse, EcsError> {
        let cluster_name = req.cluster.unwrap_or_else(|| "default".to_string());
        let cluster_clean = cluster_name.rsplit('/').next().unwrap_or(&cluster_name);
        let cluster = self.clusters.get(cluster_clean).ok_or_else(|| EcsError::ClusterNotFound(cluster_clean.to_string()))?;

        let td_resp = self.describe_task_definition(DescribeTaskDefinitionRequest {
            task_definition: req.task_definition.clone(),
        })?;

        let service_arn = format!("arn:aws:ecs:{}:{}:service/{}/{}", self.region, self.account_id, cluster_clean, req.service_name);
        let count = req.desired_count.unwrap_or(1);
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;

        let service = Service {
            service_arn,
            service_name: req.service_name.clone(),
            cluster_arn: cluster.cluster_arn.clone(),
            task_definition: td_resp.task_definition.task_definition_arn,
            status: "ACTIVE".to_string(),
            desired_count: count,
            running_count: count,
            pending_count: 0,
            launch_type: req.launch_type.or_else(|| Some("FARGATE".to_string())),
            created_at: now,
        };

        self.services.insert(req.service_name, service.clone());
        Ok(CreateServiceResponse { service })
    }

    pub fn describe_services(&self, req: DescribeServicesRequest) -> DescribeServicesResponse {
        let mut services = Vec::new();
        let mut failures = Vec::new();

        for name in req.services {
            let clean = name.rsplit('/').next().unwrap_or(&name);
            if let Some(s) = self.services.get(clean) {
                services.push(s.clone());
            } else {
                failures.push(serde_json::json!({
                    "arn": name,
                    "reason": "MISSING"
                }));
            }
        }

        DescribeServicesResponse { services, failures }
    }

    pub fn update_service(&self, req: UpdateServiceRequest) -> Result<UpdateServiceResponse, EcsError> {
        let clean = req.service.rsplit('/').next().unwrap_or(&req.service);
        let mut service = self.services.get_mut(clean).ok_or_else(|| EcsError::ServiceNotFound(req.service))?;
        if let Some(count) = req.desired_count {
            service.desired_count = count;
            service.running_count = count;
        }
        if let Some(td) = req.task_definition {
            let td_resp = self.describe_task_definition(DescribeTaskDefinitionRequest {
                task_definition: td,
            })?;
            service.task_definition = td_resp.task_definition.task_definition_arn;
        }
        Ok(UpdateServiceResponse { service: service.clone() })
    }

    pub fn delete_service(&self, req: DeleteServiceRequest) -> Result<DeleteServiceResponse, EcsError> {
        let clean = req.service.rsplit('/').next().unwrap_or(&req.service);
        let (_, service) = self.services.remove(clean).ok_or_else(|| EcsError::ServiceNotFound(req.service))?;
        Ok(DeleteServiceResponse { service })
    }

    pub fn list_services(&self, _req: ListServicesRequest) -> ListServicesResponse {
        let service_arns = self.services.iter().map(|kv| kv.value().service_arn.clone()).collect();
        ListServicesResponse { service_arns }
    }
}

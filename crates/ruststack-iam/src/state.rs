use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum IamError {
    #[error("NoSuchEntity: {0}")]
    NotFound(String),
    #[error("EntityAlreadyExists: {0}")]
    AlreadyExists(String),
    #[error("ValidationError: {0}")]
    Validation(String),
}

#[derive(Clone)]
pub struct IamState {
    account_id: String,
    roles: Arc<DashMap<String, IamRole>>,
    policies: Arc<DashMap<String, IamPolicy>>,
    users: Arc<DashMap<String, IamUser>>,
    access_keys: Arc<DashMap<String, IamAccessKey>>,
}

impl IamState {
    pub fn new(account_id: String) -> Self {
        let state = Self {
            account_id,
            roles: Arc::new(DashMap::new()),
            policies: Arc::new(DashMap::new()),
            users: Arc::new(DashMap::new()),
            access_keys: Arc::new(DashMap::new()),
        };

        // Seed default AWS managed policies
        state.seed_aws_managed_policy("AdministratorAccess", r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Action":"*","Resource":"*"}]}"#);
        state.seed_aws_managed_policy("AmazonS3FullAccess", r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Action":"s3:*","Resource":"*"}]}"#);
        state.seed_aws_managed_policy("AmazonDynamoDBFullAccess", r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Action":"dynamodb:*","Resource":"*"}]}"#);
        state.seed_aws_managed_policy("AmazonSQSFullAccess", r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Action":"sqs:*","Resource":"*"}]}"#);
        state.seed_aws_managed_policy("AWSLambdaBasicExecutionRole", r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Action":["logs:CreateLogGroup","logs:CreateLogStream","logs:PutLogEvents"],"Resource":"*"}]}"#);

        state
    }

    fn seed_aws_managed_policy(&self, name: &str, doc: &str) {
        let arn = format!("arn:aws:iam::aws:policy/{}", name);
        let now = Utc::now().to_rfc3339();
        let policy = IamPolicy {
            policy_name: name.to_string(),
            policy_id: format!("ANPA{}", Uuid::new_v4().simple().to_string().to_uppercase()),
            arn: arn.clone(),
            path: "/".to_string(),
            default_version_id: "v1".to_string(),
            policy_document: doc.to_string(),
            description: Some(format!("AWS Managed Policy for {}", name)),
            create_date: now.clone(),
            update_date: now,
            attachment_count: 0,
            tags: HashMap::new(),
        };
        self.policies.insert(arn, policy);
    }

    fn role_arn(&self, role_name: &str) -> String {
        format!("arn:aws:iam::{}:role/{}", self.account_id, role_name)
    }

    fn user_arn(&self, user_name: &str) -> String {
        format!("arn:aws:iam::{}:user/{}", self.account_id, user_name)
    }

    fn policy_arn(&self, policy_name: &str) -> String {
        format!("arn:aws:iam::{}:policy/{}", self.account_id, policy_name)
    }

    pub fn create_role(
        &self,
        role_name: String,
        assume_role_policy_document: String,
        path: Option<String>,
        description: Option<String>,
    ) -> Result<IamRole, IamError> {
        if self.roles.contains_key(&role_name) {
            return Err(IamError::AlreadyExists(format!(
                "Role {} already exists",
                role_name
            )));
        }

        let role_id = format!("AROA{}", Uuid::new_v4().simple().to_string().to_uppercase());
        let arn = self.role_arn(&role_name);
        let now = Utc::now().to_rfc3339();

        let role = IamRole {
            role_name: role_name.clone(),
            role_id,
            arn,
            path: path.unwrap_or_else(|| "/".to_string()),
            assume_role_policy_document,
            description,
            create_date: now,
            max_session_duration: 3600,
            attached_policies: Vec::new(),
            inline_policies: HashMap::new(),
            tags: HashMap::new(),
        };

        self.roles.insert(role_name, role.clone());
        Ok(role)
    }

    pub fn get_role(&self, role_name: &str) -> Result<IamRole, IamError> {
        self.roles
            .get(role_name)
            .map(|r| r.clone())
            .ok_or_else(|| IamError::NotFound(format!("Role {} does not exist", role_name)))
    }

    pub fn delete_role(&self, role_name: &str) -> Result<(), IamError> {
        if self.roles.remove(role_name).is_some() {
            Ok(())
        } else {
            Err(IamError::NotFound(format!(
                "Role {} does not exist",
                role_name
            )))
        }
    }

    pub fn list_roles(&self) -> Vec<IamRole> {
        let mut list: Vec<IamRole> = self.roles.iter().map(|r| r.clone()).collect();
        list.sort_by(|a, b| a.role_name.cmp(&b.role_name));
        list
    }

    pub fn create_policy(
        &self,
        policy_name: String,
        policy_document: String,
        path: Option<String>,
        description: Option<String>,
    ) -> Result<IamPolicy, IamError> {
        let arn = self.policy_arn(&policy_name);
        if self.policies.contains_key(&arn) {
            return Err(IamError::AlreadyExists(format!(
                "Policy {} already exists",
                policy_name
            )));
        }

        let policy_id = format!("ANPA{}", Uuid::new_v4().simple().to_string().to_uppercase());
        let now = Utc::now().to_rfc3339();

        let policy = IamPolicy {
            policy_name,
            policy_id,
            arn: arn.clone(),
            path: path.unwrap_or_else(|| "/".to_string()),
            default_version_id: "v1".to_string(),
            policy_document,
            description,
            create_date: now.clone(),
            update_date: now,
            attachment_count: 0,
            tags: HashMap::new(),
        };

        self.policies.insert(arn, policy.clone());
        Ok(policy)
    }

    pub fn get_policy(&self, policy_arn: &str) -> Result<IamPolicy, IamError> {
        self.policies
            .get(policy_arn)
            .map(|p| p.clone())
            .ok_or_else(|| IamError::NotFound(format!("Policy {} does not exist", policy_arn)))
    }

    pub fn delete_policy(&self, policy_arn: &str) -> Result<(), IamError> {
        if self.policies.remove(policy_arn).is_some() {
            Ok(())
        } else {
            Err(IamError::NotFound(format!(
                "Policy {} does not exist",
                policy_arn
            )))
        }
    }

    pub fn list_policies(&self) -> Vec<IamPolicy> {
        let mut list: Vec<IamPolicy> = self.policies.iter().map(|p| p.clone()).collect();
        list.sort_by(|a, b| a.policy_name.cmp(&b.policy_name));
        list
    }

    pub fn attach_role_policy(&self, role_name: &str, policy_arn: &str) -> Result<(), IamError> {
        let mut role = self
            .roles
            .get_mut(role_name)
            .ok_or_else(|| IamError::NotFound(format!("Role {} does not exist", role_name)))?;

        if !role.attached_policies.contains(&policy_arn.to_string()) {
            role.attached_policies.push(policy_arn.to_string());
        }

        if let Some(mut pol) = self.policies.get_mut(policy_arn) {
            pol.attachment_count += 1;
        }

        Ok(())
    }

    pub fn detach_role_policy(&self, role_name: &str, policy_arn: &str) -> Result<(), IamError> {
        let mut role = self
            .roles
            .get_mut(role_name)
            .ok_or_else(|| IamError::NotFound(format!("Role {} does not exist", role_name)))?;

        role.attached_policies.retain(|a| a != policy_arn);

        if let Some(mut pol) = self.policies.get_mut(policy_arn) {
            pol.attachment_count = (pol.attachment_count - 1).max(0);
        }

        Ok(())
    }

    pub fn list_attached_role_policies(
        &self,
        role_name: &str,
    ) -> Result<Vec<(String, String)>, IamError> {
        let role = self
            .roles
            .get(role_name)
            .ok_or_else(|| IamError::NotFound(format!("Role {} does not exist", role_name)))?;

        let mut list = Vec::new();
        for arn in &role.attached_policies {
            let name = self
                .policies
                .get(arn)
                .map(|p| p.policy_name.clone())
                .unwrap_or_else(|| arn.split('/').last().unwrap_or("policy").to_string());
            list.push((name, arn.clone()));
        }
        Ok(list)
    }

    pub fn put_role_policy(
        &self,
        role_name: &str,
        policy_name: &str,
        policy_document: &str,
    ) -> Result<(), IamError> {
        let mut role = self
            .roles
            .get_mut(role_name)
            .ok_or_else(|| IamError::NotFound(format!("Role {} does not exist", role_name)))?;

        role.inline_policies
            .insert(policy_name.to_string(), policy_document.to_string());
        Ok(())
    }

    pub fn get_role_policy(&self, role_name: &str, policy_name: &str) -> Result<String, IamError> {
        let role = self
            .roles
            .get(role_name)
            .ok_or_else(|| IamError::NotFound(format!("Role {} does not exist", role_name)))?;

        role.inline_policies
            .get(policy_name)
            .cloned()
            .ok_or_else(|| {
                IamError::NotFound(format!(
                    "Policy {} does not exist for role {}",
                    policy_name, role_name
                ))
            })
    }

    pub fn delete_role_policy(&self, role_name: &str, policy_name: &str) -> Result<(), IamError> {
        let mut role = self
            .roles
            .get_mut(role_name)
            .ok_or_else(|| IamError::NotFound(format!("Role {} does not exist", role_name)))?;

        role.inline_policies.remove(policy_name);
        Ok(())
    }

    pub fn list_role_policies(&self, role_name: &str) -> Result<Vec<String>, IamError> {
        let role = self
            .roles
            .get(role_name)
            .ok_or_else(|| IamError::NotFound(format!("Role {} does not exist", role_name)))?;

        Ok(role.inline_policies.keys().cloned().collect())
    }

    pub fn create_user(
        &self,
        user_name: String,
        path: Option<String>,
    ) -> Result<IamUser, IamError> {
        if self.users.contains_key(&user_name) {
            return Err(IamError::AlreadyExists(format!(
                "User {} already exists",
                user_name
            )));
        }

        let user_id = format!("AIDA{}", Uuid::new_v4().simple().to_string().to_uppercase());
        let arn = self.user_arn(&user_name);
        let now = Utc::now().to_rfc3339();

        let user = IamUser {
            user_name: user_name.clone(),
            user_id,
            arn,
            path: path.unwrap_or_else(|| "/".to_string()),
            create_date: now,
            attached_policies: Vec::new(),
            inline_policies: HashMap::new(),
            tags: HashMap::new(),
        };

        self.users.insert(user_name, user.clone());
        Ok(user)
    }

    pub fn get_user(&self, user_name: &str) -> Result<IamUser, IamError> {
        self.users
            .get(user_name)
            .map(|u| u.clone())
            .ok_or_else(|| IamError::NotFound(format!("User {} does not exist", user_name)))
    }

    pub fn delete_user(&self, user_name: &str) -> Result<(), IamError> {
        if self.users.remove(user_name).is_some() {
            Ok(())
        } else {
            Err(IamError::NotFound(format!(
                "User {} does not exist",
                user_name
            )))
        }
    }

    pub fn list_users(&self) -> Vec<IamUser> {
        let mut list: Vec<IamUser> = self.users.iter().map(|u| u.clone()).collect();
        list.sort_by(|a, b| a.user_name.cmp(&b.user_name));
        list
    }

    pub fn create_access_key(&self, user_name: &str) -> Result<IamAccessKey, IamError> {
        if !self.users.contains_key(user_name) {
            return Err(IamError::NotFound(format!(
                "User {} does not exist",
                user_name
            )));
        }

        let access_key_id = format!("AKIA{}", Uuid::new_v4().simple().to_string().to_uppercase());
        let mut secret_bytes = [0u8; 30];
        for b in &mut secret_bytes {
            *b = fastrand::alphanumeric() as u8;
        }
        let secret_access_key = String::from_utf8_lossy(&secret_bytes).to_string();
        let now = Utc::now().to_rfc3339();

        let key = IamAccessKey {
            access_key_id: access_key_id.clone(),
            secret_access_key,
            user_name: user_name.to_string(),
            status: "Active".to_string(),
            create_date: now,
        };

        self.access_keys.insert(access_key_id, key.clone());
        Ok(key)
    }

    pub fn list_access_keys(&self, user_name: &str) -> Vec<IamAccessKey> {
        self.access_keys
            .iter()
            .filter(|k| k.user_name == user_name)
            .map(|k| k.clone())
            .collect()
    }

    pub fn delete_access_key(&self, access_key_id: &str) -> Result<(), IamError> {
        if self.access_keys.remove(access_key_id).is_some() {
            Ok(())
        } else {
            Err(IamError::NotFound(format!(
                "AccessKey {} does not exist",
                access_key_id
            )))
        }
    }

    pub fn export_snapshot(&self) -> IamStateSnapshot {
        IamStateSnapshot {
            roles: self.roles.iter().map(|r| r.clone()).collect(),
            policies: self.policies.iter().map(|p| p.clone()).collect(),
            users: self.users.iter().map(|u| u.clone()).collect(),
            access_keys: self.access_keys.iter().map(|k| k.clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: IamStateSnapshot) {
        self.roles.clear();
        self.policies.clear();
        self.users.clear();
        self.access_keys.clear();

        for r in snapshot.roles {
            self.roles.insert(r.role_name.clone(), r);
        }
        for p in snapshot.policies {
            self.policies.insert(p.arn.clone(), p);
        }
        for u in snapshot.users {
            self.users.insert(u.user_name.clone(), u);
        }
        for k in snapshot.access_keys {
            self.access_keys.insert(k.access_key_id.clone(), k);
        }
    }

    pub fn reset(&self) {
        self.roles.clear();
        self.policies.clear();
        self.users.clear();
        self.access_keys.clear();
    }
}

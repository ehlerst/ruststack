use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum CognitoError {
    #[error("ResourceNotFoundException: {0}")]
    ResourceNotFound(String),
    #[error("UsernameExistsException: {0}")]
    UsernameExists(String),
    #[error("UserNotFoundException: {0}")]
    UserNotFound(String),
    #[error("NotAuthorizedException: {0}")]
    NotAuthorized(String),
    #[error("InvalidParameterException: {0}")]
    InvalidParameter(String),
    #[error("CodeMismatchException: {0}")]
    CodeMismatch(String),
}

#[derive(Clone)]
pub struct CognitoState {
    pub account_id: String,
    pub region: String,
    user_pools: Arc<DashMap<String, Arc<RwLock<StoredUserPool>>>>,
    client_to_pool: Arc<DashMap<String, String>>,
}

impl CognitoState {
    pub fn new(account_id: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            account_id: account_id.into(),
            region: region.into(),
            user_pools: Arc::new(DashMap::new()),
            client_to_pool: Arc::new(DashMap::new()),
        }
    }

    pub fn format_user_pool_arn(&self, pool_id: &str) -> String {
        format!(
            "arn:aws:cognito-idp:{}:{}:userpool/{}",
            self.region, self.account_id, pool_id
        )
    }

    fn hash_password(password: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn generate_jwt(
        &self,
        user_pool_id: &str,
        client_id: &str,
        username: &str,
        sub: &str,
        email: Option<&str>,
        token_use: &str,
    ) -> String {
        let now = Utc::now().timestamp();
        let exp = now + 3600;

        let header = serde_json::json!({
            "kid": "ruststack-cognito-key-1",
            "alg": "RS256",
            "typ": "JWT"
        });

        let mut claims = serde_json::json!({
            "sub": sub,
            "cognito:username": username,
            "username": username,
            "iss": format!("http://localhost:4566/{}", user_pool_id),
            "aud": client_id,
            "client_id": client_id,
            "token_use": token_use,
            "auth_time": now,
            "iat": now,
            "exp": exp,
        });

        if let Some(e) = email {
            claims["email"] = serde_json::Value::String(e.to_string());
            claims["email_verified"] = serde_json::Value::Bool(true);
        }

        let header_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            serde_json::to_vec(&header).unwrap(),
        );
        let payload_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            serde_json::to_vec(&claims).unwrap(),
        );
        let sig_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            b"ruststack-mock-signature",
        );

        format!("{}.{}.{}", header_b64, payload_b64, sig_b64)
    }

    pub fn get_jwks(&self) -> serde_json::Value {
        serde_json::json!({
            "keys": [
                {
                    "alg": "RS256",
                    "e": "AQAB",
                    "kid": "ruststack-cognito-key-1",
                    "kty": "RSA",
                    "n": "u1lZma3wmtSmvSmMockCognitoKeyRustStackPublicKeyMock1234567890",
                    "use": "sig"
                }
            ]
        })
    }

    pub fn create_user_pool(
        &self,
        req: CreateUserPoolRequest,
    ) -> Result<CreateUserPoolResponse, CognitoError> {
        let pool_id = format!("{}_{}", self.region, fastrand::alphanumeric().to_lowercase());
        let arn = self.format_user_pool_arn(&pool_id);
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;

        let pool = StoredUserPool {
            id: pool_id.clone(),
            name: req.pool_name.clone(),
            arn: arn.clone(),
            created_at: now,
            modified_at: now,
            clients: HashMap::new(),
            users: HashMap::new(),
        };

        self.user_pools
            .insert(pool_id.clone(), Arc::new(RwLock::new(pool)));

        Ok(CreateUserPoolResponse {
            user_pool: UserPoolType {
                id: pool_id,
                name: req.pool_name,
                arn,
                status: Some("Enabled".to_string()),
                last_modified_date: now,
                creation_date: now,
            },
        })
    }

    pub fn describe_user_pool(
        &self,
        req: DescribeUserPoolRequest,
    ) -> Result<DescribeUserPoolResponse, CognitoError> {
        let pool_entry = self
            .user_pools
            .get(&req.user_pool_id)
            .ok_or_else(|| CognitoError::ResourceNotFound("User pool not found".to_string()))?;

        let pool = pool_entry.read();
        Ok(DescribeUserPoolResponse {
            user_pool: UserPoolType {
                id: pool.id.clone(),
                name: pool.name.clone(),
                arn: pool.arn.clone(),
                status: Some("Enabled".to_string()),
                last_modified_date: pool.modified_at,
                creation_date: pool.created_at,
            },
        })
    }

    pub fn list_user_pools(
        &self,
        req: ListUserPoolsRequest,
    ) -> Result<ListUserPoolsResponse, CognitoError> {
        let mut list = Vec::new();
        for item in self.user_pools.iter() {
            let pool = item.value().read();
            list.push(UserPoolDescriptionType {
                id: pool.id.clone(),
                name: pool.name.clone(),
                last_modified_date: pool.modified_at,
                creation_date: pool.created_at,
            });
        }
        list.sort_by(|a, b| a.name.cmp(&b.name));
        let limit = req.max_results.unwrap_or(50);
        if list.len() > limit {
            list.truncate(limit);
        }
        Ok(ListUserPoolsResponse {
            user_pools: list,
            next_token: None,
        })
    }

    pub fn delete_user_pool(&self, req: DeleteUserPoolRequest) -> Result<(), CognitoError> {
        self.user_pools
            .remove(&req.user_pool_id)
            .ok_or_else(|| CognitoError::ResourceNotFound("User pool not found".to_string()))?;
        self.client_to_pool
            .retain(|_, v| v != &req.user_pool_id);
        Ok(())
    }

    pub fn create_user_pool_client(
        &self,
        req: CreateUserPoolClientRequest,
    ) -> Result<CreateUserPoolClientResponse, CognitoError> {
        let pool_entry = self
            .user_pools
            .get(&req.user_pool_id)
            .ok_or_else(|| CognitoError::ResourceNotFound("User pool not found".to_string()))?;

        let client_id = format!("{:x}", Uuid::new_v4().as_u128())[..26].to_string();
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;
        let secret = if req.generate_secret.unwrap_or(false) {
            Some(format!("{:x}", Uuid::new_v4().as_u128()))
        } else {
            None
        };

        let client = StoredUserPoolClient {
            client_id: client_id.clone(),
            client_name: req.client_name.clone(),
            client_secret: secret.clone(),
            created_at: now,
            modified_at: now,
        };

        pool_entry
            .write()
            .clients
            .insert(client_id.clone(), client);
        self.client_to_pool
            .insert(client_id.clone(), req.user_pool_id.clone());

        Ok(CreateUserPoolClientResponse {
            user_pool_client: UserPoolClientType {
                user_pool_id: req.user_pool_id,
                client_id,
                client_name: req.client_name,
                client_secret: secret,
                last_modified_date: now,
                creation_date: now,
            },
        })
    }

    pub fn describe_user_pool_client(
        &self,
        req: DescribeUserPoolClientRequest,
    ) -> Result<DescribeUserPoolClientResponse, CognitoError> {
        let pool_entry = self
            .user_pools
            .get(&req.user_pool_id)
            .ok_or_else(|| CognitoError::ResourceNotFound("User pool not found".to_string()))?;

        let pool = pool_entry.read();
        let client = pool
            .clients
            .get(&req.client_id)
            .ok_or_else(|| CognitoError::ResourceNotFound("User pool client not found".to_string()))?;

        Ok(DescribeUserPoolClientResponse {
            user_pool_client: UserPoolClientType {
                user_pool_id: req.user_pool_id,
                client_id: client.client_id.clone(),
                client_name: client.client_name.clone(),
                client_secret: client.client_secret.clone(),
                last_modified_date: client.modified_at,
                creation_date: client.created_at,
            },
        })
    }

    pub fn list_user_pool_clients(
        &self,
        req: ListUserPoolClientsRequest,
    ) -> Result<ListUserPoolClientsResponse, CognitoError> {
        let pool_entry = self
            .user_pools
            .get(&req.user_pool_id)
            .ok_or_else(|| CognitoError::ResourceNotFound("User pool not found".to_string()))?;

        let pool = pool_entry.read();
        let mut clients: Vec<UserPoolClientDescription> = pool
            .clients
            .values()
            .map(|c| UserPoolClientDescription {
                client_id: c.client_id.clone(),
                client_name: c.client_name.clone(),
                user_pool_id: req.user_pool_id.clone(),
            })
            .collect();

        clients.sort_by(|a, b| a.client_name.cmp(&b.client_name));
        Ok(ListUserPoolClientsResponse {
            user_pool_clients: clients,
            next_token: None,
        })
    }

    pub fn delete_user_pool_client(
        &self,
        req: DeleteUserPoolClientRequest,
    ) -> Result<(), CognitoError> {
        let pool_entry = self
            .user_pools
            .get(&req.user_pool_id)
            .ok_or_else(|| CognitoError::ResourceNotFound("User pool not found".to_string()))?;

        pool_entry.write().clients.remove(&req.client_id);
        self.client_to_pool.remove(&req.client_id);
        Ok(())
    }

    pub fn sign_up(&self, req: SignUpRequest) -> Result<SignUpResponse, CognitoError> {
        let pool_id = self
            .client_to_pool
            .get(&req.client_id)
            .ok_or_else(|| CognitoError::ResourceNotFound("Client not found".to_string()))?
            .clone();

        let pool_entry = self.user_pools.get(&pool_id).unwrap();
        let mut pool = pool_entry.write();

        if pool.users.contains_key(&req.username) {
            return Err(CognitoError::UsernameExists(format!(
                "User {} already exists",
                req.username
            )));
        }

        let sub = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;
        let mut attrs = HashMap::new();
        attrs.insert("sub".to_string(), sub.clone());
        if let Some(list) = req.user_attributes {
            for a in list {
                if let Some(v) = a.value {
                    attrs.insert(a.name, v);
                }
            }
        }

        let user = StoredUser {
            username: req.username.clone(),
            password_hash: Self::hash_password(&req.password),
            attributes: attrs,
            sub: sub.clone(),
            enabled: true,
            status: UserStatusType::Confirmed, // Automatically confirmed in test mode
            created_at: now,
            modified_at: now,
        };

        pool.users.insert(req.username, user);

        Ok(SignUpResponse {
            user_confirmed: true,
            user_sub: sub,
        })
    }

    pub fn admin_create_user(
        &self,
        req: AdminCreateUserRequest,
    ) -> Result<AdminCreateUserResponse, CognitoError> {
        let pool_entry = self
            .user_pools
            .get(&req.user_pool_id)
            .ok_or_else(|| CognitoError::ResourceNotFound("User pool not found".to_string()))?;

        let mut pool = pool_entry.write();

        if pool.users.contains_key(&req.username) {
            return Err(CognitoError::UsernameExists(format!(
                "User {} already exists",
                req.username
            )));
        }

        let sub = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;
        let mut attrs = HashMap::new();
        attrs.insert("sub".to_string(), sub.clone());
        if let Some(list) = req.user_attributes {
            for a in list {
                if let Some(v) = a.value {
                    attrs.insert(a.name, v);
                }
            }
        }

        let pass = req
            .temporary_password
            .unwrap_or_else(|| "TemporaryPassword123!".to_string());
        let user = StoredUser {
            username: req.username.clone(),
            password_hash: Self::hash_password(&pass),
            attributes: attrs.clone(),
            sub,
            enabled: true,
            status: UserStatusType::Confirmed,
            created_at: now,
            modified_at: now,
        };

        pool.users.insert(req.username.clone(), user);

        let attr_vec: Vec<AttributeType> = attrs
            .into_iter()
            .map(|(k, v)| AttributeType {
                name: k,
                value: Some(v),
            })
            .collect();

        Ok(AdminCreateUserResponse {
            user: UserType {
                username: req.username,
                attributes: attr_vec,
                user_create_date: now,
                user_last_modified_date: now,
                enabled: true,
                user_status: UserStatusType::Confirmed,
            },
        })
    }

    pub fn admin_get_user(
        &self,
        req: AdminGetUserRequest,
    ) -> Result<AdminGetUserResponse, CognitoError> {
        let pool_entry = self
            .user_pools
            .get(&req.user_pool_id)
            .ok_or_else(|| CognitoError::ResourceNotFound("User pool not found".to_string()))?;

        let pool = pool_entry.read();
        let user = pool
            .users
            .get(&req.username)
            .ok_or_else(|| CognitoError::UserNotFound("User not found".to_string()))?;

        let mut attr_vec: Vec<AttributeType> = user
            .attributes
            .iter()
            .map(|(k, v)| AttributeType {
                name: k.clone(),
                value: Some(v.clone()),
            })
            .collect();
        attr_vec.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(AdminGetUserResponse {
            username: user.username.clone(),
            user_attributes: attr_vec,
            user_create_date: user.created_at,
            user_last_modified_date: user.modified_at,
            enabled: user.enabled,
            user_status: user.status.clone(),
        })
    }

    pub fn admin_set_user_password(
        &self,
        req: AdminSetUserPasswordRequest,
    ) -> Result<(), CognitoError> {
        let pool_entry = self
            .user_pools
            .get(&req.user_pool_id)
            .ok_or_else(|| CognitoError::ResourceNotFound("User pool not found".to_string()))?;

        let mut pool = pool_entry.write();
        let user = pool
            .users
            .get_mut(&req.username)
            .ok_or_else(|| CognitoError::UserNotFound("User not found".to_string()))?;

        user.password_hash = Self::hash_password(&req.password);
        if req.permanent.unwrap_or(false) {
            user.status = UserStatusType::Confirmed;
        }
        Ok(())
    }

    pub fn admin_delete_user(&self, req: AdminDeleteUserRequest) -> Result<(), CognitoError> {
        let pool_entry = self
            .user_pools
            .get(&req.user_pool_id)
            .ok_or_else(|| CognitoError::ResourceNotFound("User pool not found".to_string()))?;

        pool_entry.write().users.remove(&req.username);
        Ok(())
    }

    pub fn list_users(&self, req: ListUsersRequest) -> Result<ListUsersResponse, CognitoError> {
        let pool_entry = self
            .user_pools
            .get(&req.user_pool_id)
            .ok_or_else(|| CognitoError::ResourceNotFound("User pool not found".to_string()))?;

        let pool = pool_entry.read();
        let mut users = Vec::new();
        for u in pool.users.values() {
            let mut attr_vec: Vec<AttributeType> = u
                .attributes
                .iter()
                .map(|(k, v)| AttributeType {
                    name: k.clone(),
                    value: Some(v.clone()),
                })
                .collect();
            attr_vec.sort_by(|a, b| a.name.cmp(&b.name));

            users.push(UserType {
                username: u.username.clone(),
                attributes: attr_vec,
                user_create_date: u.created_at,
                user_last_modified_date: u.modified_at,
                enabled: u.enabled,
                user_status: u.status.clone(),
            });
        }

        users.sort_by(|a, b| a.username.cmp(&b.username));
        let limit = req.limit.unwrap_or(60);
        if users.len() > limit {
            users.truncate(limit);
        }

        Ok(ListUsersResponse {
            users,
            pagination_token: None,
        })
    }

    pub fn initiate_auth(&self, req: InitiateAuthRequest) -> Result<InitiateAuthResponse, CognitoError> {
        let pool_id = self
            .client_to_pool
            .get(&req.client_id)
            .ok_or_else(|| CognitoError::ResourceNotFound("Client not found".to_string()))?
            .clone();

        let params = req.auth_parameters.unwrap_or_default();
        let username = params
            .get("USERNAME")
            .cloned()
            .ok_or_else(|| CognitoError::InvalidParameter("Missing USERNAME in auth parameters".to_string()))?;
        let password = params
            .get("PASSWORD")
            .cloned()
            .ok_or_else(|| CognitoError::InvalidParameter("Missing PASSWORD in auth parameters".to_string()))?;

        let pool_entry = self.user_pools.get(&pool_id).unwrap();
        let pool = pool_entry.read();

        let user = pool
            .users
            .get(&username)
            .ok_or_else(|| CognitoError::UserNotFound("User not found".to_string()))?;

        if user.password_hash != Self::hash_password(&password) {
            return Err(CognitoError::NotAuthorized("Incorrect username or password".to_string()));
        }

        let email = user.attributes.get("email").map(|s| s.as_str());
        let access_token = self.generate_jwt(&pool_id, &req.client_id, &username, &user.sub, email, "access");
        let id_token = self.generate_jwt(&pool_id, &req.client_id, &username, &user.sub, email, "id");
        let refresh_token = Uuid::new_v4().to_string();

        Ok(InitiateAuthResponse {
            authentication_result: Some(AuthenticationResultType {
                access_token: Some(access_token),
                expires_in: Some(3600),
                token_type: Some("Bearer".to_string()),
                refresh_token: Some(refresh_token),
                id_token: Some(id_token),
            }),
            challenge_name: None,
            challenge_parameters: None,
            session: None,
        })
    }

    pub fn admin_initiate_auth(
        &self,
        req: AdminInitiateAuthRequest,
    ) -> Result<AdminInitiateAuthResponse, CognitoError> {
        let init_req = InitiateAuthRequest {
            auth_flow: req.auth_flow,
            client_id: req.client_id,
            auth_parameters: req.auth_parameters,
        };
        let res = self.initiate_auth(init_req)?;
        Ok(AdminInitiateAuthResponse {
            authentication_result: res.authentication_result,
            challenge_name: res.challenge_name,
            challenge_parameters: res.challenge_parameters,
            session: res.session,
        })
    }

    pub fn export_snapshot(&self) -> CognitoStateSnapshot {
        let mut map = HashMap::new();
        for item in self.user_pools.iter() {
            let pool = item.value().read().clone();
            map.insert(item.key().clone(), pool);
        }
        CognitoStateSnapshot { user_pools: map }
    }

    pub fn import_snapshot(&self, snapshot: CognitoStateSnapshot) {
        self.user_pools.clear();
        self.client_to_pool.clear();
        for (k, v) in snapshot.user_pools {
            for cid in v.clients.keys() {
                self.client_to_pool.insert(cid.clone(), k.clone());
            }
            self.user_pools.insert(k, Arc::new(RwLock::new(v)));
        }
    }

    pub fn reset(&self) {
        self.user_pools.clear();
        self.client_to_pool.clear();
    }
}

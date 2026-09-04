use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserStatusType {
    Unconfirmed,
    Confirmed,
    Archived,
    Compromised,
    Unknown,
    ResetRequired,
    ForceChangePassword,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AttributeType {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserType {
    pub username: String,
    pub attributes: Vec<AttributeType>,
    pub user_create_date: f64,
    pub user_last_modified_date: f64,
    pub enabled: bool,
    pub user_status: UserStatusType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserPoolType {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub status: Option<String>,
    pub last_modified_date: f64,
    pub creation_date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserPoolClientType {
    pub user_pool_id: String,
    pub client_id: String,
    pub client_name: String,
    pub client_secret: Option<String>,
    pub last_modified_date: f64,
    pub creation_date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthenticationResultType {
    pub access_token: Option<String>,
    pub expires_in: Option<i64>,
    pub token_type: Option<String>,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
}

// ----------------------------------------------------------------------------
// Requests / Responses
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateUserPoolRequest {
    pub pool_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateUserPoolResponse {
    pub user_pool: UserPoolType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeUserPoolRequest {
    pub user_pool_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeUserPoolResponse {
    pub user_pool: UserPoolType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ListUserPoolsRequest {
    pub max_results: Option<usize>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserPoolDescriptionType {
    pub id: String,
    pub name: String,
    pub last_modified_date: f64,
    pub creation_date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ListUserPoolsResponse {
    pub user_pools: Vec<UserPoolDescriptionType>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeleteUserPoolRequest {
    pub user_pool_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateUserPoolClientRequest {
    pub user_pool_id: String,
    pub client_name: String,
    pub generate_secret: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateUserPoolClientResponse {
    pub user_pool_client: UserPoolClientType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeUserPoolClientRequest {
    pub user_pool_id: String,
    pub client_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeUserPoolClientResponse {
    pub user_pool_client: UserPoolClientType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListUserPoolClientsRequest {
    pub user_pool_id: String,
    pub max_results: Option<usize>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserPoolClientDescription {
    pub client_id: String,
    pub client_name: String,
    pub user_pool_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ListUserPoolClientsResponse {
    pub user_pool_clients: Vec<UserPoolClientDescription>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeleteUserPoolClientRequest {
    pub user_pool_id: String,
    pub client_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SignUpRequest {
    pub client_id: String,
    pub username: String,
    pub password: String,
    pub user_attributes: Option<Vec<AttributeType>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SignUpResponse {
    pub user_confirmed: bool,
    pub user_sub: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfirmSignUpRequest {
    pub client_id: String,
    pub username: String,
    pub confirmation_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdminCreateUserRequest {
    pub user_pool_id: String,
    pub username: String,
    pub user_attributes: Option<Vec<AttributeType>>,
    pub temporary_password: Option<String>,
    pub message_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdminCreateUserResponse {
    pub user: UserType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdminGetUserRequest {
    pub user_pool_id: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdminGetUserResponse {
    pub username: String,
    pub user_attributes: Vec<AttributeType>,
    pub user_create_date: f64,
    pub user_last_modified_date: f64,
    pub enabled: bool,
    pub user_status: UserStatusType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdminSetUserPasswordRequest {
    pub user_pool_id: String,
    pub username: String,
    pub password: String,
    pub permanent: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdminDeleteUserRequest {
    pub user_pool_id: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListUsersRequest {
    pub user_pool_id: String,
    pub limit: Option<usize>,
    pub pagination_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ListUsersResponse {
    pub users: Vec<UserType>,
    pub pagination_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InitiateAuthRequest {
    pub auth_flow: String,
    pub client_id: String,
    pub auth_parameters: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InitiateAuthResponse {
    pub authentication_result: Option<AuthenticationResultType>,
    pub challenge_name: Option<String>,
    pub challenge_parameters: Option<HashMap<String, String>>,
    pub session: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdminInitiateAuthRequest {
    pub user_pool_id: String,
    pub client_id: String,
    pub auth_flow: String,
    pub auth_parameters: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdminInitiateAuthResponse {
    pub authentication_result: Option<AuthenticationResultType>,
    pub challenge_name: Option<String>,
    pub challenge_parameters: Option<HashMap<String, String>>,
    pub session: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetUserRequest {
    pub access_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetUserResponse {
    pub username: String,
    pub user_attributes: Vec<AttributeType>,
}

// ----------------------------------------------------------------------------
// Snapshot models
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredUser {
    pub username: String,
    pub password_hash: String,
    pub attributes: HashMap<String, String>,
    pub sub: String,
    pub enabled: bool,
    pub status: UserStatusType,
    pub created_at: f64,
    pub modified_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredUserPoolClient {
    pub client_id: String,
    pub client_name: String,
    pub client_secret: Option<String>,
    pub created_at: f64,
    pub modified_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredUserPool {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub created_at: f64,
    pub modified_at: f64,
    pub clients: HashMap<String, StoredUserPoolClient>,
    pub users: HashMap<String, StoredUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CognitoStateSnapshot {
    pub user_pools: HashMap<String, StoredUserPool>,
}

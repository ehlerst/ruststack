use crate::state::{IamError, IamState};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use std::collections::HashMap;

pub async fn handle_iam_request(
    State(state): State<IamState>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let mut params: HashMap<String, String> = HashMap::new();

    // Check query params
    if let Some(query) = uri.query() {
        for (k, v) in form_urlencoded::parse(query.as_bytes()) {
            params.insert(k.into_owned(), v.into_owned());
        }
    }

    // Check body params
    if !body.is_empty() {
        if let Ok(json_val) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(obj) = json_val.as_object() {
                for (k, v) in obj {
                    if let Some(s) = v.as_str() {
                        params.insert(k.clone(), s.to_string());
                    }
                }
            }
        } else {
            for (k, v) in form_urlencoded::parse(&body) {
                params.insert(k.into_owned(), v.into_owned());
            }
        }
    }

    let action = params.get("Action").cloned().unwrap_or_else(|| {
        headers
            .get("x-amz-target")
            .and_then(|v| v.to_str().ok())
            .map(|t| t.split('.').last().unwrap_or(t).to_string())
            .unwrap_or_default()
    });

    match action.as_str() {
        "CreateRole" => {
            let role_name = match params.get("RoleName") {
                Some(n) => n.clone(),
                None => return error_response(StatusCode::BAD_REQUEST, "ValidationError", "Missing RoleName"),
            };
            let doc = params.get("AssumeRolePolicyDocument").cloned().unwrap_or_default();
            let path = params.get("Path").cloned();
            let desc = params.get("Description").cloned();

            match state.create_role(role_name, doc, path, desc) {
                Ok(role) => xml_response(StatusCode::OK, &format!(
                    r#"<CreateRoleResponse xmlns="https://iam.amazonaws.com/doc/2010-05-08/">
  <CreateRoleResult>
    <Role>
      <Path>{}</Path>
      <RoleName>{}</RoleName>
      <RoleId>{}</RoleId>
      <Arn>{}</Arn>
      <CreateDate>{}</CreateDate>
      <AssumeRolePolicyDocument>{}</AssumeRolePolicyDocument>
    </Role>
  </CreateRoleResult>
  <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</CreateRoleResponse>"#,
                    role.path, role.role_name, role.role_id, role.arn, role.create_date,
                    quick_xml::escape::escape(&role.assume_role_policy_document),
                    uuid::Uuid::new_v4()
                )),
                Err(e) => map_iam_error(e),
            }
        }
        "GetRole" => {
            let role_name = match params.get("RoleName") {
                Some(n) => n,
                None => return error_response(StatusCode::BAD_REQUEST, "ValidationError", "Missing RoleName"),
            };
            match state.get_role(role_name) {
                Ok(role) => xml_response(StatusCode::OK, &format!(
                    r#"<GetRoleResponse xmlns="https://iam.amazonaws.com/doc/2010-05-08/">
  <GetRoleResult>
    <Role>
      <Path>{}</Path>
      <RoleName>{}</RoleName>
      <RoleId>{}</RoleId>
      <Arn>{}</Arn>
      <CreateDate>{}</CreateDate>
      <AssumeRolePolicyDocument>{}</AssumeRolePolicyDocument>
    </Role>
  </GetRoleResult>
  <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</GetRoleResponse>"#,
                    role.path, role.role_name, role.role_id, role.arn, role.create_date,
                    quick_xml::escape::escape(&role.assume_role_policy_document),
                    uuid::Uuid::new_v4()
                )),
                Err(e) => map_iam_error(e),
            }
        }
        "DeleteRole" => {
            let role_name = match params.get("RoleName") {
                Some(n) => n,
                None => return error_response(StatusCode::BAD_REQUEST, "ValidationError", "Missing RoleName"),
            };
            match state.delete_role(role_name) {
                Ok(()) => xml_response(StatusCode::OK, &format!(
                    r#"<DeleteRoleResponse xmlns="https://iam.amazonaws.com/doc/2010-05-08/"><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></DeleteRoleResponse>"#,
                    uuid::Uuid::new_v4()
                )),
                Err(e) => map_iam_error(e),
            }
        }
        "ListRoles" => {
            let roles = state.list_roles();
            let mut members = String::new();
            for r in roles {
                members.push_str(&format!(
                    "<member><RoleName>{}</RoleName><RoleId>{}</RoleId><Arn>{}</Arn><Path>{}</Path><CreateDate>{}</CreateDate></member>",
                    r.role_name, r.role_id, r.arn, r.path, r.create_date
                ));
            }
            xml_response(StatusCode::OK, &format!(
                r#"<ListRolesResponse xmlns="https://iam.amazonaws.com/doc/2010-05-08/">
  <ListRolesResult>
    <Roles>{}</Roles>
    <IsTruncated>false</IsTruncated>
  </ListRolesResult>
  <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</ListRolesResponse>"#,
                members, uuid::Uuid::new_v4()
            ))
        }
        "CreatePolicy" => {
            let policy_name = match params.get("PolicyName") {
                Some(n) => n.clone(),
                None => return error_response(StatusCode::BAD_REQUEST, "ValidationError", "Missing PolicyName"),
            };
            let doc = params.get("PolicyDocument").cloned().unwrap_or_default();
            let path = params.get("Path").cloned();
            let desc = params.get("Description").cloned();

            match state.create_policy(policy_name, doc, path, desc) {
                Ok(policy) => xml_response(StatusCode::OK, &format!(
                    r#"<CreatePolicyResponse xmlns="https://iam.amazonaws.com/doc/2010-05-08/">
  <CreatePolicyResult>
    <Policy>
      <PolicyName>{}</PolicyName>
      <PolicyId>{}</PolicyId>
      <Arn>{}</Arn>
      <Path>{}</Path>
      <DefaultVersionId>{}</DefaultVersionId>
      <AttachmentCount>0</AttachmentCount>
      <CreateDate>{}</CreateDate>
      <UpdateDate>{}</UpdateDate>
    </Policy>
  </CreatePolicyResult>
  <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</CreatePolicyResponse>"#,
                    policy.policy_name, policy.policy_id, policy.arn, policy.path, policy.default_version_id, policy.create_date, policy.update_date,
                    uuid::Uuid::new_v4()
                )),
                Err(e) => map_iam_error(e),
            }
        }
        "GetPolicy" => {
            let policy_arn = match params.get("PolicyArn") {
                Some(n) => n,
                None => return error_response(StatusCode::BAD_REQUEST, "ValidationError", "Missing PolicyArn"),
            };
            match state.get_policy(policy_arn) {
                Ok(policy) => xml_response(StatusCode::OK, &format!(
                    r#"<GetPolicyResponse xmlns="https://iam.amazonaws.com/doc/2010-05-08/">
  <GetPolicyResult>
    <Policy>
      <PolicyName>{}</PolicyName>
      <PolicyId>{}</PolicyId>
      <Arn>{}</Arn>
      <Path>{}</Path>
      <DefaultVersionId>{}</DefaultVersionId>
      <AttachmentCount>{}</AttachmentCount>
      <CreateDate>{}</CreateDate>
      <UpdateDate>{}</UpdateDate>
    </Policy>
  </GetPolicyResult>
  <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</GetPolicyResponse>"#,
                    policy.policy_name, policy.policy_id, policy.arn, policy.path, policy.default_version_id, policy.attachment_count, policy.create_date, policy.update_date,
                    uuid::Uuid::new_v4()
                )),
                Err(e) => map_iam_error(e),
            }
        }
        "AttachRolePolicy" => {
            let role_name = match params.get("RoleName") {
                Some(n) => n,
                None => return error_response(StatusCode::BAD_REQUEST, "ValidationError", "Missing RoleName"),
            };
            let policy_arn = match params.get("PolicyArn") {
                Some(n) => n,
                None => return error_response(StatusCode::BAD_REQUEST, "ValidationError", "Missing PolicyArn"),
            };
            match state.attach_role_policy(role_name, policy_arn) {
                Ok(()) => xml_response(StatusCode::OK, &format!(
                    r#"<AttachRolePolicyResponse xmlns="https://iam.amazonaws.com/doc/2010-05-08/"><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></AttachRolePolicyResponse>"#,
                    uuid::Uuid::new_v4()
                )),
                Err(e) => map_iam_error(e),
            }
        }
        "DetachRolePolicy" => {
            let role_name = match params.get("RoleName") {
                Some(n) => n,
                None => return error_response(StatusCode::BAD_REQUEST, "ValidationError", "Missing RoleName"),
            };
            let policy_arn = match params.get("PolicyArn") {
                Some(n) => n,
                None => return error_response(StatusCode::BAD_REQUEST, "ValidationError", "Missing PolicyArn"),
            };
            match state.detach_role_policy(role_name, policy_arn) {
                Ok(()) => xml_response(StatusCode::OK, &format!(
                    r#"<DetachRolePolicyResponse xmlns="https://iam.amazonaws.com/doc/2010-05-08/"><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></DetachRolePolicyResponse>"#,
                    uuid::Uuid::new_v4()
                )),
                Err(e) => map_iam_error(e),
            }
        }
        "ListAttachedRolePolicies" => {
            let role_name = match params.get("RoleName") {
                Some(n) => n,
                None => return error_response(StatusCode::BAD_REQUEST, "ValidationError", "Missing RoleName"),
            };
            match state.list_attached_role_policies(role_name) {
                Ok(policies) => {
                    let mut members = String::new();
                    for (name, arn) in policies {
                        members.push_str(&format!(
                            "<member><PolicyName>{}</PolicyName><PolicyArn>{}</PolicyArn></member>",
                            name, arn
                        ));
                    }
                    xml_response(StatusCode::OK, &format!(
                        r#"<ListAttachedRolePoliciesResponse xmlns="https://iam.amazonaws.com/doc/2010-05-08/">
  <ListAttachedRolePoliciesResult>
    <AttachedPolicies>{}</AttachedPolicies>
    <IsTruncated>false</IsTruncated>
  </ListAttachedRolePoliciesResult>
  <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</ListAttachedRolePoliciesResponse>"#,
                        members, uuid::Uuid::new_v4()
                    ))
                }
                Err(e) => map_iam_error(e),
            }
        }
        "CreateUser" => {
            let user_name = match params.get("UserName") {
                Some(n) => n.clone(),
                None => return error_response(StatusCode::BAD_REQUEST, "ValidationError", "Missing UserName"),
            };
            let path = params.get("Path").cloned();
            match state.create_user(user_name, path) {
                Ok(user) => xml_response(StatusCode::OK, &format!(
                    r#"<CreateUserResponse xmlns="https://iam.amazonaws.com/doc/2010-05-08/">
  <CreateUserResult>
    <User>
      <Path>{}</Path>
      <UserName>{}</UserName>
      <UserId>{}</UserId>
      <Arn>{}</Arn>
      <CreateDate>{}</CreateDate>
    </User>
  </CreateUserResult>
  <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</CreateUserResponse>"#,
                    user.path, user.user_name, user.user_id, user.arn, user.create_date, uuid::Uuid::new_v4()
                )),
                Err(e) => map_iam_error(e),
            }
        }
        "GetUser" => {
            let user_name = params.get("UserName").map(|s| s.as_str()).unwrap_or("admin");
            match state.get_user(user_name) {
                Ok(user) => xml_response(StatusCode::OK, &format!(
                    r#"<GetUserResponse xmlns="https://iam.amazonaws.com/doc/2010-05-08/">
  <GetUserResult>
    <User>
      <Path>{}</Path>
      <UserName>{}</UserName>
      <UserId>{}</UserId>
      <Arn>{}</Arn>
      <CreateDate>{}</CreateDate>
    </User>
  </GetUserResult>
  <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</GetUserResponse>"#,
                    user.path, user.user_name, user.user_id, user.arn, user.create_date, uuid::Uuid::new_v4()
                )),
                Err(_) => {
                    // Fallback for default user
                    xml_response(StatusCode::OK, &format!(
                        r#"<GetUserResponse xmlns="https://iam.amazonaws.com/doc/2010-05-08/">
  <GetUserResult>
    <User>
      <Path>/</Path>
      <UserName>{}</UserName>
      <UserId>AIDA00000000000000000</UserId>
      <Arn>arn:aws:iam::000000000000:user/{}</Arn>
      <CreateDate>2026-01-01T00:00:00Z</CreateDate>
    </User>
  </GetUserResult>
  <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</GetUserResponse>"#,
                        user_name, user_name, uuid::Uuid::new_v4()
                    ))
                }
            }
        }
        "CreateAccessKey" => {
            let user_name = match params.get("UserName") {
                Some(n) => n,
                None => "admin",
            };
            let _ = state.create_user(user_name.to_string(), None);
            match state.create_access_key(user_name) {
                Ok(key) => xml_response(StatusCode::OK, &format!(
                    r#"<CreateAccessKeyResponse xmlns="https://iam.amazonaws.com/doc/2010-05-08/">
  <CreateAccessKeyResult>
    <AccessKey>
      <UserName>{}</UserName>
      <AccessKeyId>{}</AccessKeyId>
      <Status>Active</Status>
      <SecretAccessKey>{}</SecretAccessKey>
      <CreateDate>{}</CreateDate>
    </AccessKey>
  </CreateAccessKeyResult>
  <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</CreateAccessKeyResponse>"#,
                    key.user_name, key.access_key_id, key.secret_access_key, key.create_date, uuid::Uuid::new_v4()
                )),
                Err(e) => map_iam_error(e),
            }
        }
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("Unknown IAM action: {}", action),
        ),
    }
}

fn xml_response(status: StatusCode, xml: &str) -> Response {
    (
        status,
        [("content-type", "text/xml")],
        xml.to_string(),
    )
        .into_response()
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response {
    let xml = format!(
        r#"<ErrorResponse xmlns="https://iam.amazonaws.com/doc/2010-05-08/">
  <Error>
    <Type>Sender</Type>
    <Code>{}</Code>
    <Message>{}</Message>
  </Error>
  <RequestId>{}</RequestId>
</ErrorResponse>"#,
        code, message, uuid::Uuid::new_v4()
    );
    xml_response(status, &xml)
}

fn map_iam_error(err: IamError) -> Response {
    match err {
        IamError::NotFound(msg) => error_response(StatusCode::NOT_FOUND, "NoSuchEntity", &msg),
        IamError::AlreadyExists(msg) => error_response(StatusCode::CONFLICT, "EntityAlreadyExists", &msg),
        IamError::Validation(msg) => error_response(StatusCode::BAD_REQUEST, "ValidationError", &msg),
    }
}

use crate::types::{AssumedRoleUser, Credentials, GetCallerIdentityResult};
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use ruststack_core::RustStackError;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

pub struct StsEngine {
    pub account_id: String,
    pub region: String,
}

impl StsEngine {
    pub fn new(account_id: String, region: String) -> Self {
        Self { account_id, region }
    }

    pub fn get_caller_identity(&self, caller_arn_opt: Option<&str>) -> GetCallerIdentityResult {
        let arn = caller_arn_opt
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("arn:aws:iam::{}:root", self.account_id));
        let user_id = self.account_id.clone();
        GetCallerIdentityResult {
            account: self.account_id.clone(),
            arn,
            user_id,
        }
    }

    pub fn assume_role(
        &self,
        role_arn: &str,
        role_session_name: &str,
        duration_seconds: Option<i64>,
    ) -> (Credentials, AssumedRoleUser) {
        let duration = duration_seconds.unwrap_or(3600);
        let expiration = Utc::now() + Duration::seconds(duration);

        let access_key_id = format!(
            "ASIA{}",
            &uuid::Uuid::new_v4().to_string().replace('-', "")[..16].to_uppercase()
        );
        let secret_access_key = uuid::Uuid::new_v4().to_string();
        let session_token = format!("mock-sts-session-token-{}", uuid::Uuid::new_v4());

        let credentials = Credentials {
            access_key_id: access_key_id.clone(),
            secret_access_key,
            session_token,
            expiration,
        };

        let role_name = role_arn.split('/').next_back().unwrap_or("role");
        let assumed_role_id = format!("{}:{}", access_key_id, role_session_name);
        let assumed_arn = format!(
            "arn:aws:sts::{}:assumed-role/{}/{}",
            self.account_id, role_name, role_session_name
        );

        let user = AssumedRoleUser {
            arn: assumed_arn,
            assumed_role_id,
        };

        (credentials, user)
    }

    pub fn get_session_token(&self, duration_seconds: Option<i64>) -> Credentials {
        let duration = duration_seconds.unwrap_or(3600);
        let expiration = Utc::now() + Duration::seconds(duration);

        Credentials {
            access_key_id: format!(
                "ASIA{}",
                &uuid::Uuid::new_v4().to_string().replace('-', "")[..16].to_uppercase()
            ),
            secret_access_key: uuid::Uuid::new_v4().to_string(),
            session_token: format!("mock-session-token-{}", uuid::Uuid::new_v4()),
            expiration,
        }
    }
}

pub async fn handle_sts_request(engine: Arc<StsEngine>, req: Request<Body>) -> Response<Body> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let (parts, body) = req.into_parts();
    let method = parts.method;
    let uri = parts.uri;
    let headers = parts.headers;

    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return make_sts_error_response(
                &RustStackError::BadRequest(e.to_string()),
                &request_id,
                false,
            );
        }
    };

    let is_json = headers.get("x-amz-target").is_some()
        || headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|ct| ct.contains("json"))
            .unwrap_or(false);

    let result = if is_json {
        handle_sts_json(
            engine.as_ref(),
            &method,
            &uri,
            &headers,
            &body_bytes,
            &request_id,
        )
        .await
    } else {
        handle_sts_query(
            engine.as_ref(),
            &method,
            &uri,
            &headers,
            &body_bytes,
            &request_id,
        )
        .await
    };

    match result {
        Ok(res) => res,
        Err(err) => make_sts_error_response(&err, &request_id, is_json),
    }
}

pub fn make_sts_error_response(
    err: &RustStackError,
    request_id: &str,
    is_json: bool,
) -> Response<Body> {
    let status = err.status_code();
    if is_json {
        let json_err = err.to_sts_json();
        Response::builder()
            .status(status)
            .header("content-type", "application/x-amz-json-1.1")
            .header("x-amzn-requestid", request_id)
            .body(Body::from(json_err.to_string()))
            .unwrap()
    } else {
        let xml_err = err.to_sts_xml(request_id);
        Response::builder()
            .status(status)
            .header("content-type", "text/xml")
            .body(Body::from(xml_err))
            .unwrap()
    }
}

async fn handle_sts_query(
    engine: &StsEngine,
    _method: &Method,
    uri: &Uri,
    _headers: &HeaderMap,
    body: &[u8],
    request_id: &str,
) -> Result<Response<Body>, RustStackError> {
    let mut params = HashMap::new();

    if let Some(query) = uri.query() {
        for (k, v) in form_urlencoded::parse(query.as_bytes()) {
            params.insert(k.to_string(), v.to_string());
        }
    }

    if !body.is_empty() {
        for (k, v) in form_urlencoded::parse(body) {
            params.insert(k.to_string(), v.to_string());
        }
    }

    let action = params.get("Action").map(|s| s.as_str()).unwrap_or("");

    match action {
        "GetCallerIdentity" => {
            let res = engine.get_caller_identity(None);
            let xml = format!(
                r#"<GetCallerIdentityResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
    <GetCallerIdentityResult>
        <Arn>{}</Arn>
        <UserId>{}</UserId>
        <Account>{}</Account>
    </GetCallerIdentityResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</GetCallerIdentityResponse>"#,
                quick_xml::escape::escape(&res.arn),
                quick_xml::escape::escape(&res.user_id),
                quick_xml::escape::escape(&res.account),
                request_id
            );

            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/xml")
                .body(Body::from(xml))
                .map_err(|e| RustStackError::Internal(e.to_string()))
        }

        "AssumeRole" => {
            let role_arn = params.get("RoleArn").map(|s| s.as_str()).unwrap_or("");
            let role_session_name = params
                .get("RoleSessionName")
                .map(|s| s.as_str())
                .unwrap_or("session");
            let duration_seconds = params
                .get("DurationSeconds")
                .and_then(|s| s.parse::<i64>().ok());

            let (creds, user) = engine.assume_role(role_arn, role_session_name, duration_seconds);
            let xml = format!(
                r#"<AssumeRoleResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
    <AssumeRoleResult>
        <Credentials>
            <AccessKeyId>{}</AccessKeyId>
            <SecretAccessKey>{}</SecretAccessKey>
            <SessionToken>{}</SessionToken>
            <Expiration>{}</Expiration>
        </Credentials>
        <AssumedRoleUser>
            <Arn>{}</Arn>
            <AssumedRoleId>{}</AssumedRoleId>
        </AssumedRoleUser>
    </AssumeRoleResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</AssumeRoleResponse>"#,
                quick_xml::escape::escape(&creds.access_key_id),
                quick_xml::escape::escape(&creds.secret_access_key),
                quick_xml::escape::escape(&creds.session_token),
                creds.expiration.to_rfc3339(),
                quick_xml::escape::escape(&user.arn),
                quick_xml::escape::escape(&user.assumed_role_id),
                request_id
            );

            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/xml")
                .body(Body::from(xml))
                .map_err(|e| RustStackError::Internal(e.to_string()))
        }

        "GetSessionToken" => {
            let duration_seconds = params
                .get("DurationSeconds")
                .and_then(|s| s.parse::<i64>().ok());
            let creds = engine.get_session_token(duration_seconds);

            let xml = format!(
                r#"<GetSessionTokenResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
    <GetSessionTokenResult>
        <Credentials>
            <AccessKeyId>{}</AccessKeyId>
            <SecretAccessKey>{}</SecretAccessKey>
            <SessionToken>{}</SessionToken>
            <Expiration>{}</Expiration>
        </Credentials>
    </GetSessionTokenResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</GetSessionTokenResponse>"#,
                quick_xml::escape::escape(&creds.access_key_id),
                quick_xml::escape::escape(&creds.secret_access_key),
                quick_xml::escape::escape(&creds.session_token),
                creds.expiration.to_rfc3339(),
                request_id
            );

            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/xml")
                .body(Body::from(xml))
                .map_err(|e| RustStackError::Internal(e.to_string()))
        }

        _ => Err(RustStackError::sts_bad_request(
            "InvalidAction",
            format!("Action {} is not supported by STS.", action),
        )),
    }
}

async fn handle_sts_json(
    engine: &StsEngine,
    _method: &Method,
    _uri: &Uri,
    headers: &HeaderMap,
    body: &[u8],
    _request_id: &str,
) -> Result<Response<Body>, RustStackError> {
    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let action = target
        .strip_prefix("AWSSecurityTokenServiceV20110615.")
        .or_else(|| target.strip_prefix("STS."))
        .unwrap_or(target);

    let json_val: serde_json::Value = if body.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_slice(body).map_err(|e| RustStackError::BadRequest(e.to_string()))?
    };

    match action {
        "GetCallerIdentity" => {
            let res = engine.get_caller_identity(None);
            let resp = json!({
                "Account": res.account,
                "Arn": res.arn,
                "UserId": res.user_id
            });

            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(resp.to_string()))
                .map_err(|e| RustStackError::Internal(e.to_string()))
        }

        "AssumeRole" => {
            let role_arn = json_val
                .get("RoleArn")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let role_session_name = json_val
                .get("RoleSessionName")
                .and_then(|v| v.as_str())
                .unwrap_or("session");
            let duration_seconds = json_val.get("DurationSeconds").and_then(|v| v.as_i64());

            let (creds, user) = engine.assume_role(role_arn, role_session_name, duration_seconds);
            let resp = json!({
                "Credentials": {
                    "AccessKeyId": creds.access_key_id,
                    "SecretAccessKey": creds.secret_access_key,
                    "SessionToken": creds.session_token,
                    "Expiration": creds.expiration.to_rfc3339()
                },
                "AssumedRoleUser": {
                    "Arn": user.arn,
                    "AssumedRoleId": user.assumed_role_id
                }
            });

            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(resp.to_string()))
                .map_err(|e| RustStackError::Internal(e.to_string()))
        }

        _ => Err(RustStackError::sts_bad_request(
            "InvalidAction",
            format!("Action {} is not supported by STS.", action),
        )),
    }
}

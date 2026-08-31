use crate::state::{SesError, SesState};
use crate::types::*;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use quick_xml::escape::escape;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn handle_ses_request(
    State(state): State<SesState>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request_id = Uuid::new_v4().to_string();

    let is_json = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .map(|t| t.starts_with("SimpleEmailService") || t.starts_with("SES"))
        .unwrap_or(false)
        || headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|ct| ct.contains("application/x-amz-json") || ct.contains("application/json"))
            .unwrap_or(false);

    let mut params: HashMap<String, String> = HashMap::new();

    // 1. Query parameters
    if let Some(query) = uri.query() {
        for (k, v) in form_urlencoded::parse(query.as_bytes()) {
            params.insert(k.into_owned(), v.into_owned());
        }
    }

    // 2. Form url-encoded or JSON body
    if !body.is_empty() {
        if let Ok(json_val) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(obj) = json_val.as_object() {
                for (k, v) in obj {
                    if let Some(s) = v.as_str() {
                        params.insert(k.clone(), s.to_string());
                    } else if let Some(b) = v.as_bool() {
                        params.insert(k.clone(), b.to_string());
                    } else if let Some(n) = v.as_i64() {
                        params.insert(k.clone(), n.to_string());
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

    if action.is_empty() {
        return make_error_response(
            is_json,
            StatusCode::BAD_REQUEST,
            "MissingAction",
            "Missing Action parameter or x-amz-target header",
            &request_id,
        );
    }

    match action.as_str() {
        "SendEmail" => {
            let req = if is_json && !body.is_empty() {
                match serde_json::from_slice::<SendEmailRequest>(&body) {
                    Ok(r) => r,
                    Err(_) => parse_send_email_query(&params),
                }
            } else {
                parse_send_email_query(&params)
            };

            match state.send_email(req) {
                Ok(resp) => {
                    if is_json {
                        json_response(StatusCode::OK, json!({ "MessageId": resp.message_id }))
                    } else {
                        let xml = format!(
                            r#"<SendEmailResponse xmlns="http://ses.amazonaws.com/doc/2010-12-01/">
  <SendEmailResult>
    <MessageId>{}</MessageId>
  </SendEmailResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</SendEmailResponse>"#,
                            escape(&resp.message_id),
                            request_id
                        );
                        xml_response(StatusCode::OK, &xml)
                    }
                }
                Err(e) => map_ses_error(is_json, e, &request_id),
            }
        }

        "SendRawEmail" => {
            let req = if is_json && !body.is_empty() {
                match serde_json::from_slice::<SendRawEmailRequest>(&body) {
                    Ok(r) => r,
                    Err(_) => parse_send_raw_email_query(&params),
                }
            } else {
                parse_send_raw_email_query(&params)
            };

            match state.send_raw_email(req) {
                Ok(resp) => {
                    if is_json {
                        json_response(StatusCode::OK, json!({ "MessageId": resp.message_id }))
                    } else {
                        let xml = format!(
                            r#"<SendRawEmailResponse xmlns="http://ses.amazonaws.com/doc/2010-12-01/">
  <SendRawEmailResult>
    <MessageId>{}</MessageId>
  </SendRawEmailResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</SendRawEmailResponse>"#,
                            escape(&resp.message_id),
                            request_id
                        );
                        xml_response(StatusCode::OK, &xml)
                    }
                }
                Err(e) => map_ses_error(is_json, e, &request_id),
            }
        }

        "VerifyEmailIdentity" => {
            let email = match params.get("EmailAddress") {
                Some(e) => e.clone(),
                None => {
                    if is_json {
                        match serde_json::from_slice::<VerifyEmailIdentityRequest>(&body) {
                            Ok(r) => r.email_address,
                            Err(_) => {
                                return make_error_response(
                                    is_json,
                                    StatusCode::BAD_REQUEST,
                                    "InvalidParameterValue",
                                    "Missing EmailAddress",
                                    &request_id,
                                )
                            }
                        }
                    } else {
                        return make_error_response(
                            is_json,
                            StatusCode::BAD_REQUEST,
                            "InvalidParameterValue",
                            "Missing EmailAddress",
                            &request_id,
                        );
                    }
                }
            };

            match state.verify_email_identity(VerifyEmailIdentityRequest {
                email_address: email,
            }) {
                Ok(()) => {
                    if is_json {
                        json_response(StatusCode::OK, json!({}))
                    } else {
                        let xml = format!(
                            r#"<VerifyEmailIdentityResponse xmlns="http://ses.amazonaws.com/doc/2010-12-01/">
  <VerifyEmailIdentityResult/>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</VerifyEmailIdentityResponse>"#,
                            request_id
                        );
                        xml_response(StatusCode::OK, &xml)
                    }
                }
                Err(e) => map_ses_error(is_json, e, &request_id),
            }
        }

        "VerifyDomainIdentity" => {
            let domain = match params.get("Domain") {
                Some(d) => d.clone(),
                None => {
                    if is_json {
                        match serde_json::from_slice::<VerifyDomainIdentityRequest>(&body) {
                            Ok(r) => r.domain,
                            Err(_) => {
                                return make_error_response(
                                    is_json,
                                    StatusCode::BAD_REQUEST,
                                    "InvalidParameterValue",
                                    "Missing Domain",
                                    &request_id,
                                )
                            }
                        }
                    } else {
                        return make_error_response(
                            is_json,
                            StatusCode::BAD_REQUEST,
                            "InvalidParameterValue",
                            "Missing Domain",
                            &request_id,
                        );
                    }
                }
            };

            match state.verify_domain_identity(VerifyDomainIdentityRequest { domain }) {
                Ok(resp) => {
                    if is_json {
                        json_response(
                            StatusCode::OK,
                            json!({ "VerificationToken": resp.verification_token }),
                        )
                    } else {
                        let xml = format!(
                            r#"<VerifyDomainIdentityResponse xmlns="http://ses.amazonaws.com/doc/2010-12-01/">
  <VerifyDomainIdentityResult>
    <VerificationToken>{}</VerificationToken>
  </VerifyDomainIdentityResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</VerifyDomainIdentityResponse>"#,
                            escape(&resp.verification_token),
                            request_id
                        );
                        xml_response(StatusCode::OK, &xml)
                    }
                }
                Err(e) => map_ses_error(is_json, e, &request_id),
            }
        }

        "ListIdentities" => {
            let req = if is_json && !body.is_empty() {
                serde_json::from_slice::<ListIdentitiesRequest>(&body)
                    .unwrap_or_else(|_| parse_list_identities_query(&params))
            } else {
                parse_list_identities_query(&params)
            };

            match state.list_identities(req) {
                Ok(resp) => {
                    if is_json {
                        json_response(
                            StatusCode::OK,
                            json!({
                                "Identities": resp.identities,
                                "NextToken": resp.next_token
                            }),
                        )
                    } else {
                        let mut members = String::new();
                        for id in &resp.identities {
                            members.push_str(&format!("<member>{}</member>", escape(id)));
                        }
                        let xml = format!(
                            r#"<ListIdentitiesResponse xmlns="http://ses.amazonaws.com/doc/2010-12-01/">
  <ListIdentitiesResult>
    <Identities>{}</Identities>
  </ListIdentitiesResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</ListIdentitiesResponse>"#,
                            members, request_id
                        );
                        xml_response(StatusCode::OK, &xml)
                    }
                }
                Err(e) => map_ses_error(is_json, e, &request_id),
            }
        }

        "DeleteIdentity" => {
            let identity = match params.get("Identity") {
                Some(i) => i.clone(),
                None => {
                    if is_json {
                        match serde_json::from_slice::<DeleteIdentityRequest>(&body) {
                            Ok(r) => r.identity,
                            Err(_) => {
                                return make_error_response(
                                    is_json,
                                    StatusCode::BAD_REQUEST,
                                    "InvalidParameterValue",
                                    "Missing Identity",
                                    &request_id,
                                )
                            }
                        }
                    } else {
                        return make_error_response(
                            is_json,
                            StatusCode::BAD_REQUEST,
                            "InvalidParameterValue",
                            "Missing Identity",
                            &request_id,
                        );
                    }
                }
            };

            match state.delete_identity(DeleteIdentityRequest { identity }) {
                Ok(()) => {
                    if is_json {
                        json_response(StatusCode::OK, json!({}))
                    } else {
                        let xml = format!(
                            r#"<DeleteIdentityResponse xmlns="http://ses.amazonaws.com/doc/2010-12-01/">
  <DeleteIdentityResult/>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</DeleteIdentityResponse>"#,
                            request_id
                        );
                        xml_response(StatusCode::OK, &xml)
                    }
                }
                Err(e) => map_ses_error(is_json, e, &request_id),
            }
        }

        "GetSendQuota" => match state.get_send_quota() {
            Ok(resp) => {
                if is_json {
                    json_response(
                        StatusCode::OK,
                        json!({
                            "Max24HourSend": resp.max_24_hour_send,
                            "MaxSendRate": resp.max_send_rate,
                            "SentLast24Hours": resp.sent_last_24_hours
                        }),
                    )
                } else {
                    let xml = format!(
                        r#"<GetSendQuotaResponse xmlns="http://ses.amazonaws.com/doc/2010-12-01/">
  <GetSendQuotaResult>
    <Max24HourSend>{:.1}</Max24HourSend>
    <MaxSendRate>{:.1}</MaxSendRate>
    <SentLast24Hours>{:.1}</SentLast24Hours>
  </GetSendQuotaResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</GetSendQuotaResponse>"#,
                        resp.max_24_hour_send,
                        resp.max_send_rate,
                        resp.sent_last_24_hours,
                        request_id
                    );
                    xml_response(StatusCode::OK, &xml)
                }
            }
            Err(e) => map_ses_error(is_json, e, &request_id),
        },

        "GetSendStatistics" => match state.get_send_statistics() {
            Ok(resp) => {
                if is_json {
                    json_response(
                        StatusCode::OK,
                        json!({
                            "SendDataPoints": resp.send_data_points
                        }),
                    )
                } else {
                    let mut points = String::new();
                    for p in &resp.send_data_points {
                        points.push_str(&format!(
                                "<member><Timestamp>{}</Timestamp><DeliveryAttempts>{}</DeliveryAttempts><Bounces>{}</Bounces><Complaints>{}</Complaints><Rejects>{}</Rejects></member>",
                                escape(&p.timestamp), p.delivery_attempts, p.bounces, p.complaints, p.rejects
                            ));
                    }
                    let xml = format!(
                        r#"<GetSendStatisticsResponse xmlns="http://ses.amazonaws.com/doc/2010-12-01/">
  <GetSendStatisticsResult>
    <SendDataPoints>{}</SendDataPoints>
  </GetSendStatisticsResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</GetSendStatisticsResponse>"#,
                        points, request_id
                    );
                    xml_response(StatusCode::OK, &xml)
                }
            }
            Err(e) => map_ses_error(is_json, e, &request_id),
        },

        "GetIdentityVerificationAttributes" => {
            let identities = extract_member_list(&params, "Identities");
            let req = if identities.is_empty() && is_json && !body.is_empty() {
                serde_json::from_slice::<GetIdentityVerificationAttributesRequest>(&body)
                    .unwrap_or_default()
            } else {
                GetIdentityVerificationAttributesRequest { identities }
            };

            match state.get_identity_verification_attributes(req) {
                Ok(resp) => {
                    if is_json {
                        json_response(
                            StatusCode::OK,
                            json!({
                                "VerificationAttributes": resp.verification_attributes
                            }),
                        )
                    } else {
                        let mut entries = String::new();
                        for (k, v) in &resp.verification_attributes {
                            entries.push_str(&format!(
                                "<entry><key>{}</key><value><VerificationStatus>{}</VerificationStatus></value></entry>",
                                escape(k), escape(&v.verification_status)
                            ));
                        }
                        let xml = format!(
                            r#"<GetIdentityVerificationAttributesResponse xmlns="http://ses.amazonaws.com/doc/2010-12-01/">
  <GetIdentityVerificationAttributesResult>
    <VerificationAttributes>{}</VerificationAttributes>
  </GetIdentityVerificationAttributesResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</GetIdentityVerificationAttributesResponse>"#,
                            entries, request_id
                        );
                        xml_response(StatusCode::OK, &xml)
                    }
                }
                Err(e) => map_ses_error(is_json, e, &request_id),
            }
        }

        "ListVerifiedEmailAddresses" => {
            let req = ListIdentitiesRequest {
                identity_type: Some("EmailAddress".to_string()),
                max_items: None,
                next_token: None,
            };
            match state.list_identities(req) {
                Ok(resp) => {
                    if is_json {
                        json_response(
                            StatusCode::OK,
                            json!({ "VerifiedEmailAddresses": resp.identities }),
                        )
                    } else {
                        let mut members = String::new();
                        for id in &resp.identities {
                            members.push_str(&format!("<member>{}</member>", escape(id)));
                        }
                        let xml = format!(
                            r#"<ListVerifiedEmailAddressesResponse xmlns="http://ses.amazonaws.com/doc/2010-12-01/">
  <ListVerifiedEmailAddressesResult>
    <VerifiedEmailAddresses>{}</VerifiedEmailAddresses>
  </ListVerifiedEmailAddressesResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</ListVerifiedEmailAddressesResponse>"#,
                            members, request_id
                        );
                        xml_response(StatusCode::OK, &xml)
                    }
                }
                Err(e) => map_ses_error(is_json, e, &request_id),
            }
        }

        "GetAccount" => {
            if is_json {
                json_response(
                    StatusCode::OK,
                    json!({
                        "DedicatedIpAutoWarmupEnabled": true,
                        "EnforcementStatus": "HEALTHY",
                        "ProductionAccessEnabled": true,
                        "SendingEnabled": true
                    }),
                )
            } else {
                let xml = format!(
                    r#"<GetAccountResponse xmlns="http://ses.amazonaws.com/doc/2010-12-01/">
  <GetAccountResult>
    <DedicatedIpAutoWarmupEnabled>true</DedicatedIpAutoWarmupEnabled>
    <EnforcementStatus>HEALTHY</EnforcementStatus>
    <ProductionAccessEnabled>true</ProductionAccessEnabled>
    <SendingEnabled>true</SendingEnabled>
  </GetAccountResult>
  <ResponseMetadata>
    <RequestId>{}</RequestId>
  </ResponseMetadata>
</GetAccountResponse>"#,
                    request_id
                );
                xml_response(StatusCode::OK, &xml)
            }
        }

        _ => make_error_response(
            is_json,
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("The action {} is not valid for this endpoint.", action),
            &request_id,
        ),
    }
}

fn parse_send_email_query(params: &HashMap<String, String>) -> SendEmailRequest {
    let source = params.get("Source").cloned().unwrap_or_default();

    let to_addresses = extract_member_list(params, "Destination.ToAddresses");
    let cc_addresses = extract_member_list(params, "Destination.CcAddresses");
    let bcc_addresses = extract_member_list(params, "Destination.BccAddresses");

    let destination = Destination {
        to_addresses,
        cc_addresses,
        bcc_addresses,
    };

    let subject_data = params
        .get("Message.Subject.Data")
        .cloned()
        .unwrap_or_default();
    let subject_charset = params.get("Message.Subject.Charset").cloned();
    let subject = Content {
        data: subject_data,
        charset: subject_charset,
    };

    let text = params.get("Message.Body.Text.Data").map(|data| Content {
        data: data.clone(),
        charset: params.get("Message.Body.Text.Charset").cloned(),
    });

    let html = params.get("Message.Body.Html.Data").map(|data| Content {
        data: data.clone(),
        charset: params.get("Message.Body.Html.Charset").cloned(),
    });

    let body = BodyContent { text, html };
    let message = Message { subject, body };

    let reply_to = extract_member_list(params, "ReplyToAddresses");
    let reply_to_addresses = if reply_to.is_empty() {
        None
    } else {
        Some(reply_to)
    };
    let return_path = params.get("ReturnPath").cloned();

    SendEmailRequest {
        source,
        destination,
        message,
        reply_to_addresses,
        return_path,
        source_arn: params.get("SourceArn").cloned(),
        return_path_arn: params.get("ReturnPathArn").cloned(),
        tags: None,
        configuration_set_name: params.get("ConfigurationSetName").cloned(),
    }
}

fn parse_send_raw_email_query(params: &HashMap<String, String>) -> SendRawEmailRequest {
    let source = params.get("Source").cloned();
    let dests = extract_member_list(params, "Destinations");
    let destinations = if dests.is_empty() { None } else { Some(dests) };
    let raw_data = params.get("RawMessage.Data").cloned().unwrap_or_default();

    SendRawEmailRequest {
        source,
        destinations,
        raw_message: RawMessage { data: raw_data },
        from_arn: params.get("FromArn").cloned(),
        source_arn: params.get("SourceArn").cloned(),
        return_path_arn: params.get("ReturnPathArn").cloned(),
        tags: None,
        configuration_set_name: params.get("ConfigurationSetName").cloned(),
    }
}

fn parse_list_identities_query(params: &HashMap<String, String>) -> ListIdentitiesRequest {
    ListIdentitiesRequest {
        identity_type: params.get("IdentityType").cloned(),
        max_items: params.get("MaxItems").and_then(|s| s.parse().ok()),
        next_token: params.get("NextToken").cloned(),
    }
}

fn extract_member_list(params: &HashMap<String, String>, prefix: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut i = 1;
    loop {
        let k1 = format!("{}.member.{}", prefix, i);
        let k2 = format!("{}.{}", prefix, i);
        if let Some(v) = params.get(&k1).or_else(|| params.get(&k2)) {
            items.push(v.clone());
            i += 1;
        } else {
            break;
        }
    }
    if items.is_empty() {
        if let Some(v) = params.get(prefix) {
            items.push(v.clone());
        }
    }
    items
}

fn xml_response(status: StatusCode, xml: &str) -> Response {
    (
        status,
        [("content-type", "text/xml;charset=UTF-8")],
        xml.to_string(),
    )
        .into_response()
}

fn json_response(status: StatusCode, val: serde_json::Value) -> Response {
    (
        status,
        [("content-type", "application/x-amz-json-1.0")],
        val.to_string(),
    )
        .into_response()
}

fn make_error_response(
    is_json: bool,
    status: StatusCode,
    code: &str,
    message: &str,
    request_id: &str,
) -> Response {
    if is_json {
        json_response(
            status,
            json!({
                "__type": code,
                "message": message
            }),
        )
    } else {
        let xml = format!(
            r#"<ErrorResponse xmlns="http://ses.amazonaws.com/doc/2010-12-01/">
  <Error>
    <Type>Sender</Type>
    <Code>{}</Code>
    <Message>{}</Message>
  </Error>
  <RequestId>{}</RequestId>
</ErrorResponse>"#,
            escape(code),
            escape(message),
            request_id
        );
        xml_response(status, &xml)
    }
}

fn map_ses_error(is_json: bool, err: SesError, request_id: &str) -> Response {
    match err {
        SesError::NotFound(msg) => {
            make_error_response(is_json, StatusCode::NOT_FOUND, "NotFound", &msg, request_id)
        }
        SesError::AlreadyExists(msg) => make_error_response(
            is_json,
            StatusCode::CONFLICT,
            "AlreadyExists",
            &msg,
            request_id,
        ),
        SesError::Validation(msg) => make_error_response(
            is_json,
            StatusCode::BAD_REQUEST,
            "ValidationError",
            &msg,
            request_id,
        ),
        SesError::InvalidParameter(msg) => make_error_response(
            is_json,
            StatusCode::BAD_REQUEST,
            "InvalidParameterValue",
            &msg,
            request_id,
        ),
        SesError::MessageRejected(msg) => make_error_response(
            is_json,
            StatusCode::BAD_REQUEST,
            "MessageRejected",
            &msg,
            request_id,
        ),
    }
}

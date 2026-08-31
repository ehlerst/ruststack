use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri};
use bytes::Bytes;
use ruststack_ses::handlers::handle_ses_request;
use ruststack_ses::state::SesState;
use ruststack_ses::types::*;

#[tokio::test]
async fn test_ses_state_direct_lifecycle() {
    let state = SesState::new("000000000000", "us-east-1");

    // 1. Verify identities
    state
        .verify_email_identity(VerifyEmailIdentityRequest {
            email_address: "alice@example.com".to_string(),
        })
        .expect("verify email identity");

    state
        .verify_email_identity(VerifyEmailIdentityRequest {
            email_address: "bob@example.com".to_string(),
        })
        .expect("verify email identity");

    let dom_resp = state
        .verify_domain_identity(VerifyDomainIdentityRequest {
            domain: "example.org".to_string(),
        })
        .expect("verify domain identity");
    assert!(!dom_resp.verification_token.is_empty());

    // 2. List identities
    let all_identities = state
        .list_identities(ListIdentitiesRequest::default())
        .expect("list all identities");
    assert_eq!(all_identities.identities.len(), 3);
    assert!(all_identities
        .identities
        .contains(&"alice@example.com".to_string()));
    assert!(all_identities
        .identities
        .contains(&"bob@example.com".to_string()));
    assert!(all_identities
        .identities
        .contains(&"example.org".to_string()));

    let email_identities = state
        .list_identities(ListIdentitiesRequest {
            identity_type: Some("EmailAddress".to_string()),
            max_items: None,
            next_token: None,
        })
        .expect("list email identities");
    assert_eq!(email_identities.identities.len(), 2);
    assert!(!email_identities
        .identities
        .contains(&"example.org".to_string()));

    let domain_identities = state
        .list_identities(ListIdentitiesRequest {
            identity_type: Some("Domain".to_string()),
            max_items: None,
            next_token: None,
        })
        .expect("list domain identities");
    assert_eq!(domain_identities.identities.len(), 1);
    assert_eq!(domain_identities.identities[0], "example.org");

    // 3. Send email
    let send_resp = state
        .send_email(SendEmailRequest {
            source: "alice@example.com".to_string(),
            destination: Destination {
                to_addresses: vec!["recipient1@example.com".to_string()],
                cc_addresses: vec!["cc@example.com".to_string()],
                bcc_addresses: vec!["bcc@example.com".to_string()],
            },
            message: Message {
                subject: Content::from("Welcome to RustStack"),
                body: BodyContent {
                    text: Some(Content::from("Hello plain text!")),
                    html: Some(Content::from("<p>Hello HTML!</p>")),
                },
            },
            reply_to_addresses: Some(vec!["reply@example.com".to_string()]),
            return_path: Some("bounce@example.com".to_string()),
            source_arn: None,
            return_path_arn: None,
            tags: None,
            configuration_set_name: None,
        })
        .expect("send email");
    assert!(!send_resp.message_id.is_empty());

    // 4. Send raw email
    let raw_email_content = "From: alice@example.com\r\nTo: raw_recipient@example.com\r\nSubject: Raw Email Test\r\n\r\nRaw email body message";
    let raw_send_resp = state
        .send_raw_email(SendRawEmailRequest {
            source: None,
            destinations: None,
            raw_message: RawMessage {
                data: raw_email_content.to_string(),
            },
            from_arn: None,
            source_arn: None,
            return_path_arn: None,
            tags: None,
            configuration_set_name: None,
        })
        .expect("send raw email");
    assert!(!raw_send_resp.message_id.is_empty());

    // 5. Inspect sent emails
    let sent = state.get_sent_emails();
    assert_eq!(sent.len(), 2);

    let first = &sent[0];
    assert_eq!(first.source, "alice@example.com");
    assert_eq!(
        first.destinations,
        vec![
            "recipient1@example.com".to_string(),
            "cc@example.com".to_string(),
            "bcc@example.com".to_string()
        ]
    );
    assert_eq!(first.subject.as_deref(), Some("Welcome to RustStack"));
    assert_eq!(first.body_text.as_deref(), Some("Hello plain text!"));
    assert_eq!(first.body_html.as_deref(), Some("<p>Hello HTML!</p>"));
    assert!(first.raw_data.is_none());

    let second = &sent[1];
    assert_eq!(second.source, "alice@example.com");
    assert_eq!(
        second.destinations,
        vec!["raw_recipient@example.com".to_string()]
    );
    assert_eq!(second.subject.as_deref(), Some("Raw Email Test"));
    assert_eq!(second.body_text.as_deref(), Some("Raw email body message"));
    assert!(second.raw_data.is_some());

    // 6. Quota and Statistics
    let quota = state.get_send_quota().expect("get send quota");
    assert_eq!(quota.max_24_hour_send, 200.0);
    assert_eq!(quota.sent_last_24_hours, 2.0);

    let stats = state.get_send_statistics().expect("get send statistics");
    assert_eq!(stats.send_data_points.len(), 1);
    assert_eq!(stats.send_data_points[0].delivery_attempts, 2);

    // 7. Identity Verification Attributes
    let attrs = state
        .get_identity_verification_attributes(GetIdentityVerificationAttributesRequest {
            identities: vec![
                "alice@example.com".to_string(),
                "unknown@example.com".to_string(),
            ],
        })
        .expect("get identity verification attributes");
    assert_eq!(attrs.verification_attributes.len(), 1);
    assert_eq!(
        attrs
            .verification_attributes
            .get("alice@example.com")
            .unwrap()
            .verification_status,
        "Success"
    );

    // 8. Delete identity
    state
        .delete_identity(DeleteIdentityRequest {
            identity: "bob@example.com".to_string(),
        })
        .expect("delete identity");
    let after_delete = state
        .list_identities(ListIdentitiesRequest::default())
        .expect("list identities after delete");
    assert_eq!(after_delete.identities.len(), 2);
    assert!(!after_delete
        .identities
        .contains(&"bob@example.com".to_string()));

    // 9. Snapshot export, reset, import
    let snapshot = state.export_snapshot();
    assert_eq!(snapshot.identities.len(), 2);
    assert_eq!(snapshot.sent_emails.len(), 2);

    state.reset();
    assert_eq!(state.get_sent_emails().len(), 0);
    assert_eq!(
        state
            .list_identities(ListIdentitiesRequest::default())
            .unwrap()
            .identities
            .len(),
        0
    );

    state.import_snapshot(snapshot);
    assert_eq!(state.get_sent_emails().len(), 2);
    assert_eq!(
        state
            .list_identities(ListIdentitiesRequest::default())
            .unwrap()
            .identities
            .len(),
        2
    );
}

#[tokio::test]
async fn test_ses_http_query_protocol() {
    let state = SesState::new("000000000000", "us-east-1");

    // 1. VerifyEmailIdentity
    let uri: Uri = "/?Action=VerifyEmailIdentity&EmailAddress=sender%40test.com"
        .parse()
        .unwrap();
    let resp = handle_ses_request(State(state.clone()), uri, HeaderMap::new(), Bytes::new()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(resp.into_body())
        .await
        .unwrap()
        .to_bytes();
    let xml = String::from_utf8_lossy(&body);
    assert!(xml.contains("VerifyEmailIdentityResponse"));

    // 2. SendEmail via Form URL-Encoded POST
    let uri: Uri = "/".parse().unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(
        "content-type",
        HeaderValue::from_static("application/x-www-form-urlencoded"),
    );

    let form_body = form_urlencoded::Serializer::new(String::new())
        .append_pair("Action", "SendEmail")
        .append_pair("Source", "sender@test.com")
        .append_pair("Destination.ToAddresses.member.1", "recipient@test.com")
        .append_pair("Destination.CcAddresses.member.1", "copy@test.com")
        .append_pair("Message.Subject.Data", "Hello AWS SES")
        .append_pair("Message.Body.Text.Data", "Email body content")
        .finish();

    let resp = handle_ses_request(
        State(state.clone()),
        uri.clone(),
        headers.clone(),
        Bytes::from(form_body),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(resp.into_body())
        .await
        .unwrap()
        .to_bytes();
    let xml = String::from_utf8_lossy(&body);
    assert!(xml.contains("SendEmailResponse"));
    assert!(xml.contains("<MessageId>"));

    // 3. SendRawEmail via Form URL-Encoded POST
    let raw_content =
        "From: sender@test.com\r\nTo: raw@test.com\r\nSubject: Raw Subject\r\n\r\nRaw Content";
    let form_raw = form_urlencoded::Serializer::new(String::new())
        .append_pair("Action", "SendRawEmail")
        .append_pair("RawMessage.Data", raw_content)
        .finish();

    let resp = handle_ses_request(
        State(state.clone()),
        uri.clone(),
        headers.clone(),
        Bytes::from(form_raw),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(resp.into_body())
        .await
        .unwrap()
        .to_bytes();
    let xml = String::from_utf8_lossy(&body);
    assert!(xml.contains("SendRawEmailResponse"));

    // 4. ListIdentities
    let form_list = form_urlencoded::Serializer::new(String::new())
        .append_pair("Action", "ListIdentities")
        .finish();
    let resp = handle_ses_request(
        State(state.clone()),
        uri.clone(),
        headers.clone(),
        Bytes::from(form_list),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(resp.into_body())
        .await
        .unwrap()
        .to_bytes();
    let xml = String::from_utf8_lossy(&body);
    assert!(xml.contains("ListIdentitiesResponse"));
    assert!(xml.contains("<member>sender@test.com</member>"));

    // 5. GetSendQuota
    let form_quota = form_urlencoded::Serializer::new(String::new())
        .append_pair("Action", "GetSendQuota")
        .finish();
    let resp = handle_ses_request(
        State(state.clone()),
        uri.clone(),
        headers.clone(),
        Bytes::from(form_quota),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(resp.into_body())
        .await
        .unwrap()
        .to_bytes();
    let xml = String::from_utf8_lossy(&body);
    assert!(xml.contains("GetSendQuotaResponse"));
    assert!(xml.contains("<Max24HourSend>200.0</Max24HourSend>"));
    assert!(xml.contains("<SentLast24Hours>2.0</SentLast24Hours>"));

    // Verify history in state
    let sent = state.get_sent_emails();
    assert_eq!(sent.len(), 2);
}

#[tokio::test]
async fn test_ses_http_json_protocol() {
    let state = SesState::new("000000000000", "us-east-1");

    let uri: Uri = "/".parse().unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-amz-target",
        HeaderValue::from_static("SimpleEmailService.SendEmail"),
    );
    headers.insert(
        "content-type",
        HeaderValue::from_static("application/x-amz-json-1.0"),
    );

    let json_req = serde_json::json!({
        "Source": "json_sender@example.com",
        "Destination": {
            "ToAddresses": ["json_recipient@example.com"]
        },
        "Message": {
            "Subject": {
                "Data": "JSON Subject"
            },
            "Body": {
                "Text": {
                    "Data": "JSON Body Text"
                }
            }
        }
    });

    let resp = handle_ses_request(
        State(state.clone()),
        uri,
        headers,
        Bytes::from(json_req.to_string()),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(resp.into_body())
        .await
        .unwrap()
        .to_bytes();
    let json_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json_resp.get("MessageId").is_some());

    let sent = state.get_sent_emails();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].source, "json_sender@example.com");
    assert_eq!(sent[0].subject.as_deref(), Some("JSON Subject"));
}

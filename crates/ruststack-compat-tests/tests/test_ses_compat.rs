use axum::http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;

#[tokio::test]
async fn test_ses_identity_and_send_email_compat() {
    let client = RustStackTestClient::new();

    // 1. Verify Email Identity
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "VerifyEmailIdentity"),
                ("EmailAddress", "sender@example.com"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("VerifyEmailIdentityResponse"));

    // 2. List Identities
    let (status, body) = client
        .call_query("/", &[("Action", "ListIdentities")])
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("sender@example.com"));

    // 3. Send Email
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "SendEmail"),
                ("Source", "sender@example.com"),
                ("Destination.ToAddresses.member.1", "recipient@example.com"),
                ("Message.Subject.Data", "Hello from RustStack"),
                (
                    "Message.Body.Text.Data",
                    "Welcome to ultra-fast local AWS testing!",
                ),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("SendEmailResponse"));
    assert!(body.contains("MessageId"));
}

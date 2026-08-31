use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;

#[tokio::test]
async fn test_iam_query_protocol_lifecycle() {
    let client = RustStackTestClient::new();

    // 1. CreateRole
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateRole"),
                ("RoleName", "MyTestRole"),
                (
                    "AssumeRolePolicyDocument",
                    r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"ec2.amazonaws.com"},"Action":"sts:AssumeRole"}]}"#,
                ),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<RoleName>MyTestRole</RoleName>"));

    // 2. GetRole
    let (status, body) = client
        .call_query("/", &[("Action", "GetRole"), ("RoleName", "MyTestRole")])
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<RoleName>MyTestRole</RoleName>"));

    // 3. CreatePolicy
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreatePolicy"),
                ("PolicyName", "TestDynamoAccess"),
                (
                    "PolicyDocument",
                    r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Action":["dynamodb:*"],"Resource":"*"}]}"#,
                ),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<PolicyName>TestDynamoAccess</PolicyName>"));

    // 4. AttachRolePolicy
    let (status, _) = client
        .call_query(
            "/",
            &[
                ("Action", "AttachRolePolicy"),
                ("RoleName", "MyTestRole"),
                (
                    "PolicyArn",
                    "arn:aws:iam::000000000000:policy/TestDynamoAccess",
                ),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 5. ListAttachedRolePolicies
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "ListAttachedRolePolicies"),
                ("RoleName", "MyTestRole"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<PolicyName>TestDynamoAccess</PolicyName>"));

    // 6. CreateUser & CreateAccessKey
    let (status, body) = client
        .call_query(
            "/",
            &[("Action", "CreateUser"), ("UserName", "test-user-alice")],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<UserName>test-user-alice</UserName>"));

    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateAccessKey"),
                ("UserName", "test-user-alice"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<AccessKeyId>AKIA"));
    assert!(body.contains("<Status>Active</Status>"));
}

use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;

#[tokio::test]
async fn test_ec2_vpc_subnet_security_group_compat() {
    let client = RustStackTestClient::new();

    // 1. CreateVpc
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateVpc"),
                ("CidrBlock", "10.10.0.0/16"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<vpcId>"));

    let start = body.find("<vpcId>").unwrap() + 7;
    let end = body.find("</vpcId>").unwrap();
    let vpc_id = &body[start..end];

    // 2. CreateSecurityGroup
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateSecurityGroup"),
                ("GroupName", "compat-sg"),
                ("GroupDescription", "Compat Security Group"),
                ("VpcId", vpc_id),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<groupId>"));

    // 3. CreateKeyPair
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateKeyPair"),
                ("KeyName", "compat-keypair"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<keyPairId>"));
}

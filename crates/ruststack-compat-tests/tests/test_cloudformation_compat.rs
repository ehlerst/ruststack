use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;

#[tokio::test]
async fn test_cloudformation_template_deployment_compat() {
    let client = RustStackTestClient::new();

    let template = r#"
AWSTemplateFormatVersion: '2010-09-09'
Resources:
  DataBucket:
    Type: AWS::S3::Bucket
    Properties:
      BucketName: cfn-compat-bucket-xyz
"#;

    // 1. CreateStack
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateStack"),
                ("StackName", "app-stack-dev"),
                ("TemplateBody", template),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<StackId>"));

    // 2. DescribeStacks
    let (status, body) = client
        .call_query(
            "/",
            &[("Action", "DescribeStacks"), ("StackName", "app-stack-dev")],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<StackStatus>CREATE_COMPLETE</StackStatus>"));

    // 3. DescribeStackResources
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "DescribeStackResources"),
                ("StackName", "app-stack-dev"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<LogicalResourceId>DataBucket</LogicalResourceId>"));
}

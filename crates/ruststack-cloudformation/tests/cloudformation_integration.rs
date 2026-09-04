use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_cloudformation::{handle_cloudformation_request, CloudFormationState};
use ruststack_s3::{InMemoryStorage, S3Storage};
use std::sync::Arc;

#[tokio::test]
async fn test_cloudformation_stack_lifecycle_and_resource_provisioning() {
    let s3 = Arc::new(InMemoryStorage::new());
    let state = CloudFormationState::new("000000000000", "us-east-1");
    state.set_services(Some(s3.clone()), None, None, None, None);

    let template_yaml = r#"
AWSTemplateFormatVersion: '2010-09-09'
Description: App Infrastructure
Parameters:
  EnvName:
    Type: String
    Default: dev
Resources:
  AppBucket:
    Type: AWS::S3::Bucket
    Properties:
      BucketName: my-cfn-created-bucket
Outputs:
  BucketOutput:
    Value: !Ref AppBucket
    Description: Created S3 Bucket
"#;

    // 1. CreateStack
    let uri: Uri = "/".parse().unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(
        "content-type",
        "application/x-www-form-urlencoded".parse().unwrap(),
    );
    let body_str = format!(
        "Action=CreateStack&StackName=infra-stack&TemplateBody={}&Parameters.member.1.ParameterKey=EnvName&Parameters.member.1.ParameterValue=prod",
        form_urlencoded::byte_serialize(template_yaml.as_bytes()).collect::<String>()
    );
    let body = Bytes::from(body_str);

    let resp = handle_cloudformation_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let resp_str = std::str::from_utf8(&body_bytes).unwrap();
    assert!(resp_str.contains("<StackId>"));

    // Verify S3 bucket was actually created!
    let buckets = s3.list_buckets().unwrap();
    assert!(buckets.iter().any(|b| b.name == "my-cfn-created-bucket"));

    // 2. DescribeStacks
    let body = Bytes::from("Action=DescribeStacks&StackName=infra-stack");
    let resp = handle_cloudformation_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let resp_str = std::str::from_utf8(&body_bytes).unwrap();
    assert!(resp_str.contains("<StackStatus>CREATE_COMPLETE</StackStatus>"));
    assert!(resp_str.contains("<OutputValue>my-cfn-created-bucket</OutputValue>"));

    // 3. DescribeStackResources
    let body = Bytes::from("Action=DescribeStackResources&StackName=infra-stack");
    let resp = handle_cloudformation_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let resp_str = std::str::from_utf8(&body_bytes).unwrap();
    assert!(resp_str.contains("<LogicalResourceId>AppBucket</LogicalResourceId>"));

    // 4. DeleteStack
    let body = Bytes::from("Action=DeleteStack&StackName=infra-stack");
    let resp = handle_cloudformation_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

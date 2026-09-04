use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_ec2::{handle_ec2_request, Ec2State};

#[tokio::test]
async fn test_ec2_vpc_and_instance_lifecycle() {
    let state = Ec2State::new("000000000000".to_string(), "us-east-1".to_string());
    let uri: Uri = "/".parse().unwrap();
    let headers = HeaderMap::new();

    // 1. Create VPC
    let body = Bytes::from("Action=CreateVpc&CidrBlock=10.0.0.0/16");
    let resp = handle_ec2_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(xml.contains("<vpcId>"));

    // Extract VPC ID
    let start = xml.find("<vpcId>").unwrap() + 7;
    let end = xml.find("</vpcId>").unwrap();
    let vpc_id = &xml[start..end];

    // 2. Create Subnet
    let body = Bytes::from(format!("Action=CreateSubnet&VpcId={}&CidrBlock=10.0.1.0/24", vpc_id));
    let resp = handle_ec2_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(xml.contains("<subnetId>"));

    let start = xml.find("<subnetId>").unwrap() + 10;
    let end = xml.find("</subnetId>").unwrap();
    let subnet_id = &xml[start..end];

    // 3. Create Security Group & Authorize Ingress
    let body = Bytes::from(format!("Action=CreateSecurityGroup&GroupName=web-sg&GroupDescription=WebSG&VpcId={}", vpc_id));
    let resp = handle_ec2_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    let start = xml.find("<groupId>").unwrap() + 9;
    let end = xml.find("</groupId>").unwrap();
    let group_id = &xml[start..end];

    let body = Bytes::from(format!("Action=AuthorizeSecurityGroupIngress&GroupId={}&IpPermissions.1.IpProtocol=tcp&IpPermissions.1.FromPort=80&IpPermissions.1.ToPort=80", group_id));
    let resp = handle_ec2_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. Run Instance
    let body = Bytes::from(format!("Action=RunInstances&ImageId=ami-12345678&InstanceType=t3.micro&SubnetId={}&SecurityGroupId.1={}&MaxCount=1", subnet_id, group_id));
    let resp = handle_ec2_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(xml.contains("<instanceId>"));
}

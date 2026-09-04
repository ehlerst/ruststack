use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_elbv2::{handle_elbv2_request, Elbv2State};

#[tokio::test]
async fn test_elbv2_load_balancer_and_target_group_lifecycle() {
    let state = Elbv2State::new("000000000000".to_string(), "us-east-1".to_string());
    let uri: Uri = "/".parse().unwrap();
    let headers = HeaderMap::new();

    // 1. Create Target Group
    let body = Bytes::from("Action=CreateTargetGroup&Name=web-targets&Protocol=HTTP&Port=80&TargetType=instance");
    let resp = handle_elbv2_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(xml.contains("<TargetGroupArn>"));

    let start = xml.find("<TargetGroupArn>").unwrap() + 16;
    let end = xml.find("</TargetGroupArn>").unwrap();
    let tg_arn = &xml[start..end];

    // 2. Register Targets
    let body = Bytes::from(format!("Action=RegisterTargets&TargetGroupArn={}&Targets.member.1.Id=i-1234567890abcdef0&Targets.member.1.Port=80", tg_arn));
    let resp = handle_elbv2_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Create Load Balancer
    let body = Bytes::from("Action=CreateLoadBalancer&Name=web-alb&Subnets.member.1=subnet-1&Subnets.member.2=subnet-2&Type=application");
    let resp = handle_elbv2_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(xml.contains("<LoadBalancerArn>"));

    let start = xml.find("<LoadBalancerArn>").unwrap() + 17;
    let end = xml.find("</LoadBalancerArn>").unwrap();
    let lb_arn = &xml[start..end];

    // 4. Create Listener
    let body = Bytes::from(format!("Action=CreateListener&LoadBalancerArn={}&Port=80&Protocol=HTTP&DefaultActions.member.1.Type=forward&DefaultActions.member.1.TargetGroupArn={}", lb_arn, tg_arn));
    let resp = handle_elbv2_request(State(state.clone()), uri.clone(), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(xml.contains("<ListenerArn>"));
}

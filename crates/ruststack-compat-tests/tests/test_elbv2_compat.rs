use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;

#[tokio::test]
async fn test_elbv2_target_group_and_load_balancer_compat() {
    let client = RustStackTestClient::new();

    // 1. CreateTargetGroup
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateTargetGroup"),
                ("Name", "compat-tg"),
                ("Protocol", "HTTP"),
                ("Port", "80"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<TargetGroupArn>"));

    // 2. CreateLoadBalancer
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateLoadBalancer"),
                ("Name", "compat-alb"),
                ("Subnets.member.1", "subnet-01"),
                ("Subnets.member.2", "subnet-02"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<LoadBalancerArn>"));
}

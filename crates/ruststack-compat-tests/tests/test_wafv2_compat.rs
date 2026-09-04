use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_wafv2_web_acl_and_associations_compat() {
    let client = RustStackTestClient::new();

    // 1. Create WebACL
    let (status, body) = client
        .call_json(
            "AWSWAF_20190729.CreateWebACL",
            json!({
                "Name": "prod-waf",
                "Scope": "REGIONAL",
                "DefaultAction": { "Allow": {} },
                "VisibilityConfig": {
                    "SampledRequestsEnabled": true,
                    "CloudWatchMetricsEnabled": true,
                    "MetricName": "ProdWafMetric"
                }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let id = body["Summary"]["Id"].as_str().unwrap();
    let arn = body["Summary"]["Arn"].as_str().unwrap();

    // 2. Get WebACL
    let (status, body) = client
        .call_json(
            "AWSWAF_20190729.GetWebACL",
            json!({
                "Name": "prod-waf",
                "Id": id,
                "Scope": "REGIONAL"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["WebACL"]["Name"], "prod-waf");

    // 3. Associate with ALB ARN
    let alb_arn = "arn:aws:elasticloadbalancing:us-east-1:000000000000:loadbalancer/app/my-alb/50dc6c495c0c9188";
    let (status, _) = client
        .call_json(
            "AWSWAF_20190729.AssociateWebACL",
            json!({
                "WebACLArn": arn,
                "ResourceArn": alb_arn
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 4. Get WebACL for Resource
    let (status, body) = client
        .call_json(
            "AWSWAF_20190729.GetWebACLForResource",
            json!({
                "ResourceArn": alb_arn
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["WebACL"]["Name"], "prod-waf");
}

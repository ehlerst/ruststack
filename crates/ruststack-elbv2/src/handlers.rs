use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode, Uri};
use bytes::Bytes;
use std::collections::HashMap;
use uuid::Uuid;

use crate::state::Elbv2State;
use crate::types::*;

pub async fn handle_elbv2_request(
    State(state): State<Elbv2State>,
    uri: Uri,
    _headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let mut params = HashMap::new();

    // 1. Parse from URI query
    if let Some(query) = uri.query() {
        for (k, v) in form_urlencoded::parse(query.as_bytes()) {
            params.insert(k.to_string(), v.to_string());
        }
    }

    // 2. Parse from Form Body
    if !body.is_empty() {
        for (k, v) in form_urlencoded::parse(&body) {
            params.insert(k.to_string(), v.to_string());
        }
    }

    let action = params.get("Action").map(|s| s.as_str()).unwrap_or("");
    let request_id = Uuid::new_v4().to_string();

    match action {
        "CreateLoadBalancer" => {
            let name = params.get("Name").cloned().unwrap_or_else(|| "my-lb".to_string());
            let subnets = parse_indexed_list(&params, "Subnets.member");
            let sgs = parse_indexed_list(&params, "SecurityGroups.member");
            let scheme = params.get("Scheme").cloned();
            let lb_type = params.get("Type").cloned();
            let tags = parse_tags(&params);

            match state.create_load_balancer(name, subnets, sgs, scheme, lb_type, tags) {
                Ok(lb) => {
                    let xml = format!(
                        r#"<CreateLoadBalancerResponse xmlns="http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/">
    <CreateLoadBalancerResult>
        <LoadBalancers>
            <member>
                <LoadBalancerArn>{}</LoadBalancerArn>
                <DNSName>{}</DNSName>
                <LoadBalancerName>{}</LoadBalancerName>
                <Scheme>{}</Scheme>
                <VpcId>{}</VpcId>
                <State><Code>{}</Code></State>
                <Type>{}</Type>
            </member>
        </LoadBalancers>
    </CreateLoadBalancerResult>
    <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</CreateLoadBalancerResponse>"#,
                        lb.load_balancer_arn, lb.dns_name, lb.load_balancer_name, lb.scheme, lb.vpc_id.unwrap_or_default(), lb.state, lb.lb_type, request_id
                    );
                    xml_response(StatusCode::OK, xml)
                }
                Err(e) => elbv2_error_response(StatusCode::BAD_REQUEST, "DuplicateLoadBalancerName", &e.to_string(), &request_id),
            }
        }
        "DescribeLoadBalancers" => {
            let names = parse_indexed_list(&params, "Names.member");
            let arns = parse_indexed_list(&params, "LoadBalancerArns.member");
            let lbs = state.describe_load_balancers(
                if names.is_empty() { None } else { Some(names) },
                if arns.is_empty() { None } else { Some(arns) },
            );
            let mut members_xml = String::new();
            for lb in lbs {
                members_xml.push_str(&format!(
                    r#"<member>
        <LoadBalancerArn>{}</LoadBalancerArn>
        <DNSName>{}</DNSName>
        <LoadBalancerName>{}</LoadBalancerName>
        <Scheme>{}</Scheme>
        <VpcId>{}</VpcId>
        <State><Code>{}</Code></State>
        <Type>{}</Type>
    </member>"#,
                    lb.load_balancer_arn, lb.dns_name, lb.load_balancer_name, lb.scheme, lb.vpc_id.unwrap_or_default(), lb.state, lb.lb_type
                ));
            }
            let xml = format!(
                r#"<DescribeLoadBalancersResponse xmlns="http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/">
    <DescribeLoadBalancersResult>
        <LoadBalancers>{}</LoadBalancers>
    </DescribeLoadBalancersResult>
    <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</DescribeLoadBalancersResponse>"#,
                members_xml, request_id
            );
            xml_response(StatusCode::OK, xml)
        }
        "DeleteLoadBalancer" => {
            let arn = params.get("LoadBalancerArn").map(|s| s.as_str()).unwrap_or("");
            match state.delete_load_balancer(arn) {
                Ok(()) => {
                    let xml = format!(
                        r#"<DeleteLoadBalancerResponse xmlns="http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/">
    <DeleteLoadBalancerResult/>
    <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</DeleteLoadBalancerResponse>"#,
                        request_id
                    );
                    xml_response(StatusCode::OK, xml)
                }
                Err(e) => elbv2_error_response(StatusCode::BAD_REQUEST, "LoadBalancerNotFound", &e.to_string(), &request_id),
            }
        }
        "CreateTargetGroup" => {
            let name = params.get("Name").cloned().unwrap_or_else(|| "my-tg".to_string());
            let protocol = params.get("Protocol").cloned();
            let port = params.get("Port").and_then(|p| p.parse().ok());
            let vpc_id = params.get("VpcId").cloned();
            let target_type = params.get("TargetType").cloned();
            let tags = parse_tags(&params);

            match state.create_target_group(name, protocol, port, vpc_id, target_type, tags) {
                Ok(tg) => {
                    let xml = format!(
                        r#"<CreateTargetGroupResponse xmlns="http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/">
    <CreateTargetGroupResult>
        <TargetGroups>
            <member>
                <TargetGroupArn>{}</TargetGroupArn>
                <TargetGroupName>{}</TargetGroupName>
                <Protocol>{}</Protocol>
                <Port>{}</Port>
                <VpcId>{}</VpcId>
                <TargetType>{}</TargetType>
            </member>
        </TargetGroups>
    </CreateTargetGroupResult>
    <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</CreateTargetGroupResponse>"#,
                        tg.target_group_arn, tg.target_group_name, tg.protocol.unwrap_or_default(), tg.port.unwrap_or_default(), tg.vpc_id.unwrap_or_default(), tg.target_type, request_id
                    );
                    xml_response(StatusCode::OK, xml)
                }
                Err(e) => elbv2_error_response(StatusCode::BAD_REQUEST, "DuplicateTargetGroupName", &e.to_string(), &request_id),
            }
        }
        "DescribeTargetGroups" => {
            let names = parse_indexed_list(&params, "Names.member");
            let arns = parse_indexed_list(&params, "TargetGroupArns.member");
            let lb_arn = params.get("LoadBalancerArn").cloned();
            let tgs = state.describe_target_groups(
                if names.is_empty() { None } else { Some(names) },
                if arns.is_empty() { None } else { Some(arns) },
                lb_arn,
            );
            let mut members_xml = String::new();
            for tg in tgs {
                members_xml.push_str(&format!(
                    r#"<member>
        <TargetGroupArn>{}</TargetGroupArn>
        <TargetGroupName>{}</TargetGroupName>
        <Protocol>{}</Protocol>
        <Port>{}</Port>
        <VpcId>{}</VpcId>
        <TargetType>{}</TargetType>
    </member>"#,
                    tg.target_group_arn, tg.target_group_name, tg.protocol.unwrap_or_default(), tg.port.unwrap_or_default(), tg.vpc_id.unwrap_or_default(), tg.target_type
                ));
            }
            let xml = format!(
                r#"<DescribeTargetGroupsResponse xmlns="http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/">
    <DescribeTargetGroupsResult>
        <TargetGroups>{}</TargetGroups>
    </DescribeTargetGroupsResult>
    <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</DescribeTargetGroupsResponse>"#,
                members_xml, request_id
            );
            xml_response(StatusCode::OK, xml)
        }
        "DeleteTargetGroup" => {
            let tg_arn = params.get("TargetGroupArn").map(|s| s.as_str()).unwrap_or("");
            match state.delete_target_group(tg_arn) {
                Ok(()) => {
                    let xml = format!(
                        r#"<DeleteTargetGroupResponse xmlns="http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/">
    <DeleteTargetGroupResult/>
    <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</DeleteTargetGroupResponse>"#,
                        request_id
                    );
                    xml_response(StatusCode::OK, xml)
                }
                Err(e) => elbv2_error_response(StatusCode::BAD_REQUEST, "TargetGroupNotFound", &e.to_string(), &request_id),
            }
        }
        "RegisterTargets" => {
            let tg_arn = params.get("TargetGroupArn").map(|s| s.as_str()).unwrap_or("");
            let targets = parse_targets(&params);
            match state.register_targets(tg_arn, targets) {
                Ok(()) => {
                    let xml = format!(
                        r#"<RegisterTargetsResponse xmlns="http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/">
    <RegisterTargetsResult/>
    <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</RegisterTargetsResponse>"#,
                        request_id
                    );
                    xml_response(StatusCode::OK, xml)
                }
                Err(e) => elbv2_error_response(StatusCode::BAD_REQUEST, "TargetGroupNotFound", &e.to_string(), &request_id),
            }
        }
        "DeregisterTargets" => {
            let tg_arn = params.get("TargetGroupArn").map(|s| s.as_str()).unwrap_or("");
            let targets = parse_targets(&params);
            match state.deregister_targets(tg_arn, targets) {
                Ok(()) => {
                    let xml = format!(
                        r#"<DeregisterTargetsResponse xmlns="http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/">
    <DeregisterTargetsResult/>
    <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</DeregisterTargetsResponse>"#,
                        request_id
                    );
                    xml_response(StatusCode::OK, xml)
                }
                Err(e) => elbv2_error_response(StatusCode::BAD_REQUEST, "TargetGroupNotFound", &e.to_string(), &request_id),
            }
        }
        "DescribeTargetHealth" => {
            let tg_arn = params.get("TargetGroupArn").map(|s| s.as_str()).unwrap_or("");
            match state.describe_target_health(tg_arn) {
                Ok(healths) => {
                    let mut health_xml = String::new();
                    for h in healths {
                        health_xml.push_str(&format!(
                            r#"<member>
        <Target>
            <Id>{}</Id>
            <Port>{}</Port>
        </Target>
        <TargetHealth>
            <State>{}</State>
        </TargetHealth>
    </member>"#,
                            h.target.id, h.target.port.unwrap_or(80), h.target_health
                        ));
                    }
                    let xml = format!(
                        r#"<DescribeTargetHealthResponse xmlns="http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/">
    <DescribeTargetHealthResult>
        <TargetHealthDescriptions>{}</TargetHealthDescriptions>
    </DescribeTargetHealthResult>
    <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</DescribeTargetHealthResponse>"#,
                        health_xml, request_id
                    );
                    xml_response(StatusCode::OK, xml)
                }
                Err(e) => elbv2_error_response(StatusCode::BAD_REQUEST, "TargetGroupNotFound", &e.to_string(), &request_id),
            }
        }
        "CreateListener" => {
            let lb_arn = params.get("LoadBalancerArn").cloned().unwrap_or_default();
            let port = params.get("Port").and_then(|p| p.parse().ok()).unwrap_or(80);
            let protocol = params.get("Protocol").cloned().unwrap_or_else(|| "HTTP".to_string());
            let tg_arn = params.get("DefaultActions.member.1.TargetGroupArn").cloned();
            let action_type = params.get("DefaultActions.member.1.Type").cloned().unwrap_or_else(|| "forward".to_string());

            let actions = vec![ListenerAction {
                action_type,
                target_group_arn: tg_arn,
            }];

            match state.create_listener(lb_arn, port, protocol, actions) {
                Ok(l) => {
                    let xml = format!(
                        r#"<CreateListenerResponse xmlns="http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/">
    <CreateListenerResult>
        <Listeners>
            <member>
                <ListenerArn>{}</ListenerArn>
                <LoadBalancerArn>{}</LoadBalancerArn>
                <Port>{}</Port>
                <Protocol>{}</Protocol>
            </member>
        </Listeners>
    </CreateListenerResult>
    <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</CreateListenerResponse>"#,
                        l.listener_arn, l.load_balancer_arn, l.port, l.protocol, request_id
                    );
                    xml_response(StatusCode::OK, xml)
                }
                Err(e) => elbv2_error_response(StatusCode::BAD_REQUEST, "LoadBalancerNotFound", &e.to_string(), &request_id),
            }
        }
        "DescribeListeners" => {
            let lb_arn = params.get("LoadBalancerArn").cloned();
            let listener_arns = parse_indexed_list(&params, "ListenerArns.member");
            let listeners = state.describe_listeners(lb_arn, if listener_arns.is_empty() { None } else { Some(listener_arns) });

            let mut members_xml = String::new();
            for l in listeners {
                members_xml.push_str(&format!(
                    r#"<member>
        <ListenerArn>{}</ListenerArn>
        <LoadBalancerArn>{}</LoadBalancerArn>
        <Port>{}</Port>
        <Protocol>{}</Protocol>
    </member>"#,
                    l.listener_arn, l.load_balancer_arn, l.port, l.protocol
                ));
            }
            let xml = format!(
                r#"<DescribeListenersResponse xmlns="http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/">
    <DescribeListenersResult>
        <Listeners>{}</Listeners>
    </DescribeListenersResult>
    <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</DescribeListenersResponse>"#,
                members_xml, request_id
            );
            xml_response(StatusCode::OK, xml)
        }
        "DeleteListener" => {
            let arn = params.get("ListenerArn").map(|s| s.as_str()).unwrap_or("");
            match state.delete_listener(arn) {
                Ok(()) => {
                    let xml = format!(
                        r#"<DeleteListenerResponse xmlns="http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/">
    <DeleteListenerResult/>
    <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
</DeleteListenerResponse>"#,
                        request_id
                    );
                    xml_response(StatusCode::OK, xml)
                }
                Err(e) => elbv2_error_response(StatusCode::BAD_REQUEST, "ListenerNotFound", &e.to_string(), &request_id),
            }
        }
        _ => elbv2_error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("The action {} is not valid for Elastic Load Balancing v2.", action),
            &request_id,
        ),
    }
}

fn parse_indexed_list(params: &HashMap<String, String>, prefix: &str) -> Vec<String> {
    let mut items = Vec::new();
    if let Some(val) = params.get(prefix) {
        items.push(val.clone());
    }
    let mut i = 1;
    while let Some(val) = params.get(&format!("{}.{}", prefix, i)) {
        items.push(val.clone());
        i += 1;
    }
    items
}

fn parse_targets(params: &HashMap<String, String>) -> Vec<TargetDescription> {
    let mut targets = Vec::new();
    let mut i = 1;
    while let Some(id) = params.get(&format!("Targets.member.{}.Id", i)) {
        let port = params.get(&format!("Targets.member.{}.Port", i)).and_then(|p| p.parse().ok());
        let az = params.get(&format!("Targets.member.{}.AvailabilityZone", i)).cloned();
        targets.push(TargetDescription {
            id: id.clone(),
            port,
            availability_zone: az,
        });
        i += 1;
    }
    targets
}

fn parse_tags(params: &HashMap<String, String>) -> Vec<Tag> {
    let mut tags = Vec::new();
    let mut i = 1;
    while let Some(key) = params.get(&format!("Tags.member.{}.Key", i)) {
        let value = params.get(&format!("Tags.member.{}.Value", i)).cloned().unwrap_or_default();
        tags.push(Tag { key: key.clone(), value });
        i += 1;
    }
    tags
}

fn xml_response(status: StatusCode, body: String) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", "text/xml")
        .body(Body::from(body))
        .unwrap()
}

fn elbv2_error_response(status: StatusCode, code: &str, message: &str, request_id: &str) -> Response<Body> {
    let xml = format!(
        r#"<ErrorResponse xmlns="http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/"><Error><Code>{}</Code><Message>{}</Message></Error><RequestId>{}</RequestId></ErrorResponse>"#,
        code, message, request_id
    );
    xml_response(status, xml)
}

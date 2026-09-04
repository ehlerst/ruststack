use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode, Uri};
use bytes::Bytes;
use std::collections::HashMap;
use uuid::Uuid;

use crate::state::Ec2State;
use crate::types::*;

pub async fn handle_ec2_request(
    State(state): State<Ec2State>,
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
        "CreateVpc" => {
            let cidr = params.get("CidrBlock").cloned().unwrap_or_else(|| "10.0.0.0/16".to_string());
            let tags = parse_tags(&params);
            let vpc = state.create_vpc(cidr, tags);
            let xml = format!(
                r#"<CreateVpcResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <vpc>
        <vpcId>{}</vpcId>
        <cidrBlock>{}</cidrBlock>
        <state>{}</state>
        <isDefault>{}</isDefault>
    </vpc>
</CreateVpcResponse>"#,
                request_id, vpc.vpc_id, vpc.cidr_block, vpc.state, vpc.is_default
            );
            xml_response(StatusCode::OK, xml)
        }
        "DescribeVpcs" => {
            let ids = parse_indexed_list(&params, "VpcId");
            let vpcs = state.describe_vpcs(if ids.is_empty() { None } else { Some(ids) });
            let mut vpcs_xml = String::new();
            for v in vpcs {
                vpcs_xml.push_str(&format!(
                    r#"<item>
        <vpcId>{}</vpcId>
        <cidrBlock>{}</cidrBlock>
        <state>{}</state>
        <isDefault>{}</isDefault>
    </item>"#,
                    v.vpc_id, v.cidr_block, v.state, v.is_default
                ));
            }
            let xml = format!(
                r#"<DescribeVpcsResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <vpcSet>{}</vpcSet>
</DescribeVpcsResponse>"#,
                request_id, vpcs_xml
            );
            xml_response(StatusCode::OK, xml)
        }
        "DeleteVpc" => {
            let vpc_id = params.get("VpcId").map(|s| s.as_str()).unwrap_or("");
            match state.delete_vpc(vpc_id) {
                Ok(()) => {
                    let xml = format!(
                        r#"<DeleteVpcResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <return>true</return>
</DeleteVpcResponse>"#,
                        request_id
                    );
                    xml_response(StatusCode::OK, xml)
                }
                Err(e) => ec2_error_response(StatusCode::BAD_REQUEST, "InvalidVpcID.NotFound", &e.to_string(), &request_id),
            }
        }
        "CreateSubnet" => {
            let vpc_id = params.get("VpcId").cloned().unwrap_or_default();
            let cidr = params.get("CidrBlock").cloned().unwrap_or_else(|| "10.0.1.0/24".to_string());
            let az = params.get("AvailabilityZone").cloned();
            let tags = parse_tags(&params);
            match state.create_subnet(vpc_id, cidr, az, tags) {
                Ok(s) => {
                    let xml = format!(
                        r#"<CreateSubnetResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <subnet>
        <subnetId>{}</subnetId>
        <vpcId>{}</vpcId>
        <cidrBlock>{}</cidrBlock>
        <availabilityZone>{}</availabilityZone>
        <state>{}</state>
        <availableIpAddressCount>{}</availableIpAddressCount>
        <defaultForAz>{}</defaultForAz>
    </subnet>
</CreateSubnetResponse>"#,
                        request_id, s.subnet_id, s.vpc_id, s.cidr_block, s.availability_zone, s.state, s.available_ip_address_count, s.default_for_az
                    );
                    xml_response(StatusCode::OK, xml)
                }
                Err(e) => ec2_error_response(StatusCode::BAD_REQUEST, "InvalidVpcID.NotFound", &e.to_string(), &request_id),
            }
        }
        "DescribeSubnets" => {
            let ids = parse_indexed_list(&params, "SubnetId");
            let subnets = state.describe_subnets(if ids.is_empty() { None } else { Some(ids) });
            let mut subnets_xml = String::new();
            for s in subnets {
                subnets_xml.push_str(&format!(
                    r#"<item>
        <subnetId>{}</subnetId>
        <vpcId>{}</vpcId>
        <cidrBlock>{}</cidrBlock>
        <availabilityZone>{}</availabilityZone>
        <state>{}</state>
        <availableIpAddressCount>{}</availableIpAddressCount>
        <defaultForAz>{}</defaultForAz>
    </item>"#,
                    s.subnet_id, s.vpc_id, s.cidr_block, s.availability_zone, s.state, s.available_ip_address_count, s.default_for_az
                ));
            }
            let xml = format!(
                r#"<DescribeSubnetsResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <subnetSet>{}</subnetSet>
</DescribeSubnetsResponse>"#,
                request_id, subnets_xml
            );
            xml_response(StatusCode::OK, xml)
        }
        "DeleteSubnet" => {
            let subnet_id = params.get("SubnetId").map(|s| s.as_str()).unwrap_or("");
            match state.delete_subnet(subnet_id) {
                Ok(()) => {
                    let xml = format!(
                        r#"<DeleteSubnetResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <return>true</return>
</DeleteSubnetResponse>"#,
                        request_id
                    );
                    xml_response(StatusCode::OK, xml)
                }
                Err(e) => ec2_error_response(StatusCode::BAD_REQUEST, "InvalidSubnetID.NotFound", &e.to_string(), &request_id),
            }
        }
        "CreateSecurityGroup" => {
            let name = params.get("GroupName").cloned().unwrap_or_else(|| "default".to_string());
            let desc = params.get("GroupDescription").cloned().unwrap_or_else(|| "Security group".to_string());
            let vpc_id = params.get("VpcId").cloned();
            let tags = parse_tags(&params);
            let sg = state.create_security_group(name, desc, vpc_id, tags);
            let xml = format!(
                r#"<CreateSecurityGroupResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <return>true</return>
    <groupId>{}</groupId>
</CreateSecurityGroupResponse>"#,
                request_id, sg.group_id
            );
            xml_response(StatusCode::OK, xml)
        }
        "DescribeSecurityGroups" => {
            let ids = parse_indexed_list(&params, "GroupId");
            let names = parse_indexed_list(&params, "GroupName");
            let sgs = state.describe_security_groups(
                if ids.is_empty() { None } else { Some(ids) },
                if names.is_empty() { None } else { Some(names) },
            );
            let mut sgs_xml = String::new();
            for g in sgs {
                sgs_xml.push_str(&format!(
                    r#"<item>
        <groupId>{}</groupId>
        <groupName>{}</groupName>
        <groupDescription>{}</groupDescription>
        <vpcId>{}</vpcId>
    </item>"#,
                    g.group_id, g.group_name, g.description, g.vpc_id
                ));
            }
            let xml = format!(
                r#"<DescribeSecurityGroupsResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <securityGroupInfo>{}</securityGroupInfo>
</DescribeSecurityGroupsResponse>"#,
                request_id, sgs_xml
            );
            xml_response(StatusCode::OK, xml)
        }
        "AuthorizeSecurityGroupIngress" => {
            let group_id = params.get("GroupId").map(|s| s.as_str()).unwrap_or("");
            let protocol = params.get("IpPermissions.1.IpProtocol").cloned().unwrap_or_else(|| "tcp".to_string());
            let from_port = params.get("IpPermissions.1.FromPort").and_then(|p| p.parse().ok());
            let to_port = params.get("IpPermissions.1.ToPort").and_then(|p| p.parse().ok());
            let cidr = params.get("IpPermissions.1.IpRanges.1.CidrIp").cloned().unwrap_or_else(|| "0.0.0.0/0".to_string());

            let perm = IpPermission {
                ip_protocol: protocol,
                from_port,
                to_port,
                ip_ranges: vec![IpRange { cidr_ip: cidr, description: None }],
            };

            match state.authorize_security_group_ingress(group_id, vec![perm]) {
                Ok(()) => {
                    let xml = format!(
                        r#"<AuthorizeSecurityGroupIngressResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <return>true</return>
</AuthorizeSecurityGroupIngressResponse>"#,
                        request_id
                    );
                    xml_response(StatusCode::OK, xml)
                }
                Err(e) => ec2_error_response(StatusCode::BAD_REQUEST, "InvalidGroupID.NotFound", &e.to_string(), &request_id),
            }
        }
        "DeleteSecurityGroup" => {
            let group_id = params.get("GroupId").map(|s| s.as_str()).unwrap_or("");
            match state.delete_security_group(group_id) {
                Ok(()) => {
                    let xml = format!(
                        r#"<DeleteSecurityGroupResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <return>true</return>
</DeleteSecurityGroupResponse>"#,
                        request_id
                    );
                    xml_response(StatusCode::OK, xml)
                }
                Err(e) => ec2_error_response(StatusCode::BAD_REQUEST, "InvalidGroupID.NotFound", &e.to_string(), &request_id),
            }
        }
        "CreateKeyPair" => {
            let key_name = params.get("KeyName").cloned().unwrap_or_else(|| "my-key".to_string());
            let kp = state.create_key_pair(key_name);
            let xml = format!(
                r#"<CreateKeyPairResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <keyName>{}</keyName>
    <keyPairId>{}</keyPairId>
    <keyFingerprint>{}</keyFingerprint>
    <keyMaterial>{}</keyMaterial>
</CreateKeyPairResponse>"#,
                request_id, kp.key_name, kp.key_pair_id, kp.key_fingerprint, kp.key_material.unwrap_or_default()
            );
            xml_response(StatusCode::OK, xml)
        }
        "DescribeKeyPairs" => {
            let names = parse_indexed_list(&params, "KeyName");
            let key_pairs = state.describe_key_pairs(if names.is_empty() { None } else { Some(names) });
            let mut kps_xml = String::new();
            for kp in key_pairs {
                kps_xml.push_str(&format!(
                    r#"<item>
        <keyName>{}</keyName>
        <keyPairId>{}</keyPairId>
        <keyFingerprint>{}</keyFingerprint>
    </item>"#,
                    kp.key_name, kp.key_pair_id, kp.key_fingerprint
                ));
            }
            let xml = format!(
                r#"<DescribeKeyPairsResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <keySet>{}</keySet>
</DescribeKeyPairsResponse>"#,
                request_id, kps_xml
            );
            xml_response(StatusCode::OK, xml)
        }
        "DeleteKeyPair" => {
            let key_name = params.get("KeyName").map(|s| s.as_str()).unwrap_or("");
            let _ = state.delete_key_pair(key_name);
            let xml = format!(
                r#"<DeleteKeyPairResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <return>true</return>
</DeleteKeyPairResponse>"#,
                request_id
            );
            xml_response(StatusCode::OK, xml)
        }
        "RunInstances" => {
            let image_id = params.get("ImageId").cloned().unwrap_or_else(|| "ami-12345678".to_string());
            let inst_type = params.get("InstanceType").cloned();
            let key_name = params.get("KeyName").cloned();
            let subnet_id = params.get("SubnetId").cloned();
            let sg_ids = parse_indexed_list(&params, "SecurityGroupId");
            let count = params.get("MaxCount").and_then(|c| c.parse().ok()).or_else(|| params.get("MinCount").and_then(|c| c.parse().ok()));
            let tags = parse_tags(&params);

            let instances = state.run_instances(image_id, inst_type, key_name, subnet_id, sg_ids, count, tags);
            let mut inst_xml = String::new();
            for inst in instances {
                inst_xml.push_str(&format!(
                    r#"<item>
        <instanceId>{}</instanceId>
        <imageId>{}</imageId>
        <instanceType>{}</instanceType>
        <instanceState>
            <code>{}</code>
            <name>{}</name>
        </instanceState>
        <privateIpAddress>{}</privateIpAddress>
        <ipAddress>{}</ipAddress>
        <launchTime>{}</launchTime>
    </item>"#,
                    inst.instance_id, inst.image_id, inst.instance_type, inst.state_code, inst.state, inst.private_ip_address, inst.public_ip_address.unwrap_or_default(), inst.launch_time
                ));
            }
            let xml = format!(
                r#"<RunInstancesResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <instancesSet>{}</instancesSet>
</RunInstancesResponse>"#,
                request_id, inst_xml
            );
            xml_response(StatusCode::OK, xml)
        }
        "DescribeInstances" => {
            let ids = parse_indexed_list(&params, "InstanceId");
            let instances = state.describe_instances(if ids.is_empty() { None } else { Some(ids) });
            let mut inst_xml = String::new();
            for inst in instances {
                inst_xml.push_str(&format!(
                    r#"<item>
        <instanceId>{}</instanceId>
        <imageId>{}</imageId>
        <instanceType>{}</instanceType>
        <instanceState>
            <code>{}</code>
            <name>{}</name>
        </instanceState>
        <privateIpAddress>{}</privateIpAddress>
        <ipAddress>{}</ipAddress>
        <launchTime>{}</launchTime>
    </item>"#,
                    inst.instance_id, inst.image_id, inst.instance_type, inst.state_code, inst.state, inst.private_ip_address, inst.public_ip_address.unwrap_or_default(), inst.launch_time
                ));
            }
            let xml = format!(
                r#"<DescribeInstancesResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <reservationSet>
        <item>
            <reservationId>r-00000000000000001</reservationId>
            <instancesSet>{}</instancesSet>
        </item>
    </reservationSet>
</DescribeInstancesResponse>"#,
                request_id, inst_xml
            );
            xml_response(StatusCode::OK, xml)
        }
        "TerminateInstances" => {
            let ids = parse_indexed_list(&params, "InstanceId");
            let terminated = state.terminate_instances(ids);
            let mut term_xml = String::new();
            for inst in terminated {
                term_xml.push_str(&format!(
                    r#"<item>
        <instanceId>{}</instanceId>
        <currentState>
            <code>48</code>
            <name>terminated</name>
        </currentState>
        <previousState>
            <code>16</code>
            <name>running</name>
        </previousState>
    </item>"#,
                    inst.instance_id
                ));
            }
            let xml = format!(
                r#"<TerminateInstancesResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{}</requestId>
    <instancesSet>{}</instancesSet>
</TerminateInstancesResponse>"#,
                request_id, term_xml
            );
            xml_response(StatusCode::OK, xml)
        }
        _ => ec2_error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("The action {} is not valid for Amazon EC2.", action),
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

fn parse_tags(params: &HashMap<String, String>) -> Vec<Tag> {
    let mut tags = Vec::new();
    let mut i = 1;
    while let Some(key) = params.get(&format!("TagSpecification.1.Tag.{}.Key", i)) {
        let value = params.get(&format!("TagSpecification.1.Tag.{}.Value", i)).cloned().unwrap_or_default();
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

fn ec2_error_response(status: StatusCode, code: &str, message: &str, request_id: &str) -> Response<Body> {
    let xml = format!(
        r#"<Response><Errors><Error><Code>{}</Code><Message>{}</Message></Error></Errors><RequestID>{}</RequestID></Response>"#,
        code, message, request_id
    );
    xml_response(status, xml)
}

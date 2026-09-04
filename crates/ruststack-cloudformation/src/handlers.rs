use crate::state::{CloudFormationError, CloudFormationState};
use crate::types::*;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode, Uri};
use bytes::Bytes;
use std::collections::HashMap;

pub async fn handle_cloudformation_request(
    State(state): State<CloudFormationState>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let mut params: HashMap<String, String> = HashMap::new();

    if let Some(query) = uri.query() {
        for (k, v) in form_urlencoded::parse(query.as_bytes()) {
            params.insert(k.into_owned(), v.into_owned());
        }
    }

    if let Ok(body_str) = std::str::from_utf8(&body) {
        if headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .contains("application/x-www-form-urlencoded")
        {
            for (k, v) in form_urlencoded::parse(body_str.as_bytes()) {
                params.insert(k.into_owned(), v.into_owned());
            }
        }
    }

    let action = params.get("Action").cloned().unwrap_or_else(|| {
        headers
            .get("x-amz-target")
            .and_then(|v| v.to_str().ok())
            .map(|t| t.rsplit('.').next().unwrap_or(t).to_string())
            .unwrap_or_default()
    });

    match action.as_str() {
        "CreateStack" => {
            let stack_name = params.get("StackName").cloned().unwrap_or_default();
            let template_body = params.get("TemplateBody").cloned().unwrap_or_default();

            let mut parsed_params = Vec::new();
            let mut i = 1;
            while let Some(k) = params.get(&format!("Parameters.member.{}.ParameterKey", i)) {
                let v = params
                    .get(&format!("Parameters.member.{}.ParameterValue", i))
                    .cloned()
                    .unwrap_or_default();
                parsed_params.push(Parameter {
                    parameter_key: k.clone(),
                    parameter_value: v,
                });
                i += 1;
            }

            match state.create_stack(stack_name, template_body, parsed_params) {
                Ok(stack_id) => {
                    let xml = format!(
                        r#"<CreateStackResponse xmlns="http://cloudformation.amazonaws.com/doc/2010-05-15/"><CreateStackResult><StackId>{}</StackId></CreateStackResult><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></CreateStackResponse>"#,
                        stack_id,
                        uuid::Uuid::new_v4()
                    );
                    xml_response(StatusCode::OK, &xml)
                }
                Err(e) => error_response(e),
            }
        }
        "DescribeStacks" => {
            let stack_name = params.get("StackName").map(|s| s.as_str());
            match state.describe_stacks(stack_name) {
                Ok(stacks) => {
                    let mut stacks_xml = String::new();
                    for s in stacks {
                        let mut outputs_xml = String::new();
                        for out in s.outputs {
                            outputs_xml.push_str(&format!(
                                r#"<member><OutputKey>{}</OutputKey><OutputValue>{}</OutputValue>{}</member>"#,
                                out.output_key,
                                out.output_value,
                                out.description.map(|d| format!("<Description>{}</Description>", d)).unwrap_or_default()
                            ));
                        }
                        let mut params_xml = String::new();
                        for p in s.parameters {
                            params_xml.push_str(&format!(
                                r#"<member><ParameterKey>{}</ParameterKey><ParameterValue>{}</ParameterValue></member>"#,
                                p.parameter_key, p.parameter_value
                            ));
                        }

                        stacks_xml.push_str(&format!(
                            r#"<member><StackId>{}</StackId><StackName>{}</StackName><CreationTime>{}</CreationTime><StackStatus>{}</StackStatus><Outputs>{}</Outputs><Parameters>{}</Parameters></member>"#,
                            s.stack_id,
                            s.stack_name,
                            s.creation_time.to_rfc3339(),
                            s.status,
                            outputs_xml,
                            params_xml
                        ));
                    }

                    let xml = format!(
                        r#"<DescribeStacksResponse xmlns="http://cloudformation.amazonaws.com/doc/2010-05-15/"><DescribeStacksResult><Stacks>{}</Stacks></DescribeStacksResult><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></DescribeStacksResponse>"#,
                        stacks_xml,
                        uuid::Uuid::new_v4()
                    );
                    xml_response(StatusCode::OK, &xml)
                }
                Err(e) => error_response(e),
            }
        }
        "DescribeStackResources" => {
            let stack_name = params.get("StackName").map(|s| s.as_str()).unwrap_or("");
            match state.describe_stack_resources(stack_name) {
                Ok(resources) => {
                    let mut res_xml = String::new();
                    for r in resources {
                        res_xml.push_str(&format!(
                            r#"<member><LogicalResourceId>{}</LogicalResourceId><PhysicalResourceId>{}</PhysicalResourceId><ResourceType>{}</ResourceType><ResourceStatus>{}</ResourceStatus><Timestamp>{}</Timestamp></member>"#,
                            r.logical_resource_id,
                            r.physical_resource_id,
                            r.resource_type,
                            r.resource_status,
                            r.timestamp.to_rfc3339()
                        ));
                    }
                    let xml = format!(
                        r#"<DescribeStackResourcesResponse xmlns="http://cloudformation.amazonaws.com/doc/2010-05-15/"><DescribeStackResourcesResult><StackResources>{}</StackResources></DescribeStackResourcesResult><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></DescribeStackResourcesResponse>"#,
                        res_xml,
                        uuid::Uuid::new_v4()
                    );
                    xml_response(StatusCode::OK, &xml)
                }
                Err(e) => error_response(e),
            }
        }
        "DescribeStackEvents" => {
            let stack_name = params.get("StackName").map(|s| s.as_str()).unwrap_or("");
            match state.describe_stack_events(stack_name) {
                Ok(events) => {
                    let mut ev_xml = String::new();
                    for ev in events {
                        ev_xml.push_str(&format!(
                            r#"<member><EventId>{}</EventId><StackId>{}</StackId><StackName>{}</StackName><LogicalResourceId>{}</LogicalResourceId><PhysicalResourceId>{}</PhysicalResourceId><ResourceType>{}</ResourceType><Timestamp>{}</Timestamp><ResourceStatus>{}</ResourceStatus></member>"#,
                            ev.event_id,
                            ev.stack_id,
                            ev.stack_name,
                            ev.logical_resource_id,
                            ev.physical_resource_id,
                            ev.resource_type,
                            ev.timestamp.to_rfc3339(),
                            ev.resource_status
                        ));
                    }
                    let xml = format!(
                        r#"<DescribeStackEventsResponse xmlns="http://cloudformation.amazonaws.com/doc/2010-05-15/"><DescribeStackEventsResult><StackEvents>{}</StackEvents></DescribeStackEventsResult><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></DescribeStackEventsResponse>"#,
                        ev_xml,
                        uuid::Uuid::new_v4()
                    );
                    xml_response(StatusCode::OK, &xml)
                }
                Err(e) => error_response(e),
            }
        }
        "GetTemplate" => {
            let stack_name = params.get("StackName").map(|s| s.as_str()).unwrap_or("");
            match state.get_template(stack_name) {
                Ok(body) => {
                    let xml = format!(
                        r#"<GetTemplateResponse xmlns="http://cloudformation.amazonaws.com/doc/2010-05-15/"><GetTemplateResult><TemplateBody>{}</TemplateBody></GetTemplateResult><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></GetTemplateResponse>"#,
                        quick_xml::escape::escape(&body),
                        uuid::Uuid::new_v4()
                    );
                    xml_response(StatusCode::OK, &xml)
                }
                Err(e) => error_response(e),
            }
        }
        "DeleteStack" => {
            let stack_name = params.get("StackName").map(|s| s.as_str()).unwrap_or("");
            let _ = state.delete_stack(stack_name);
            let xml = format!(
                r#"<DeleteStackResponse xmlns="http://cloudformation.amazonaws.com/doc/2010-05-15/"><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></DeleteStackResponse>"#,
                uuid::Uuid::new_v4()
            );
            xml_response(StatusCode::OK, &xml)
        }
        _ => {
            let xml = format!(
                r#"<ErrorResponse xmlns="http://cloudformation.amazonaws.com/doc/2010-05-15/"><Error><Type>Sender</Type><Code>InvalidAction</Code><Message>Action {} is not valid</Message></Error><RequestId>{}</RequestId></ErrorResponse>"#,
                action,
                uuid::Uuid::new_v4()
            );
            xml_response(StatusCode::BAD_REQUEST, &xml)
        }
    }
}

fn xml_response(status: StatusCode, xml: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", "text/xml; charset=utf-8")
        .body(Body::from(xml.to_string()))
        .unwrap()
}

fn error_response(err: CloudFormationError) -> Response<Body> {
    let (code, msg) = match &err {
        CloudFormationError::StackAlreadyExists(name) => {
            ("AlreadyExistsException", format!("Stack [{}] already exists", name))
        }
        CloudFormationError::StackNotFound(name) => {
            ("ValidationError", format!("Stack with id {} does not exist", name))
        }
        CloudFormationError::TemplateFormatError(msg) => {
            ("ValidationError", format!("Template format error: {}", msg))
        }
    };

    let xml = format!(
        r#"<ErrorResponse xmlns="http://cloudformation.amazonaws.com/doc/2010-05-15/"><Error><Type>Sender</Type><Code>{}</Code><Message>{}</Message></Error><RequestId>{}</RequestId></ErrorResponse>"#,
        code,
        msg,
        uuid::Uuid::new_v4()
    );
    xml_response(StatusCode::BAD_REQUEST, &xml)
}

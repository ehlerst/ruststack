use crate::types::{BatchErrorEntry, PublishBatchResultEntry, Subscription};
use serde_json::json;
use std::collections::HashMap;

// --- XML Response Codecs (Query Protocol) ---

pub fn xml_create_topic_response(topic_arn: &str, request_id: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<CreateTopicResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/">
    <CreateTopicResult>
        <TopicArn>{}</TopicArn>
    </CreateTopicResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</CreateTopicResponse>"#,
        quick_xml::escape::escape(topic_arn),
        quick_xml::escape::escape(request_id)
    )
}

pub fn xml_delete_topic_response(request_id: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<DeleteTopicResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/">
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</DeleteTopicResponse>"#,
        quick_xml::escape::escape(request_id)
    )
}

pub fn xml_list_topics_response(topic_arns: &[String], request_id: &str) -> String {
    let mut xml = r#"<?xml version="1.0"?>
<ListTopicsResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/">
    <ListTopicsResult>
        <Topics>"#
        .to_string();

    for arn in topic_arns {
        xml.push_str(&format!(
            r#"
            <member>
                <TopicArn>{}</TopicArn>
            </member>"#,
            quick_xml::escape::escape(arn)
        ));
    }

    xml.push_str(&format!(
        r#"
        </Topics>
    </ListTopicsResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</ListTopicsResponse>"#,
        quick_xml::escape::escape(request_id)
    ));

    xml
}

pub fn xml_get_topic_attributes_response(
    attributes: &HashMap<String, String>,
    request_id: &str,
) -> String {
    let mut xml = r#"<?xml version="1.0"?>
<GetTopicAttributesResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/">
    <GetTopicAttributesResult>
        <Attributes>"#
        .to_string();

    for (k, v) in attributes {
        xml.push_str(&format!(
            r#"
            <entry>
                <key>{}</key>
                <value>{}</value>
            </entry>"#,
            quick_xml::escape::escape(k),
            quick_xml::escape::escape(v)
        ));
    }

    xml.push_str(&format!(
        r#"
        </Attributes>
    </GetTopicAttributesResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</GetTopicAttributesResponse>"#,
        quick_xml::escape::escape(request_id)
    ));

    xml
}

pub fn xml_set_topic_attributes_response(request_id: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<SetTopicAttributesResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/">
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</SetTopicAttributesResponse>"#,
        quick_xml::escape::escape(request_id)
    )
}

pub fn xml_subscribe_response(subscription_arn: &str, request_id: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<SubscribeResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/">
    <SubscribeResult>
        <SubscriptionArn>{}</SubscriptionArn>
    </SubscribeResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</SubscribeResponse>"#,
        quick_xml::escape::escape(subscription_arn),
        quick_xml::escape::escape(request_id)
    )
}

pub fn xml_unsubscribe_response(request_id: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<UnsubscribeResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/">
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</UnsubscribeResponse>"#,
        quick_xml::escape::escape(request_id)
    )
}

pub fn xml_list_subscriptions_response(subscriptions: &[Subscription], request_id: &str) -> String {
    let mut xml = r#"<?xml version="1.0"?>
<ListSubscriptionsResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/">
    <ListSubscriptionsResult>
        <Subscriptions>"#
        .to_string();

    for sub in subscriptions {
        xml.push_str(&format!(
            r#"
            <member>
                <SubscriptionArn>{}</SubscriptionArn>
                <Owner>{}</Owner>
                <Protocol>{}</Protocol>
                <Endpoint>{}</Endpoint>
                <TopicArn>{}</TopicArn>
            </member>"#,
            quick_xml::escape::escape(&sub.subscription_arn),
            quick_xml::escape::escape(&sub.owner),
            quick_xml::escape::escape(&sub.protocol),
            quick_xml::escape::escape(&sub.endpoint),
            quick_xml::escape::escape(&sub.topic_arn)
        ));
    }

    xml.push_str(&format!(
        r#"
        </Subscriptions>
    </ListSubscriptionsResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</ListSubscriptionsResponse>"#,
        quick_xml::escape::escape(request_id)
    ));

    xml
}

pub fn xml_get_subscription_attributes_response(
    attributes: &HashMap<String, String>,
    request_id: &str,
) -> String {
    let mut xml = r#"<?xml version="1.0"?>
<GetSubscriptionAttributesResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/">
    <GetSubscriptionAttributesResult>
        <Attributes>"#
        .to_string();

    for (k, v) in attributes {
        xml.push_str(&format!(
            r#"
            <entry>
                <key>{}</key>
                <value>{}</value>
            </entry>"#,
            quick_xml::escape::escape(k),
            quick_xml::escape::escape(v)
        ));
    }

    xml.push_str(&format!(
        r#"
        </Attributes>
    </GetSubscriptionAttributesResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</GetSubscriptionAttributesResponse>"#,
        quick_xml::escape::escape(request_id)
    ));

    xml
}

pub fn xml_set_subscription_attributes_response(request_id: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<SetSubscriptionAttributesResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/">
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</SetSubscriptionAttributesResponse>"#,
        quick_xml::escape::escape(request_id)
    )
}

pub fn xml_publish_response(
    message_id: &str,
    sequence_number: Option<&str>,
    request_id: &str,
) -> String {
    let seq_tag = match sequence_number {
        Some(seq) => format!(
            "<SequenceNumber>{}</SequenceNumber>",
            quick_xml::escape::escape(seq)
        ),
        None => String::new(),
    };

    format!(
        r#"<?xml version="1.0"?>
<PublishResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/">
    <PublishResult>
        <MessageId>{}</MessageId>
        {}
    </PublishResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</PublishResponse>"#,
        quick_xml::escape::escape(message_id),
        seq_tag,
        quick_xml::escape::escape(request_id)
    )
}

// --- JSON Response Codecs (JSON Protocol) ---

pub fn json_create_topic_response(topic_arn: &str) -> serde_json::Value {
    json!({
        "TopicArn": topic_arn
    })
}

pub fn json_list_topics_response(topic_arns: &[String]) -> serde_json::Value {
    let topics: Vec<_> = topic_arns
        .iter()
        .map(|arn| json!({ "TopicArn": arn }))
        .collect();
    json!({
        "Topics": topics
    })
}

pub fn json_get_topic_attributes_response(
    attributes: &HashMap<String, String>,
) -> serde_json::Value {
    json!({
        "Attributes": attributes
    })
}

pub fn json_subscribe_response(subscription_arn: &str) -> serde_json::Value {
    json!({
        "SubscriptionArn": subscription_arn
    })
}

pub fn json_list_subscriptions_response(subscriptions: &[Subscription]) -> serde_json::Value {
    let subs: Vec<_> = subscriptions
        .iter()
        .map(|s| {
            json!({
                "SubscriptionArn": s.subscription_arn,
                "Owner": s.owner,
                "Protocol": s.protocol,
                "Endpoint": s.endpoint,
                "TopicArn": s.topic_arn
            })
        })
        .collect();

    json!({
        "Subscriptions": subs
    })
}

pub fn json_get_subscription_attributes_response(
    attributes: &HashMap<String, String>,
) -> serde_json::Value {
    json!({
        "Attributes": attributes
    })
}

pub fn json_publish_response(message_id: &str, sequence_number: Option<&str>) -> serde_json::Value {
    let mut val = json!({
        "MessageId": message_id
    });
    if let Some(seq) = sequence_number {
        val["SequenceNumber"] = json!(seq);
    }
    val
}

pub fn json_publish_batch_response(
    successful: &[PublishBatchResultEntry],
    failed: &[BatchErrorEntry],
) -> serde_json::Value {
    let succ_vals: Vec<_> = successful
        .iter()
        .map(|s| {
            let mut v = json!({
                "Id": s.id,
                "MessageId": s.message_id
            });
            if let Some(ref seq) = s.sequence_number {
                v["SequenceNumber"] = json!(seq);
            }
            v
        })
        .collect();

    let fail_vals: Vec<_> = failed
        .iter()
        .map(|f| {
            json!({
                "Id": f.id,
                "Code": f.code,
                "Message": f.message,
                "SenderFault": f.sender_fault
            })
        })
        .collect();

    json!({
        "Successful": succ_vals,
        "Failed": fail_vals
    })
}

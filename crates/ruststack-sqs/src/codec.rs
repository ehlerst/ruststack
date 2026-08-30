use crate::types::{BatchErrorEntry, SendMessageBatchResultEntry, SqsMessage};
use serde_json::json;
use std::collections::HashMap;

pub fn xml_create_queue_response(queue_url: &str, request_id: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<CreateQueueResponse xmlns="http://queue.amazonaws.com/doc/2012-11-05/">
    <CreateQueueResult>
        <QueueUrl>{}</QueueUrl>
    </CreateQueueResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</CreateQueueResponse>"#,
        quick_xml::escape::escape(queue_url),
        quick_xml::escape::escape(request_id)
    )
}

pub fn xml_get_queue_url_response(queue_url: &str, request_id: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<GetQueueUrlResponse xmlns="http://queue.amazonaws.com/doc/2012-11-05/">
    <GetQueueUrlResult>
        <QueueUrl>{}</QueueUrl>
    </GetQueueUrlResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</GetQueueUrlResponse>"#,
        quick_xml::escape::escape(queue_url),
        quick_xml::escape::escape(request_id)
    )
}

pub fn xml_list_queues_response(queue_urls: &[String], request_id: &str) -> String {
    let mut xml = r#"<?xml version="1.0"?>
<ListQueuesResponse xmlns="http://queue.amazonaws.com/doc/2012-11-05/">
    <ListQueuesResult>"#
        .to_string();

    for url in queue_urls {
        xml.push_str(&format!(
            r#"
        <QueueUrl>{}</QueueUrl>"#,
            quick_xml::escape::escape(url)
        ));
    }

    xml.push_str(&format!(
        r#"
    </ListQueuesResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</ListQueuesResponse>"#,
        quick_xml::escape::escape(request_id)
    ));

    xml
}

pub fn xml_get_queue_attributes_response(
    attributes: &HashMap<String, String>,
    request_id: &str,
) -> String {
    let mut xml = r#"<?xml version="1.0"?>
<GetQueueAttributesResponse xmlns="http://queue.amazonaws.com/doc/2012-11-05/">
    <GetQueueAttributesResult>"#
        .to_string();

    for (k, v) in attributes {
        xml.push_str(&format!(
            r#"
        <Attribute>
            <Name>{}</Name>
            <Value>{}</Value>
        </Attribute>"#,
            quick_xml::escape::escape(k),
            quick_xml::escape::escape(v)
        ));
    }

    xml.push_str(&format!(
        r#"
    </GetQueueAttributesResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</GetQueueAttributesResponse>"#,
        quick_xml::escape::escape(request_id)
    ));

    xml
}

pub fn xml_send_message_response(
    message_id: &str,
    md5: &str,
    seq: Option<&str>,
    request_id: &str,
) -> String {
    let seq_tag = if let Some(s) = seq {
        format!(
            "\n        <SequenceNumber>{}</SequenceNumber>",
            quick_xml::escape::escape(s)
        )
    } else {
        String::new()
    };

    format!(
        r#"<?xml version="1.0"?>
<SendMessageResponse xmlns="http://queue.amazonaws.com/doc/2012-11-05/">
    <SendMessageResult>
        <MD5OfMessageBody>{}</MD5OfMessageBody>
        <MessageId>{}</MessageId>{}
    </SendMessageResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</SendMessageResponse>"#,
        quick_xml::escape::escape(md5),
        quick_xml::escape::escape(message_id),
        seq_tag,
        quick_xml::escape::escape(request_id)
    )
}

pub fn xml_send_message_batch_response(
    successful: &[SendMessageBatchResultEntry],
    failed: &[BatchErrorEntry],
    request_id: &str,
) -> String {
    let mut xml = r#"<?xml version="1.0"?>
<SendMessageBatchResponse xmlns="http://queue.amazonaws.com/doc/2012-11-05/">
    <SendMessageBatchResult>"#
        .to_string();

    for s in successful {
        let seq_tag = if let Some(ref seq) = s.sequence_number {
            format!(
                "\n            <SequenceNumber>{}</SequenceNumber>",
                quick_xml::escape::escape(seq)
            )
        } else {
            String::new()
        };

        xml.push_str(&format!(
            r#"
        <SendMessageBatchResultEntry>
            <Id>{}</Id>
            <MessageId>{}</MessageId>
            <MD5OfMessageBody>{}</MD5OfMessageBody>{}
        </SendMessageBatchResultEntry>"#,
            quick_xml::escape::escape(&s.id),
            quick_xml::escape::escape(&s.message_id),
            quick_xml::escape::escape(&s.md5_of_message_body),
            seq_tag
        ));
    }

    for f in failed {
        xml.push_str(&format!(
            r#"
        <BatchResultErrorEntry>
            <Id>{}</Id>
            <SenderFault>{}</SenderFault>
            <Code>{}</Code>
            <Message>{}</Message>
        </BatchResultErrorEntry>"#,
            quick_xml::escape::escape(&f.id),
            f.sender_fault,
            quick_xml::escape::escape(&f.code),
            quick_xml::escape::escape(&f.message)
        ));
    }

    xml.push_str(&format!(
        r#"
    </SendMessageBatchResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</SendMessageBatchResponse>"#,
        quick_xml::escape::escape(request_id)
    ));

    xml
}

pub fn xml_receive_message_response(messages: &[SqsMessage], request_id: &str) -> String {
    let mut xml = r#"<?xml version="1.0"?>
<ReceiveMessageResponse xmlns="http://queue.amazonaws.com/doc/2012-11-05/">
    <ReceiveMessageResult>"#
        .to_string();

    for msg in messages {
        xml.push_str(&format!(
            r#"
        <Message>
            <MessageId>{}</MessageId>
            <ReceiptHandle>{}</ReceiptHandle>
            <MD5OfBody>{}</MD5OfBody>
            <Body>{}</Body>"#,
            quick_xml::escape::escape(&msg.message_id),
            quick_xml::escape::escape(&msg.receipt_handle),
            quick_xml::escape::escape(&msg.md5_of_body),
            quick_xml::escape::escape(&msg.body)
        ));

        for (k, v) in &msg.attributes {
            xml.push_str(&format!(
                r#"
            <Attribute>
                <Name>{}</Name>
                <Value>{}</Value>
            </Attribute>"#,
                quick_xml::escape::escape(k),
                quick_xml::escape::escape(v)
            ));
        }

        for (k, v) in &msg.message_attributes {
            let str_val = v.string_value.as_deref().unwrap_or("");
            xml.push_str(&format!(
                r#"
            <MessageAttribute>
                <Name>{}</Name>
                <Value>
                    <DataType>{}</DataType>
                    <StringValue>{}</StringValue>
                </Value>
            </MessageAttribute>"#,
                quick_xml::escape::escape(k),
                quick_xml::escape::escape(&v.data_type),
                quick_xml::escape::escape(str_val)
            ));
        }

        xml.push_str("\n        </Message>");
    }

    xml.push_str(&format!(
        r#"
    </ReceiveMessageResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</ReceiveMessageResponse>"#,
        quick_xml::escape::escape(request_id)
    ));

    xml
}

pub fn xml_empty_response(action: &str, request_id: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<{}Response xmlns="http://queue.amazonaws.com/doc/2012-11-05/">
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</{}Response>"#,
        quick_xml::escape::escape(action),
        quick_xml::escape::escape(request_id),
        quick_xml::escape::escape(action)
    )
}

pub fn xml_delete_message_batch_response(
    successful: &[String],
    failed: &[BatchErrorEntry],
    request_id: &str,
) -> String {
    let mut xml = r#"<?xml version="1.0"?>
<DeleteMessageBatchResponse xmlns="http://queue.amazonaws.com/doc/2012-11-05/">
    <DeleteMessageBatchResult>"#
        .to_string();

    for s in successful {
        xml.push_str(&format!(
            r#"
        <DeleteMessageBatchResultEntry>
            <Id>{}</Id>
        </DeleteMessageBatchResultEntry>"#,
            quick_xml::escape::escape(s)
        ));
    }

    for f in failed {
        xml.push_str(&format!(
            r#"
        <BatchResultErrorEntry>
            <Id>{}</Id>
            <SenderFault>{}</SenderFault>
            <Code>{}</Code>
            <Message>{}</Message>
        </BatchResultErrorEntry>"#,
            quick_xml::escape::escape(&f.id),
            f.sender_fault,
            quick_xml::escape::escape(&f.code),
            quick_xml::escape::escape(&f.message)
        ));
    }

    xml.push_str(&format!(
        r#"
    </DeleteMessageBatchResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</DeleteMessageBatchResponse>"#,
        quick_xml::escape::escape(request_id)
    ));

    xml
}

// JSON 1.0 Formats for modern AWS SDKs
pub fn json_create_queue_response(queue_url: &str) -> serde_json::Value {
    json!({
        "QueueUrl": queue_url
    })
}

pub fn json_get_queue_url_response(queue_url: &str) -> serde_json::Value {
    json!({
        "QueueUrl": queue_url
    })
}

pub fn json_list_queues_response(queue_urls: &[String]) -> serde_json::Value {
    json!({
        "QueueUrls": queue_urls
    })
}

pub fn json_get_queue_attributes_response(
    attributes: &HashMap<String, String>,
) -> serde_json::Value {
    json!({
        "Attributes": attributes
    })
}

pub fn json_send_message_response(
    message_id: &str,
    md5: &str,
    seq: Option<&str>,
) -> serde_json::Value {
    let mut val = json!({
        "MessageId": message_id,
        "MD5OfMessageBody": md5
    });
    if let Some(s) = seq {
        val.as_object_mut()
            .unwrap()
            .insert("SequenceNumber".to_string(), json!(s));
    }
    val
}

pub fn json_send_message_batch_response(
    successful: &[SendMessageBatchResultEntry],
    failed: &[BatchErrorEntry],
) -> serde_json::Value {
    let succ_vals: Vec<_> = successful
        .iter()
        .map(|s| {
            let mut o = json!({
                "Id": s.id,
                "MessageId": s.message_id,
                "MD5OfMessageBody": s.md5_of_message_body
            });
            if let Some(ref seq) = s.sequence_number {
                o.as_object_mut()
                    .unwrap()
                    .insert("SequenceNumber".to_string(), json!(seq));
            }
            o
        })
        .collect();

    let fail_vals: Vec<_> = failed
        .iter()
        .map(|f| {
            json!({
                "Id": f.id,
                "SenderFault": f.sender_fault,
                "Code": f.code,
                "Message": f.message
            })
        })
        .collect();

    json!({
        "Successful": succ_vals,
        "Failed": fail_vals
    })
}

pub fn json_receive_message_response(messages: &[SqsMessage]) -> serde_json::Value {
    let msg_vals: Vec<_> = messages
        .iter()
        .map(|m| {
            let mut attrs = serde_json::Map::new();
            for (k, v) in &m.attributes {
                attrs.insert(k.clone(), json!(v));
            }

            let mut msg_attrs = serde_json::Map::new();
            for (k, v) in &m.message_attributes {
                msg_attrs.insert(
                    k.clone(),
                    json!({
                        "DataType": v.data_type,
                        "StringValue": v.string_value
                    }),
                );
            }

            json!({
                "MessageId": m.message_id,
                "ReceiptHandle": m.receipt_handle,
                "MD5OfBody": m.md5_of_body,
                "Body": m.body,
                "Attributes": attrs,
                "MessageAttributes": msg_attrs
            })
        })
        .collect();

    json!({
        "Messages": msg_vals
    })
}

pub fn json_delete_message_batch_response(
    successful: &[String],
    failed: &[BatchErrorEntry],
) -> serde_json::Value {
    let succ_vals: Vec<_> = successful.iter().map(|id| json!({ "Id": id })).collect();

    let fail_vals: Vec<_> = failed
        .iter()
        .map(|f| {
            json!({
                "Id": f.id,
                "SenderFault": f.sender_fault,
                "Code": f.code,
                "Message": f.message
            })
        })
        .collect();

    json!({
        "Successful": succ_vals,
        "Failed": fail_vals
    })
}

pub fn xml_list_dead_letter_source_queues_response(
    queue_urls: &[String],
    request_id: &str,
) -> String {
    let mut xml = r#"<?xml version="1.0"?>
<ListDeadLetterSourceQueuesResponse xmlns="http://queue.amazonaws.com/doc/2012-11-05/">
    <ListDeadLetterSourceQueuesResult>"#
        .to_string();

    for url in queue_urls {
        xml.push_str(&format!(
            r#"
        <QueueUrl>{}</QueueUrl>"#,
            quick_xml::escape::escape(url)
        ));
    }

    xml.push_str(&format!(
        r#"
    </ListDeadLetterSourceQueuesResult>
    <ResponseMetadata>
        <RequestId>{}</RequestId>
    </ResponseMetadata>
</ListDeadLetterSourceQueuesResponse>"#,
        quick_xml::escape::escape(request_id)
    ));

    xml
}

pub fn json_list_dead_letter_source_queues_response(queue_urls: &[String]) -> serde_json::Value {
    json!({
        "queueUrls": queue_urls
    })
}

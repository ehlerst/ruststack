use crate::types::{
    BucketInfo, BucketNotificationConfig, CompletedPart, DeleteObjectsResult, ListObjectsV2Result,
    NotificationFilter, NotificationFilterRule, PartInfo, QueueNotificationConfig,
    TopicNotificationConfig,
};
use ruststack_core::RustStackError;

pub fn serialize_list_buckets(buckets: &[BucketInfo], _owner_id: &str) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
    <Owner>
        <ID>000000000000</ID>
        <DisplayName>ruststack</DisplayName>
    </Owner>
    <Buckets>"#,
    );

    for b in buckets {
        xml.push_str(&format!(
            r#"
        <Bucket>
            <Name>{}</Name>
            <CreationDate>{}</CreationDate>
        </Bucket>"#,
            quick_xml::escape::escape(&b.name),
            b.creation_date.to_rfc3339()
        ));
    }

    xml.push_str(
        r#"
    </Buckets>
</ListAllMyBucketsResult>"#,
    );

    xml
}

pub fn serialize_list_objects_v2(res: &ListObjectsV2Result) -> String {
    let mut xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
    <Name>{}</Name>
    <Prefix>{}</Prefix>
    <KeyCount>{}</KeyCount>
    <MaxKeys>{}</MaxKeys>
    <IsTruncated>{}</IsTruncated>"#,
        quick_xml::escape::escape(&res.name),
        quick_xml::escape::escape(&res.prefix),
        res.key_count,
        res.max_keys,
        res.is_truncated
    );

    if let Some(ref delim) = res.delimiter {
        xml.push_str(&format!(
            "\n    <Delimiter>{}</Delimiter>",
            quick_xml::escape::escape(delim)
        ));
    }

    if let Some(ref token) = res.continuation_token {
        xml.push_str(&format!(
            "\n    <ContinuationToken>{}</ContinuationToken>",
            quick_xml::escape::escape(token)
        ));
    }

    if let Some(ref next_token) = res.next_continuation_token {
        xml.push_str(&format!(
            "\n    <NextContinuationToken>{}</NextContinuationToken>",
            quick_xml::escape::escape(next_token)
        ));
    }

    if let Some(ref start_after) = res.start_after {
        xml.push_str(&format!(
            "\n    <StartAfter>{}</StartAfter>",
            quick_xml::escape::escape(start_after)
        ));
    }

    for obj in &res.contents {
        xml.push_str(&format!(
            r#"
    <Contents>
        <Key>{}</Key>
        <LastModified>{}</LastModified>
        <ETag>{}</ETag>
        <Size>{}</Size>
        <StorageClass>STANDARD</StorageClass>
    </Contents>"#,
            quick_xml::escape::escape(&obj.key),
            obj.last_modified.to_rfc3339(),
            quick_xml::escape::escape(&obj.etag),
            obj.size
        ));
    }

    for prefix in &res.common_prefixes {
        xml.push_str(&format!(
            r#"
    <CommonPrefixes>
        <Prefix>{}</Prefix>
    </CommonPrefixes>"#,
            quick_xml::escape::escape(prefix)
        ));
    }

    xml.push_str("\n</ListBucketResult>");
    xml
}

pub fn serialize_delete_objects_result(res: &DeleteObjectsResult) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<DeleteResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">"#,
    );

    for key in &res.deleted {
        xml.push_str(&format!(
            r#"
    <Deleted>
        <Key>{}</Key>
    </Deleted>"#,
            quick_xml::escape::escape(key)
        ));
    }

    for (key, err) in &res.errors {
        xml.push_str(&format!(
            r#"
    <Error>
        <Key>{}</Key>
        <Code>InternalError</Code>
        <Message>{}</Message>
    </Error>"#,
            quick_xml::escape::escape(key),
            quick_xml::escape::escape(err)
        ));
    }

    xml.push_str("\n</DeleteResult>");
    xml
}

pub fn serialize_delete_result(res: &DeleteObjectsResult) -> String {
    serialize_delete_objects_result(res)
}

pub fn serialize_initiate_multipart(bucket: &str, key: &str, upload_id: &str) -> String {
    serialize_initiate_multipart_upload(bucket, key, upload_id)
}

pub fn serialize_complete_multipart(bucket: &str, key: &str, etag: &str, location: &str) -> String {
    serialize_complete_multipart_upload(bucket, key, etag, location)
}

pub fn serialize_copy_object_result(
    etag: &str,
    last_modified: chrono::DateTime<chrono::Utc>,
) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CopyObjectResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
    <LastModified>{}</LastModified>
    <ETag>{}</ETag>
</CopyObjectResult>"#,
        last_modified.to_rfc3339(),
        quick_xml::escape::escape(etag)
    )
}

pub fn serialize_initiate_multipart_upload(bucket: &str, key: &str, upload_id: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<InitiateMultipartUploadResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
    <Bucket>{}</Bucket>
    <Key>{}</Key>
    <UploadId>{}</UploadId>
</InitiateMultipartUploadResult>"#,
        quick_xml::escape::escape(bucket),
        quick_xml::escape::escape(key),
        quick_xml::escape::escape(upload_id)
    )
}

pub fn serialize_complete_multipart_upload(
    bucket: &str,
    key: &str,
    etag: &str,
    location: &str,
) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CompleteMultipartUploadResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
    <Location>{}</Location>
    <Bucket>{}</Bucket>
    <Key>{}</Key>
    <ETag>{}</ETag>
</CompleteMultipartUploadResult>"#,
        quick_xml::escape::escape(location),
        quick_xml::escape::escape(bucket),
        quick_xml::escape::escape(key),
        quick_xml::escape::escape(etag)
    )
}

pub fn serialize_list_parts(
    bucket: &str,
    key: &str,
    upload_id: &str,
    parts: &[PartInfo],
) -> String {
    let mut xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListPartsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
    <Bucket>{}</Bucket>
    <Key>{}</Key>
    <UploadId>{}</UploadId>
    <StorageClass>STANDARD</StorageClass>
    <PartNumberMarker>0</PartNumberMarker>
    <NextPartNumberMarker>0</NextPartNumberMarker>
    <MaxParts>1000</MaxParts>
    <IsTruncated>false</IsTruncated>"#,
        quick_xml::escape::escape(bucket),
        quick_xml::escape::escape(key),
        quick_xml::escape::escape(upload_id)
    );

    for p in parts {
        xml.push_str(&format!(
            r#"
    <Part>
        <PartNumber>{}</PartNumber>
        <LastModified>{}</LastModified>
        <ETag>{}</ETag>
        <Size>{}</Size>
    </Part>"#,
            p.part_number,
            p.last_modified.to_rfc3339(),
            quick_xml::escape::escape(&p.etag),
            p.size
        ));
    }

    xml.push_str("\n</ListPartsResult>");
    xml
}

pub fn serialize_bucket_location(region: &str) -> String {
    if region.is_empty() || region == "us-east-1" {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<LocationConstraint xmlns="http://s3.amazonaws.com/doc/2006-03-01/"/>"#
            .to_string()
    } else {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<LocationConstraint xmlns="http://s3.amazonaws.com/doc/2006-03-01/">{}</LocationConstraint>"#,
            quick_xml::escape::escape(region)
        )
    }
}

pub fn serialize_notification_configuration(config: &BucketNotificationConfig) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<NotificationConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/">"#,
    );

    for q in &config.queue_configurations {
        xml.push_str("\n    <QueueConfiguration>");
        if !q.id.is_empty() {
            xml.push_str(&format!(
                "\n        <Id>{}</Id>",
                quick_xml::escape::escape(&q.id)
            ));
        }
        xml.push_str(&format!(
            "\n        <Queue>{}</Queue>",
            quick_xml::escape::escape(&q.queue_arn)
        ));
        for ev in &q.events {
            xml.push_str(&format!(
                "\n        <Event>{}</Event>",
                quick_xml::escape::escape(ev)
            ));
        }
        if let Some(ref filter) = q.filter {
            xml.push_str("\n        <Filter>\n            <S3Key>");
            for rule in &filter.rules {
                xml.push_str(&format!(
                    "\n                <FilterRule>\n                    <Name>{}</Name>\n                    <Value>{}</Value>\n                </FilterRule>",
                    quick_xml::escape::escape(&rule.name),
                    quick_xml::escape::escape(&rule.value)
                ));
            }
            xml.push_str("\n            </S3Key>\n        </Filter>");
        }
        xml.push_str("\n    </QueueConfiguration>");
    }

    for t in &config.topic_configurations {
        xml.push_str("\n    <TopicConfiguration>");
        if !t.id.is_empty() {
            xml.push_str(&format!(
                "\n        <Id>{}</Id>",
                quick_xml::escape::escape(&t.id)
            ));
        }
        xml.push_str(&format!(
            "\n        <Topic>{}</Topic>",
            quick_xml::escape::escape(&t.topic_arn)
        ));
        for ev in &t.events {
            xml.push_str(&format!(
                "\n        <Event>{}</Event>",
                quick_xml::escape::escape(ev)
            ));
        }
        if let Some(ref filter) = t.filter {
            xml.push_str("\n        <Filter>\n            <S3Key>");
            for rule in &filter.rules {
                xml.push_str(&format!(
                    "\n                <FilterRule>\n                    <Name>{}</Name>\n                    <Value>{}</Value>\n                </FilterRule>",
                    quick_xml::escape::escape(&rule.name),
                    quick_xml::escape::escape(&rule.value)
                ));
            }
            xml.push_str("\n            </S3Key>\n        </Filter>");
        }
        xml.push_str("\n    </TopicConfiguration>");
    }

    if config.eventbridge_enabled {
        xml.push_str("\n    <EventBridgeConfiguration/>");
    }

    xml.push_str("\n</NotificationConfiguration>");
    xml
}

pub fn parse_notification_configuration(
    body: &[u8],
) -> Result<BucketNotificationConfig, RustStackError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(body);
    reader.config_mut().trim_text(true);

    let mut config = BucketNotificationConfig::default();
    let mut buf = Vec::new();

    let mut in_queue_config = false;
    let mut in_topic_config = false;
    let mut in_filter_rule = false;

    let mut cur_q = QueueNotificationConfig::default();
    let mut cur_t = TopicNotificationConfig::default();
    let mut cur_rules = Vec::new();
    let mut cur_rule_name = String::new();
    let mut cur_rule_value = String::new();

    let mut current_tag = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_tag = tag.clone();
                if tag == "QueueConfiguration" {
                    in_queue_config = true;
                    cur_q = QueueNotificationConfig::default();
                    cur_rules.clear();
                } else if tag == "TopicConfiguration" {
                    in_topic_config = true;
                    cur_t = TopicNotificationConfig::default();
                    cur_rules.clear();
                } else if tag == "EventBridgeConfiguration" {
                    config.eventbridge_enabled = true;
                } else if tag == "FilterRule" {
                    in_filter_rule = true;
                    cur_rule_name.clear();
                    cur_rule_value.clear();
                }
            }
            Ok(Event::Empty(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "EventBridgeConfiguration" {
                    config.eventbridge_enabled = true;
                }
            }
            Ok(Event::Text(e)) => {
                let text = e
                    .unescape()
                    .map_err(|err| RustStackError::BadRequest(err.to_string()))?;
                let text_str = text.into_owned();

                if in_filter_rule {
                    if current_tag == "Name" {
                        cur_rule_name = text_str;
                    } else if current_tag == "Value" {
                        cur_rule_value = text_str;
                    }
                } else if in_queue_config {
                    if current_tag == "Id" {
                        cur_q.id = text_str;
                    } else if current_tag == "Queue" || current_tag == "QueueConfiguration" {
                        cur_q.queue_arn = text_str;
                    } else if current_tag == "Event" {
                        cur_q.events.push(text_str);
                    }
                } else if in_topic_config {
                    if current_tag == "Id" {
                        cur_t.id = text_str;
                    } else if current_tag == "Topic" || current_tag == "TopicConfiguration" {
                        cur_t.topic_arn = text_str;
                    } else if current_tag == "Event" {
                        cur_t.events.push(text_str);
                    }
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "FilterRule" {
                    if !cur_rule_name.is_empty() && !cur_rule_value.is_empty() {
                        cur_rules.push(NotificationFilterRule {
                            name: cur_rule_name.clone(),
                            value: cur_rule_value.clone(),
                        });
                    }
                    in_filter_rule = false;
                } else if tag == "QueueConfiguration" {
                    if !cur_rules.is_empty() {
                        cur_q.filter = Some(NotificationFilter {
                            rules: cur_rules.clone(),
                        });
                    }
                    if !cur_q.queue_arn.is_empty() {
                        config.queue_configurations.push(cur_q.clone());
                    }
                    in_queue_config = false;
                } else if tag == "TopicConfiguration" {
                    if !cur_rules.is_empty() {
                        cur_t.filter = Some(NotificationFilter {
                            rules: cur_rules.clone(),
                        });
                    }
                    if !cur_t.topic_arn.is_empty() {
                        config.topic_configurations.push(cur_t.clone());
                    }
                    in_topic_config = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(RustStackError::BadRequest(format!(
                    "XML parse error: {}",
                    e
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(config)
}

pub fn parse_delete_objects_request(body: &[u8]) -> Result<(Vec<String>, bool), RustStackError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(body);
    reader.config_mut().trim_text(true);

    let mut keys = Vec::new();
    let mut quiet = false;
    let mut current_tag = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                let text = e
                    .unescape()
                    .map_err(|err| RustStackError::BadRequest(err.to_string()))?;
                if current_tag == "Key" {
                    keys.push(text.into_owned());
                } else if current_tag == "Quiet" {
                    quiet = text.as_ref() == "true";
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(RustStackError::BadRequest(format!(
                    "XML parse error: {}",
                    e
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok((keys, quiet))
}

pub fn parse_complete_multipart_request(body: &[u8]) -> Result<Vec<CompletedPart>, RustStackError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(body);
    reader.config_mut().trim_text(true);

    let mut parts = Vec::new();
    let mut current_part_num: Option<i32> = None;
    let mut current_etag: Option<String> = None;
    let mut current_tag = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                let text = e
                    .unescape()
                    .map_err(|err| RustStackError::BadRequest(err.to_string()))?;
                if current_tag == "PartNumber" {
                    current_part_num = text.parse().ok();
                } else if current_tag == "ETag" {
                    current_etag = Some(text.trim_matches('"').to_string());
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "Part" {
                    if let (Some(num), Some(etag)) = (current_part_num.take(), current_etag.take())
                    {
                        parts.push(CompletedPart {
                            part_number: num,
                            etag: format!("\"{}\"", etag),
                        });
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(RustStackError::BadRequest(format!(
                    "XML parse error: {}",
                    e
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    parts.sort_by_key(|p| p.part_number);
    Ok(parts)
}

pub fn serialize_get_bucket_versioning(status: Option<&str>) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<VersioningConfiguration xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">");
    if let Some(s) = status {
        xml.push_str(&format!("\n    <Status>{}</Status>", s));
    }
    xml.push_str("\n</VersioningConfiguration>");
    xml
}

pub fn parse_put_bucket_versioning(body: &[u8]) -> Result<String, RustStackError> {
    let text = String::from_utf8_lossy(body);
    if text.contains("<Status>Enabled</Status>") || text.contains("<Status>enabled</Status>") {
        Ok("Enabled".to_string())
    } else if text.contains("<Status>Suspended</Status>") || text.contains("<Status>suspended</Status>") {
        Ok("Suspended".to_string())
    } else {
        Ok("Enabled".to_string())
    }
}

pub fn serialize_list_object_versions(res: &crate::types::ListObjectVersionsResult) -> String {
    let mut xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListVersionsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
    <Name>{}</Name>
    <Prefix>{}</Prefix>
    <MaxKeys>{}</MaxKeys>
    <IsTruncated>{}</IsTruncated>"#,
        quick_xml::escape::escape(&res.name),
        quick_xml::escape::escape(&res.prefix),
        res.max_keys,
        res.is_truncated
    );

    if let Some(ref km) = res.key_marker {
        xml.push_str(&format!("\n    <KeyMarker>{}</KeyMarker>", quick_xml::escape::escape(km)));
    }
    if let Some(ref vm) = res.version_id_marker {
        xml.push_str(&format!("\n    <VersionIdMarker>{}</VersionIdMarker>", quick_xml::escape::escape(vm)));
    }
    if let Some(ref nkm) = res.next_key_marker {
        xml.push_str(&format!("\n    <NextKeyMarker>{}</NextKeyMarker>", quick_xml::escape::escape(nkm)));
    }
    if let Some(ref nvm) = res.next_version_id_marker {
        xml.push_str(&format!("\n    <NextVersionIdMarker>{}</NextVersionIdMarker>", quick_xml::escape::escape(nvm)));
    }

    for v in &res.versions {
        xml.push_str(&format!(
            r#"
    <Version>
        <Key>{}</Key>
        <VersionId>{}</VersionId>
        <IsLatest>{}</IsLatest>
        <LastModified>{}</LastModified>
        <ETag>{}</ETag>
        <Size>{}</Size>
        <Owner>
            <ID>000000000000</ID>
            <DisplayName>ruststack</DisplayName>
        </Owner>
        <StorageClass>STANDARD</StorageClass>
    </Version>"#,
            quick_xml::escape::escape(&v.key),
            v.version_id,
            v.is_latest,
            v.last_modified.to_rfc3339(),
            quick_xml::escape::escape(&v.etag),
            v.size
        ));
    }

    for d in &res.delete_markers {
        xml.push_str(&format!(
            r#"
    <DeleteMarker>
        <Key>{}</Key>
        <VersionId>{}</VersionId>
        <IsLatest>{}</IsLatest>
        <LastModified>{}</LastModified>
        <Owner>
            <ID>000000000000</ID>
            <DisplayName>ruststack</DisplayName>
        </Owner>
    </DeleteMarker>"#,
            quick_xml::escape::escape(&d.key),
            d.version_id,
            d.is_latest,
            d.last_modified.to_rfc3339()
        ));
    }

    xml.push_str("\n</ListVersionsResult>");
    xml
}

pub fn serialize_get_bucket_lifecycle(config: &crate::types::BucketLifecycleConfig) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<LifecycleConfiguration xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">");
    for r in &config.rules {
        xml.push_str("\n    <Rule>");
        if let Some(ref id) = r.id {
            xml.push_str(&format!("\n        <ID>{}</ID>", quick_xml::escape::escape(id)));
        }
        xml.push_str(&format!("\n        <Status>{}</Status>", r.status));
        if let Some(ref prefix) = r.prefix {
            xml.push_str(&format!("\n        <Filter><Prefix>{}</Prefix></Filter>", quick_xml::escape::escape(prefix)));
        }
        if let Some(days) = r.expiration_days {
            xml.push_str(&format!("\n        <Expiration><Days>{}</Days></Expiration>", days));
        }
        xml.push_str("\n    </Rule>");
    }
    xml.push_str("\n</LifecycleConfiguration>");
    xml
}

pub fn parse_put_bucket_lifecycle(body: &[u8]) -> Result<crate::types::BucketLifecycleConfig, RustStackError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(body);
    reader.config_mut().trim_text(true);

    let mut rules = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_status = "Enabled".to_string();
    let mut current_prefix: Option<String> = None;
    let mut current_days: Option<i32> = None;
    let mut current_tag = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().map_err(|err| RustStackError::BadRequest(err.to_string()))?;
                match current_tag.as_str() {
                    "ID" => current_id = Some(text.to_string()),
                    "Status" => current_status = text.to_string(),
                    "Prefix" => current_prefix = Some(text.to_string()),
                    "Days" => current_days = text.parse().ok(),
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "Rule" {
                    rules.push(crate::types::LifecycleRule {
                        id: current_id.take(),
                        status: current_status.clone(),
                        prefix: current_prefix.take(),
                        expiration_days: current_days.take(),
                    });
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(RustStackError::BadRequest(format!("XML parse error: {}", e))),
            _ => {}
        }
        buf.clear();
    }

    Ok(crate::types::BucketLifecycleConfig { rules })
}

pub fn serialize_get_bucket_cors(config: &crate::types::BucketCorsConfig) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<CORSConfiguration xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">");
    for r in &config.rules {
        xml.push_str("\n    <CORSRule>");
        if let Some(ref id) = r.id {
            xml.push_str(&format!("\n        <ID>{}</ID>", quick_xml::escape::escape(id)));
        }
        for m in &r.allowed_methods {
            xml.push_str(&format!("\n        <AllowedMethod>{}</AllowedMethod>", quick_xml::escape::escape(m)));
        }
        for o in &r.allowed_origins {
            xml.push_str(&format!("\n        <AllowedOrigin>{}</AllowedOrigin>", quick_xml::escape::escape(o)));
        }
        for h in &r.allowed_headers {
            xml.push_str(&format!("\n        <AllowedHeader>{}</AllowedHeader>", quick_xml::escape::escape(h)));
        }
        for eh in &r.expose_headers {
            xml.push_str(&format!("\n        <ExposeHeader>{}</ExposeHeader>", quick_xml::escape::escape(eh)));
        }
        if let Some(max_age) = r.max_age_seconds {
            xml.push_str(&format!("\n        <MaxAgeSeconds>{}</MaxAgeSeconds>", max_age));
        }
        xml.push_str("\n    </CORSRule>");
    }
    xml.push_str("\n</CORSConfiguration>");
    xml
}

pub fn parse_put_bucket_cors(body: &[u8]) -> Result<crate::types::BucketCorsConfig, RustStackError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(body);
    reader.config_mut().trim_text(true);

    let mut rules = Vec::new();
    let mut current_id: Option<String> = None;
    let mut methods = Vec::new();
    let mut origins = Vec::new();
    let mut headers = Vec::new();
    let mut expose_headers = Vec::new();
    let mut max_age: Option<i32> = None;
    let mut current_tag = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().map_err(|err| RustStackError::BadRequest(err.to_string()))?;
                match current_tag.as_str() {
                    "ID" => current_id = Some(text.to_string()),
                    "AllowedMethod" => methods.push(text.to_string()),
                    "AllowedOrigin" => origins.push(text.to_string()),
                    "AllowedHeader" => headers.push(text.to_string()),
                    "ExposeHeader" => expose_headers.push(text.to_string()),
                    "MaxAgeSeconds" => max_age = text.parse().ok(),
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "CORSRule" {
                    rules.push(crate::types::CorsRule {
                        id: current_id.take(),
                        allowed_methods: std::mem::take(&mut methods),
                        allowed_origins: std::mem::take(&mut origins),
                        allowed_headers: std::mem::take(&mut headers),
                        expose_headers: std::mem::take(&mut expose_headers),
                        max_age_seconds: max_age.take(),
                    });
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(RustStackError::BadRequest(format!("XML parse error: {}", e))),
            _ => {}
        }
        buf.clear();
    }

    Ok(crate::types::BucketCorsConfig { rules })
}

pub fn serialize_get_bucket_tagging(tags: &std::collections::HashMap<String, String>) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Tagging xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n    <TagSet>");
    for (k, v) in tags {
        xml.push_str(&format!(
            "\n        <Tag><Key>{}</Key><Value>{}</Value></Tag>",
            quick_xml::escape::escape(k),
            quick_xml::escape::escape(v)
        ));
    }
    xml.push_str("\n    </TagSet>\n</Tagging>");
    xml
}

pub fn parse_put_bucket_tagging(body: &[u8]) -> Result<std::collections::HashMap<String, String>, RustStackError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(body);
    reader.config_mut().trim_text(true);

    let mut tags = std::collections::HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_val: Option<String> = None;
    let mut current_tag = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().map_err(|err| RustStackError::BadRequest(err.to_string()))?;
                if current_tag == "Key" {
                    current_key = Some(text.to_string());
                } else if current_tag == "Value" {
                    current_val = Some(text.to_string());
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "Tag" {
                    if let (Some(k), Some(v)) = (current_key.take(), current_val.take()) {
                        tags.insert(k, v);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(RustStackError::BadRequest(format!("XML parse error: {}", e))),
            _ => {}
        }
        buf.clear();
    }

    Ok(tags)
}

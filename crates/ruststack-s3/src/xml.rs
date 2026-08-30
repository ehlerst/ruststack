use crate::types::{BucketInfo, CompletedPart, DeleteObjectsResult, ListObjectsV2Result, PartInfo};
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

pub fn serialize_delete_result(res: &DeleteObjectsResult) -> String {
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

pub fn serialize_initiate_multipart(bucket: &str, key: &str, upload_id: &str) -> String {
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

pub fn serialize_complete_multipart(bucket: &str, key: &str, location: &str, etag: &str) -> String {
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
    <NextPartNumberMarker>{}</NextPartNumberMarker>
    <MaxParts>1000</MaxParts>
    <IsTruncated>false</IsTruncated>"#,
        quick_xml::escape::escape(bucket),
        quick_xml::escape::escape(key),
        quick_xml::escape::escape(upload_id),
        parts.last().map(|p| p.part_number).unwrap_or(0)
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

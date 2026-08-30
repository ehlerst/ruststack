use crate::storage::S3Storage;
use crate::types::ByteRange;
use crate::xml;
use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Method, Request, Response, StatusCode, Uri};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_core::RustStackError;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn handle_s3_request(storage: Arc<dyn S3Storage>, req: Request<Body>) -> Response<Body> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let (parts, body) = req.into_parts();
    let method = parts.method;
    let uri = parts.uri;
    let headers = parts.headers;

    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return make_s3_error_response(&RustStackError::BadRequest(e.to_string()), &request_id);
        }
    };

    let (bucket_opt, key_opt) = extract_bucket_and_key(&uri, &headers);
    let query_map = parse_query(uri.query());

    let result = dispatch_s3_op(
        storage.as_ref(),
        &method,
        bucket_opt.as_deref(),
        key_opt.as_deref(),
        &query_map,
        &headers,
        body_bytes,
        &request_id,
    );

    match result {
        Ok(res) => res,
        Err(err) => make_s3_error_response(&err, &request_id),
    }
}

fn extract_bucket_and_key(uri: &Uri, headers: &HeaderMap) -> (Option<String>, Option<String>) {
    let path = uri.path();
    let mut virtual_bucket = None;

    if let Some(host) = headers.get("host").and_then(|v| v.to_str().ok()) {
        let host_name = host.split(':').next().unwrap_or(host);
        if let Some(pos) = host_name.find(".s3.") {
            virtual_bucket = Some(host_name[..pos].to_string());
        } else if let Some(stripped) = host_name.strip_suffix(".localhost") {
            if !stripped.is_empty() && stripped != "localhost" && stripped != "s3" {
                virtual_bucket = Some(stripped.to_string());
            }
        }
    }

    if let Some(b) = virtual_bucket {
        let key = path.trim_start_matches('/').to_string();
        let key_opt = if key.is_empty() { None } else { Some(key) };
        return (Some(b), key_opt);
    }

    // Path-style: /<bucket>/<key...>
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return (None, None);
    }

    let mut parts = trimmed.splitn(2, '/');
    let bucket = parts.next().map(|s| s.to_string());
    let key = parts
        .next()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());

    (bucket, key)
}

fn parse_query(query_opt: Option<&str>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(q) = query_opt {
        for (k, v) in form_urlencoded::parse(q.as_bytes()) {
            map.insert(k.into_owned(), v.into_owned());
        }
    }
    map
}

#[allow(clippy::too_many_arguments)]
fn dispatch_s3_op(
    storage: &dyn S3Storage,
    method: &Method,
    bucket_opt: Option<&str>,
    key_opt: Option<&str>,
    query: &HashMap<String, String>,
    headers: &HeaderMap,
    body: Bytes,
    request_id: &str,
) -> Result<Response<Body>, RustStackError> {
    match (method, bucket_opt, key_opt) {
        // GET / -> ListBuckets
        (&Method::GET, None, None) => {
            let buckets = storage.list_buckets()?;
            let xml_body = xml::serialize_list_buckets(&buckets, "000000000000");
            let mut res = Response::new(Body::from(xml_body));
            *res.status_mut() = StatusCode::OK;
            res.headers_mut()
                .insert("content-type", HeaderValue::from_static("application/xml"));
            res.headers_mut().insert(
                "x-amz-request-id",
                HeaderValue::from_str(request_id).unwrap(),
            );
            Ok(res)
        }

        // PUT /<bucket> -> CreateBucket
        (&Method::PUT, Some(bucket), None) => {
            let region = "us-east-1";
            storage.create_bucket(bucket, region)?;
            let mut res = Response::new(Body::empty());
            *res.status_mut() = StatusCode::OK;
            res.headers_mut().insert(
                "location",
                HeaderValue::from_str(&format!("/{}", bucket)).unwrap(),
            );
            res.headers_mut().insert(
                "x-amz-request-id",
                HeaderValue::from_str(request_id).unwrap(),
            );
            Ok(res)
        }

        // DELETE /<bucket> -> DeleteBucket
        (&Method::DELETE, Some(bucket), None) => {
            storage.delete_bucket(bucket)?;
            let mut res = Response::new(Body::empty());
            *res.status_mut() = StatusCode::NO_CONTENT;
            res.headers_mut().insert(
                "x-amz-request-id",
                HeaderValue::from_str(request_id).unwrap(),
            );
            Ok(res)
        }

        // HEAD /<bucket> -> HeadBucket
        (&Method::HEAD, Some(bucket), None) => {
            let _ = storage.head_bucket(bucket)?;
            let mut res = Response::new(Body::empty());
            *res.status_mut() = StatusCode::OK;
            res.headers_mut().insert(
                "x-amz-request-id",
                HeaderValue::from_str(request_id).unwrap(),
            );
            Ok(res)
        }

        // GET /<bucket> -> ListObjectsV2 / GetBucketLocation
        (&Method::GET, Some(bucket), None) => {
            if query.contains_key("location") {
                let info = storage.head_bucket(bucket)?;
                let xml_body = xml::serialize_bucket_location(&info.region);
                let mut res = Response::new(Body::from(xml_body));
                *res.status_mut() = StatusCode::OK;
                res.headers_mut()
                    .insert("content-type", HeaderValue::from_static("application/xml"));
                res.headers_mut().insert(
                    "x-amz-request-id",
                    HeaderValue::from_str(request_id).unwrap(),
                );
                return Ok(res);
            }

            let prefix = query.get("prefix").map(|s| s.as_str());
            let delimiter = query.get("delimiter").map(|s| s.as_str());
            let max_keys = query
                .get("max-keys")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000);
            let continuation_token = query.get("continuation-token").map(|s| s.as_str());
            let start_after = query.get("start-after").map(|s| s.as_str());

            let list_res = storage.list_objects_v2(
                bucket,
                prefix,
                delimiter,
                max_keys,
                continuation_token,
                start_after,
            )?;

            let xml_body = xml::serialize_list_objects_v2(&list_res);
            let mut res = Response::new(Body::from(xml_body));
            *res.status_mut() = StatusCode::OK;
            res.headers_mut()
                .insert("content-type", HeaderValue::from_static("application/xml"));
            res.headers_mut().insert(
                "x-amz-request-id",
                HeaderValue::from_str(request_id).unwrap(),
            );
            Ok(res)
        }

        // POST /<bucket>?delete -> DeleteObjects
        (&Method::POST, Some(bucket), None) if query.contains_key("delete") => {
            let (keys, quiet) = xml::parse_delete_objects_request(&body)?;
            let del_res = storage.delete_objects(bucket, keys, quiet)?;
            let xml_body = xml::serialize_delete_result(&del_res);
            let mut res = Response::new(Body::from(xml_body));
            *res.status_mut() = StatusCode::OK;
            res.headers_mut()
                .insert("content-type", HeaderValue::from_static("application/xml"));
            res.headers_mut().insert(
                "x-amz-request-id",
                HeaderValue::from_str(request_id).unwrap(),
            );
            Ok(res)
        }

        // PUT /<bucket>/<key> -> PutObject / CopyObject / UploadPart
        (&Method::PUT, Some(bucket), Some(key)) => {
            // Check if UploadPart: ?uploadId=...&partNumber=...
            if let (Some(upload_id), Some(part_num_str)) =
                (query.get("uploadId"), query.get("partNumber"))
            {
                let part_num: i32 = part_num_str.parse().map_err(|_| {
                    RustStackError::s3_bad_request("InvalidArgument", "Invalid partNumber")
                })?;
                let etag = storage.upload_part(bucket, key, upload_id, part_num, body)?;
                let mut res = Response::new(Body::empty());
                *res.status_mut() = StatusCode::OK;
                res.headers_mut()
                    .insert("etag", HeaderValue::from_str(&etag).unwrap());
                res.headers_mut().insert(
                    "x-amz-request-id",
                    HeaderValue::from_str(request_id).unwrap(),
                );
                return Ok(res);
            }

            // Check if CopyObject: x-amz-copy-source header
            if let Some(copy_src) = headers
                .get("x-amz-copy-source")
                .and_then(|v| v.to_str().ok())
            {
                let clean_src = copy_src.trim_start_matches('/');
                let mut src_parts = clean_src.splitn(2, '/');
                let src_bucket = src_parts.next().unwrap_or("");
                let src_key = src_parts.next().unwrap_or("");

                let meta = storage.copy_object(src_bucket, src_key, bucket, key)?;
                let xml_body = format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<CopyObjectResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
    <LastModified>{}</LastModified>
    <ETag>{}</ETag>
</CopyObjectResult>"#,
                    meta.last_modified.to_rfc3339(),
                    quick_xml::escape::escape(&meta.etag)
                );
                let mut res = Response::new(Body::from(xml_body));
                *res.status_mut() = StatusCode::OK;
                res.headers_mut()
                    .insert("content-type", HeaderValue::from_static("application/xml"));
                res.headers_mut().insert(
                    "x-amz-request-id",
                    HeaderValue::from_str(request_id).unwrap(),
                );
                return Ok(res);
            }

            // Standard PutObject
            let content_type = headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            let mut user_metadata = HashMap::new();
            for (k, v) in headers.iter() {
                let name = k.as_str();
                if name.starts_with("x-amz-meta-") {
                    if let Ok(val_str) = v.to_str() {
                        user_metadata.insert(name.to_string(), val_str.to_string());
                    }
                }
            }

            let meta = storage.put_object(bucket, key, body, content_type, user_metadata)?;
            let mut res = Response::new(Body::empty());
            *res.status_mut() = StatusCode::OK;
            res.headers_mut()
                .insert("etag", HeaderValue::from_str(&meta.etag).unwrap());
            res.headers_mut().insert(
                "x-amz-request-id",
                HeaderValue::from_str(request_id).unwrap(),
            );
            Ok(res)
        }

        // GET /<bucket>/<key> -> GetObject / ListParts
        (&Method::GET, Some(bucket), Some(key)) => {
            // Check if ListParts: ?uploadId=...
            if let Some(upload_id) = query.get("uploadId") {
                let parts = storage.list_parts(bucket, key, upload_id)?;
                let xml_body = xml::serialize_list_parts(bucket, key, upload_id, &parts);
                let mut res = Response::new(Body::from(xml_body));
                *res.status_mut() = StatusCode::OK;
                res.headers_mut()
                    .insert("content-type", HeaderValue::from_static("application/xml"));
                res.headers_mut().insert(
                    "x-amz-request-id",
                    HeaderValue::from_str(request_id).unwrap(),
                );
                return Ok(res);
            }

            let range_header = headers.get("range").and_then(|v| v.to_str().ok());
            let meta = storage.head_object(bucket, key)?;

            let range_opt = if let Some(rh) = range_header {
                ByteRange::parse(rh, meta.size)
            } else {
                None
            };

            let (meta, data, content_range) = storage.get_object(bucket, key, range_opt)?;
            let mut res = Response::new(Body::from(data));

            if let Some(cr) = content_range {
                *res.status_mut() = StatusCode::PARTIAL_CONTENT;
                res.headers_mut()
                    .insert("content-range", HeaderValue::from_str(&cr).unwrap());
            } else {
                *res.status_mut() = StatusCode::OK;
            }

            res.headers_mut().insert(
                "content-type",
                HeaderValue::from_str(&meta.content_type).unwrap(),
            );
            res.headers_mut().insert(
                "content-length",
                HeaderValue::from_str(&meta.size.to_string()).unwrap(),
            );
            res.headers_mut()
                .insert("etag", HeaderValue::from_str(&meta.etag).unwrap());
            res.headers_mut().insert(
                "last-modified",
                HeaderValue::from_str(&meta.last_modified.to_rfc2822()).unwrap(),
            );
            res.headers_mut()
                .insert("accept-ranges", HeaderValue::from_static("bytes"));
            res.headers_mut().insert(
                "x-amz-request-id",
                HeaderValue::from_str(request_id).unwrap(),
            );

            for (k, v) in &meta.user_metadata {
                if let Ok(val) = HeaderValue::from_str(v) {
                    if let Ok(name) = http::header::HeaderName::from_bytes(k.as_bytes()) {
                        res.headers_mut().insert(name, val);
                    }
                }
            }

            Ok(res)
        }

        // HEAD /<bucket>/<key> -> HeadObject
        (&Method::HEAD, Some(bucket), Some(key)) => {
            let meta = storage.head_object(bucket, key)?;
            let mut res = Response::new(Body::empty());
            *res.status_mut() = StatusCode::OK;
            res.headers_mut().insert(
                "content-type",
                HeaderValue::from_str(&meta.content_type).unwrap(),
            );
            res.headers_mut().insert(
                "content-length",
                HeaderValue::from_str(&meta.size.to_string()).unwrap(),
            );
            res.headers_mut()
                .insert("etag", HeaderValue::from_str(&meta.etag).unwrap());
            res.headers_mut().insert(
                "last-modified",
                HeaderValue::from_str(&meta.last_modified.to_rfc2822()).unwrap(),
            );
            res.headers_mut()
                .insert("accept-ranges", HeaderValue::from_static("bytes"));
            res.headers_mut().insert(
                "x-amz-request-id",
                HeaderValue::from_str(request_id).unwrap(),
            );

            for (k, v) in &meta.user_metadata {
                if let Ok(val) = HeaderValue::from_str(v) {
                    if let Ok(name) = http::header::HeaderName::from_bytes(k.as_bytes()) {
                        res.headers_mut().insert(name, val);
                    }
                }
            }

            Ok(res)
        }

        // DELETE /<bucket>/<key> -> DeleteObject / AbortMultipartUpload
        (&Method::DELETE, Some(bucket), Some(key)) => {
            if let Some(upload_id) = query.get("uploadId") {
                storage.abort_multipart_upload(bucket, key, upload_id)?;
                let mut res = Response::new(Body::empty());
                *res.status_mut() = StatusCode::NO_CONTENT;
                res.headers_mut().insert(
                    "x-amz-request-id",
                    HeaderValue::from_str(request_id).unwrap(),
                );
                return Ok(res);
            }

            storage.delete_object(bucket, key)?;
            let mut res = Response::new(Body::empty());
            *res.status_mut() = StatusCode::NO_CONTENT;
            res.headers_mut().insert(
                "x-amz-request-id",
                HeaderValue::from_str(request_id).unwrap(),
            );
            Ok(res)
        }

        // POST /<bucket>/<key> -> CreateMultipartUpload / CompleteMultipartUpload
        (&Method::POST, Some(bucket), Some(key)) => {
            if query.contains_key("uploads") {
                let content_type = headers
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                let mut user_metadata = HashMap::new();
                for (k, v) in headers.iter() {
                    let name = k.as_str();
                    if name.starts_with("x-amz-meta-") {
                        if let Ok(val_str) = v.to_str() {
                            user_metadata.insert(name.to_string(), val_str.to_string());
                        }
                    }
                }

                let upload_id =
                    storage.create_multipart_upload(bucket, key, content_type, user_metadata)?;
                let xml_body = xml::serialize_initiate_multipart(bucket, key, &upload_id);
                let mut res = Response::new(Body::from(xml_body));
                *res.status_mut() = StatusCode::OK;
                res.headers_mut()
                    .insert("content-type", HeaderValue::from_static("application/xml"));
                res.headers_mut().insert(
                    "x-amz-request-id",
                    HeaderValue::from_str(request_id).unwrap(),
                );
                return Ok(res);
            }

            if let Some(upload_id) = query.get("uploadId") {
                let parts = xml::parse_complete_multipart_request(&body)?;
                let meta = storage.complete_multipart_upload(bucket, key, upload_id, parts)?;
                let location = format!("http://localhost:4566/{}/{}", bucket, key);
                let xml_body =
                    xml::serialize_complete_multipart(bucket, key, &location, &meta.etag);
                let mut res = Response::new(Body::from(xml_body));
                *res.status_mut() = StatusCode::OK;
                res.headers_mut()
                    .insert("content-type", HeaderValue::from_static("application/xml"));
                res.headers_mut().insert(
                    "x-amz-request-id",
                    HeaderValue::from_str(request_id).unwrap(),
                );
                return Ok(res);
            }

            Err(RustStackError::BadRequest(
                "Unsupported POST operation on key".to_string(),
            ))
        }

        _ => Err(RustStackError::NotFound("Endpoint not found".to_string())),
    }
}

pub fn make_s3_error_response(err: &RustStackError, request_id: &str) -> Response<Body> {
    let xml_err = err.to_s3_xml(request_id);
    let mut res = Response::new(Body::from(xml_err));
    *res.status_mut() = err.status_code();
    res.headers_mut()
        .insert("content-type", HeaderValue::from_static("application/xml"));
    res.headers_mut().insert(
        "x-amz-request-id",
        HeaderValue::from_str(request_id).unwrap(),
    );
    res
}

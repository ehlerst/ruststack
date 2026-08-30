use crate::types::{
    BucketInfo, BucketNotificationConfig, BucketSnapshot, ByteRange, CompletedPart,
    DeleteObjectsResult, ListObjectsV2Result, ObjectMetadata, PartInfo, S3Snapshot,
    StoredObjectSnapshot,
};
use base64::Engine;
use bytes::Bytes;
use chrono::Utc;
use dashmap::DashMap;
use md5::{Digest, Md5};
use parking_lot::RwLock;
use ruststack_core::RustStackError;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

pub trait S3NotificationTarget: Send + Sync {
    fn send_sqs(&self, queue_arn: &str, payload: &str);
    fn send_sns(&self, topic_arn: &str, payload: &str);
    fn send_eventbridge(&self, source: &str, detail_type: &str, detail: &str);
}

pub trait S3Storage: Send + Sync {
    fn reset(&self);
    fn dump_state(&self) -> S3Snapshot;
    fn load_state(&self, snapshot: S3Snapshot);

    fn create_bucket(&self, bucket: &str, region: &str) -> Result<(), RustStackError>;
    fn delete_bucket(&self, bucket: &str) -> Result<(), RustStackError>;
    fn head_bucket(&self, bucket: &str) -> Result<BucketInfo, RustStackError>;
    fn list_buckets(&self) -> Result<Vec<BucketInfo>, RustStackError>;

    fn put_object(
        &self,
        bucket: &str,
        key: &str,
        data: Bytes,
        content_type: Option<String>,
        user_metadata: HashMap<String, String>,
    ) -> Result<ObjectMetadata, RustStackError>;

    fn get_object(
        &self,
        bucket: &str,
        key: &str,
        range: Option<ByteRange>,
    ) -> Result<(ObjectMetadata, Bytes, Option<String>), RustStackError>;

    fn head_object(&self, bucket: &str, key: &str) -> Result<ObjectMetadata, RustStackError>;
    fn delete_object(&self, bucket: &str, key: &str) -> Result<(), RustStackError>;
    fn delete_objects(
        &self,
        bucket: &str,
        keys: Vec<String>,
        quiet: bool,
    ) -> Result<DeleteObjectsResult, RustStackError>;

    fn list_objects_v2(
        &self,
        bucket: &str,
        prefix: Option<&str>,
        delimiter: Option<&str>,
        max_keys: usize,
        continuation_token: Option<&str>,
        start_after: Option<&str>,
    ) -> Result<ListObjectsV2Result, RustStackError>;

    fn copy_object(
        &self,
        src_bucket: &str,
        src_key: &str,
        dest_bucket: &str,
        dest_key: &str,
    ) -> Result<ObjectMetadata, RustStackError>;

    // Multipart
    fn create_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        content_type: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Result<String, RustStackError>;

    fn upload_part(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        part_number: i32,
        data: Bytes,
    ) -> Result<String, RustStackError>;

    fn complete_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        parts: Vec<CompletedPart>,
    ) -> Result<ObjectMetadata, RustStackError>;

    fn abort_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
    ) -> Result<(), RustStackError>;

    fn list_parts(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
    ) -> Result<Vec<PartInfo>, RustStackError>;

    // Notification Configurations
    fn get_bucket_notification_configuration(
        &self,
        bucket: &str,
    ) -> Result<BucketNotificationConfig, RustStackError>;

    fn put_bucket_notification_configuration(
        &self,
        bucket: &str,
        config: BucketNotificationConfig,
    ) -> Result<(), RustStackError>;

    fn set_notification_target(&self, target: Arc<dyn S3NotificationTarget>);
}

struct StoredObject {
    metadata: ObjectMetadata,
    data: Bytes,
}

struct StoredPart {
    part_number: i32,
    etag: String,
    size: u64,
    data: Bytes,
    last_modified: chrono::DateTime<Utc>,
}

#[allow(dead_code)]
struct StoredMultipartUpload {
    upload_id: String,
    key: String,
    content_type: Option<String>,
    user_metadata: HashMap<String, String>,
    parts: RwLock<BTreeMap<i32, StoredPart>>,
}

struct Bucket {
    info: BucketInfo,
    objects: DashMap<String, StoredObject>,
    multipart_uploads: DashMap<String, StoredMultipartUpload>,
    notifications: RwLock<BucketNotificationConfig>,
}

#[derive(Default)]
pub struct InMemoryStorage {
    buckets: DashMap<String, Bucket>,
    notification_target: RwLock<Option<Arc<dyn S3NotificationTarget>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            buckets: DashMap::new(),
            notification_target: RwLock::new(None),
        }
    }

    fn calculate_etag(data: &[u8]) -> String {
        let mut hasher = Md5::new();
        hasher.update(data);
        let result = hasher.finalize();
        format!("\"{}\"", hex::encode(result))
    }

    fn event_matches(pattern: &str, event_name: &str) -> bool {
        if pattern == "*" || pattern == event_name {
            return true;
        }
        if let Some(prefix) = pattern.strip_suffix('*') {
            if event_name.starts_with(prefix) {
                return true;
            }
        }
        false
    }

    fn dispatch_event(&self, bucket: &str, key: &str, size: u64, etag: &str, event_name: &str) {
        let target_opt = self.notification_target.read().clone();
        let target = match target_opt {
            Some(t) => t,
            None => return,
        };

        let bucket_entry = match self.buckets.get(bucket) {
            Some(b) => b,
            None => return,
        };

        let notif_config = bucket_entry.notifications.read().clone();
        if notif_config.is_empty() {
            return;
        }

        let event_time = Utc::now().to_rfc3339();
        let s3_event_payload = serde_json::json!({
            "Records": [
                {
                    "eventVersion": "2.1",
                    "eventSource": "aws:s3",
                    "awsRegion": bucket_entry.info.region,
                    "eventTime": event_time,
                    "eventName": event_name,
                    "userIdentity": {
                        "principalId": "000000000000"
                    },
                    "requestParameters": {
                        "sourceIPAddress": "127.0.0.1"
                    },
                    "responseElements": {
                        "x-amz-request-id": uuid::Uuid::new_v4().to_string(),
                        "x-amz-id-2": uuid::Uuid::new_v4().to_string()
                    },
                    "s3": {
                        "s3SchemaVersion": "1.0",
                        "configurationId": "ruststack-notification",
                        "bucket": {
                            "name": bucket,
                            "ownerIdentity": {
                                "principalId": "000000000000"
                            },
                            "arn": format!("arn:aws:s3:::{}", bucket)
                        },
                        "object": {
                            "key": key,
                            "size": size,
                            "eTag": etag.trim_matches('"'),
                            "sequencer": format!("{:016X}", Utc::now().timestamp_millis())
                        }
                    }
                }
            ]
        })
        .to_string();

        // 1. Dispatch SQS Queue Notifications
        for q in &notif_config.queue_configurations {
            let matched_event = q
                .events
                .iter()
                .any(|ev| Self::event_matches(ev, event_name));
            let matched_filter = q.filter.as_ref().map(|f| f.matches(key)).unwrap_or(true);
            if matched_event && matched_filter {
                target.send_sqs(&q.queue_arn, &s3_event_payload);
            }
        }

        // 2. Dispatch SNS Topic Notifications
        for t in &notif_config.topic_configurations {
            let matched_event = t
                .events
                .iter()
                .any(|ev| Self::event_matches(ev, event_name));
            let matched_filter = t.filter.as_ref().map(|f| f.matches(key)).unwrap_or(true);
            if matched_event && matched_filter {
                target.send_sns(&t.topic_arn, &s3_event_payload);
            }
        }

        // 3. Dispatch EventBridge Notification if enabled
        if notif_config.eventbridge_enabled {
            let detail = serde_json::json!({
                "version": "0",
                "bucket": {
                    "name": bucket
                },
                "object": {
                    "key": key,
                    "size": size,
                    "etag": etag.trim_matches('"'),
                    "sequencer": format!("{:016X}", Utc::now().timestamp_millis())
                },
                "request-id": uuid::Uuid::new_v4().to_string(),
                "requester": "000000000000",
                "destination": {
                    "bucket-name": bucket
                }
            })
            .to_string();

            let detail_type = match event_name {
                name if name.starts_with("s3:ObjectCreated") => "Object Created",
                name if name.starts_with("s3:ObjectRemoved") => "Object Deleted",
                _ => "Object Event",
            };

            target.send_eventbridge("aws.s3", detail_type, &detail);
        }
    }
}

impl S3Storage for InMemoryStorage {
    fn set_notification_target(&self, target: Arc<dyn S3NotificationTarget>) {
        *self.notification_target.write() = Some(target);
    }

    fn get_bucket_notification_configuration(
        &self,
        bucket: &str,
    ) -> Result<BucketNotificationConfig, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        let config = entry.notifications.read().clone();
        Ok(config)
    }

    fn put_bucket_notification_configuration(
        &self,
        bucket: &str,
        config: BucketNotificationConfig,
    ) -> Result<(), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        *entry.notifications.write() = config;
        Ok(())
    }

    fn create_bucket(&self, bucket: &str, region: &str) -> Result<(), RustStackError> {
        if self.buckets.contains_key(bucket) {
            return Err(RustStackError::S3 {
                code: "BucketAlreadyOwnedByYou".to_string(),
                message: "Your previous request to create the named bucket succeeded and you already own it.".to_string(),
                status: http::StatusCode::CONFLICT,
                resource: Some(bucket.to_string()),
            });
        }

        self.buckets.insert(
            bucket.to_string(),
            Bucket {
                info: BucketInfo {
                    name: bucket.to_string(),
                    creation_date: Utc::now(),
                    region: region.to_string(),
                },
                objects: DashMap::new(),
                multipart_uploads: DashMap::new(),
                notifications: RwLock::new(BucketNotificationConfig::default()),
            },
        );

        Ok(())
    }

    fn delete_bucket(&self, bucket: &str) -> Result<(), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        if !entry.objects.is_empty() {
            return Err(RustStackError::S3 {
                code: "BucketNotEmpty".to_string(),
                message: "The bucket you tried to delete is not empty".to_string(),
                status: http::StatusCode::CONFLICT,
                resource: Some(bucket.to_string()),
            });
        }
        drop(entry);

        self.buckets.remove(bucket);
        Ok(())
    }

    fn head_bucket(&self, bucket: &str) -> Result<BucketInfo, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        Ok(entry.info.clone())
    }

    fn list_buckets(&self) -> Result<Vec<BucketInfo>, RustStackError> {
        let mut list: Vec<BucketInfo> = self.buckets.iter().map(|b| b.info.clone()).collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(list)
    }

    fn put_object(
        &self,
        bucket: &str,
        key: &str,
        data: Bytes,
        content_type: Option<String>,
        user_metadata: HashMap<String, String>,
    ) -> Result<ObjectMetadata, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let etag = Self::calculate_etag(&data);
        let size = data.len() as u64;
        let metadata = ObjectMetadata {
            key: key.to_string(),
            size,
            etag: etag.clone(),
            last_modified: Utc::now(),
            content_type: content_type.unwrap_or_else(|| "application/octet-stream".to_string()),
            user_metadata,
        };

        entry.objects.insert(
            key.to_string(),
            StoredObject {
                metadata: metadata.clone(),
                data,
            },
        );

        drop(entry);
        self.dispatch_event(bucket, key, size, &etag, "s3:ObjectCreated:Put");

        Ok(metadata)
    }

    fn get_object(
        &self,
        bucket: &str,
        key: &str,
        range: Option<ByteRange>,
    ) -> Result<(ObjectMetadata, Bytes, Option<String>), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let obj = entry.objects.get(key).ok_or_else(|| {
            RustStackError::s3_not_found("NoSuchKey", "The specified key does not exist.")
        })?;

        if let Some(r) = range {
            let total = obj.metadata.size;
            if r.start >= total {
                return Err(RustStackError::S3 {
                    code: "InvalidRange".to_string(),
                    message: "The requested range cannot be satisfied".to_string(),
                    status: http::StatusCode::RANGE_NOT_SATISFIABLE,
                    resource: Some(key.to_string()),
                });
            }
            let end = r
                .end
                .unwrap_or(total.saturating_sub(1))
                .min(total.saturating_sub(1));
            let slice = obj.data.slice((r.start as usize)..=(end as usize));
            let content_range = format!("bytes {}-{}/{}", r.start, end, total);
            Ok((obj.metadata.clone(), slice, Some(content_range)))
        } else {
            Ok((obj.metadata.clone(), obj.data.clone(), None))
        }
    }

    fn head_object(&self, bucket: &str, key: &str) -> Result<ObjectMetadata, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let obj = entry.objects.get(key).ok_or_else(|| {
            RustStackError::s3_not_found("NoSuchKey", "The specified key does not exist.")
        })?;

        Ok(obj.metadata.clone())
    }

    fn delete_object(&self, bucket: &str, key: &str) -> Result<(), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let removed = entry.objects.remove(key);
        drop(entry);

        if let Some((_, obj)) = removed {
            self.dispatch_event(
                bucket,
                key,
                obj.metadata.size,
                &obj.metadata.etag,
                "s3:ObjectRemoved:Delete",
            );
        }

        Ok(())
    }

    fn delete_objects(
        &self,
        bucket: &str,
        keys: Vec<String>,
        _quiet: bool,
    ) -> Result<DeleteObjectsResult, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let mut res = DeleteObjectsResult::default();
        let mut deleted_events = Vec::new();

        for key in keys {
            if let Some((_, obj)) = entry.objects.remove(&key) {
                deleted_events.push((key.clone(), obj.metadata.size, obj.metadata.etag));
            }
            res.deleted.push(key);
        }

        drop(entry);
        for (k, size, etag) in deleted_events {
            self.dispatch_event(bucket, &k, size, &etag, "s3:ObjectRemoved:Delete");
        }

        Ok(res)
    }

    fn list_objects_v2(
        &self,
        bucket: &str,
        prefix: Option<&str>,
        delimiter: Option<&str>,
        max_keys: usize,
        continuation_token: Option<&str>,
        start_after: Option<&str>,
    ) -> Result<ListObjectsV2Result, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let pfx = prefix.unwrap_or("");
        let mut all_matching: Vec<ObjectMetadata> = entry
            .objects
            .iter()
            .filter(|item| item.key().starts_with(pfx))
            .map(|item| item.value().metadata.clone())
            .collect();

        all_matching.sort_by(|a, b| a.key.cmp(&b.key));

        let mut start_idx = 0;
        if let Some(token) = continuation_token {
            if let Some(pos) = all_matching.iter().position(|m| m.key.as_str() > token) {
                start_idx = pos;
            } else {
                start_idx = all_matching.len();
            }
        } else if let Some(sa) = start_after {
            if let Some(pos) = all_matching.iter().position(|m| m.key.as_str() > sa) {
                start_idx = pos;
            } else {
                start_idx = all_matching.len();
            }
        }

        let mut common_prefixes = BTreeSet::new();
        let mut contents = Vec::new();
        let mut truncated = false;
        let mut next_token = None;

        for item in all_matching.into_iter().skip(start_idx) {
            if let Some(delim) = delimiter {
                let suffix = &item.key[pfx.len()..];
                if let Some(idx) = suffix.find(delim) {
                    let prefix_end = pfx.len() + idx + delim.len();
                    let common_prefix = item.key[..prefix_end].to_string();
                    common_prefixes.insert(common_prefix);
                    continue;
                }
            }

            if contents.len() >= max_keys {
                truncated = true;
                next_token = contents.last().map(|m: &ObjectMetadata| m.key.clone());
                break;
            }
            contents.push(item);
        }

        Ok(ListObjectsV2Result {
            is_truncated: truncated,
            contents: contents.clone(),
            name: bucket.to_string(),
            prefix: pfx.to_string(),
            delimiter: delimiter.map(|s| s.to_string()),
            max_keys,
            common_prefixes: common_prefixes.into_iter().collect(),
            key_count: contents.len(),
            continuation_token: continuation_token.map(|s| s.to_string()),
            next_continuation_token: next_token,
            start_after: start_after.map(|s| s.to_string()),
        })
    }

    fn copy_object(
        &self,
        src_bucket: &str,
        src_key: &str,
        dest_bucket: &str,
        dest_key: &str,
    ) -> Result<ObjectMetadata, RustStackError> {
        let (src_meta, data, _) = self.get_object(src_bucket, src_key, None)?;
        let res = self.put_object(
            dest_bucket,
            dest_key,
            data,
            Some(src_meta.content_type),
            src_meta.user_metadata,
        )?;
        self.dispatch_event(
            dest_bucket,
            dest_key,
            res.size,
            &res.etag,
            "s3:ObjectCreated:Copy",
        );
        Ok(res)
    }

    fn create_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        content_type: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Result<String, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let upload_id = uuid::Uuid::new_v4().to_string();
        entry.multipart_uploads.insert(
            upload_id.clone(),
            StoredMultipartUpload {
                upload_id: upload_id.clone(),
                key: key.to_string(),
                content_type,
                user_metadata: metadata,
                parts: RwLock::new(BTreeMap::new()),
            },
        );

        Ok(upload_id)
    }

    fn upload_part(
        &self,
        bucket: &str,
        _key: &str,
        upload_id: &str,
        part_number: i32,
        data: Bytes,
    ) -> Result<String, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let mp = entry
            .multipart_uploads
            .get(upload_id)
            .ok_or_else(|| RustStackError::S3 {
                code: "NoSuchUpload".to_string(),
                message: "The specified multipart upload does not exist".to_string(),
                status: http::StatusCode::NOT_FOUND,
                resource: Some(upload_id.to_string()),
            })?;

        let etag = Self::calculate_etag(&data);
        mp.parts.write().insert(
            part_number,
            StoredPart {
                part_number,
                etag: etag.clone(),
                size: data.len() as u64,
                data,
                last_modified: Utc::now(),
            },
        );

        Ok(etag)
    }

    fn complete_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        parts: Vec<CompletedPart>,
    ) -> Result<ObjectMetadata, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let (_, mp) =
            entry
                .multipart_uploads
                .remove(upload_id)
                .ok_or_else(|| RustStackError::S3 {
                    code: "NoSuchUpload".to_string(),
                    message: "The specified multipart upload does not exist".to_string(),
                    status: http::StatusCode::NOT_FOUND,
                    resource: Some(upload_id.to_string()),
                })?;

        let stored_parts = mp.parts.read();
        let mut combined = Vec::new();

        for p in parts {
            let part_data = stored_parts
                .get(&p.part_number)
                .ok_or_else(|| RustStackError::S3 {
                    code: "InvalidPart".to_string(),
                    message: "One or more of the specified parts was not found".to_string(),
                    status: http::StatusCode::BAD_REQUEST,
                    resource: Some(upload_id.to_string()),
                })?;
            combined.extend_from_slice(&part_data.data);
        }

        let combined_bytes = Bytes::from(combined);
        let res = self.put_object(
            bucket,
            key,
            combined_bytes,
            mp.content_type,
            mp.user_metadata,
        )?;

        self.dispatch_event(
            bucket,
            key,
            res.size,
            &res.etag,
            "s3:ObjectCreated:CompleteMultipartUpload",
        );
        Ok(res)
    }

    fn abort_multipart_upload(
        &self,
        bucket: &str,
        _key: &str,
        upload_id: &str,
    ) -> Result<(), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        entry
            .multipart_uploads
            .remove(upload_id)
            .ok_or_else(|| RustStackError::S3 {
                code: "NoSuchUpload".to_string(),
                message: "The specified multipart upload does not exist".to_string(),
                status: http::StatusCode::NOT_FOUND,
                resource: Some(upload_id.to_string()),
            })?;

        Ok(())
    }

    fn list_parts(
        &self,
        bucket: &str,
        _key: &str,
        upload_id: &str,
    ) -> Result<Vec<PartInfo>, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let mp = entry
            .multipart_uploads
            .get(upload_id)
            .ok_or_else(|| RustStackError::S3 {
                code: "NoSuchUpload".to_string(),
                message: "The specified multipart upload does not exist".to_string(),
                status: http::StatusCode::NOT_FOUND,
                resource: Some(upload_id.to_string()),
            })?;

        let list: Vec<PartInfo> = mp
            .parts
            .read()
            .values()
            .map(|p| PartInfo {
                part_number: p.part_number,
                etag: p.etag.clone(),
                size: p.size,
                last_modified: p.last_modified,
            })
            .collect();

        Ok(list)
    }

    fn reset(&self) {
        self.buckets.clear();
    }

    fn dump_state(&self) -> S3Snapshot {
        let mut buckets_snap = Vec::new();
        for entry in self.buckets.iter() {
            let bucket = entry.value();
            let mut objs = Vec::new();
            for obj_entry in bucket.objects.iter() {
                let obj = obj_entry.value();
                objs.push(StoredObjectSnapshot {
                    metadata: obj.metadata.clone(),
                    data_base64: base64::engine::general_purpose::STANDARD.encode(&obj.data),
                });
            }
            buckets_snap.push(BucketSnapshot {
                info: bucket.info.clone(),
                notifications: bucket.notifications.read().clone(),
                objects: objs,
            });
        }
        S3Snapshot {
            buckets: buckets_snap,
        }
    }

    fn load_state(&self, snapshot: S3Snapshot) {
        self.buckets.clear();
        for b_snap in snapshot.buckets {
            let bucket = Bucket {
                info: b_snap.info.clone(),
                objects: DashMap::new(),
                multipart_uploads: DashMap::new(),
                notifications: RwLock::new(b_snap.notifications),
            };
            for obj_snap in b_snap.objects {
                if let Ok(decoded_bytes) =
                    base64::engine::general_purpose::STANDARD.decode(&obj_snap.data_base64)
                {
                    bucket.objects.insert(
                        obj_snap.metadata.key.clone(),
                        StoredObject {
                            metadata: obj_snap.metadata,
                            data: Bytes::from(decoded_bytes),
                        },
                    );
                }
            }
            self.buckets.insert(b_snap.info.name.clone(), bucket);
        }
    }
}

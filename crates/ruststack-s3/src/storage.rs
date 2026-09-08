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

    fn create_bucket(&self, bucket: &str, region: &str) -> Result<(), RustStackError> {
        let is_dir = bucket.ends_with("--x-s3");
        self.create_bucket_with_config(bucket, region, is_dir, None)
    }

    fn create_bucket_with_config(
        &self,
        bucket: &str,
        region: &str,
        is_directory: bool,
        az: Option<&str>,
    ) -> Result<(), RustStackError>;

    fn create_session(
        &self,
        bucket: &str,
        mode: &str,
    ) -> Result<crate::types::SessionCredentials, RustStackError>;

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
    ) -> Result<ObjectMetadata, RustStackError> {
        self.put_object_with_checksums(
            bucket,
            key,
            data,
            content_type,
            user_metadata,
            None,
            None,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn put_object_with_checksums(
        &self,
        bucket: &str,
        key: &str,
        data: Bytes,
        content_type: Option<String>,
        user_metadata: HashMap<String, String>,
        checksum_crc32: Option<String>,
        checksum_crc32c: Option<String>,
        checksum_sha1: Option<String>,
        checksum_sha256: Option<String>,
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
    ) -> Result<String, RustStackError> {
        self.create_multipart_upload_with_checksum(bucket, key, content_type, metadata, None)
    }

    fn create_multipart_upload_with_checksum(
        &self,
        bucket: &str,
        key: &str,
        content_type: Option<String>,
        metadata: HashMap<String, String>,
        checksum_algorithm: Option<crate::checksum::ChecksumAlgorithm>,
    ) -> Result<String, RustStackError>;

    fn upload_part(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        part_number: i32,
        data: Bytes,
    ) -> Result<String, RustStackError> {
        self.upload_part_with_checksum(
            bucket,
            key,
            upload_id,
            part_number,
            data,
            None,
            None,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn upload_part_with_checksum(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        part_number: i32,
        data: Bytes,
        checksum_crc32: Option<String>,
        checksum_crc32c: Option<String>,
        checksum_sha1: Option<String>,
        checksum_sha256: Option<String>,
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

    // Versioning
    fn get_bucket_versioning(&self, bucket: &str) -> Result<Option<String>, RustStackError>;
    fn put_bucket_versioning(&self, bucket: &str, status: &str) -> Result<(), RustStackError>;
    fn list_object_versions(
        &self,
        bucket: &str,
        prefix: Option<&str>,
        delimiter: Option<&str>,
        key_marker: Option<&str>,
        version_id_marker: Option<&str>,
        max_keys: usize,
    ) -> Result<crate::types::ListObjectVersionsResult, RustStackError>;
    fn get_object_version(
        &self,
        bucket: &str,
        key: &str,
        version_id: &str,
    ) -> Result<(ObjectMetadata, Bytes), RustStackError>;
    fn delete_object_version(
        &self,
        bucket: &str,
        key: &str,
        version_id: Option<&str>,
    ) -> Result<Option<String>, RustStackError>;

    // Lifecycle
    fn get_bucket_lifecycle(
        &self,
        bucket: &str,
    ) -> Result<Option<crate::types::BucketLifecycleConfig>, RustStackError>;
    fn put_bucket_lifecycle(
        &self,
        bucket: &str,
        config: crate::types::BucketLifecycleConfig,
    ) -> Result<(), RustStackError>;
    fn delete_bucket_lifecycle(&self, bucket: &str) -> Result<(), RustStackError>;

    // CORS
    fn get_bucket_cors(
        &self,
        bucket: &str,
    ) -> Result<Option<crate::types::BucketCorsConfig>, RustStackError>;
    fn put_bucket_cors(
        &self,
        bucket: &str,
        config: crate::types::BucketCorsConfig,
    ) -> Result<(), RustStackError>;
    fn delete_bucket_cors(&self, bucket: &str) -> Result<(), RustStackError>;

    // Policy
    fn get_bucket_policy(&self, bucket: &str) -> Result<Option<String>, RustStackError>;
    fn put_bucket_policy(&self, bucket: &str, policy: String) -> Result<(), RustStackError>;
    fn delete_bucket_policy(&self, bucket: &str) -> Result<(), RustStackError>;

    // Tagging
    fn get_bucket_tagging(&self, bucket: &str) -> Result<HashMap<String, String>, RustStackError>;
    fn put_bucket_tagging(
        &self,
        bucket: &str,
        tags: HashMap<String, String>,
    ) -> Result<(), RustStackError>;
    fn delete_bucket_tagging(&self, bucket: &str) -> Result<(), RustStackError>;

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
    checksum_crc32: Option<String>,
    checksum_crc32c: Option<String>,
    checksum_sha1: Option<String>,
    checksum_sha256: Option<String>,
}

#[allow(dead_code)]
struct StoredMultipartUpload {
    upload_id: String,
    key: String,
    content_type: Option<String>,
    user_metadata: HashMap<String, String>,
    parts: RwLock<BTreeMap<i32, StoredPart>>,
    checksum_algorithm: Option<crate::checksum::ChecksumAlgorithm>,
}

#[derive(Clone)]
struct StoredObjectVersion {
    version_id: String,
    is_latest: bool,
    is_delete_marker: bool,
    metadata: ObjectMetadata,
    data: Bytes,
}

struct Bucket {
    info: BucketInfo,
    objects: DashMap<String, StoredObject>,
    versions: DashMap<String, Vec<StoredObjectVersion>>,
    versioning: RwLock<Option<String>>,
    lifecycle: RwLock<Option<crate::types::BucketLifecycleConfig>>,
    cors: RwLock<Option<crate::types::BucketCorsConfig>>,
    policy: RwLock<Option<String>>,
    tagging: RwLock<HashMap<String, String>>,
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

    fn create_bucket_with_config(
        &self,
        bucket: &str,
        region: &str,
        mut is_directory: bool,
        az: Option<&str>,
    ) -> Result<(), RustStackError> {
        if self.buckets.contains_key(bucket) {
            return Err(RustStackError::S3 {
                code: "BucketAlreadyOwnedByYou".to_string(),
                message: "Your previous request to create the named bucket succeeded and you already own it.".to_string(),
                status: http::StatusCode::CONFLICT,
                resource: Some(bucket.to_string()),
            });
        }

        let mut az_opt = az.map(|s| s.to_string());
        if bucket.ends_with("--x-s3") {
            is_directory = true;
            if az_opt.is_none() {
                let parts: Vec<&str> = bucket.split("--").collect();
                if parts.len() >= 3 {
                    az_opt = Some(parts[parts.len() - 2].to_string());
                }
            }
        }

        let storage_class = if is_directory {
            "EXPRESS_ONEZONE".to_string()
        } else {
            "STANDARD".to_string()
        };

        self.buckets.insert(
            bucket.to_string(),
            Bucket {
                info: BucketInfo {
                    name: bucket.to_string(),
                    creation_date: Utc::now(),
                    region: region.to_string(),
                    is_directory_bucket: is_directory,
                    storage_class,
                    availability_zone: az_opt,
                },
                objects: DashMap::new(),
                versions: DashMap::new(),
                versioning: RwLock::new(None),
                lifecycle: RwLock::new(None),
                cors: RwLock::new(None),
                policy: RwLock::new(None),
                tagging: RwLock::new(HashMap::new()),
                multipart_uploads: DashMap::new(),
                notifications: RwLock::new(BucketNotificationConfig::default()),
            },
        );

        Ok(())
    }

    fn create_session(
        &self,
        bucket: &str,
        _mode: &str,
    ) -> Result<crate::types::SessionCredentials, RustStackError> {
        if !self.buckets.contains_key(bucket) {
            return Err(RustStackError::S3 {
                code: "NoSuchBucket".to_string(),
                message: "The specified bucket does not exist".to_string(),
                status: http::StatusCode::NOT_FOUND,
                resource: Some(bucket.to_string()),
            });
        }

        let session_id = uuid::Uuid::new_v4().to_string();
        let short_id = &session_id[..8];
        let creds = crate::types::SessionCredentials {
            access_key_id: format!("ASIA_RUSTSTACK_EXPRESS_{}", short_id),
            secret_access_key: uuid::Uuid::new_v4().to_string(),
            session_token: base64::engine::general_purpose::STANDARD
                .encode(format!("ruststack-session-{}-{}", bucket, session_id)),
            expiration: Utc::now() + chrono::Duration::minutes(5),
        };
        Ok(creds)
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

    #[allow(clippy::too_many_arguments)]
    fn put_object_with_checksums(
        &self,
        bucket: &str,
        key: &str,
        data: Bytes,
        content_type: Option<String>,
        user_metadata: HashMap<String, String>,
        checksum_crc32: Option<String>,
        checksum_crc32c: Option<String>,
        checksum_sha1: Option<String>,
        checksum_sha256: Option<String>,
    ) -> Result<ObjectMetadata, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        // Checksum verification if provided (unless it is a composite multipart checksum with '-' suffix)
        if let Some(ref expected) = checksum_crc32 {
            if !expected.contains('-') {
                crate::checksum::verify_checksum(
                    crate::checksum::ChecksumAlgorithm::Crc32,
                    &data,
                    expected,
                )?;
            }
        }
        if let Some(ref expected) = checksum_crc32c {
            if !expected.contains('-') {
                crate::checksum::verify_checksum(
                    crate::checksum::ChecksumAlgorithm::Crc32c,
                    &data,
                    expected,
                )?;
            }
        }
        if let Some(ref expected) = checksum_sha1 {
            if !expected.contains('-') {
                crate::checksum::verify_checksum(
                    crate::checksum::ChecksumAlgorithm::Sha1,
                    &data,
                    expected,
                )?;
            }
        }
        if let Some(ref expected) = checksum_sha256 {
            if !expected.contains('-') {
                crate::checksum::verify_checksum(
                    crate::checksum::ChecksumAlgorithm::Sha256,
                    &data,
                    expected,
                )?;
            }
        }

        let crc32_val = checksum_crc32.or_else(|| {
            Some(crate::checksum::calculate_checksum(
                crate::checksum::ChecksumAlgorithm::Crc32,
                &data,
            ))
        });
        let crc32c_val = checksum_crc32c.or_else(|| {
            Some(crate::checksum::calculate_checksum(
                crate::checksum::ChecksumAlgorithm::Crc32c,
                &data,
            ))
        });
        let sha1_val = checksum_sha1.or_else(|| {
            Some(crate::checksum::calculate_checksum(
                crate::checksum::ChecksumAlgorithm::Sha1,
                &data,
            ))
        });
        let sha256_val = checksum_sha256.or_else(|| {
            Some(crate::checksum::calculate_checksum(
                crate::checksum::ChecksumAlgorithm::Sha256,
                &data,
            ))
        });

        let storage_class = entry.info.storage_class.clone();
        let etag = Self::calculate_etag(&data);
        let size = data.len() as u64;
        let metadata = ObjectMetadata {
            key: key.to_string(),
            size,
            etag: etag.clone(),
            last_modified: Utc::now(),
            content_type: content_type.unwrap_or_else(|| "application/octet-stream".to_string()),
            user_metadata,
            checksum_crc32: crc32_val,
            checksum_crc32c: crc32c_val,
            checksum_sha1: sha1_val,
            checksum_sha256: sha256_val,
            storage_class,
        };

        let is_versioned = entry.versioning.read().as_deref() == Some("Enabled");
        if is_versioned {
            let version_id = uuid::Uuid::new_v4().to_string();
            let mut v_list = entry.versions.entry(key.to_string()).or_default();
            for v in v_list.iter_mut() {
                v.is_latest = false;
            }
            v_list.push(StoredObjectVersion {
                version_id,
                is_latest: true,
                is_delete_marker: false,
                metadata: metadata.clone(),
                data: data.clone(),
            });
        }

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

    fn create_multipart_upload_with_checksum(
        &self,
        bucket: &str,
        key: &str,
        content_type: Option<String>,
        metadata: HashMap<String, String>,
        checksum_algorithm: Option<crate::checksum::ChecksumAlgorithm>,
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
                checksum_algorithm,
            },
        );

        Ok(upload_id)
    }

    #[allow(clippy::too_many_arguments)]
    fn upload_part_with_checksum(
        &self,
        bucket: &str,
        _key: &str,
        upload_id: &str,
        part_number: i32,
        data: Bytes,
        checksum_crc32: Option<String>,
        checksum_crc32c: Option<String>,
        checksum_sha1: Option<String>,
        checksum_sha256: Option<String>,
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

        // Checksum verification if provided
        if let Some(ref expected) = checksum_crc32 {
            crate::checksum::verify_checksum(
                crate::checksum::ChecksumAlgorithm::Crc32,
                &data,
                expected,
            )?;
        }
        if let Some(ref expected) = checksum_crc32c {
            crate::checksum::verify_checksum(
                crate::checksum::ChecksumAlgorithm::Crc32c,
                &data,
                expected,
            )?;
        }
        if let Some(ref expected) = checksum_sha1 {
            crate::checksum::verify_checksum(
                crate::checksum::ChecksumAlgorithm::Sha1,
                &data,
                expected,
            )?;
        }
        if let Some(ref expected) = checksum_sha256 {
            crate::checksum::verify_checksum(
                crate::checksum::ChecksumAlgorithm::Sha256,
                &data,
                expected,
            )?;
        }

        let computed_crc32 = checksum_crc32.or_else(|| {
            if mp.checksum_algorithm == Some(crate::checksum::ChecksumAlgorithm::Crc32) {
                Some(crate::checksum::calculate_checksum(
                    crate::checksum::ChecksumAlgorithm::Crc32,
                    &data,
                ))
            } else {
                None
            }
        });
        let computed_crc32c = checksum_crc32c.or_else(|| {
            if mp.checksum_algorithm == Some(crate::checksum::ChecksumAlgorithm::Crc32c) {
                Some(crate::checksum::calculate_checksum(
                    crate::checksum::ChecksumAlgorithm::Crc32c,
                    &data,
                ))
            } else {
                None
            }
        });
        let computed_sha1 = checksum_sha1.or_else(|| {
            if mp.checksum_algorithm == Some(crate::checksum::ChecksumAlgorithm::Sha1) {
                Some(crate::checksum::calculate_checksum(
                    crate::checksum::ChecksumAlgorithm::Sha1,
                    &data,
                ))
            } else {
                None
            }
        });
        let computed_sha256 = checksum_sha256.or_else(|| {
            if mp.checksum_algorithm == Some(crate::checksum::ChecksumAlgorithm::Sha256) {
                Some(crate::checksum::calculate_checksum(
                    crate::checksum::ChecksumAlgorithm::Sha256,
                    &data,
                ))
            } else {
                None
            }
        });

        let etag = Self::calculate_etag(&data);
        mp.parts.write().insert(
            part_number,
            StoredPart {
                part_number,
                etag: etag.clone(),
                size: data.len() as u64,
                data,
                last_modified: Utc::now(),
                checksum_crc32: computed_crc32,
                checksum_crc32c: computed_crc32c,
                checksum_sha1: computed_sha1,
                checksum_sha256: computed_sha256,
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
        let mut part_crc32s = Vec::new();
        let mut part_crc32cs = Vec::new();
        let mut part_sha1s = Vec::new();
        let mut part_sha256s = Vec::new();

        for p in parts {
            let part_data = stored_parts
                .get(&p.part_number)
                .ok_or_else(|| RustStackError::S3 {
                    code: "InvalidPart".to_string(),
                    message: "One or more of the specified parts was not found".to_string(),
                    status: http::StatusCode::BAD_REQUEST,
                    resource: Some(upload_id.to_string()),
                })?;

            // Validate part checksum if passed in CompletedPart XML
            if let Some(ref expected) = p.checksum_crc32 {
                crate::checksum::verify_checksum(
                    crate::checksum::ChecksumAlgorithm::Crc32,
                    &part_data.data,
                    expected,
                )?;
            }
            if let Some(ref expected) = p.checksum_crc32c {
                crate::checksum::verify_checksum(
                    crate::checksum::ChecksumAlgorithm::Crc32c,
                    &part_data.data,
                    expected,
                )?;
            }
            if let Some(ref expected) = p.checksum_sha1 {
                crate::checksum::verify_checksum(
                    crate::checksum::ChecksumAlgorithm::Sha1,
                    &part_data.data,
                    expected,
                )?;
            }
            if let Some(ref expected) = p.checksum_sha256 {
                crate::checksum::verify_checksum(
                    crate::checksum::ChecksumAlgorithm::Sha256,
                    &part_data.data,
                    expected,
                )?;
            }

            let c32 = p
                .checksum_crc32
                .or_else(|| part_data.checksum_crc32.clone());
            if let Some(c) = c32 {
                part_crc32s.push(c);
            }

            let c32c = p
                .checksum_crc32c
                .or_else(|| part_data.checksum_crc32c.clone());
            if let Some(c) = c32c {
                part_crc32cs.push(c);
            }

            let s1 = p.checksum_sha1.or_else(|| part_data.checksum_sha1.clone());
            if let Some(c) = s1 {
                part_sha1s.push(c);
            }

            let s256 = p
                .checksum_sha256
                .or_else(|| part_data.checksum_sha256.clone());
            if let Some(c) = s256 {
                part_sha256s.push(c);
            }

            combined.extend_from_slice(&part_data.data);
        }

        let combined_bytes = Bytes::from(combined);

        // Compute composite checksums if parts had checksums or multipart had algorithm
        let composite_crc32 = if !part_crc32s.is_empty() {
            crate::checksum::compute_composite_checksum(
                crate::checksum::ChecksumAlgorithm::Crc32,
                &part_crc32s,
            )
            .ok()
        } else if mp.checksum_algorithm == Some(crate::checksum::ChecksumAlgorithm::Crc32) {
            Some(crate::checksum::calculate_checksum(
                crate::checksum::ChecksumAlgorithm::Crc32,
                &combined_bytes,
            ))
        } else {
            None
        };

        let composite_crc32c = if !part_crc32cs.is_empty() {
            crate::checksum::compute_composite_checksum(
                crate::checksum::ChecksumAlgorithm::Crc32c,
                &part_crc32cs,
            )
            .ok()
        } else if mp.checksum_algorithm == Some(crate::checksum::ChecksumAlgorithm::Crc32c) {
            Some(crate::checksum::calculate_checksum(
                crate::checksum::ChecksumAlgorithm::Crc32c,
                &combined_bytes,
            ))
        } else {
            None
        };

        let composite_sha1 = if !part_sha1s.is_empty() {
            crate::checksum::compute_composite_checksum(
                crate::checksum::ChecksumAlgorithm::Sha1,
                &part_sha1s,
            )
            .ok()
        } else if mp.checksum_algorithm == Some(crate::checksum::ChecksumAlgorithm::Sha1) {
            Some(crate::checksum::calculate_checksum(
                crate::checksum::ChecksumAlgorithm::Sha1,
                &combined_bytes,
            ))
        } else {
            None
        };

        let composite_sha256 = if !part_sha256s.is_empty() {
            crate::checksum::compute_composite_checksum(
                crate::checksum::ChecksumAlgorithm::Sha256,
                &part_sha256s,
            )
            .ok()
        } else if mp.checksum_algorithm == Some(crate::checksum::ChecksumAlgorithm::Sha256) {
            Some(crate::checksum::calculate_checksum(
                crate::checksum::ChecksumAlgorithm::Sha256,
                &combined_bytes,
            ))
        } else {
            None
        };

        let res = self.put_object_with_checksums(
            bucket,
            key,
            combined_bytes,
            mp.content_type,
            mp.user_metadata,
            composite_crc32,
            composite_crc32c,
            composite_sha1,
            composite_sha256,
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
                checksum_crc32: p.checksum_crc32.clone(),
                checksum_crc32c: p.checksum_crc32c.clone(),
                checksum_sha1: p.checksum_sha1.clone(),
                checksum_sha256: p.checksum_sha256.clone(),
            })
            .collect();

        Ok(list)
    }

    fn get_bucket_versioning(&self, bucket: &str) -> Result<Option<String>, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        let res = entry.versioning.read().clone();
        Ok(res)
    }

    fn put_bucket_versioning(&self, bucket: &str, status: &str) -> Result<(), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        *entry.versioning.write() = Some(status.to_string());
        Ok(())
    }

    fn list_object_versions(
        &self,
        bucket: &str,
        prefix: Option<&str>,
        _delimiter: Option<&str>,
        key_marker: Option<&str>,
        version_id_marker: Option<&str>,
        max_keys: usize,
    ) -> Result<crate::types::ListObjectVersionsResult, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let mut versions_out = Vec::new();
        let mut delete_markers_out = Vec::new();
        let prefix_str = prefix.unwrap_or("");

        for pair in entry.versions.iter() {
            let key = pair.key();
            if !key.starts_with(prefix_str) {
                continue;
            }
            for v in pair.value() {
                let obj_ver = crate::types::ObjectVersion {
                    key: key.clone(),
                    version_id: v.version_id.clone(),
                    is_latest: v.is_latest,
                    last_modified: v.metadata.last_modified,
                    etag: v.metadata.etag.clone(),
                    size: v.metadata.size,
                    is_delete_marker: v.is_delete_marker,
                };
                if v.is_delete_marker {
                    delete_markers_out.push(obj_ver);
                } else {
                    versions_out.push(obj_ver);
                }
            }
        }

        Ok(crate::types::ListObjectVersionsResult {
            name: bucket.to_string(),
            prefix: prefix_str.to_string(),
            key_marker: key_marker.map(|s| s.to_string()),
            version_id_marker: version_id_marker.map(|s| s.to_string()),
            next_key_marker: None,
            next_version_id_marker: None,
            max_keys,
            is_truncated: false,
            versions: versions_out,
            delete_markers: delete_markers_out,
            common_prefixes: Vec::new(),
        })
    }

    fn get_object_version(
        &self,
        bucket: &str,
        key: &str,
        version_id: &str,
    ) -> Result<(ObjectMetadata, Bytes), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let v_list = entry.versions.get(key).ok_or_else(|| {
            RustStackError::s3_not_found("NoSuchKey", "The specified key does not exist.")
        })?;

        let found = v_list
            .iter()
            .find(|v| v.version_id == version_id)
            .ok_or_else(|| {
                RustStackError::s3_not_found(
                    "NoSuchVersion",
                    "The specified version does not exist.",
                )
            })?;

        if found.is_delete_marker {
            return Err(RustStackError::S3 {
                code: "MethodNotAllowed".to_string(),
                message: "The specified method is not allowed against this resource.".to_string(),
                status: http::StatusCode::METHOD_NOT_ALLOWED,
                resource: Some(key.to_string()),
            });
        }

        Ok((found.metadata.clone(), found.data.clone()))
    }

    fn delete_object_version(
        &self,
        bucket: &str,
        key: &str,
        version_id: Option<&str>,
    ) -> Result<Option<String>, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        if let Some(vid) = version_id {
            if let Some(mut v_list) = entry.versions.get_mut(key) {
                v_list.retain(|v| v.version_id != vid);
                if let Some(last) = v_list.last_mut() {
                    last.is_latest = true;
                    if !last.is_delete_marker {
                        entry.objects.insert(
                            key.to_string(),
                            StoredObject {
                                metadata: last.metadata.clone(),
                                data: last.data.clone(),
                            },
                        );
                    } else {
                        entry.objects.remove(key);
                    }
                } else {
                    entry.objects.remove(key);
                }
            }
            return Ok(Some(vid.to_string()));
        }

        let is_versioned = entry.versioning.read().as_deref() == Some("Enabled");
        if is_versioned {
            let delete_marker_id = uuid::Uuid::new_v4().to_string();
            let mut v_list = entry.versions.entry(key.to_string()).or_default();
            for v in v_list.iter_mut() {
                v.is_latest = false;
            }
            let metadata = ObjectMetadata {
                key: key.to_string(),
                size: 0,
                etag: String::new(),
                last_modified: Utc::now(),
                content_type: "application/octet-stream".to_string(),
                user_metadata: HashMap::new(),
                checksum_crc32: None,
                checksum_crc32c: None,
                checksum_sha1: None,
                checksum_sha256: None,
                storage_class: entry.info.storage_class.clone(),
            };
            v_list.push(StoredObjectVersion {
                version_id: delete_marker_id.clone(),
                is_latest: true,
                is_delete_marker: true,
                metadata,
                data: Bytes::new(),
            });
            entry.objects.remove(key);
            Ok(Some(delete_marker_id))
        } else {
            entry.objects.remove(key);
            entry.versions.remove(key);
            Ok(None)
        }
    }

    fn get_bucket_lifecycle(
        &self,
        bucket: &str,
    ) -> Result<Option<crate::types::BucketLifecycleConfig>, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        let res = entry.lifecycle.read().clone();
        Ok(res)
    }

    fn put_bucket_lifecycle(
        &self,
        bucket: &str,
        config: crate::types::BucketLifecycleConfig,
    ) -> Result<(), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        *entry.lifecycle.write() = Some(config);
        Ok(())
    }

    fn delete_bucket_lifecycle(&self, bucket: &str) -> Result<(), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        *entry.lifecycle.write() = None;
        Ok(())
    }

    fn get_bucket_cors(
        &self,
        bucket: &str,
    ) -> Result<Option<crate::types::BucketCorsConfig>, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        let res = entry.cors.read().clone();
        Ok(res)
    }

    fn put_bucket_cors(
        &self,
        bucket: &str,
        config: crate::types::BucketCorsConfig,
    ) -> Result<(), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        *entry.cors.write() = Some(config);
        Ok(())
    }

    fn delete_bucket_cors(&self, bucket: &str) -> Result<(), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        *entry.cors.write() = None;
        Ok(())
    }

    fn get_bucket_policy(&self, bucket: &str) -> Result<Option<String>, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        let res = entry.policy.read().clone();
        Ok(res)
    }

    fn put_bucket_policy(&self, bucket: &str, policy: String) -> Result<(), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        *entry.policy.write() = Some(policy);
        Ok(())
    }

    fn delete_bucket_policy(&self, bucket: &str) -> Result<(), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        *entry.policy.write() = None;
        Ok(())
    }

    fn get_bucket_tagging(&self, bucket: &str) -> Result<HashMap<String, String>, RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        let res = entry.tagging.read().clone();
        Ok(res)
    }

    fn put_bucket_tagging(
        &self,
        bucket: &str,
        tags: HashMap<String, String>,
    ) -> Result<(), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        *entry.tagging.write() = tags;
        Ok(())
    }

    fn delete_bucket_tagging(&self, bucket: &str) -> Result<(), RustStackError> {
        let entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;
        entry.tagging.write().clear();
        Ok(())
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
                    version_id: None,
                    is_delete_marker: false,
                });
            }
            buckets_snap.push(BucketSnapshot {
                info: bucket.info.clone(),
                notifications: bucket.notifications.read().clone(),
                objects: objs,
                versioning: bucket.versioning.read().clone(),
                lifecycle: bucket.lifecycle.read().clone(),
                cors: bucket.cors.read().clone(),
                policy: bucket.policy.read().clone(),
                tagging: bucket.tagging.read().clone(),
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
                versions: DashMap::new(),
                versioning: RwLock::new(b_snap.versioning),
                lifecycle: RwLock::new(b_snap.lifecycle),
                cors: RwLock::new(b_snap.cors),
                policy: RwLock::new(b_snap.policy),
                tagging: RwLock::new(b_snap.tagging),
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

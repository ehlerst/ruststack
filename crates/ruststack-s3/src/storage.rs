use crate::types::{
    BucketInfo, ByteRange, CompletedPart, DeleteObjectsResult, ListObjectsV2Result, ObjectMetadata,
    PartInfo,
};
use bytes::Bytes;
use chrono::Utc;
use dashmap::DashMap;
use md5::{Digest, Md5};
use parking_lot::RwLock;
use ruststack_core::RustStackError;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub trait S3Storage: Send + Sync {
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
    ) -> Result<(ObjectMetadata, Bytes, Option<String>), RustStackError>; // returns (meta, data, content_range_str)

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
}

#[derive(Default)]
pub struct InMemoryStorage {
    buckets: DashMap<String, Bucket>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            buckets: DashMap::new(),
        }
    }

    fn calculate_etag(data: &[u8]) -> String {
        let mut hasher = Md5::new();
        hasher.update(data);
        let result = hasher.finalize();
        format!("\"{}\"", hex::encode(result))
    }
}

impl S3Storage for InMemoryStorage {
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
        let bucket_entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let etag = Self::calculate_etag(&data);
        let metadata = ObjectMetadata {
            key: key.to_string(),
            size: data.len() as u64,
            etag,
            last_modified: Utc::now(),
            content_type: content_type.unwrap_or_else(|| "application/octet-stream".to_string()),
            user_metadata,
        };

        bucket_entry.objects.insert(
            key.to_string(),
            StoredObject {
                metadata: metadata.clone(),
                data,
            },
        );

        Ok(metadata)
    }

    fn get_object(
        &self,
        bucket: &str,
        key: &str,
        range: Option<ByteRange>,
    ) -> Result<(ObjectMetadata, Bytes, Option<String>), RustStackError> {
        let bucket_entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let obj = bucket_entry
            .objects
            .get(key)
            .ok_or_else(|| RustStackError::S3 {
                code: "NoSuchKey".to_string(),
                message: "The specified key does not exist.".to_string(),
                status: http::StatusCode::NOT_FOUND,
                resource: Some(key.to_string()),
            })?;

        let metadata = obj.metadata.clone();
        let total_size = obj.data.len() as u64;

        if let Some(r) = range {
            let start = r.start.min(total_size);
            let end = r
                .end
                .unwrap_or(total_size.saturating_sub(1))
                .min(total_size.saturating_sub(1));

            if start > end || start >= total_size {
                return Err(RustStackError::S3 {
                    code: "InvalidRange".to_string(),
                    message: "The requested range cannot be satisfied".to_string(),
                    status: http::StatusCode::RANGE_NOT_SATISFIABLE,
                    resource: Some(key.to_string()),
                });
            }

            let slice = obj.data.slice((start as usize)..=((end) as usize));
            let content_range = format!("bytes {}-{}/{}", start, end, total_size);
            Ok((metadata, slice, Some(content_range)))
        } else {
            Ok((metadata, obj.data.clone(), None))
        }
    }

    fn head_object(&self, bucket: &str, key: &str) -> Result<ObjectMetadata, RustStackError> {
        let bucket_entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let obj = bucket_entry
            .objects
            .get(key)
            .ok_or_else(|| RustStackError::S3 {
                code: "NoSuchKey".to_string(),
                message: "The specified key does not exist.".to_string(),
                status: http::StatusCode::NOT_FOUND,
                resource: Some(key.to_string()),
            })?;

        Ok(obj.metadata.clone())
    }

    fn delete_object(&self, bucket: &str, key: &str) -> Result<(), RustStackError> {
        let bucket_entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        bucket_entry.objects.remove(key);
        Ok(())
    }

    fn delete_objects(
        &self,
        bucket: &str,
        keys: Vec<String>,
        _quiet: bool,
    ) -> Result<DeleteObjectsResult, RustStackError> {
        let bucket_entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let mut deleted = Vec::new();
        for key in keys {
            bucket_entry.objects.remove(&key);
            deleted.push(key);
        }

        Ok(DeleteObjectsResult {
            deleted,
            errors: Vec::new(),
        })
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
        let bucket_entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let prefix_str = prefix.unwrap_or("");
        let mut keys: Vec<String> = bucket_entry
            .objects
            .iter()
            .map(|e| e.key().clone())
            .filter(|k| k.starts_with(prefix_str))
            .collect();
        keys.sort();

        // Handle continuation token or start_after
        let skip_key = continuation_token.or(start_after);
        let iter = if let Some(sk) = skip_key {
            keys.into_iter()
                .filter(|k| k.as_str() > sk)
                .collect::<Vec<_>>()
        } else {
            keys
        };

        let mut contents = Vec::new();
        let mut common_prefixes_set = BTreeSet::new();
        let mut next_token = None;
        let mut count = 0;

        for key in iter {
            if count >= max_keys {
                next_token = Some(key);
                break;
            }

            if let Some(delim) = delimiter {
                let rest = &key[prefix_str.len()..];
                if let Some(idx) = rest.find(delim) {
                    let common_prefix = format!("{}{}{}", prefix_str, &rest[..idx], delim);
                    common_prefixes_set.insert(common_prefix);
                    continue;
                }
            }

            if let Some(obj) = bucket_entry.objects.get(&key) {
                contents.push(obj.metadata.clone());
                count += 1;
            }
        }

        let is_truncated = next_token.is_some();
        let common_prefixes: Vec<String> = common_prefixes_set.into_iter().collect();
        let key_count = contents.len() + common_prefixes.len();

        Ok(ListObjectsV2Result {
            is_truncated,
            contents,
            name: bucket.to_string(),
            prefix: prefix_str.to_string(),
            delimiter: delimiter.map(|s| s.to_string()),
            max_keys,
            common_prefixes,
            key_count,
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
        self.put_object(
            dest_bucket,
            dest_key,
            data,
            Some(src_meta.content_type),
            src_meta.user_metadata,
        )
    }

    fn create_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        content_type: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Result<String, RustStackError> {
        let bucket_entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let upload_id = uuid::Uuid::new_v4().to_string();
        bucket_entry.multipart_uploads.insert(
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
        let bucket_entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let mp = bucket_entry
            .multipart_uploads
            .get(upload_id)
            .ok_or_else(|| RustStackError::S3 {
                code: "NoSuchUpload".to_string(),
                message: "The specified multipart upload does not exist.".to_string(),
                status: http::StatusCode::NOT_FOUND,
                resource: Some(upload_id.to_string()),
            })?;

        let etag = Self::calculate_etag(&data);
        let part = StoredPart {
            part_number,
            etag: etag.clone(),
            size: data.len() as u64,
            data,
            last_modified: Utc::now(),
        };

        mp.parts.write().insert(part_number, part);
        Ok(etag)
    }

    fn complete_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        parts: Vec<CompletedPart>,
    ) -> Result<ObjectMetadata, RustStackError> {
        let bucket_entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let (_, mp) = bucket_entry
            .multipart_uploads
            .remove(upload_id)
            .ok_or_else(|| RustStackError::S3 {
                code: "NoSuchUpload".to_string(),
                message: "The specified multipart upload does not exist.".to_string(),
                status: http::StatusCode::NOT_FOUND,
                resource: Some(upload_id.to_string()),
            })?;

        let stored_parts = mp.parts.read();
        let mut combined = Vec::new();
        let mut md5_concatenated = Vec::new();

        for part in &parts {
            let stored = stored_parts
                .get(&part.part_number)
                .ok_or_else(|| RustStackError::S3 {
                    code: "InvalidPart".to_string(),
                    message: format!("Part number {} was not found", part.part_number),
                    status: http::StatusCode::BAD_REQUEST,
                    resource: Some(upload_id.to_string()),
                })?;

            combined.extend_from_slice(&stored.data);

            let clean_etag = stored.etag.trim_matches('"');
            if let Ok(bytes) = hex::decode(clean_etag) {
                md5_concatenated.extend_from_slice(&bytes);
            }
        }

        // S3 multipart etag format: <md5-of-concatenated-part-md5s>-<num_parts>
        let mut hasher = Md5::new();
        hasher.update(&md5_concatenated);
        let final_etag = format!("\"{}-{}\"", hex::encode(hasher.finalize()), parts.len());

        let metadata = ObjectMetadata {
            key: key.to_string(),
            size: combined.len() as u64,
            etag: final_etag,
            last_modified: Utc::now(),
            content_type: mp
                .content_type
                .unwrap_or_else(|| "application/octet-stream".to_string()),
            user_metadata: mp.user_metadata,
        };

        bucket_entry.objects.insert(
            key.to_string(),
            StoredObject {
                metadata: metadata.clone(),
                data: Bytes::from(combined),
            },
        );

        Ok(metadata)
    }

    fn abort_multipart_upload(
        &self,
        bucket: &str,
        _key: &str,
        upload_id: &str,
    ) -> Result<(), RustStackError> {
        let bucket_entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        bucket_entry.multipart_uploads.remove(upload_id);
        Ok(())
    }

    fn list_parts(
        &self,
        bucket: &str,
        _key: &str,
        upload_id: &str,
    ) -> Result<Vec<PartInfo>, RustStackError> {
        let bucket_entry = self.buckets.get(bucket).ok_or_else(|| RustStackError::S3 {
            code: "NoSuchBucket".to_string(),
            message: "The specified bucket does not exist".to_string(),
            status: http::StatusCode::NOT_FOUND,
            resource: Some(bucket.to_string()),
        })?;

        let mp = bucket_entry
            .multipart_uploads
            .get(upload_id)
            .ok_or_else(|| RustStackError::S3 {
                code: "NoSuchUpload".to_string(),
                message: "The specified multipart upload does not exist.".to_string(),
                status: http::StatusCode::NOT_FOUND,
                resource: Some(upload_id.to_string()),
            })?;

        let parts = mp
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

        Ok(parts)
    }
}

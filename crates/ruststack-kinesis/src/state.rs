use crate::types::*;
use base64::Engine;
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum KinesisError {
    #[error("ResourceNotFoundException: {0}")]
    ResourceNotFound(String),
    #[error("ResourceInUseException: {0}")]
    ResourceInUse(String),
    #[error("InvalidArgumentException: {0}")]
    InvalidArgument(String),
    #[error("ExpiredIteratorException: {0}")]
    ExpiredIterator(String),
    #[error("LimitExceededException: {0}")]
    LimitExceeded(String),
    #[error("ValidationException: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ShardIteratorPayload {
    stream_name: String,
    shard_id: String,
    iterator_type: ShardIteratorType,
    sequence_number: Option<String>,
    timestamp: Option<f64>,
    created_at_ms: i64,
    next_index: usize,
}

#[derive(Clone)]
pub struct KinesisState {
    account_id: String,
    region: String,
    streams: Arc<DashMap<String, StoredStream>>,
    global_seq_counter: Arc<AtomicU64>,
}

impl KinesisState {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            streams: Arc::new(DashMap::new()),
            global_seq_counter: Arc::new(AtomicU64::new(1)),
        }
    }

    fn next_seq_number(&self) -> u64 {
        self.global_seq_counter.fetch_add(1, Ordering::SeqCst)
    }

    pub fn stream_arn(&self, stream_name: &str) -> String {
        format!(
            "arn:aws:kinesis:{}:{}:stream/{}",
            self.region, self.account_id, stream_name
        )
    }

    fn resolve_stream_name<'a>(
        &self,
        stream_name: Option<&'a str>,
        stream_arn: Option<&'a str>,
    ) -> Result<&'a str, KinesisError> {
        if let Some(name) = stream_name {
            Ok(name)
        } else if let Some(arn) = stream_arn {
            if let Some(pos) = arn.rfind('/') {
                Ok(&arn[pos + 1..])
            } else if let Some(pos) = arn.rfind(':') {
                Ok(&arn[pos + 1..])
            } else {
                Err(KinesisError::InvalidArgument(
                    "Invalid StreamARN format".to_string(),
                ))
            }
        } else {
            Err(KinesisError::InvalidArgument(
                "Either StreamName or StreamARN must be provided".to_string(),
            ))
        }
    }

    pub fn create_stream(&self, req: CreateStreamRequest) -> Result<(), KinesisError> {
        if req.stream_name.is_empty() {
            return Err(KinesisError::InvalidArgument(
                "Stream name cannot be empty".to_string(),
            ));
        }

        if self.streams.contains_key(&req.stream_name) {
            return Err(KinesisError::ResourceInUse(format!(
                "Stream {} under account {} already exists.",
                req.stream_name, self.account_id
            )));
        }

        let shard_count = req.shard_count.unwrap_or(1);
        if shard_count == 0 {
            return Err(KinesisError::InvalidArgument(
                "ShardCount must be greater than 0".to_string(),
            ));
        }

        let stream_mode = req
            .stream_mode_details
            .map(|d| d.stream_mode)
            .unwrap_or(StreamMode::Provisioned);

        let arn = self.stream_arn(&req.stream_name);
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;

        // Partition 128-bit hash range across shards
        let shards = Self::create_shards(shard_count);

        let stored = StoredStream {
            stream_name: req.stream_name.clone(),
            arn,
            stream_status: StreamStatus::Active,
            stream_mode,
            retention_period_hours: 24,
            stream_creation_timestamp: now,
            shards,
            tags: HashMap::new(),
            encryption_type: EncryptionType::None,
            key_id: None,
        };

        self.streams.insert(req.stream_name, stored);
        Ok(())
    }

    fn create_shards(shard_count: u32) -> Vec<StoredShard> {
        let max_hash = u128::MAX;
        let count = shard_count as u128;
        let step = max_hash / count;

        let mut shards = Vec::with_capacity(shard_count as usize);
        for i in 0..shard_count {
            let idx = i as u128;
            let start = idx * step;
            let end = if i == shard_count - 1 {
                max_hash
            } else {
                (idx + 1) * step - 1
            };

            shards.push(StoredShard {
                shard_id: format!("shardId-{:012}", i),
                parent_shard_id: None,
                adjacent_parent_shard_id: None,
                starting_hash_key: start.to_string(),
                ending_hash_key: end.to_string(),
                starting_sequence_number:
                    "49500000000000000000000000000000000000000000000000000000".to_string(),
                ending_sequence_number: None,
                records: Vec::new(),
            });
        }
        shards
    }

    pub fn delete_stream(&self, req: DeleteStreamRequest) -> Result<(), KinesisError> {
        let name =
            self.resolve_stream_name(req.stream_name.as_deref(), req.stream_arn.as_deref())?;
        if self.streams.remove(name).is_some() {
            Ok(())
        } else {
            Err(KinesisError::ResourceNotFound(format!(
                "Stream {} under account {} not found.",
                name, self.account_id
            )))
        }
    }

    pub fn describe_stream(
        &self,
        req: DescribeStreamRequest,
    ) -> Result<StreamDescription, KinesisError> {
        let name =
            self.resolve_stream_name(req.stream_name.as_deref(), req.stream_arn.as_deref())?;
        let stream = self.streams.get(name).ok_or_else(|| {
            KinesisError::ResourceNotFound(format!(
                "Stream {} under account {} not found.",
                name, self.account_id
            ))
        })?;

        let mut all_shards: Vec<Shard> = stream
            .shards
            .iter()
            .map(|s| Shard {
                shard_id: s.shard_id.clone(),
                parent_shard_id: s.parent_shard_id.clone(),
                adjacent_parent_shard_id: s.adjacent_parent_shard_id.clone(),
                hash_key_range: HashKeyRange {
                    starting_hash_key: s.starting_hash_key.clone(),
                    ending_hash_key: s.ending_hash_key.clone(),
                },
                sequence_number_range: SequenceNumberRange {
                    starting_sequence_number: s.starting_sequence_number.clone(),
                    ending_sequence_number: s.ending_sequence_number.clone(),
                },
            })
            .collect();

        all_shards.sort_by(|a, b| a.shard_id.cmp(&b.shard_id));

        if let Some(ref start_shard_id) = req.exclusive_start_shard_id {
            if let Some(pos) = all_shards
                .iter()
                .position(|s| &s.shard_id == start_shard_id)
            {
                all_shards = all_shards.split_off(pos + 1);
            }
        }

        let limit = req.limit.unwrap_or(100);
        let has_more = all_shards.len() > limit;
        if has_more {
            all_shards.truncate(limit);
        }

        Ok(StreamDescription {
            stream_name: stream.stream_name.clone(),
            stream_arn: stream.arn.clone(),
            stream_status: stream.stream_status,
            stream_mode_details: Some(StreamModeDetails {
                stream_mode: stream.stream_mode,
            }),
            shards: all_shards,
            has_more_shards: has_more,
            retention_period_hours: stream.retention_period_hours,
            stream_creation_timestamp: stream.stream_creation_timestamp,
            enhanced_monitoring: vec![EnhancedMetrics {
                shard_level_metrics: vec![],
            }],
            encryption_type: stream.encryption_type,
            key_id: stream.key_id.clone(),
        })
    }

    pub fn describe_stream_summary(
        &self,
        req: DescribeStreamSummaryRequest,
    ) -> Result<StreamDescriptionSummary, KinesisError> {
        let name =
            self.resolve_stream_name(req.stream_name.as_deref(), req.stream_arn.as_deref())?;
        let stream = self.streams.get(name).ok_or_else(|| {
            KinesisError::ResourceNotFound(format!(
                "Stream {} under account {} not found.",
                name, self.account_id
            ))
        })?;

        Ok(StreamDescriptionSummary {
            stream_name: stream.stream_name.clone(),
            stream_arn: stream.arn.clone(),
            stream_status: stream.stream_status,
            stream_mode_details: Some(StreamModeDetails {
                stream_mode: stream.stream_mode,
            }),
            retention_period_hours: stream.retention_period_hours,
            stream_creation_timestamp: stream.stream_creation_timestamp,
            enhanced_monitoring: vec![EnhancedMetrics {
                shard_level_metrics: vec![],
            }],
            encryption_type: stream.encryption_type,
            key_id: stream.key_id.clone(),
            open_shard_count: stream
                .shards
                .iter()
                .filter(|s| s.ending_sequence_number.is_none())
                .count() as u32,
            consumer_count: 0,
        })
    }

    pub fn list_streams(
        &self,
        req: ListStreamsRequest,
    ) -> Result<ListStreamsResponse, KinesisError> {
        let mut names: Vec<String> = self
            .streams
            .iter()
            .map(|item| item.stream_name.clone())
            .collect();

        names.sort();

        if let Some(ref start_name) = req.exclusive_start_stream_name {
            names.retain(|n| n > start_name);
        }

        if let Some(ref filter) = req.stream_name {
            names.retain(|n| n.contains(filter));
        }

        let limit = req.limit.unwrap_or(100);
        let has_more = names.len() > limit;
        if has_more {
            names.truncate(limit);
        }

        Ok(ListStreamsResponse {
            stream_names: names,
            has_more_streams: has_more,
            next_token: None,
            stream_summaries: None,
        })
    }

    fn route_hash(
        shards: &[StoredShard],
        partition_key: &str,
        explicit_hash_key: Option<&str>,
    ) -> usize {
        use md5::Digest;
        let hash = if let Some(explicit) = explicit_hash_key {
            explicit.parse::<u128>().unwrap_or_else(|_| {
                let digest = md5::Md5::digest(partition_key.as_bytes());
                let arr: [u8; 16] = digest.into();
                u128::from_be_bytes(arr)
            })
        } else {
            let digest = md5::Md5::digest(partition_key.as_bytes());
            let arr: [u8; 16] = digest.into();
            u128::from_be_bytes(arr)
        };

        for (idx, shard) in shards.iter().enumerate() {
            let start = shard.starting_hash_key.parse::<u128>().unwrap_or(0);
            let end = shard.ending_hash_key.parse::<u128>().unwrap_or(u128::MAX);
            if hash >= start && hash <= end {
                return idx;
            }
        }
        0
    }

    fn generate_sequence_number(&self) -> String {
        let seq = self.global_seq_counter.fetch_add(1, Ordering::SeqCst);
        format!("495{:053}", seq)
    }

    pub fn put_record(&self, req: PutRecordRequest) -> Result<PutRecordResponse, KinesisError> {
        let name =
            self.resolve_stream_name(req.stream_name.as_deref(), req.stream_arn.as_deref())?;
        let mut stream = self.streams.get_mut(name).ok_or_else(|| {
            KinesisError::ResourceNotFound(format!(
                "Stream {} under account {} not found.",
                name, self.account_id
            ))
        })?;

        if stream.shards.is_empty() {
            return Err(KinesisError::ResourceNotFound(
                "Stream has no active shards".to_string(),
            ));
        }

        let shard_idx = Self::route_hash(
            &stream.shards,
            &req.partition_key,
            req.explicit_hash_key.as_deref(),
        );

        let seq_str = self.generate_sequence_number();
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;
        let enc_type = stream.encryption_type;

        let shard = &mut stream.shards[shard_idx];
        let shard_id = shard.shard_id.clone();

        shard.records.push(StoredRecord {
            data: req.data,
            partition_key: req.partition_key,
            sequence_number: seq_str.clone(),
            approximate_arrival_timestamp: now,
            encryption_type: enc_type,
        });

        Ok(PutRecordResponse {
            sequence_number: seq_str,
            shard_id,
            encryption_type: Some(enc_type),
        })
    }

    pub fn put_records(&self, req: PutRecordsRequest) -> Result<PutRecordsResponse, KinesisError> {
        let name =
            self.resolve_stream_name(req.stream_name.as_deref(), req.stream_arn.as_deref())?;
        let mut stream = self.streams.get_mut(name).ok_or_else(|| {
            KinesisError::ResourceNotFound(format!(
                "Stream {} under account {} not found.",
                name, self.account_id
            ))
        })?;

        if stream.shards.is_empty() {
            return Err(KinesisError::ResourceNotFound(
                "Stream has no active shards".to_string(),
            ));
        }

        let enc_type = stream.encryption_type;
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;
        let mut results = Vec::with_capacity(req.records.len());

        for entry in req.records {
            let shard_idx = Self::route_hash(
                &stream.shards,
                &entry.partition_key,
                entry.explicit_hash_key.as_deref(),
            );

            let seq_str = self.generate_sequence_number();
            let shard = &mut stream.shards[shard_idx];
            let shard_id = shard.shard_id.clone();

            shard.records.push(StoredRecord {
                data: entry.data,
                partition_key: entry.partition_key,
                sequence_number: seq_str.clone(),
                approximate_arrival_timestamp: now,
                encryption_type: enc_type,
            });

            results.push(PutRecordsResultEntry {
                sequence_number: Some(seq_str),
                shard_id: Some(shard_id),
                error_code: None,
                error_message: None,
            });
        }

        Ok(PutRecordsResponse {
            failed_record_count: 0,
            records: results,
            encryption_type: Some(enc_type),
        })
    }

    pub fn get_shard_iterator(
        &self,
        req: GetShardIteratorRequest,
    ) -> Result<GetShardIteratorResponse, KinesisError> {
        let name =
            self.resolve_stream_name(req.stream_name.as_deref(), req.stream_arn.as_deref())?;
        let stream = self.streams.get(name).ok_or_else(|| {
            KinesisError::ResourceNotFound(format!(
                "Stream {} under account {} not found.",
                name, self.account_id
            ))
        })?;

        let shard = stream
            .shards
            .iter()
            .find(|s| s.shard_id == req.shard_id)
            .ok_or_else(|| {
                KinesisError::ResourceNotFound(format!(
                    "Shard {} in stream {} not found.",
                    req.shard_id, name
                ))
            })?;

        let next_index = match req.shard_iterator_type {
            ShardIteratorType::TrimHorizon => 0,
            ShardIteratorType::Latest => shard.records.len(),
            ShardIteratorType::AtSequenceNumber => {
                let target_seq = req.starting_sequence_number.as_deref().ok_or_else(|| {
                    KinesisError::InvalidArgument(
                        "StartingSequenceNumber must be provided for AT_SEQUENCE_NUMBER"
                            .to_string(),
                    )
                })?;
                shard
                    .records
                    .iter()
                    .position(|r| r.sequence_number.as_str() >= target_seq)
                    .unwrap_or(shard.records.len())
            }
            ShardIteratorType::AfterSequenceNumber => {
                let target_seq = req.starting_sequence_number.as_deref().ok_or_else(|| {
                    KinesisError::InvalidArgument(
                        "StartingSequenceNumber must be provided for AFTER_SEQUENCE_NUMBER"
                            .to_string(),
                    )
                })?;
                shard
                    .records
                    .iter()
                    .position(|r| r.sequence_number.as_str() > target_seq)
                    .unwrap_or(shard.records.len())
            }
            ShardIteratorType::AtTimestamp => {
                let target_ts = req.timestamp.ok_or_else(|| {
                    KinesisError::InvalidArgument(
                        "Timestamp must be provided for AT_TIMESTAMP".to_string(),
                    )
                })?;
                shard
                    .records
                    .iter()
                    .position(|r| r.approximate_arrival_timestamp >= target_ts)
                    .unwrap_or(shard.records.len())
            }
        };

        let now_ms = Utc::now().timestamp_millis();
        let payload = ShardIteratorPayload {
            stream_name: name.to_string(),
            shard_id: req.shard_id,
            iterator_type: req.shard_iterator_type,
            sequence_number: req.starting_sequence_number,
            timestamp: req.timestamp,
            created_at_ms: now_ms,
            next_index,
        };

        let payload_json = serde_json::to_string(&payload)
            .map_err(|e| KinesisError::InvalidArgument(e.to_string()))?;
        let iter_encoded = base64::engine::general_purpose::STANDARD.encode(payload_json);

        Ok(GetShardIteratorResponse {
            shard_iterator: Some(iter_encoded),
        })
    }

    pub fn get_records(&self, req: GetRecordsRequest) -> Result<GetRecordsResponse, KinesisError> {
        let decoded_bytes = base64::engine::general_purpose::STANDARD
            .decode(&req.shard_iterator)
            .map_err(|_| KinesisError::InvalidArgument("Invalid ShardIterator".to_string()))?;

        let payload: ShardIteratorPayload =
            serde_json::from_slice(&decoded_bytes).map_err(|_| {
                KinesisError::InvalidArgument("Invalid ShardIterator format".to_string())
            })?;

        let now_ms = Utc::now().timestamp_millis();
        // AWS iterators expire after 300 seconds
        if now_ms - payload.created_at_ms > 300_000 {
            return Err(KinesisError::ExpiredIterator(
                "Iterator has expired.".to_string(),
            ));
        }

        let stream = self.streams.get(&payload.stream_name).ok_or_else(|| {
            KinesisError::ResourceNotFound(format!(
                "Stream {} under account {} not found.",
                payload.stream_name, self.account_id
            ))
        })?;

        let shard = stream
            .shards
            .iter()
            .find(|s| s.shard_id == payload.shard_id)
            .ok_or_else(|| {
                KinesisError::ResourceNotFound(format!(
                    "Shard {} in stream {} not found.",
                    payload.shard_id, payload.stream_name
                ))
            })?;

        let limit = req.limit.unwrap_or(10_000).min(10_000);
        let total_records = shard.records.len();
        let start_idx = payload.next_index.min(total_records);
        let end_idx = (start_idx + limit).min(total_records);

        let records: Vec<Record> = shard.records[start_idx..end_idx]
            .iter()
            .map(|r| Record {
                data: r.data.clone(),
                partition_key: r.partition_key.clone(),
                sequence_number: r.sequence_number.clone(),
                approximate_arrival_timestamp: r.approximate_arrival_timestamp,
                encryption_type: Some(r.encryption_type),
            })
            .collect();

        let millis_behind_latest = if total_records > 0 && end_idx < total_records {
            let latest_ts = shard.records.last().unwrap().approximate_arrival_timestamp;
            let current_ts = if end_idx > 0 {
                shard.records[end_idx - 1].approximate_arrival_timestamp
            } else {
                shard.records[0].approximate_arrival_timestamp
            };
            ((latest_ts - current_ts).max(0.0) * 1000.0) as u64
        } else {
            0
        };

        let next_payload = ShardIteratorPayload {
            stream_name: payload.stream_name,
            shard_id: payload.shard_id,
            iterator_type: payload.iterator_type,
            sequence_number: None,
            timestamp: None,
            created_at_ms: now_ms,
            next_index: end_idx,
        };

        let next_payload_json = serde_json::to_string(&next_payload)
            .map_err(|e| KinesisError::InvalidArgument(e.to_string()))?;
        let next_iter_encoded = base64::engine::general_purpose::STANDARD.encode(next_payload_json);

        Ok(GetRecordsResponse {
            records,
            next_shard_iterator: Some(next_iter_encoded),
            millis_behind_latest,
            child_shards: Some(vec![]),
        })
    }

    pub fn add_tags_to_stream(&self, req: AddTagsToStreamRequest) -> Result<(), KinesisError> {
        let name =
            self.resolve_stream_name(req.stream_name.as_deref(), req.stream_arn.as_deref())?;
        let mut stream = self.streams.get_mut(name).ok_or_else(|| {
            KinesisError::ResourceNotFound(format!(
                "Stream {} under account {} not found.",
                name, self.account_id
            ))
        })?;

        for (k, v) in req.tags {
            stream.tags.insert(k, v);
        }
        Ok(())
    }

    pub fn remove_tags_from_stream(
        &self,
        req: RemoveTagsFromStreamRequest,
    ) -> Result<(), KinesisError> {
        let name =
            self.resolve_stream_name(req.stream_name.as_deref(), req.stream_arn.as_deref())?;
        let mut stream = self.streams.get_mut(name).ok_or_else(|| {
            KinesisError::ResourceNotFound(format!(
                "Stream {} under account {} not found.",
                name, self.account_id
            ))
        })?;

        for k in req.tag_keys {
            stream.tags.remove(&k);
        }
        Ok(())
    }

    pub fn list_tags_for_stream(
        &self,
        req: ListTagsForStreamRequest,
    ) -> Result<ListTagsForStreamResponse, KinesisError> {
        let name =
            self.resolve_stream_name(req.stream_name.as_deref(), req.stream_arn.as_deref())?;
        let stream = self.streams.get(name).ok_or_else(|| {
            KinesisError::ResourceNotFound(format!(
                "Stream {} under account {} not found.",
                name, self.account_id
            ))
        })?;

        let mut tags: Vec<Tag> = stream
            .tags
            .iter()
            .map(|(k, v)| Tag {
                key: k.clone(),
                value: v.clone(),
            })
            .collect();

        tags.sort_by(|a, b| a.key.cmp(&b.key));

        if let Some(ref start_key) = req.exclusive_start_tag_key {
            tags.retain(|t| &t.key > start_key);
        }

        let limit = req.limit.unwrap_or(50);
        let has_more = tags.len() > limit;
        if has_more {
            tags.truncate(limit);
        }

        Ok(ListTagsForStreamResponse {
            tags,
            has_more_tags: has_more,
        })
    }

    pub fn split_shard(&self, req: SplitShardRequest) -> Result<(), KinesisError> {
        let name =
            self.resolve_stream_name(req.stream_name.as_deref(), req.stream_arn.as_deref())?;
        let mut stream = self.streams.get_mut(name).ok_or_else(|| {
            KinesisError::ResourceNotFound(format!(
                "Stream {} under account {} not found.",
                name, self.account_id
            ))
        })?;

        let split_hash: u128 = req
            .new_starting_hash_key
            .parse()
            .map_err(|_| KinesisError::InvalidArgument("Invalid NewStartingHashKey".to_string()))?;

        let shard_idx = stream
            .shards
            .iter()
            .position(|s| s.shard_id == req.shard_to_split)
            .ok_or_else(|| {
                KinesisError::ResourceNotFound(format!("Shard {} not found", req.shard_to_split))
            })?;

        let (parent_start_str, parent_end_str) = {
            let parent = &mut stream.shards[shard_idx];
            if parent.ending_sequence_number.is_some() {
                return Err(KinesisError::InvalidArgument(format!(
                    "Shard {} is already closed",
                    req.shard_to_split
                )));
            }
            parent.ending_sequence_number = Some(format!("{:012}", self.next_seq_number()));
            (
                parent.starting_hash_key.clone(),
                parent.ending_hash_key.clone(),
            )
        };

        let parent_start: u128 = parent_start_str.parse().unwrap_or(0);
        let parent_end: u128 = parent_end_str.parse().unwrap_or(u128::MAX);

        if split_hash <= parent_start || split_hash > parent_end {
            return Err(KinesisError::InvalidArgument(
                "NewStartingHashKey is out of range".to_string(),
            ));
        }

        let new_shard_1_id = format!("shardId-{:012}", stream.shards.len());
        let new_shard_2_id = format!("shardId-{:012}", stream.shards.len() + 1);

        let shard1 = StoredShard {
            shard_id: new_shard_1_id,
            parent_shard_id: Some(req.shard_to_split.clone()),
            adjacent_parent_shard_id: None,
            starting_hash_key: parent_start_str,
            ending_hash_key: (split_hash - 1).to_string(),
            starting_sequence_number: format!("{:012}", self.next_seq_number()),
            ending_sequence_number: None,
            records: Vec::new(),
        };

        let shard2 = StoredShard {
            shard_id: new_shard_2_id,
            parent_shard_id: Some(req.shard_to_split),
            adjacent_parent_shard_id: None,
            starting_hash_key: split_hash.to_string(),
            ending_hash_key: parent_end_str,
            starting_sequence_number: format!("{:012}", self.next_seq_number()),
            ending_sequence_number: None,
            records: Vec::new(),
        };

        stream.shards.push(shard1);
        stream.shards.push(shard2);

        Ok(())
    }

    pub fn merge_shards(&self, req: MergeShardsRequest) -> Result<(), KinesisError> {
        let name =
            self.resolve_stream_name(req.stream_name.as_deref(), req.stream_arn.as_deref())?;
        let mut stream = self.streams.get_mut(name).ok_or_else(|| {
            KinesisError::ResourceNotFound(format!(
                "Stream {} under account {} not found.",
                name, self.account_id
            ))
        })?;

        let idx1 = stream
            .shards
            .iter()
            .position(|s| s.shard_id == req.shard_to_merge)
            .ok_or_else(|| {
                KinesisError::ResourceNotFound(format!("Shard {} not found", req.shard_to_merge))
            })?;

        let idx2 = stream
            .shards
            .iter()
            .position(|s| s.shard_id == req.adjacent_shard_to_merge)
            .ok_or_else(|| {
                KinesisError::ResourceNotFound(format!(
                    "Shard {} not found",
                    req.adjacent_shard_to_merge
                ))
            })?;

        let (start1, end1) = {
            let s1 = &mut stream.shards[idx1];
            s1.ending_sequence_number = Some(format!("{:012}", self.next_seq_number()));
            (s1.starting_hash_key.clone(), s1.ending_hash_key.clone())
        };

        let (start2, end2) = {
            let s2 = &mut stream.shards[idx2];
            s2.ending_sequence_number = Some(format!("{:012}", self.next_seq_number()));
            (s2.starting_hash_key.clone(), s2.ending_hash_key.clone())
        };

        let u_start1: u128 = start1.parse().unwrap_or(0);
        let u_start2: u128 = start2.parse().unwrap_or(0);
        let u_end1: u128 = end1.parse().unwrap_or(0);
        let u_end2: u128 = end2.parse().unwrap_or(0);

        let merged_start = u_start1.min(u_start2).to_string();
        let merged_end = u_end1.max(u_end2).to_string();

        let new_shard_id = format!("shardId-{:012}", stream.shards.len());
        let merged_shard = StoredShard {
            shard_id: new_shard_id,
            parent_shard_id: Some(req.shard_to_merge),
            adjacent_parent_shard_id: Some(req.adjacent_shard_to_merge),
            starting_hash_key: merged_start,
            ending_hash_key: merged_end,
            starting_sequence_number: format!("{:012}", self.next_seq_number()),
            ending_sequence_number: None,
            records: Vec::new(),
        };

        stream.shards.push(merged_shard);
        Ok(())
    }

    pub fn update_shard_count(
        &self,
        req: UpdateShardCountRequest,
    ) -> Result<UpdateShardCountResponse, KinesisError> {
        let name =
            self.resolve_stream_name(req.stream_name.as_deref(), req.stream_arn.as_deref())?;
        let mut stream = self.streams.get_mut(name).ok_or_else(|| {
            KinesisError::ResourceNotFound(format!(
                "Stream {} under account {} not found.",
                name, self.account_id
            ))
        })?;

        if req.target_shard_count == 0 {
            return Err(KinesisError::InvalidArgument(
                "TargetShardCount must be greater than 0".to_string(),
            ));
        }

        let current_open_shards: Vec<usize> = stream
            .shards
            .iter()
            .enumerate()
            .filter(|(_, s)| s.ending_sequence_number.is_none())
            .map(|(i, _)| i)
            .collect();

        let current_count = current_open_shards.len();

        // Close current open shards
        let seq = format!("{:012}", self.next_seq_number());
        for idx in current_open_shards {
            stream.shards[idx].ending_sequence_number = Some(seq.clone());
        }

        // Create new target shards
        let max_hash = u128::MAX;
        let count = req.target_shard_count as u128;
        let step = max_hash / count;

        let base_len = stream.shards.len();
        for i in 0..req.target_shard_count {
            let idx = i as u128;
            let start = idx * step;
            let end = if i == req.target_shard_count - 1 {
                max_hash
            } else {
                (idx + 1) * step - 1
            };

            stream.shards.push(StoredShard {
                shard_id: format!("shardId-{:012}", base_len + i),
                parent_shard_id: None,
                adjacent_parent_shard_id: None,
                starting_hash_key: start.to_string(),
                ending_hash_key: end.to_string(),
                starting_sequence_number: format!("{:012}", self.next_seq_number()),
                ending_sequence_number: None,
                records: Vec::new(),
            });
        }

        Ok(UpdateShardCountResponse {
            stream_arn: Some(stream.arn.clone()),
            stream_name: Some(stream.stream_name.clone()),
            current_shard_count: current_count,
            target_shard_count: req.target_shard_count,
        })
    }

    pub fn export_snapshot(&self) -> KinesisStateSnapshot {
        let streams = self
            .streams
            .iter()
            .map(|item| item.value().clone())
            .collect();

        KinesisStateSnapshot {
            streams,
            global_seq_counter: self.global_seq_counter.load(Ordering::SeqCst),
        }
    }

    pub fn import_snapshot(&self, snapshot: KinesisStateSnapshot) {
        self.streams.clear();
        for stream in snapshot.streams {
            self.streams.insert(stream.stream_name.clone(), stream);
        }
        self.global_seq_counter
            .store(snapshot.global_seq_counter, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.streams.clear();
        self.global_seq_counter.store(1, Ordering::SeqCst);
    }
}

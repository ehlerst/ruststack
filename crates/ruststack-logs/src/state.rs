use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum LogsError {
    #[error("ResourceNotFoundException: {0}")]
    NotFound(String),
    #[error("ResourceAlreadyExistsException: {0}")]
    AlreadyExists(String),
    #[error("InvalidParameterException: {0}")]
    InvalidParameter(String),
    #[error("InvalidSequenceTokenException: {0}")]
    InvalidSequenceToken(String),
}

#[derive(Clone)]
pub struct LogsState {
    account_id: String,
    region: String,
    groups: Arc<DashMap<String, StoredLogGroup>>,
}

impl LogsState {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            groups: Arc::new(DashMap::new()),
        }
    }

    fn log_group_arn(&self, group_name: &str) -> String {
        format!(
            "arn:aws:logs:{}:{}:log-group:{}:*",
            self.region, self.account_id, group_name
        )
    }

    fn log_stream_arn(&self, group_name: &str, stream_name: &str) -> String {
        format!(
            "arn:aws:logs:{}:{}:log-group:{}:log-stream:{}",
            self.region, self.account_id, group_name, stream_name
        )
    }

    pub fn create_log_group(&self, req: CreateLogGroupRequest) -> Result<(), LogsError> {
        if self.groups.contains_key(&req.log_group_name) {
            return Err(LogsError::AlreadyExists(format!(
                "The specified log group already exists: {}",
                req.log_group_name
            )));
        }

        let now = Utc::now().timestamp_millis();
        let arn = self.log_group_arn(&req.log_group_name);

        let group = LogGroup {
            log_group_name: req.log_group_name.clone(),
            arn,
            creation_time: now,
            retention_in_days: req.retention_in_days,
            metric_filter_count: 0,
            stored_bytes: 0,
        };

        let stored = StoredLogGroup {
            group,
            tags: req.tags.unwrap_or_default(),
            streams: HashMap::new(),
        };

        self.groups.insert(req.log_group_name, stored);
        Ok(())
    }

    pub fn delete_log_group(&self, req: DeleteLogGroupRequest) -> Result<(), LogsError> {
        if self.groups.remove(&req.log_group_name).is_some() {
            Ok(())
        } else {
            Err(LogsError::NotFound(format!(
                "The specified log group does not exist: {}",
                req.log_group_name
            )))
        }
    }

    pub fn describe_log_groups(
        &self,
        req: DescribeLogGroupsRequest,
    ) -> Result<(Vec<LogGroup>, Option<String>), LogsError> {
        let mut list = Vec::new();
        for item in self.groups.iter() {
            let name = &item.group.log_group_name;
            if let Some(ref prefix) = req.log_group_name_prefix {
                if !name.starts_with(prefix) {
                    continue;
                }
            }
            if let Some(ref pattern) = req.log_group_name_pattern {
                if !name.contains(pattern) {
                    continue;
                }
            }
            list.push(item.group.clone());
        }

        list.sort_by(|a, b| a.log_group_name.cmp(&b.log_group_name));
        let limit = req.limit.unwrap_or(50);
        let truncated = list.len() > limit;
        if truncated {
            list.truncate(limit);
        }

        Ok((list, None))
    }

    pub fn create_log_stream(&self, req: CreateLogStreamRequest) -> Result<(), LogsError> {
        let mut group_entry = self.groups.get_mut(&req.log_group_name).ok_or_else(|| {
            LogsError::NotFound(format!(
                "The specified log group does not exist: {}",
                req.log_group_name
            ))
        })?;

        if group_entry.streams.contains_key(&req.log_stream_name) {
            return Err(LogsError::AlreadyExists(format!(
                "The specified log stream already exists: {}",
                req.log_stream_name
            )));
        }

        let now = Utc::now().timestamp_millis();
        let arn = self.log_stream_arn(&req.log_group_name, &req.log_stream_name);

        let stream = LogStream {
            log_stream_name: req.log_stream_name.clone(),
            arn,
            creation_time: now,
            first_event_timestamp: None,
            last_event_timestamp: None,
            last_ingress_time: None,
            upload_sequence_token: Some("1".to_string()),
            stored_bytes: 0,
        };

        let stored = StoredLogStream {
            stream,
            events: Vec::new(),
        };

        group_entry.streams.insert(req.log_stream_name, stored);
        Ok(())
    }

    pub fn delete_log_stream(&self, req: DeleteLogStreamRequest) -> Result<(), LogsError> {
        let mut group_entry = self.groups.get_mut(&req.log_group_name).ok_or_else(|| {
            LogsError::NotFound(format!(
                "The specified log group does not exist: {}",
                req.log_group_name
            ))
        })?;

        if group_entry.streams.remove(&req.log_stream_name).is_some() {
            Ok(())
        } else {
            Err(LogsError::NotFound(format!(
                "The specified log stream does not exist: {}",
                req.log_stream_name
            )))
        }
    }

    pub fn describe_log_streams(
        &self,
        req: DescribeLogStreamsRequest,
    ) -> Result<(Vec<LogStream>, Option<String>), LogsError> {
        let group_entry = self.groups.get(&req.log_group_name).ok_or_else(|| {
            LogsError::NotFound(format!(
                "The specified log group does not exist: {}",
                req.log_group_name
            ))
        })?;

        let mut list = Vec::new();
        for (_, stored) in &group_entry.streams {
            if let Some(ref prefix) = req.log_stream_name_prefix {
                if !stored.stream.log_stream_name.starts_with(prefix) {
                    continue;
                }
            }
            list.push(stored.stream.clone());
        }

        let descending = req.descending.unwrap_or(false);
        if req.order_by.as_deref() == Some("LastEventTime") {
            list.sort_by(|a, b| {
                let cmp = a.last_event_timestamp.cmp(&b.last_event_timestamp);
                if descending {
                    cmp.reverse()
                } else {
                    cmp
                }
            });
        } else {
            list.sort_by(|a, b| {
                let cmp = a.log_stream_name.cmp(&b.log_stream_name);
                if descending {
                    cmp.reverse()
                } else {
                    cmp
                }
            });
        }

        let limit = req.limit.unwrap_or(50);
        if list.len() > limit {
            list.truncate(limit);
        }

        Ok((list, None))
    }

    pub fn put_log_events(&self, req: PutLogEventsRequest) -> Result<String, LogsError> {
        let mut group_entry = self.groups.get_mut(&req.log_group_name).ok_or_else(|| {
            LogsError::NotFound(format!(
                "The specified log group does not exist: {}",
                req.log_group_name
            ))
        })?;

        let next_token = {
            let stream_entry = group_entry
                .streams
                .get_mut(&req.log_stream_name)
                .ok_or_else(|| {
                    LogsError::NotFound(format!(
                        "The specified log stream does not exist: {}",
                        req.log_stream_name
                    ))
                })?;

            let now = Utc::now().timestamp_millis();
            let mut added_bytes = 0i64;

            for event in req.log_events {
                let msg_len = event.message.as_bytes().len() as i64 + 26;
                added_bytes += msg_len;

                let ts = event.timestamp;
                stream_entry.stream.first_event_timestamp =
                    Some(match stream_entry.stream.first_event_timestamp {
                        Some(prev) => prev.min(ts),
                        None => ts,
                    });
                stream_entry.stream.last_event_timestamp =
                    Some(match stream_entry.stream.last_event_timestamp {
                        Some(prev) => prev.max(ts),
                        None => ts,
                    });
                stream_entry.stream.last_ingress_time = Some(now);

                stream_entry.events.push(StoredLogEvent {
                    event_id: Uuid::new_v4().to_string(),
                    timestamp: ts,
                    message: event.message,
                    ingestion_time: now,
                });
            }

            stream_entry.stream.stored_bytes += added_bytes;

            let cur_token: u64 = stream_entry
                .stream
                .upload_sequence_token
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
            let next_token = (cur_token + 1).to_string();
            stream_entry.stream.upload_sequence_token = Some(next_token.clone());

            (next_token, added_bytes)
        };

        group_entry.group.stored_bytes += next_token.1;
        Ok(next_token.0)
    }

    pub fn get_log_events(
        &self,
        req: GetLogEventsRequest,
    ) -> Result<Vec<OutputLogEvent>, LogsError> {
        let group_entry = self.groups.get(&req.log_group_name).ok_or_else(|| {
            LogsError::NotFound(format!(
                "The specified log group does not exist: {}",
                req.log_group_name
            ))
        })?;

        let stream_entry = group_entry
            .streams
            .get(&req.log_stream_name)
            .ok_or_else(|| {
                LogsError::NotFound(format!(
                    "The specified log stream does not exist: {}",
                    req.log_stream_name
                ))
            })?;

        let mut events: Vec<OutputLogEvent> = stream_entry
            .events
            .iter()
            .filter(|e| {
                if let Some(st) = req.start_time {
                    if e.timestamp < st {
                        return false;
                    }
                }
                if let Some(et) = req.end_time {
                    if e.timestamp >= et {
                        return false;
                    }
                }
                true
            })
            .map(|e| OutputLogEvent {
                timestamp: e.timestamp,
                message: e.message.clone(),
                ingestion_time: e.ingestion_time,
            })
            .collect();

        events.sort_by_key(|e| e.timestamp);
        if let Some(limit) = req.limit {
            if events.len() > limit {
                events.truncate(limit);
            }
        }

        Ok(events)
    }

    pub fn filter_log_events(
        &self,
        req: FilterLogEventsRequest,
    ) -> Result<Vec<FilteredLogEvent>, LogsError> {
        let group_entry = self.groups.get(&req.log_group_name).ok_or_else(|| {
            LogsError::NotFound(format!(
                "The specified log group does not exist: {}",
                req.log_group_name
            ))
        })?;

        let mut results = Vec::new();

        for (stream_name, stream_entry) in &group_entry.streams {
            if let Some(ref target_streams) = req.log_stream_names {
                if !target_streams.contains(stream_name) {
                    continue;
                }
            }
            if let Some(ref prefix) = req.log_stream_name_prefix {
                if !stream_name.starts_with(prefix) {
                    continue;
                }
            }

            for e in &stream_entry.events {
                if let Some(st) = req.start_time {
                    if e.timestamp < st {
                        continue;
                    }
                }
                if let Some(et) = req.end_time {
                    if e.timestamp >= et {
                        continue;
                    }
                }
                if let Some(ref pat) = req.filter_pattern {
                    if !e.message.contains(pat) {
                        continue;
                    }
                }

                results.push(FilteredLogEvent {
                    event_id: e.event_id.clone(),
                    log_stream_name: stream_name.clone(),
                    timestamp: e.timestamp,
                    message: e.message.clone(),
                    ingestion_time: e.ingestion_time,
                });
            }
        }

        results.sort_by_key(|e| e.timestamp);
        let limit = req.limit.unwrap_or(100);
        if results.len() > limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    pub fn export_snapshot(&self) -> LogsStateSnapshot {
        let mut groups = Vec::new();
        for item in self.groups.iter() {
            groups.push(item.clone());
        }
        LogsStateSnapshot { groups }
    }

    pub fn import_snapshot(&self, snapshot: LogsStateSnapshot) {
        self.groups.clear();
        for g in snapshot.groups {
            self.groups.insert(g.group.log_group_name.clone(), g);
        }
    }

    pub fn reset(&self) {
        self.groups.clear();
    }
}

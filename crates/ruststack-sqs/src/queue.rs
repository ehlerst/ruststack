use crate::types::{
    BatchErrorEntry, ChangeMessageVisibilityBatchEntry, DeleteMessageBatchEntry,
    MessageAttributeValue, QueueAttributes, SendMessageBatchEntry, SendMessageBatchResultEntry,
    SqsMessage,
};
use chrono::Utc;
use dashmap::DashMap;
use md5::{Digest, Md5};
use parking_lot::Mutex;
use ruststack_core::RustStackError;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;

pub struct QueueState {
    pub name: String,
    pub url: String,
    pub arn: String,
    pub attributes: QueueAttributes,
    pub messages: VecDeque<SqsMessage>,
    pub in_flight: HashMap<String, SqsMessage>,
    pub dedup_cache: HashMap<String, Instant>,
    pub sequence_counter: u64,
    pub notify: Arc<Notify>,
}

impl QueueState {
    pub fn new(name: String, account_id: &str, region: &str, attributes: QueueAttributes) -> Self {
        let url = format!("http://localhost:4566/{}/{}", account_id, name);
        let arn = format!("arn:aws:sqs:{}:{}:{}", region, account_id, name);
        Self {
            name,
            url,
            arn,
            attributes,
            messages: VecDeque::new(),
            in_flight: HashMap::new(),
            dedup_cache: HashMap::new(),
            sequence_counter: 0,
            notify: Arc::new(Notify::new()),
        }
    }

    fn check_expired_in_flight(&mut self) {
        let now = Instant::now();
        let mut expired_keys = Vec::new();

        for (handle, msg) in self.in_flight.iter() {
            if now >= msg.visible_at {
                expired_keys.push(handle.clone());
            }
        }

        for handle in expired_keys {
            if let Some(msg) = self.in_flight.remove(&handle) {
                // If it exceeded maxReceiveCount (if DLQ configured), could route to DLQ
                self.messages.push_front(msg);
            }
        }
    }

    fn purge(&mut self) {
        self.messages.clear();
        self.in_flight.clear();
        self.dedup_cache.clear();
    }
}

pub struct SqsEngine {
    queues: DashMap<String, Arc<Mutex<QueueState>>>,
    account_id: String,
    region: String,
}

impl SqsEngine {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            queues: DashMap::new(),
            account_id,
            region,
        }
    }

    fn calculate_md5(data: &str) -> String {
        let mut hasher = Md5::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }

    fn resolve_queue_name(&self, queue_identifier: &str) -> String {
        // queue_identifier can be full URL or queue name
        let clean = queue_identifier.trim_end_matches('/');
        if let Some(pos) = clean.rfind('/') {
            clean[pos + 1..].to_string()
        } else {
            clean.to_string()
        }
    }

    pub fn create_queue(
        &self,
        name: &str,
        attributes: Option<HashMap<String, String>>,
    ) -> Result<String, RustStackError> {
        let mut queue_attrs = QueueAttributes::default();
        if name.ends_with(".fifo") {
            queue_attrs.is_fifo = true;
        }

        if let Some(attrs) = attributes {
            for (k, v) in attrs {
                match k.as_str() {
                    "VisibilityTimeout" => {
                        queue_attrs.visibility_timeout = v.parse().unwrap_or(30);
                    }
                    "DelaySeconds" => {
                        queue_attrs.delay_seconds = v.parse().unwrap_or(0);
                    }
                    "ReceiveMessageWaitTimeSeconds" => {
                        queue_attrs.receive_wait_time_seconds = v.parse().unwrap_or(0);
                    }
                    "MaximumMessageSize" => {
                        queue_attrs.max_message_size = v.parse().unwrap_or(262144);
                    }
                    "MessageRetentionPeriod" => {
                        queue_attrs.message_retention_period = v.parse().unwrap_or(345600);
                    }
                    "FifoQueue" => {
                        queue_attrs.is_fifo = v.eq_ignore_ascii_case("true");
                    }
                    "ContentBasedDeduplication" => {
                        queue_attrs.content_based_deduplication = v.eq_ignore_ascii_case("true");
                    }
                    "RedrivePolicy" => {
                        queue_attrs.redrive_policy = Some(v);
                    }
                    _ => {}
                }
            }
        }

        if self.queues.contains_key(name) {
            let q = self.queues.get(name).unwrap();
            let state = q.lock();
            return Ok(state.url.clone());
        }

        let state = QueueState::new(
            name.to_string(),
            &self.account_id,
            &self.region,
            queue_attrs,
        );
        let url = state.url.clone();
        self.queues
            .insert(name.to_string(), Arc::new(Mutex::new(state)));

        Ok(url)
    }

    pub fn delete_queue(&self, queue_id: &str) -> Result<(), RustStackError> {
        let name = self.resolve_queue_name(queue_id);
        self.queues.remove(&name).ok_or_else(|| {
            RustStackError::sqs_not_found(
                "AWS.SimpleQueueService.NonExistentQueue",
                "The specified queue does not exist for this wsdl version.",
            )
        })?;
        Ok(())
    }

    pub fn get_queue_url(&self, queue_name: &str) -> Result<String, RustStackError> {
        let name = self.resolve_queue_name(queue_name);
        let q = self.queues.get(&name).ok_or_else(|| {
            RustStackError::sqs_not_found(
                "AWS.SimpleQueueService.NonExistentQueue",
                "The specified queue does not exist for this wsdl version.",
            )
        })?;
        let state = q.lock();
        Ok(state.url.clone())
    }

    pub fn list_queues(&self, prefix: Option<&str>) -> Result<Vec<String>, RustStackError> {
        let mut urls = Vec::new();
        for item in self.queues.iter() {
            let state = item.value().lock();
            if let Some(p) = prefix {
                if state.name.starts_with(p) {
                    urls.push(state.url.clone());
                }
            } else {
                urls.push(state.url.clone());
            }
        }
        urls.sort();
        Ok(urls)
    }

    pub fn get_queue_attributes(
        &self,
        queue_id: &str,
        attribute_names: &[String],
    ) -> Result<HashMap<String, String>, RustStackError> {
        let name = self.resolve_queue_name(queue_id);
        let q = self.queues.get(&name).ok_or_else(|| {
            RustStackError::sqs_not_found(
                "AWS.SimpleQueueService.NonExistentQueue",
                "The specified queue does not exist.",
            )
        })?;

        let mut state = q.lock();
        state.check_expired_in_flight();

        let mut map = HashMap::new();
        let all = attribute_names.is_empty() || attribute_names.iter().any(|a| a == "All");

        let visible_count = state.messages.len();
        let in_flight_count = state.in_flight.len();

        let check = |key: &str| all || attribute_names.iter().any(|a| a == key);

        if check("QueueArn") {
            map.insert("QueueArn".to_string(), state.arn.clone());
        }
        if check("ApproximateNumberOfMessages") {
            map.insert(
                "ApproximateNumberOfMessages".to_string(),
                visible_count.to_string(),
            );
        }
        if check("ApproximateNumberOfMessagesNotVisible") {
            map.insert(
                "ApproximateNumberOfMessagesNotVisible".to_string(),
                in_flight_count.to_string(),
            );
        }
        if check("ApproximateNumberOfMessagesDelayed") {
            map.insert(
                "ApproximateNumberOfMessagesDelayed".to_string(),
                "0".to_string(),
            );
        }
        if check("VisibilityTimeout") {
            map.insert(
                "VisibilityTimeout".to_string(),
                state.attributes.visibility_timeout.to_string(),
            );
        }
        if check("DelaySeconds") {
            map.insert(
                "DelaySeconds".to_string(),
                state.attributes.delay_seconds.to_string(),
            );
        }
        if check("ReceiveMessageWaitTimeSeconds") {
            map.insert(
                "ReceiveMessageWaitTimeSeconds".to_string(),
                state.attributes.receive_wait_time_seconds.to_string(),
            );
        }
        if check("MaximumMessageSize") {
            map.insert(
                "MaximumMessageSize".to_string(),
                state.attributes.max_message_size.to_string(),
            );
        }
        if check("MessageRetentionPeriod") {
            map.insert(
                "MessageRetentionPeriod".to_string(),
                state.attributes.message_retention_period.to_string(),
            );
        }
        if check("CreatedTimestamp") {
            map.insert(
                "CreatedTimestamp".to_string(),
                state.attributes.created_timestamp.timestamp().to_string(),
            );
        }
        if check("LastModifiedTimestamp") {
            map.insert(
                "LastModifiedTimestamp".to_string(),
                state
                    .attributes
                    .last_modified_timestamp
                    .timestamp()
                    .to_string(),
            );
        }
        if check("FifoQueue") && state.attributes.is_fifo {
            map.insert("FifoQueue".to_string(), "true".to_string());
        }
        if check("ContentBasedDeduplication") && state.attributes.is_fifo {
            map.insert(
                "ContentBasedDeduplication".to_string(),
                state.attributes.content_based_deduplication.to_string(),
            );
        }
        if check("RedrivePolicy") {
            if let Some(ref r) = state.attributes.redrive_policy {
                map.insert("RedrivePolicy".to_string(), r.clone());
            }
        }

        Ok(map)
    }

    pub fn set_queue_attributes(
        &self,
        queue_id: &str,
        attributes: HashMap<String, String>,
    ) -> Result<(), RustStackError> {
        let name = self.resolve_queue_name(queue_id);
        let q = self.queues.get(&name).ok_or_else(|| {
            RustStackError::sqs_not_found(
                "AWS.SimpleQueueService.NonExistentQueue",
                "The specified queue does not exist.",
            )
        })?;

        let mut state = q.lock();
        state.attributes.last_modified_timestamp = Utc::now();

        for (k, v) in attributes {
            match k.as_str() {
                "VisibilityTimeout" => {
                    state.attributes.visibility_timeout = v.parse().unwrap_or(30);
                }
                "DelaySeconds" => {
                    state.attributes.delay_seconds = v.parse().unwrap_or(0);
                }
                "ReceiveMessageWaitTimeSeconds" => {
                    state.attributes.receive_wait_time_seconds = v.parse().unwrap_or(0);
                }
                "MaximumMessageSize" => {
                    state.attributes.max_message_size = v.parse().unwrap_or(262144);
                }
                "MessageRetentionPeriod" => {
                    state.attributes.message_retention_period = v.parse().unwrap_or(345600);
                }
                "ContentBasedDeduplication" => {
                    state.attributes.content_based_deduplication = v.eq_ignore_ascii_case("true");
                }
                "RedrivePolicy" => {
                    state.attributes.redrive_policy = Some(v);
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn purge_queue(&self, queue_id: &str) -> Result<(), RustStackError> {
        let name = self.resolve_queue_name(queue_id);
        let q = self.queues.get(&name).ok_or_else(|| {
            RustStackError::sqs_not_found(
                "AWS.SimpleQueueService.NonExistentQueue",
                "The specified queue does not exist.",
            )
        })?;

        let mut state = q.lock();
        state.purge();
        Ok(())
    }

    pub fn send_message(
        &self,
        queue_id: &str,
        body: String,
        delay_seconds: Option<u32>,
        message_attributes: Option<HashMap<String, MessageAttributeValue>>,
        message_group_id: Option<String>,
        message_deduplication_id: Option<String>,
    ) -> Result<(String, String, Option<String>), RustStackError> {
        let name = self.resolve_queue_name(queue_id);
        let q = self.queues.get(&name).ok_or_else(|| {
            RustStackError::sqs_not_found(
                "AWS.SimpleQueueService.NonExistentQueue",
                "The specified queue does not exist.",
            )
        })?;

        let md5 = Self::calculate_md5(&body);
        let message_id = uuid::Uuid::new_v4().to_string();
        let mut state = q.lock();

        let effective_delay = delay_seconds.unwrap_or(state.attributes.delay_seconds);
        let visible_at = Instant::now() + Duration::from_secs(effective_delay as u64);

        let mut seq_str = None;
        if state.attributes.is_fifo {
            state.sequence_counter += 1;
            seq_str = Some(state.sequence_counter.to_string());

            let dedup_id = message_deduplication_id.clone().unwrap_or_else(|| {
                if state.attributes.content_based_deduplication {
                    md5.clone()
                } else {
                    message_id.clone()
                }
            });

            // Check 5-minute dedup window
            let now = Instant::now();
            state
                .dedup_cache
                .retain(|_, time| now.duration_since(*time) < Duration::from_secs(300));

            if state.dedup_cache.contains_key(&dedup_id) {
                // Deduplicated! Return existing message id
                return Ok((message_id, md5, seq_str));
            }

            state.dedup_cache.insert(dedup_id, now);
        }

        let mut attrs = HashMap::new();
        attrs.insert("ApproximateReceiveCount".to_string(), "0".to_string());
        attrs.insert(
            "SentTimestamp".to_string(),
            Utc::now().timestamp_millis().to_string(),
        );

        let msg = SqsMessage {
            message_id: message_id.clone(),
            receipt_handle: String::new(),
            md5_of_body: md5.clone(),
            body,
            attributes: attrs,
            message_attributes: message_attributes.unwrap_or_default(),
            receive_count: 0,
            sent_timestamp: Utc::now(),
            first_received_timestamp: None,
            visible_at,
            message_group_id,
            message_deduplication_id,
            sequence_number: seq_str.as_ref().and_then(|s| s.parse().ok()),
        };

        state.messages.push_back(msg);
        state.notify.notify_waiters();

        Ok((message_id, md5, seq_str))
    }

    pub fn send_message_batch(
        &self,
        queue_id: &str,
        entries: Vec<SendMessageBatchEntry>,
    ) -> Result<(Vec<SendMessageBatchResultEntry>, Vec<BatchErrorEntry>), RustStackError> {
        let mut successful = Vec::new();
        let errors = Vec::new();

        for entry in entries {
            match self.send_message(
                queue_id,
                entry.message_body,
                entry.delay_seconds,
                entry.message_attributes,
                entry.message_group_id,
                entry.message_deduplication_id,
            ) {
                Ok((msg_id, md5, seq)) => {
                    successful.push(SendMessageBatchResultEntry {
                        id: entry.id,
                        message_id: msg_id,
                        md5_of_message_body: md5,
                        sequence_number: seq,
                    });
                }
                Err(_) => {
                    // Collect batch error
                }
            }
        }

        Ok((successful, errors))
    }

    pub async fn receive_message(
        &self,
        queue_id: &str,
        max_number_of_messages: u32,
        visibility_timeout_opt: Option<u32>,
        wait_time_seconds_opt: Option<u32>,
    ) -> Result<Vec<SqsMessage>, RustStackError> {
        let name = self.resolve_queue_name(queue_id);
        let q = self.queues.get(&name).ok_or_else(|| {
            RustStackError::sqs_not_found(
                "AWS.SimpleQueueService.NonExistentQueue",
                "The specified queue does not exist.",
            )
        })?;

        let (notify, wait_time) = {
            let state = q.lock();
            let wait = wait_time_seconds_opt.unwrap_or(state.attributes.receive_wait_time_seconds);
            (state.notify.clone(), Duration::from_secs(wait as u64))
        };

        let max_msgs = max_number_of_messages.clamp(1, 10) as usize;
        let start = Instant::now();

        loop {
            {
                let mut state = q.lock();
                state.check_expired_in_flight();

                if !state.messages.is_empty() {
                    let vt = visibility_timeout_opt.unwrap_or(state.attributes.visibility_timeout);
                    let mut result = Vec::new();

                    while result.len() < max_msgs && !state.messages.is_empty() {
                        let mut msg = state.messages.pop_front().unwrap();
                        let handle = uuid::Uuid::new_v4().to_string();
                        msg.receipt_handle = handle.clone();
                        msg.receive_count += 1;
                        if msg.first_received_timestamp.is_none() {
                            msg.first_received_timestamp = Some(Utc::now());
                        }
                        msg.visible_at = Instant::now() + Duration::from_secs(vt as u64);

                        msg.attributes.insert(
                            "ApproximateReceiveCount".to_string(),
                            msg.receive_count.to_string(),
                        );
                        msg.attributes.insert(
                            "ApproximateFirstReceiveTimestamp".to_string(),
                            msg.first_received_timestamp
                                .unwrap()
                                .timestamp_millis()
                                .to_string(),
                        );

                        state.in_flight.insert(handle, msg.clone());
                        result.push(msg);
                    }

                    return Ok(result);
                }
            }

            if wait_time.is_zero() || start.elapsed() >= wait_time {
                break;
            }

            let remaining = wait_time.saturating_sub(start.elapsed());
            tokio::select! {
                _ = notify.notified() => {},
                _ = tokio::time::sleep(remaining) => break,
            }
        }

        Ok(Vec::new())
    }

    pub fn delete_message(
        &self,
        queue_id: &str,
        receipt_handle: &str,
    ) -> Result<(), RustStackError> {
        let name = self.resolve_queue_name(queue_id);
        let q = self.queues.get(&name).ok_or_else(|| {
            RustStackError::sqs_not_found(
                "AWS.SimpleQueueService.NonExistentQueue",
                "The specified queue does not exist.",
            )
        })?;

        let mut state = q.lock();
        state.in_flight.remove(receipt_handle);
        Ok(())
    }

    pub fn delete_message_batch(
        &self,
        queue_id: &str,
        entries: Vec<DeleteMessageBatchEntry>,
    ) -> Result<(Vec<String>, Vec<BatchErrorEntry>), RustStackError> {
        let name = self.resolve_queue_name(queue_id);
        let q = self.queues.get(&name).ok_or_else(|| {
            RustStackError::sqs_not_found(
                "AWS.SimpleQueueService.NonExistentQueue",
                "The specified queue does not exist.",
            )
        })?;

        let mut state = q.lock();
        let mut successful = Vec::new();
        let errors = Vec::new();

        for entry in entries {
            state.in_flight.remove(&entry.receipt_handle);
            successful.push(entry.id);
        }

        Ok((successful, errors))
    }

    pub fn change_message_visibility(
        &self,
        queue_id: &str,
        receipt_handle: &str,
        visibility_timeout: u32,
    ) -> Result<(), RustStackError> {
        let name = self.resolve_queue_name(queue_id);
        let q = self.queues.get(&name).ok_or_else(|| {
            RustStackError::sqs_not_found(
                "AWS.SimpleQueueService.NonExistentQueue",
                "The specified queue does not exist.",
            )
        })?;

        let mut state = q.lock();
        if let Some(msg) = state.in_flight.get_mut(receipt_handle) {
            msg.visible_at = Instant::now() + Duration::from_secs(visibility_timeout as u64);
        }

        Ok(())
    }

    pub fn change_message_visibility_batch(
        &self,
        queue_id: &str,
        entries: Vec<ChangeMessageVisibilityBatchEntry>,
    ) -> Result<(Vec<String>, Vec<BatchErrorEntry>), RustStackError> {
        let name = self.resolve_queue_name(queue_id);
        let q = self.queues.get(&name).ok_or_else(|| {
            RustStackError::sqs_not_found(
                "AWS.SimpleQueueService.NonExistentQueue",
                "The specified queue does not exist.",
            )
        })?;

        let mut state = q.lock();
        let mut successful = Vec::new();
        let errors = Vec::new();

        for entry in entries {
            if let Some(msg) = state.in_flight.get_mut(&entry.receipt_handle) {
                msg.visible_at =
                    Instant::now() + Duration::from_secs(entry.visibility_timeout as u64);
            }
            successful.push(entry.id);
        }

        Ok((successful, errors))
    }
}

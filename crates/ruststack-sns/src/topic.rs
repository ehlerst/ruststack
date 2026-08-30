use crate::types::{
    BatchErrorEntry, MessageAttributeValue, PublishBatchEntry, PublishBatchResultEntry,
    Subscription, SubscriptionAttributes, Topic, TopicAttributes,
};
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use ruststack_core::RustStackError;
use ruststack_sqs::{MessageAttributeValue as SqsMessageAttributeValue, SqsEngine};
use std::collections::HashMap;
use std::sync::Arc;

pub struct SnsEngine {
    topics: DashMap<String, Arc<RwLock<Topic>>>,
    subscriptions: DashMap<String, Arc<RwLock<Subscription>>>,
    sqs_engine: Arc<SqsEngine>,
    account_id: String,
    region: String,
}

impl SnsEngine {
    pub fn new(sqs_engine: Arc<SqsEngine>, account_id: String, region: String) -> Self {
        Self {
            topics: DashMap::new(),
            subscriptions: DashMap::new(),
            sqs_engine,
            account_id,
            region,
        }
    }

    pub fn format_topic_arn(&self, name: &str) -> String {
        format!("arn:aws:sns:{}:{}:{}", self.region, self.account_id, name)
    }

    pub fn format_subscription_arn(&self, topic_name: &str, sub_id: &str) -> String {
        format!(
            "arn:aws:sns:{}:{}:{}:{}",
            self.region, self.account_id, topic_name, sub_id
        )
    }

    pub fn create_topic(
        &self,
        name: &str,
        attributes: Option<HashMap<String, String>>,
    ) -> Result<String, RustStackError> {
        let arn = self.format_topic_arn(name);

        if self.topics.contains_key(&arn) {
            return Ok(arn);
        }

        let mut topic_attrs = TopicAttributes::default();
        if name.ends_with(".fifo") {
            topic_attrs.is_fifo = true;
        }

        if let Some(attrs) = attributes {
            for (k, v) in attrs {
                match k.as_str() {
                    "DisplayName" => topic_attrs.display_name = Some(v),
                    "FifoTopic" => topic_attrs.is_fifo = v.eq_ignore_ascii_case("true"),
                    "ContentBasedDeduplication" => {
                        topic_attrs.content_based_deduplication = v.eq_ignore_ascii_case("true");
                    }
                    "KmsMasterKeyId" => topic_attrs.kms_master_key_id = Some(v),
                    "Policy" => topic_attrs.policy = Some(v),
                    "DeliveryPolicy" => topic_attrs.delivery_policy = Some(v),
                    _ => {}
                }
            }
        }

        let topic = Topic {
            arn: arn.clone(),
            name: name.to_string(),
            attributes: topic_attrs,
            created_timestamp: Utc::now(),
        };

        self.topics
            .insert(arn.clone(), Arc::new(RwLock::new(topic)));
        Ok(arn)
    }

    pub fn delete_topic(&self, topic_arn: &str) -> Result<(), RustStackError> {
        self.topics.remove(topic_arn).ok_or_else(|| {
            RustStackError::sns_not_found(
                "NotFound",
                format!("Topic does not exist: {}", topic_arn),
            )
        })?;

        // Remove associated subscriptions
        self.subscriptions
            .retain(|_, sub| sub.read().topic_arn != topic_arn);

        Ok(())
    }

    pub fn list_topics(&self) -> Result<Vec<String>, RustStackError> {
        let mut arns: Vec<String> = self.topics.iter().map(|item| item.key().clone()).collect();
        arns.sort();
        Ok(arns)
    }

    pub fn get_topic_attributes(
        &self,
        topic_arn: &str,
    ) -> Result<HashMap<String, String>, RustStackError> {
        let topic_entry = self.topics.get(topic_arn).ok_or_else(|| {
            RustStackError::sns_not_found(
                "NotFound",
                format!("Topic does not exist: {}", topic_arn),
            )
        })?;

        let topic = topic_entry.read();
        let mut map = HashMap::new();

        map.insert("TopicArn".to_string(), topic.arn.clone());
        map.insert("Owner".to_string(), self.account_id.clone());

        let sub_count = self
            .subscriptions
            .iter()
            .filter(|s| s.value().read().topic_arn == topic_arn)
            .count();
        map.insert("SubscriptionsConfirmed".to_string(), sub_count.to_string());
        map.insert("SubscriptionsPending".to_string(), "0".to_string());
        map.insert("SubscriptionsDeleted".to_string(), "0".to_string());

        if let Some(ref d) = topic.attributes.display_name {
            map.insert("DisplayName".to_string(), d.clone());
        }
        if topic.attributes.is_fifo {
            map.insert("FifoTopic".to_string(), "true".to_string());
            map.insert(
                "ContentBasedDeduplication".to_string(),
                topic.attributes.content_based_deduplication.to_string(),
            );
        }
        if let Some(ref p) = topic.attributes.policy {
            map.insert("Policy".to_string(), p.clone());
        }

        Ok(map)
    }

    pub fn set_topic_attributes(
        &self,
        topic_arn: &str,
        attribute_name: &str,
        attribute_value: &str,
    ) -> Result<(), RustStackError> {
        let topic_entry = self.topics.get(topic_arn).ok_or_else(|| {
            RustStackError::sns_not_found(
                "NotFound",
                format!("Topic does not exist: {}", topic_arn),
            )
        })?;

        let mut topic = topic_entry.write();
        match attribute_name {
            "DisplayName" => topic.attributes.display_name = Some(attribute_value.to_string()),
            "Policy" => topic.attributes.policy = Some(attribute_value.to_string()),
            "DeliveryPolicy" => {
                topic.attributes.delivery_policy = Some(attribute_value.to_string())
            }
            "ContentBasedDeduplication" => {
                topic.attributes.content_based_deduplication =
                    attribute_value.eq_ignore_ascii_case("true");
            }
            "KmsMasterKeyId" => {
                topic.attributes.kms_master_key_id = Some(attribute_value.to_string())
            }
            _ => {}
        }

        Ok(())
    }

    pub fn subscribe(
        &self,
        topic_arn: &str,
        protocol: &str,
        endpoint: &str,
        attributes: Option<HashMap<String, String>>,
    ) -> Result<String, RustStackError> {
        if !self.topics.contains_key(topic_arn) {
            return Err(RustStackError::sns_not_found(
                "NotFound",
                format!("Topic does not exist: {}", topic_arn),
            ));
        }

        let sub_id = uuid::Uuid::new_v4().to_string();
        let topic_name = topic_arn.split(':').next_back().unwrap_or("topic");
        let sub_arn = self.format_subscription_arn(topic_name, &sub_id);

        let mut sub_attrs = SubscriptionAttributes::default();
        if let Some(attrs) = attributes {
            for (k, v) in attrs {
                match k.as_str() {
                    "RawMessageDelivery" => {
                        sub_attrs.raw_message_delivery = v.eq_ignore_ascii_case("true");
                    }
                    "FilterPolicy" => {
                        sub_attrs.filter_policy = Some(v);
                    }
                    "RedrivePolicy" => {
                        sub_attrs.redrive_policy = Some(v);
                    }
                    _ => {}
                }
            }
        }

        let sub = Subscription {
            subscription_arn: sub_arn.clone(),
            topic_arn: topic_arn.to_string(),
            protocol: protocol.to_ascii_lowercase(),
            endpoint: endpoint.to_string(),
            owner: self.account_id.clone(),
            attributes: sub_attrs,
        };

        self.subscriptions
            .insert(sub_arn.clone(), Arc::new(RwLock::new(sub)));

        Ok(sub_arn)
    }

    pub fn unsubscribe(&self, subscription_arn: &str) -> Result<(), RustStackError> {
        self.subscriptions.remove(subscription_arn).ok_or_else(|| {
            RustStackError::sns_not_found(
                "NotFound",
                format!("Subscription does not exist: {}", subscription_arn),
            )
        })?;
        Ok(())
    }

    pub fn list_subscriptions(&self) -> Result<Vec<Subscription>, RustStackError> {
        let mut list: Vec<Subscription> = self
            .subscriptions
            .iter()
            .map(|item| item.value().read().clone())
            .collect();
        list.sort_by(|a, b| a.subscription_arn.cmp(&b.subscription_arn));
        Ok(list)
    }

    pub fn list_subscriptions_by_topic(
        &self,
        topic_arn: &str,
    ) -> Result<Vec<Subscription>, RustStackError> {
        if !self.topics.contains_key(topic_arn) {
            return Err(RustStackError::sns_not_found(
                "NotFound",
                format!("Topic does not exist: {}", topic_arn),
            ));
        }

        let mut list: Vec<Subscription> = self
            .subscriptions
            .iter()
            .filter(|item| item.value().read().topic_arn == topic_arn)
            .map(|item| item.value().read().clone())
            .collect();
        list.sort_by(|a, b| a.subscription_arn.cmp(&b.subscription_arn));
        Ok(list)
    }

    pub fn get_subscription_attributes(
        &self,
        subscription_arn: &str,
    ) -> Result<HashMap<String, String>, RustStackError> {
        let sub_entry = self.subscriptions.get(subscription_arn).ok_or_else(|| {
            RustStackError::sns_not_found(
                "NotFound",
                format!("Subscription does not exist: {}", subscription_arn),
            )
        })?;

        let sub = sub_entry.read();
        let mut map = HashMap::new();

        map.insert("SubscriptionArn".to_string(), sub.subscription_arn.clone());
        map.insert("TopicArn".to_string(), sub.topic_arn.clone());
        map.insert("Protocol".to_string(), sub.protocol.clone());
        map.insert("Endpoint".to_string(), sub.endpoint.clone());
        map.insert("Owner".to_string(), sub.owner.clone());
        map.insert(
            "RawMessageDelivery".to_string(),
            sub.attributes.raw_message_delivery.to_string(),
        );

        if let Some(ref fp) = sub.attributes.filter_policy {
            map.insert("FilterPolicy".to_string(), fp.clone());
        }
        if let Some(ref rp) = sub.attributes.redrive_policy {
            map.insert("RedrivePolicy".to_string(), rp.clone());
        }

        Ok(map)
    }

    pub fn set_subscription_attributes(
        &self,
        subscription_arn: &str,
        attribute_name: &str,
        attribute_value: &str,
    ) -> Result<(), RustStackError> {
        let sub_entry = self.subscriptions.get(subscription_arn).ok_or_else(|| {
            RustStackError::sns_not_found(
                "NotFound",
                format!("Subscription does not exist: {}", subscription_arn),
            )
        })?;

        let mut sub = sub_entry.write();
        match attribute_name {
            "RawMessageDelivery" => {
                sub.attributes.raw_message_delivery = attribute_value.eq_ignore_ascii_case("true");
            }
            "FilterPolicy" => {
                sub.attributes.filter_policy = Some(attribute_value.to_string());
            }
            "RedrivePolicy" => {
                sub.attributes.redrive_policy = Some(attribute_value.to_string());
            }
            _ => {}
        }

        Ok(())
    }

    fn matches_filter_policy(
        filter_policy_str: &str,
        message_attributes: &Option<HashMap<String, MessageAttributeValue>>,
    ) -> bool {
        let policy_val: serde_json::Value = match serde_json::from_str(filter_policy_str) {
            Ok(v) => v,
            Err(_) => return true,
        };

        let policy_obj = match policy_val.as_object() {
            Some(obj) => obj,
            None => return true,
        };

        let msg_attrs = match message_attributes {
            Some(attrs) => attrs,
            None => return false,
        };

        for (key, allowed_vals) in policy_obj {
            let attr = match msg_attrs.get(key) {
                Some(a) => a,
                None => return false,
            };

            let attr_val_str = match &attr.string_value {
                Some(s) => s.as_str(),
                None => return false,
            };

            if let Some(array) = allowed_vals.as_array() {
                let mut matched = false;
                for v in array {
                    if let Some(s) = v.as_str() {
                        if s == attr_val_str {
                            matched = true;
                            break;
                        }
                    } else if let Some(n) = v.as_i64() {
                        if n.to_string() == attr_val_str {
                            matched = true;
                            break;
                        }
                    }
                }
                if !matched {
                    return false;
                }
            }
        }

        true
    }

    pub fn publish(
        &self,
        topic_arn: &str,
        message: String,
        subject: Option<String>,
        message_attributes: Option<HashMap<String, MessageAttributeValue>>,
        message_deduplication_id: Option<String>,
        message_group_id: Option<String>,
    ) -> Result<(String, Option<String>), RustStackError> {
        let topic_entry = self.topics.get(topic_arn).ok_or_else(|| {
            RustStackError::sns_not_found(
                "NotFound",
                format!("Topic does not exist: {}", topic_arn),
            )
        })?;

        let is_fifo = topic_entry.read().attributes.is_fifo;
        let message_id = uuid::Uuid::new_v4().to_string();
        let timestamp = Utc::now();

        // 1. Gather all active subscriptions for this topic
        let matching_subs: Vec<Subscription> = self
            .subscriptions
            .iter()
            .filter(|item| item.value().read().topic_arn == topic_arn)
            .map(|item| item.value().read().clone())
            .collect();

        // 2. Fanout dispatch to all matching subscriptions
        for sub in matching_subs {
            // Check FilterPolicy
            if let Some(ref fp) = sub.attributes.filter_policy {
                if !Self::matches_filter_policy(fp, &message_attributes) {
                    continue;
                }
            }

            match sub.protocol.as_str() {
                "sqs" => {
                    let (sqs_body, sqs_attrs) = if sub.attributes.raw_message_delivery {
                        let sqs_msg_attrs: HashMap<String, SqsMessageAttributeValue> =
                            message_attributes
                                .as_ref()
                                .map(|attrs| {
                                    attrs
                                        .iter()
                                        .map(|(k, v)| {
                                            (
                                                k.clone(),
                                                SqsMessageAttributeValue {
                                                    data_type: v.data_type.clone(),
                                                    string_value: v.string_value.clone(),
                                                    binary_value: v.binary_value.clone(),
                                                },
                                            )
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();

                        (message.clone(), Some(sqs_msg_attrs))
                    } else {
                        // Standard AWS SNS JSON Notification envelope
                        let mut envelope = serde_json::json!({
                            "Type": "Notification",
                            "MessageId": message_id,
                            "TopicArn": topic_arn,
                            "Message": message,
                            "Timestamp": timestamp.to_rfc3339(),
                            "SignatureVersion": "1",
                            "Signature": "EXAMPLEpH+DcEwjAPg8O9mY8dReBSwksfg2S7WKQizmGIEePVKySt+wfNTWmww5acDxXdHobdYWWSTkXdFuTvWSukIHrZeOiKKO0cgJ3WoxARfzTJxSTeyKEqMyq8hCC2+FigPftuAVvMZCONFIGEXAMPLE=",
                            "SigningCertURL": "https://sns.us-east-1.amazonaws.com/SimpleNotificationService.pem",
                            "UnsubscribeURL": format!("http://localhost:4566/?Action=Unsubscribe&SubscriptionArn={}", sub.subscription_arn)
                        });

                        if let Some(ref s) = subject {
                            envelope["Subject"] = serde_json::Value::String(s.clone());
                        }

                        if let Some(ref attrs) = message_attributes {
                            let mut attr_json = serde_json::Map::new();
                            for (k, v) in attrs {
                                let mut single = serde_json::Map::new();
                                single.insert(
                                    "Type".to_string(),
                                    serde_json::Value::String(v.data_type.clone()),
                                );
                                if let Some(ref sv) = v.string_value {
                                    single.insert(
                                        "Value".to_string(),
                                        serde_json::Value::String(sv.clone()),
                                    );
                                }
                                attr_json.insert(k.clone(), serde_json::Value::Object(single));
                            }
                            envelope["MessageAttributes"] = serde_json::Value::Object(attr_json);
                        }

                        (envelope.to_string(), None)
                    };

                    // Deliver to SQS queue
                    let _ = self.sqs_engine.send_message(
                        &sub.endpoint,
                        sqs_body,
                        None,
                        sqs_attrs,
                        message_group_id.clone(),
                        message_deduplication_id.clone(),
                    );
                }
                _ => {
                    // Future protocols (HTTP webhook, lambda, etc.)
                }
            }
        }

        let seq = if is_fifo {
            Some("10000000000000000000".to_string())
        } else {
            None
        };

        Ok((message_id, seq))
    }

    pub fn publish_batch(
        &self,
        topic_arn: &str,
        entries: Vec<PublishBatchEntry>,
    ) -> Result<(Vec<PublishBatchResultEntry>, Vec<BatchErrorEntry>), RustStackError> {
        let mut successful = Vec::new();
        let errors = Vec::new();

        for entry in entries {
            match self.publish(
                topic_arn,
                entry.message,
                entry.subject,
                entry.message_attributes,
                entry.message_deduplication_id,
                entry.message_group_id,
            ) {
                Ok((msg_id, seq)) => {
                    successful.push(PublishBatchResultEntry {
                        id: entry.id,
                        message_id: msg_id,
                        sequence_number: seq,
                    });
                }
                Err(e) => {
                    // Collect batch errors
                    tracing::warn!("Failed publish batch entry: {:?}", e);
                }
            }
        }

        Ok((successful, errors))
    }
}

use crate::pattern::matches_event_pattern;
use crate::types::{
    EventBridgeSnapshot, EventBus, PutEventsRequestEntry, PutEventsResultEntry,
    PutTargetsResultEntry, RemoveTargetsResultEntry, Rule, RuleSnapshot, Target,
};
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use ruststack_core::RustStackError;
use ruststack_sns::SnsEngine;
use ruststack_sqs::SqsEngine;
use serde_json::json;
use std::sync::Arc;

pub struct EventBridgeEngine {
    buses: DashMap<String, Arc<RwLock<EventBus>>>,
    rules: DashMap<String, Arc<RwLock<Rule>>>,
    targets: DashMap<String, Arc<RwLock<Vec<Target>>>>,
    sqs_engine: Arc<SqsEngine>,
    sns_engine: Arc<SnsEngine>,
    account_id: String,
    region: String,
}

impl EventBridgeEngine {
    pub fn new(
        sqs_engine: Arc<SqsEngine>,
        sns_engine: Arc<SnsEngine>,
        account_id: String,
        region: String,
    ) -> Self {
        let engine = Self {
            buses: DashMap::new(),
            rules: DashMap::new(),
            targets: DashMap::new(),
            sqs_engine,
            sns_engine,
            account_id: account_id.clone(),
            region: region.clone(),
        };

        // Create default event bus
        let default_arn = format!("arn:aws:events:{}:{}:event-bus/default", region, account_id);
        let default_bus = EventBus {
            name: "default".to_string(),
            arn: default_arn,
            policy: None,
            created_timestamp: Utc::now(),
        };
        engine
            .buses
            .insert("default".to_string(), Arc::new(RwLock::new(default_bus)));

        engine
    }

    pub fn format_bus_arn(&self, name: &str) -> String {
        format!(
            "arn:aws:events:{}:{}:event-bus/{}",
            self.region, self.account_id, name
        )
    }

    pub fn format_rule_arn(&self, bus_name: &str, rule_name: &str) -> String {
        if bus_name == "default" {
            format!(
                "arn:aws:events:{}:{}:rule/{}",
                self.region, self.account_id, rule_name
            )
        } else {
            format!(
                "arn:aws:events:{}:{}:rule/{}/{}",
                self.region, self.account_id, bus_name, rule_name
            )
        }
    }

    pub fn create_event_bus(&self, name: &str) -> Result<String, RustStackError> {
        let arn = self.format_bus_arn(name);
        if self.buses.contains_key(name) {
            return Ok(arn);
        }

        let bus = EventBus {
            name: name.to_string(),
            arn: arn.clone(),
            policy: None,
            created_timestamp: Utc::now(),
        };

        self.buses
            .insert(name.to_string(), Arc::new(RwLock::new(bus)));
        Ok(arn)
    }

    pub fn delete_event_bus(&self, name: &str) -> Result<(), RustStackError> {
        if name == "default" {
            return Err(RustStackError::eventbridge_bad_request(
                "InvalidParameterValueException",
                "Cannot delete default event bus.",
            ));
        }

        self.buses.remove(name).ok_or_else(|| {
            RustStackError::eventbridge_not_found(
                "ResourceNotFoundException",
                format!("Event bus {} does not exist.", name),
            )
        })?;

        // Remove rules associated with this bus
        let rules_to_remove: Vec<String> = self
            .rules
            .iter()
            .filter(|r| r.value().read().event_bus_name == name)
            .map(|r| r.key().clone())
            .collect();

        for rule_key in rules_to_remove {
            self.rules.remove(&rule_key);
            self.targets.remove(&rule_key);
        }

        Ok(())
    }

    pub fn list_event_buses(
        &self,
        name_prefix: Option<&str>,
    ) -> Result<Vec<EventBus>, RustStackError> {
        let mut buses: Vec<EventBus> = self
            .buses
            .iter()
            .filter(|b| {
                if let Some(prefix) = name_prefix {
                    b.value().read().name.starts_with(prefix)
                } else {
                    true
                }
            })
            .map(|b| b.value().read().clone())
            .collect();

        buses.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(buses)
    }

    pub fn describe_event_bus(&self, name: Option<&str>) -> Result<EventBus, RustStackError> {
        let bus_name = name.unwrap_or("default");
        let bus = self.buses.get(bus_name).ok_or_else(|| {
            RustStackError::eventbridge_not_found(
                "ResourceNotFoundException",
                format!("Event bus {} does not exist.", bus_name),
            )
        })?;

        let res = bus.read().clone();
        Ok(res)
    }

    fn rule_key(bus_name: &str, rule_name: &str) -> String {
        format!("{}:{}", bus_name, rule_name)
    }

    pub fn put_rule(
        &self,
        name: &str,
        event_bus_name_opt: Option<&str>,
        event_pattern: Option<String>,
        state_opt: Option<&str>,
        description: Option<String>,
        schedule_expression: Option<String>,
    ) -> Result<String, RustStackError> {
        let bus_name = event_bus_name_opt.unwrap_or("default");
        if !self.buses.contains_key(bus_name) {
            return Err(RustStackError::eventbridge_not_found(
                "ResourceNotFoundException",
                format!("Event bus {} does not exist.", bus_name),
            ));
        }

        let key = Self::rule_key(bus_name, name);
        let arn = self.format_rule_arn(bus_name, name);
        let state = state_opt.unwrap_or("ENABLED").to_uppercase();

        let rule = Rule {
            name: name.to_string(),
            arn: arn.clone(),
            event_bus_name: bus_name.to_string(),
            event_pattern,
            state,
            description,
            schedule_expression,
            created_timestamp: Utc::now(),
        };

        self.rules.insert(key, Arc::new(RwLock::new(rule)));
        Ok(arn)
    }

    pub fn delete_rule(
        &self,
        name: &str,
        event_bus_name_opt: Option<&str>,
        _force: bool,
    ) -> Result<(), RustStackError> {
        let bus_name = event_bus_name_opt.unwrap_or("default");
        let key = Self::rule_key(bus_name, name);

        self.rules.remove(&key).ok_or_else(|| {
            RustStackError::eventbridge_not_found(
                "ResourceNotFoundException",
                format!("Rule {} does not exist on bus {}.", name, bus_name),
            )
        })?;

        self.targets.remove(&key);
        Ok(())
    }

    pub fn list_rules(
        &self,
        event_bus_name_opt: Option<&str>,
        name_prefix: Option<&str>,
    ) -> Result<Vec<Rule>, RustStackError> {
        let bus_name = event_bus_name_opt.unwrap_or("default");
        let mut list: Vec<Rule> = self
            .rules
            .iter()
            .filter(|r| {
                let rule = r.value().read();
                if rule.event_bus_name != bus_name {
                    return false;
                }
                if let Some(prefix) = name_prefix {
                    rule.name.starts_with(prefix)
                } else {
                    true
                }
            })
            .map(|r| r.value().read().clone())
            .collect();

        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(list)
    }

    pub fn describe_rule(
        &self,
        name: &str,
        event_bus_name_opt: Option<&str>,
    ) -> Result<Rule, RustStackError> {
        let bus_name = event_bus_name_opt.unwrap_or("default");
        let key = Self::rule_key(bus_name, name);

        let rule = self.rules.get(&key).ok_or_else(|| {
            RustStackError::eventbridge_not_found(
                "ResourceNotFoundException",
                format!("Rule {} does not exist on bus {}.", name, bus_name),
            )
        })?;

        let res = rule.read().clone();
        Ok(res)
    }

    pub fn enable_rule(
        &self,
        name: &str,
        event_bus_name_opt: Option<&str>,
    ) -> Result<(), RustStackError> {
        let bus_name = event_bus_name_opt.unwrap_or("default");
        let key = Self::rule_key(bus_name, name);

        let rule = self.rules.get(&key).ok_or_else(|| {
            RustStackError::eventbridge_not_found(
                "ResourceNotFoundException",
                format!("Rule {} does not exist on bus {}.", name, bus_name),
            )
        })?;

        rule.write().state = "ENABLED".to_string();
        Ok(())
    }

    pub fn disable_rule(
        &self,
        name: &str,
        event_bus_name_opt: Option<&str>,
    ) -> Result<(), RustStackError> {
        let bus_name = event_bus_name_opt.unwrap_or("default");
        let key = Self::rule_key(bus_name, name);

        let rule = self.rules.get(&key).ok_or_else(|| {
            RustStackError::eventbridge_not_found(
                "ResourceNotFoundException",
                format!("Rule {} does not exist on bus {}.", name, bus_name),
            )
        })?;

        rule.write().state = "DISABLED".to_string();
        Ok(())
    }

    pub fn put_targets(
        &self,
        rule_name: &str,
        event_bus_name_opt: Option<&str>,
        targets: Vec<Target>,
    ) -> Result<Vec<PutTargetsResultEntry>, RustStackError> {
        let bus_name = event_bus_name_opt.unwrap_or("default");
        let key = Self::rule_key(bus_name, rule_name);

        if !self.rules.contains_key(&key) {
            return Err(RustStackError::eventbridge_not_found(
                "ResourceNotFoundException",
                format!("Rule {} does not exist on bus {}.", rule_name, bus_name),
            ));
        }

        let entry = self
            .targets
            .entry(key)
            .or_insert_with(|| Arc::new(RwLock::new(Vec::new())));
        let mut target_list = entry.write();

        let mut results = Vec::new();
        for new_target in targets {
            target_list.retain(|t| t.id != new_target.id);
            results.push(PutTargetsResultEntry {
                target_id: new_target.id.clone(),
                error_code: None,
                error_message: None,
            });
            target_list.push(new_target);
        }

        Ok(results)
    }

    pub fn remove_targets(
        &self,
        rule_name: &str,
        event_bus_name_opt: Option<&str>,
        ids: Vec<String>,
    ) -> Result<Vec<RemoveTargetsResultEntry>, RustStackError> {
        let bus_name = event_bus_name_opt.unwrap_or("default");
        let key = Self::rule_key(bus_name, rule_name);

        if let Some(entry) = self.targets.get(&key) {
            let mut target_list = entry.write();
            target_list.retain(|t| !ids.contains(&t.id));
        }

        let results = ids
            .into_iter()
            .map(|id| RemoveTargetsResultEntry {
                target_id: id,
                error_code: None,
                error_message: None,
            })
            .collect();

        Ok(results)
    }

    pub fn list_targets_by_rule(
        &self,
        rule_name: &str,
        event_bus_name_opt: Option<&str>,
    ) -> Result<Vec<Target>, RustStackError> {
        let bus_name = event_bus_name_opt.unwrap_or("default");
        let key = Self::rule_key(bus_name, rule_name);

        if let Some(entry) = self.targets.get(&key) {
            let res = entry.read().clone();
            Ok(res)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn put_events(
        &self,
        entries: Vec<PutEventsRequestEntry>,
    ) -> Result<(usize, Vec<PutEventsResultEntry>), RustStackError> {
        let mut result_entries = Vec::new();
        let failed_count = 0;

        for entry in entries {
            let event_id = uuid::Uuid::new_v4().to_string();
            let bus_name = entry.event_bus_name.as_deref().unwrap_or("default");

            // Format standard AWS CloudWatch / EventBridge envelope
            let detail_val: serde_json::Value = entry
                .detail
                .as_deref()
                .and_then(|d| serde_json::from_str(d).ok())
                .unwrap_or_else(|| json!({}));

            let event_payload = json!({
                "version": "0",
                "id": event_id,
                "detail-type": entry.detail_type.as_deref().unwrap_or(""),
                "source": entry.source.as_deref().unwrap_or(""),
                "account": self.account_id,
                "time": entry.time.unwrap_or_else(Utc::now).to_rfc3339(),
                "region": self.region,
                "resources": entry.resources.unwrap_or_default(),
                "detail": detail_val
            });

            // Find all matching enabled rules on this bus
            for rule_item in self.rules.iter() {
                let rule = rule_item.value().read();
                if rule.event_bus_name != bus_name || rule.state != "ENABLED" {
                    continue;
                }

                let matches = match rule.event_pattern {
                    Some(ref pattern) => matches_event_pattern(pattern, &event_payload),
                    None => true,
                };

                if matches {
                    let key = Self::rule_key(bus_name, &rule.name);
                    if let Some(target_entry) = self.targets.get(&key) {
                        let targets = target_entry.read().clone();
                        for target in targets {
                            let message_body = target
                                .input
                                .clone()
                                .unwrap_or_else(|| event_payload.to_string());

                            // Route by target ARN
                            if target.arn.starts_with("arn:aws:sqs:") {
                                let _ = self.sqs_engine.send_message(
                                    &target.arn,
                                    message_body,
                                    None,
                                    None,
                                    None,
                                    None,
                                );
                            } else if target.arn.starts_with("arn:aws:sns:") {
                                let _ = self.sns_engine.publish(
                                    &target.arn,
                                    message_body,
                                    entry.detail_type.clone(),
                                    None,
                                    None,
                                    None,
                                );
                            }
                        }
                    }
                }
            }

            result_entries.push(PutEventsResultEntry {
                event_id: Some(event_id),
                error_code: None,
                error_message: None,
            });
        }

        Ok((failed_count, result_entries))
    }

    pub fn reset(&self) {
        self.buses.clear();
        self.rules.clear();
        self.targets.clear();

        // Restore default event bus
        let default_arn = format!(
            "arn:aws:events:{}:{}:event-bus/default",
            self.region, self.account_id
        );
        let default_bus = EventBus {
            name: "default".to_string(),
            arn: default_arn,
            policy: None,
            created_timestamp: Utc::now(),
        };
        self.buses
            .insert("default".to_string(), Arc::new(RwLock::new(default_bus)));
    }

    pub fn dump_state(&self) -> EventBridgeSnapshot {
        let buses = self
            .buses
            .iter()
            .map(|e| e.value().read().clone())
            .collect();
        let mut rules = Vec::new();
        for entry in self.rules.iter() {
            let rule = entry.value().read().clone();
            let targets = self
                .targets
                .get(&rule.name)
                .map(|t| t.read().clone())
                .unwrap_or_default();
            rules.push(RuleSnapshot { rule, targets });
        }
        EventBridgeSnapshot {
            event_buses: buses,
            rules,
        }
    }

    pub fn load_state(&self, snapshot: EventBridgeSnapshot) {
        self.buses.clear();
        self.rules.clear();
        self.targets.clear();
        for b in snapshot.event_buses {
            self.buses.insert(b.name.clone(), Arc::new(RwLock::new(b)));
        }
        for r_snap in snapshot.rules {
            let rule_name = r_snap.rule.name.clone();
            self.rules
                .insert(rule_name.clone(), Arc::new(RwLock::new(r_snap.rule)));
            self.targets
                .insert(rule_name, Arc::new(RwLock::new(r_snap.targets)));
        }
    }
}

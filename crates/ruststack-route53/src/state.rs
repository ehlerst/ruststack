use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum Route53Error {
    #[error("NoSuchHostedZone: {0}")]
    NoSuchHostedZone(String),
    #[error("HostedZoneAlreadyExists: {0}")]
    HostedZoneAlreadyExists(String),
    #[error("InvalidInput: {0}")]
    InvalidInput(String),
    #[error("NoSuchChange: {0}")]
    NoSuchChange(String),
}

#[derive(Clone)]
pub struct Route53State {
    pub account_id: String,
    pub region: String,
    hosted_zones: Arc<DashMap<String, Arc<RwLock<StoredHostedZone>>>>,
}

impl Route53State {
    pub fn new(account_id: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            account_id: account_id.into(),
            region: region.into(),
            hosted_zones: Arc::new(DashMap::new()),
        }
    }

    fn normalize_zone_id(zone_id: &str) -> String {
        let trimmed = zone_id.trim_start_matches("/hostedzone/").trim_start_matches('/');
        trimmed.to_string()
    }

    fn normalize_name(name: &str) -> String {
        if name.ends_with('.') {
            name.to_string()
        } else {
            format!("{}.", name)
        }
    }

    pub fn create_hosted_zone(
        &self,
        req: CreateHostedZoneRequest,
    ) -> Result<CreateHostedZoneResponse, Route53Error> {
        let zone_name = Self::normalize_name(&req.name);
        let zone_id = format!("Z{:014X}", fastrand::u64(..));
        let change_id = format!("/change/C{:014X}", fastrand::u64(..));
        let now = Utc::now().to_rfc3339();

        let mut record_sets = HashMap::new();
        // Default NS and SOA records
        let ns_records = vec![
            ResourceRecord {
                value: "ns-1.awsdns.com.".to_string(),
            },
            ResourceRecord {
                value: "ns-2.awsdns.net.".to_string(),
            },
        ];
        record_sets.insert(
            (zone_name.clone(), "NS".to_string()),
            ResourceRecordSet {
                name: zone_name.clone(),
                record_type: "NS".to_string(),
                ttl: Some(172800),
                resource_records: Some(ns_records.clone()),
                set_identifier: None,
                weight: None,
            },
        );

        let stored = StoredHostedZone {
            id: zone_id.clone(),
            name: zone_name.clone(),
            caller_reference: req.caller_reference.clone(),
            comment: req.hosted_zone_config.as_ref().and_then(|c| c.comment.clone()),
            private_zone: req.hosted_zone_config.as_ref().and_then(|c| c.private_zone).unwrap_or(false),
            record_sets,
        };

        self.hosted_zones
            .insert(zone_id.clone(), Arc::new(RwLock::new(stored)));

        Ok(CreateHostedZoneResponse {
            hosted_zone: HostedZone {
                id: format!("/hostedzone/{}", zone_id),
                name: zone_name,
                caller_reference: req.caller_reference,
                config: req.hosted_zone_config,
                resource_record_set_count: Some(2),
            },
            change_info: ChangeInfo {
                id: change_id,
                status: "INSYNC".to_string(),
                submitted_at: now,
                comment: None,
            },
            delegation_set: DelegationSet {
                name_servers: ns_records.into_iter().map(|r| r.value).collect(),
            },
        })
    }

    pub fn get_hosted_zone(&self, zone_id: &str) -> Result<GetHostedZoneResponse, Route53Error> {
        let norm_id = Self::normalize_zone_id(zone_id);
        let zone_entry = self.hosted_zones.get(&norm_id).ok_or_else(|| {
            Route53Error::NoSuchHostedZone(format!("No hosted zone found with ID: {}", zone_id))
        })?;

        let zone = zone_entry.read();
        Ok(GetHostedZoneResponse {
            hosted_zone: HostedZone {
                id: format!("/hostedzone/{}", zone.id),
                name: zone.name.clone(),
                caller_reference: zone.caller_reference.clone(),
                config: Some(HostedZoneConfig {
                    comment: zone.comment.clone(),
                    private_zone: Some(zone.private_zone),
                }),
                resource_record_set_count: Some(zone.record_sets.len() as i64),
            },
            delegation_set: Some(DelegationSet {
                name_servers: vec![
                    "ns-1.awsdns.com.".to_string(),
                    "ns-2.awsdns.net.".to_string(),
                ],
            }),
        })
    }

    pub fn list_hosted_zones(&self) -> Result<ListHostedZonesResponse, Route53Error> {
        let mut list = Vec::new();
        for item in self.hosted_zones.iter() {
            let zone = item.value().read();
            list.push(HostedZone {
                id: format!("/hostedzone/{}", zone.id),
                name: zone.name.clone(),
                caller_reference: zone.caller_reference.clone(),
                config: Some(HostedZoneConfig {
                    comment: zone.comment.clone(),
                    private_zone: Some(zone.private_zone),
                }),
                resource_record_set_count: Some(zone.record_sets.len() as i64),
            });
        }
        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(ListHostedZonesResponse {
            hosted_zones: list,
            marker: None,
            is_truncated: false,
            next_marker: None,
            max_items: Some(100),
        })
    }

    pub fn delete_hosted_zone(&self, zone_id: &str) -> Result<ChangeInfo, Route53Error> {
        let norm_id = Self::normalize_zone_id(zone_id);
        self.hosted_zones.remove(&norm_id).ok_or_else(|| {
            Route53Error::NoSuchHostedZone(format!("No hosted zone found with ID: {}", zone_id))
        })?;

        Ok(ChangeInfo {
            id: format!("/change/C{:014X}", fastrand::u64(..)),
            status: "INSYNC".to_string(),
            submitted_at: Utc::now().to_rfc3339(),
            comment: None,
        })
    }

    pub fn change_resource_record_sets(
        &self,
        zone_id: &str,
        req: ChangeResourceRecordSetsRequest,
    ) -> Result<ChangeResourceRecordSetsResponse, Route53Error> {
        let norm_id = Self::normalize_zone_id(zone_id);
        let zone_entry = self.hosted_zones.get(&norm_id).ok_or_else(|| {
            Route53Error::NoSuchHostedZone(format!("No hosted zone found with ID: {}", zone_id))
        })?;

        let mut zone = zone_entry.write();
        for change in req.change_batch.changes {
            let record_name = Self::normalize_name(&change.resource_record_set.name);
            let rtype = change.resource_record_set.record_type.to_uppercase();
            let key = (record_name.clone(), rtype.clone());

            match change.action.to_uppercase().as_str() {
                "CREATE" | "UPSERT" => {
                    let mut record = change.resource_record_set;
                    record.name = record_name;
                    record.record_type = rtype;
                    zone.record_sets.insert(key, record);
                }
                "DELETE" => {
                    zone.record_sets.remove(&key);
                }
                _ => {}
            }
        }

        Ok(ChangeResourceRecordSetsResponse {
            change_info: ChangeInfo {
                id: format!("/change/C{:014X}", fastrand::u64(..)),
                status: "INSYNC".to_string(),
                submitted_at: Utc::now().to_rfc3339(),
                comment: req.change_batch.comment,
            },
        })
    }

    pub fn list_resource_record_sets(
        &self,
        zone_id: &str,
    ) -> Result<ListResourceRecordSetsResponse, Route53Error> {
        let norm_id = Self::normalize_zone_id(zone_id);
        let zone_entry = self.hosted_zones.get(&norm_id).ok_or_else(|| {
            Route53Error::NoSuchHostedZone(format!("No hosted zone found with ID: {}", zone_id))
        })?;

        let zone = zone_entry.read();
        let mut list: Vec<ResourceRecordSet> = zone.record_sets.values().cloned().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.record_type.cmp(&b.record_type)));

        Ok(ListResourceRecordSetsResponse {
            resource_record_sets: list,
            is_truncated: false,
            max_items: Some(100),
        })
    }

    pub fn export_snapshot(&self) -> Route53StateSnapshot {
        let mut map = HashMap::new();
        for item in self.hosted_zones.iter() {
            let zone = item.value().read().clone();
            map.insert(item.key().clone(), zone);
        }
        Route53StateSnapshot { hosted_zones: map }
    }

    pub fn import_snapshot(&self, snapshot: Route53StateSnapshot) {
        self.hosted_zones.clear();
        for (k, v) in snapshot.hosted_zones {
            self.hosted_zones.insert(k, Arc::new(RwLock::new(v)));
        }
    }

    pub fn reset(&self) {
        self.hosted_zones.clear();
    }
}

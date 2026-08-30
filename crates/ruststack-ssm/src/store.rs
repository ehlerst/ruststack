use crate::types::{
    Parameter, ParameterRecordSnapshot, ParameterType, PutParameterRequest, SsmSnapshot,
};
use chrono::Utc;
use parking_lot::RwLock;
use ruststack_core::RustStackError;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub struct SsmEngine {
    parameters: Arc<RwLock<BTreeMap<String, Parameter>>>,
    history: Arc<RwLock<HashMap<String, Vec<Parameter>>>>,
    account_id: String,
    region: String,
}

impl SsmEngine {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            parameters: Arc::new(RwLock::new(BTreeMap::new())),
            history: Arc::new(RwLock::new(HashMap::new())),
            account_id,
            region,
        }
    }

    pub fn format_parameter_arn(&self, name: &str) -> String {
        let clean_name = name.trim_start_matches('/');
        format!(
            "arn:aws:ssm:{}:{}:parameter/{}",
            self.region, self.account_id, clean_name
        )
    }

    pub fn put_parameter(&self, req: PutParameterRequest) -> Result<i64, RustStackError> {
        let mut params = self.parameters.write();
        let mut history = self.history.write();

        let name = req.name.clone();
        let param_type = req
            .parameter_type
            .as_deref()
            .unwrap_or("String")
            .parse()
            .unwrap_or(ParameterType::String);
        let overwrite = req.overwrite.unwrap_or(false);
        let tier = req.tier.unwrap_or_else(|| "Standard".to_string());
        let data_type = req.data_type.unwrap_or_else(|| "text".to_string());

        if let Some(existing) = params.get_mut(&name) {
            if !overwrite {
                return Err(RustStackError::ssm_bad_request(
                    "ParameterAlreadyExists",
                    format!("The parameter already exists: {}", name),
                ));
            }

            existing.version += 1;
            existing.value = req.value;
            existing.parameter_type = param_type;
            existing.description = req.description;
            existing.tier = tier;
            existing.data_type = data_type;
            existing.key_id = req.key_id;
            existing.last_modified_date = Utc::now();

            let version = existing.version;
            history.entry(name).or_default().push(existing.clone());
            Ok(version)
        } else {
            let arn = self.format_parameter_arn(&name);
            let param = Parameter {
                name: name.clone(),
                parameter_type: param_type,
                value: req.value,
                version: 1,
                last_modified_date: Utc::now(),
                arn,
                data_type,
                description: req.description,
                key_id: req.key_id,
                tier,
            };

            history.entry(name.clone()).or_default().push(param.clone());
            params.insert(name, param);
            Ok(1)
        }
    }

    pub fn get_parameter(
        &self,
        name_or_version: &str,
        _with_decryption: bool,
    ) -> Result<Parameter, RustStackError> {
        // Check if version is specified: /path/param:2
        let (name, ver_opt) = if let Some(pos) = name_or_version.rfind(':') {
            let (n, v) = name_or_version.split_at(pos);
            if let Ok(ver) = v[1..].parse::<i64>() {
                (n, Some(ver))
            } else {
                (name_or_version, None)
            }
        } else {
            (name_or_version, None)
        };

        if let Some(target_ver) = ver_opt {
            let history = self.history.read();
            if let Some(hist) = history.get(name) {
                if let Some(p) = hist.iter().find(|p| p.version == target_ver) {
                    return Ok(p.clone());
                }
            }
            return Err(RustStackError::ssm_not_found(
                "ParameterVersionNotFound",
                format!("Parameter {} version {} not found", name, target_ver),
            ));
        }

        let params = self.parameters.read();
        params.get(name).cloned().ok_or_else(|| {
            RustStackError::ssm_not_found(
                "ParameterNotFound",
                format!("Parameter {} not found.", name),
            )
        })
    }

    pub fn get_parameters(
        &self,
        names: &[String],
        _with_decryption: bool,
    ) -> Result<(Vec<Parameter>, Vec<String>), RustStackError> {
        let params = self.parameters.read();
        let mut found = Vec::new();
        let mut invalid = Vec::new();

        for n in names {
            if let Some(p) = params.get(n) {
                found.push(p.clone());
            } else {
                invalid.push(n.clone());
            }
        }

        Ok((found, invalid))
    }

    pub fn get_parameters_by_path(
        &self,
        path: &str,
        recursive: bool,
        _with_decryption: bool,
        max_results: Option<usize>,
    ) -> Result<Vec<Parameter>, RustStackError> {
        let params = self.parameters.read();
        let prefix = path.trim_end_matches('/');
        let max = max_results.unwrap_or(10).clamp(1, 50);

        let mut results = Vec::new();

        for (name, param) in params.iter() {
            if !name.starts_with(prefix) {
                continue;
            }

            let suffix = &name[prefix.len()..];
            if !suffix.starts_with('/') && !suffix.is_empty() {
                continue;
            }

            if !recursive {
                // If not recursive, only allow 1 level of slash
                let sub_slashes = suffix
                    .trim_start_matches('/')
                    .chars()
                    .filter(|c| *c == '/')
                    .count();
                if sub_slashes > 0 {
                    continue;
                }
            }

            results.push(param.clone());
            if results.len() >= max {
                break;
            }
        }

        Ok(results)
    }

    pub fn delete_parameter(&self, name: &str) -> Result<(), RustStackError> {
        let mut params = self.parameters.write();
        let mut history = self.history.write();

        params.remove(name).ok_or_else(|| {
            RustStackError::ssm_not_found(
                "ParameterNotFound",
                format!("Parameter {} not found.", name),
            )
        })?;

        history.remove(name);
        Ok(())
    }

    pub fn delete_parameters(
        &self,
        names: &[String],
    ) -> Result<(Vec<String>, Vec<String>), RustStackError> {
        let mut params = self.parameters.write();
        let mut history = self.history.write();

        let mut deleted = Vec::new();
        let mut invalid = Vec::new();

        for n in names {
            if params.remove(n).is_some() {
                history.remove(n);
                deleted.push(n.clone());
            } else {
                invalid.push(n.clone());
            }
        }

        Ok((deleted, invalid))
    }

    pub fn describe_parameters(&self) -> Result<Vec<Parameter>, RustStackError> {
        let params = self.parameters.read();
        let list: Vec<Parameter> = params.values().cloned().collect();
        Ok(list)
    }

    pub fn reset(&self) {
        self.parameters.write().clear();
        self.history.write().clear();
    }

    pub fn dump_state(&self) -> SsmSnapshot {
        let params = self.parameters.read();
        let history = self.history.read();
        let mut list = Vec::new();
        for (name, current) in params.iter() {
            let hist = history.get(name).cloned().unwrap_or_default();
            list.push(ParameterRecordSnapshot {
                current: current.clone(),
                history: hist,
            });
        }
        SsmSnapshot { parameters: list }
    }

    pub fn load_state(&self, snapshot: SsmSnapshot) {
        let mut params = self.parameters.write();
        let mut history = self.history.write();
        params.clear();
        history.clear();
        for rec in snapshot.parameters {
            let name = rec.current.name.clone();
            params.insert(name.clone(), rec.current);
            if !rec.history.is_empty() {
                history.insert(name, rec.history);
            }
        }
    }
}

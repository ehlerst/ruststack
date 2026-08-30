use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParameterType {
    String,
    StringList,
    SecureString,
}

impl ParameterType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::String => "String",
            Self::StringList => "StringList",
            Self::SecureString => "SecureString",
        }
    }
}

impl FromStr for ParameterType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "StringList" => Self::StringList,
            "SecureString" => Self::SecureString,
            _ => Self::String,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub parameter_type: ParameterType,
    pub value: String,
    pub version: i64,
    pub last_modified_date: DateTime<Utc>,
    pub arn: String,
    pub data_type: String, // "text" | "aws:ec2:image"
    pub description: Option<String>,
    pub key_id: Option<String>,
    pub tier: String, // "Standard" | "Advanced"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PutParameterRequest {
    pub name: String,
    pub value: String,
    pub parameter_type: Option<String>,
    pub description: Option<String>,
    pub overwrite: Option<bool>,
    pub key_id: Option<String>,
    pub tier: Option<String>,
    pub data_type: Option<String>,
    pub allowed_pattern: Option<String>,
}

// Snapshot Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterRecordSnapshot {
    pub current: Parameter,
    pub history: Vec<Parameter>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SsmSnapshot {
    pub parameters: Vec<ParameterRecordSnapshot>,
}

//! Parsing for a charm's source directory

use std::collections::HashMap;
use std::fs::read;
use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};
use serde_yaml::from_slice;

use crate::error::JujuError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Resource {
    #[serde(rename = "type")]
    pub kind: String,
    pub description: String,
    pub auto_fetch: bool,
    pub upstream_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Interface {
    pub interface: String,
}

mod storage_range {
    use super::Range;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(range: &Range, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:?}", range))
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Range, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let split: Vec<_> = s.splitn(2, '-').collect();

        match split.len() {
            1 => Ok(Range::Count(split[0].parse().unwrap())),
            2 => Ok(Range::Range {
                min: split[0].parse().unwrap(),
                max: Some(split[1].parse().unwrap()),
            }),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Range {
    Count(u32),
    Range { min: u32, max: Option<u32> },
}

impl Default for Range {
    fn default() -> Self {
        Range::Range { min: 0, max: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct StorageMultiple {
    #[serde(with = "storage_range")]
    range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum StorageType {
    Filesystem,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub struct Storage {
    #[serde(default)]
    description: String,
    #[serde(rename = "type")]
    kind: StorageType,
    location: String,
    #[serde(default)]
    minimum_size: String,
    #[serde(default)]
    multiple: StorageMultiple,
    #[serde(default)]
    read_only: bool,
    #[serde(default)]
    shared: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Metadata {
    pub name: String,
    pub display_name: Option<String>,
    pub summary: String,
    pub maintainers: Vec<String>,
    pub description: String,
    pub tags: Vec<String>,
    pub series: Vec<String>,
    #[serde(default)]
    pub resources: HashMap<String, Resource>,
    #[serde(default)]
    pub requires: HashMap<String, Interface>,
    #[serde(default)]
    pub provides: HashMap<String, Interface>,
    #[serde(default)]
    pub storage: HashMap<String, Storage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum ConfigOption {
    String {
        default: String,
        description: String,
    },
    #[serde(rename = "int")]
    Integer {
        default: i64,
        description: String,
    },
    Boolean {
        default: bool,
        description: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    pub options: HashMap<String, ConfigOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Layers {
    pub repo: String,
    pub includes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Charm {
    pub config: Config,
    pub layers: Layers,
    pub metadata: Metadata,
}

impl Charm {
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<Self, JujuError> {
        let p = path.into();
        let config = read(p.join("config.yaml"))?;
        let layers = read(p.join("layer.yaml"))?;
        let metadata = read(p.join("metadata.yaml"))?;

        Ok(Self {
            config: from_slice(&config).map_err(|err| JujuError::DeserError(format!("{}", err)))?,
            layers: from_slice(&layers).map_err(|err| JujuError::DeserError(format!("{}", err)))?,
            metadata: from_slice(&metadata)
                .map_err(|err| JujuError::DeserError(format!("{}", err)))?,
        })
    }
}

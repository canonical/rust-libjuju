use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

/// Config option as defined in config.yaml
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum ConfigOption {
    /// String config option
    String {
        default: Option<String>,
        description: String,
    },

    /// Integer config option
    #[serde(rename = "int")]
    Integer { default: i64, description: String },

    /// Boolean config option
    Boolean { default: bool, description: String },
}

/// A charm's config.yaml file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    pub options: HashMap<String, ConfigOption>,
}

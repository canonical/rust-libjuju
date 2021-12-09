use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Base {
    pub name: String,
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct BaseSpec {
    pub build_on: Vec<Base>,
    pub run_on: Vec<Base>,
}

/// A charm's charmcraft.yaml file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub struct Charmcraft {
    pub bases: Vec<BaseSpec>,
    #[serde(default)]
    pub architectures: Vec<String>,
}

use std::collections::HashMap;
use std::fs::{read, write};
use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};
use serde_yaml::{from_slice, to_vec};

use crate::error::JujuError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Integer(i64),
    Boolean(bool),
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum BundleType {
    Kubernetes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Annotations {
    gui_x: String,
    gui_y: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Application {
    #[serde(default)]
    pub annotations: Option<Annotations>,
    pub source: Option<String>,
    pub charm: String,
    #[serde(default)]
    pub config: HashMap<String, Value>,
    pub constraints: Option<String>,
    #[serde(default)]
    pub expose: bool,
    pub name: Option<String>,
    #[serde(default)]
    pub options: HashMap<String, String>,
    #[serde(default)]
    pub resources: HashMap<String, String>,
    #[serde(default, alias = "num_units")]
    pub scale: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    #[serde(alias = "services")]
    pub applications: HashMap<String, Application>,
    pub bundle: BundleType,
    pub description: Option<String>,
    #[serde(default)]
    pub relations: Vec<Vec<String>>,
    pub series: Option<String>,
}

impl Bundle {
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<Self, JujuError> {
        Ok(from_slice(&read(path.into())?)?)
    }

    pub fn save<P: Into<PathBuf>>(&self, path: P) -> Result<(), JujuError> {
        write(path.into(), to_vec(self)?)?;
        Ok(())
    }
}

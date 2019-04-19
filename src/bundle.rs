use std::collections::HashMap;
use std::fs::{read, write};
use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};
use serde_yaml::{from_slice, to_vec};

use crate::error::JujuError;

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
#[serde(deny_unknown_fields, untagged)]
pub enum Bundle {
    IaasBundle {
        #[serde(alias = "services")]
        applications: HashMap<String, Application>,
        description: Option<String>,
        #[serde(default)]
        relations: Vec<Vec<String>>,
        series: String,
    },
    CaasBundle {
        #[serde(alias = "services")]
        applications: HashMap<String, Application>,
        bundle: BundleType,
        description: Option<String>,
        #[serde(default)]
        relations: Vec<Vec<String>>,
    },
}

impl Bundle {
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<Self, JujuError> {
        Ok(from_slice(&read(path.into())?)?)
    }

    pub fn save<P: Into<PathBuf>>(&self, path: P) -> Result<(), JujuError> {
        write(path.into(), to_vec(self)?)?;
        Ok(())
    }

    pub fn applications(&self) -> &HashMap<String, Application> {
        match self {
            Bundle::IaasBundle { applications, .. } => applications,
            Bundle::CaasBundle { applications, .. } => applications,
        }
    }

    pub fn applications_mut(&mut self) -> &mut HashMap<String, Application> {
        match self {
            Bundle::IaasBundle { applications, .. } => applications,
            Bundle::CaasBundle { applications, .. } => applications,
        }
    }
}

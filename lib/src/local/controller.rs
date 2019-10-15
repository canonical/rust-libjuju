use std::collections::HashMap;

use ex::fs::read;
use serde_derive::Deserialize;
use serde_yaml::from_slice;

use crate::cmd::get_output;
use crate::error::JujuError;
use crate::paths::juju_data_dir;

#[derive(Debug, Clone)]
pub enum Substrate {
    CDK,
    MicroK8s,
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ControllerMachines {
    pub active: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Controller {
    pub active_controller_machine_count: u32,
    #[serde(default)]
    pub agent_version: String,
    pub api_endpoints: Vec<String>,
    pub ca_cert: String,
    pub cloud: String,
    pub controller_machine_count: u32,
    #[serde(default)]
    pub machine_count: u32,
    pub region: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub uuid: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ControllerYaml {
    pub current_controller: Option<String>,
    pub controllers: HashMap<String, Controller>,
}

impl ControllerYaml {
    pub fn load() -> Result<Self, JujuError> {
        let bytes = read(juju_data_dir().join("controllers.yaml"))?;

        Ok(from_slice(&bytes)?)
    }

    pub fn get(&self, name: Option<&str>) -> Result<&Controller, JujuError> {
        let n = match name {
            Some(n) => n,
            None => match &self.current_controller {
                Some(cc) => cc,
                None => return Err(JujuError::NoActiveController),
            },
        };
        self.controllers
            .get(n)
            .ok_or_else(|| JujuError::ControllerNotFound(n.to_string()))
    }

    pub fn validate_name(&self, name: Option<&str>) -> Result<String, JujuError> {
        match name {
            Some(n) => Ok(n.into()),
            None => match &self.current_controller {
                Some(cc) => Ok(cc.into()),
                None => Err(JujuError::NoActiveController),
            },
        }
    }

    pub fn substrate(&self, name: &str) -> Result<Substrate, JujuError> {
        let yaml = get_output(
            "juju",
            &[
                "status",
                "-m",
                &format!("{}:default", name),
                "--format",
                "yaml",
            ],
        )
        .unwrap_or_else(|_| Vec::new());

        let controller = self.get(Some(name))?;
        let is_cdk = String::from_utf8_lossy(&yaml)
            .find("kubernetes-master/0")
            .is_some();

        match (is_cdk, controller.region.as_ref().map(String::as_str)) {
            (true, _) => Ok(Substrate::CDK),
            (false, Some("localhost")) => Ok(Substrate::MicroK8s),
            (false, _) => Ok(Substrate::Unknown),
        }
    }
}

use std::collections::HashMap;
use std::process::Command;

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
    pub region: String,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub uuid: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ControllerYaml {
    pub current_controller: String,
    pub controllers: HashMap<String, Controller>,
}

impl ControllerYaml {
    pub fn load() -> Result<Self, JujuError> {
        let bytes = read(juju_data_dir().join("controllers.yaml"))?;

        Ok(from_slice(&bytes)?)
    }

    pub fn get(&self, name: Option<&str>) -> Result<&Controller, JujuError> {
        let n = name.unwrap_or(&self.current_controller);
        self.controllers
            .get(n)
            .ok_or_else(|| JujuError::ControllerNotFound(n.to_string()))
    }

    pub fn validate_name(&self, name: Option<&str>) -> String {
        name.unwrap_or(&self.current_controller).into()
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
        )?;

        let controller = self.get(Some(name))?;
        let is_cdk = String::from_utf8_lossy(&yaml)
            .find("kubernetes-master/0")
            .is_some();

        match (is_cdk, controller.region.as_ref()) {
            (true, _) => Ok(Substrate::CDK),
            (false, "localhost") => Ok(Substrate::MicroK8s),
            (false, _) => Ok(Substrate::Unknown),
        }
    }
}

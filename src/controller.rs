use std::collections::HashMap;
use std::fs::read;
use std::process::Command;

use serde_derive::Deserialize;
use serde_yaml::from_slice;

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
    pub agent_version: String,
    pub api_endpoints: Vec<String>,
    pub ca_cert: String,
    pub cloud: String,
    pub controller_machine_count: u32,
    pub machine_count: u32,
    pub region: String,
    pub uuid: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Controllers {
    pub current_controller: String,
    pub controllers: HashMap<String, Controller>,
}

impl Controllers {
    pub fn load() -> Result<Self, String> {
        let yaml = read(juju_data_dir().join("controllers.yaml"))
            .map_err(|err| format!("Couldn't read controllers file: {}", err))?;

        let mut controllers: Controllers =
            from_slice(&yaml).map_err(|err| format!("Couldn't parse controllers: {}", err))?;

        for (name, controller) in &mut controllers.controllers {
            controller.name = name.clone();
        }

        Ok(controllers)
    }

    pub fn get(&self, name: Option<&str>) -> Result<&Controller, String> {
        let n = name.unwrap_or(&self.current_controller);
        self.controllers
            .get(n)
            .ok_or_else(|| format!("Unknown controller {}", n))
    }

    pub fn substrate(&self, name: &str) -> Result<Substrate, String> {
        let yaml = Command::new("juju")
            .args(&[
                "status",
                "-m",
                &format!("{}:default", name),
                "--format",
                "yaml",
            ])
            .output()
            .map_err(|err| format!("Couldn't determine cloud type: {}", err))?
            .stdout;

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

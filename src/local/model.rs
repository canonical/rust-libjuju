use std::collections::HashMap;

use ex::fs::read;
use serde_derive::Deserialize;
use serde_yaml::from_slice;

use crate::error::JujuError;
use crate::paths::juju_data_dir;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum Model {
    Iaas {
        uuid: String,
        branch: Option<String>,
    },
    Caas {
        uuid: String,
        branch: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Models {
    #[serde(default)]
    pub models: HashMap<String, Model>,
    pub current_model: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelYaml {
    pub controllers: HashMap<String, Models>,
}

impl ModelYaml {
    pub fn load() -> Result<Self, JujuError> {
        let bytes = read(juju_data_dir().join("models.yaml"))?;

        Ok(from_slice(&bytes)?)
    }

    pub fn validate_name(
        &self,
        controller: &str,
        model: Option<&str>,
    ) -> Result<String, JujuError> {
        let models = self
            .controllers
            .get(controller)
            .ok_or_else(|| JujuError::ControllerNotFound(controller.to_string()))?;

        let user = "admin";
        match model {
            Some(name) => {
                let full_name = format!("{}/{}", user, name);
                if models.models.contains_key(&full_name) {
                    Ok(name.to_string())
                } else {
                    Err(JujuError::ModelNotFound(
                        controller.to_string(),
                        name.to_string(),
                    ))
                }
            }
            None => {
                if let Some(model) = &models.current_model {
                    let split = model.splitn(2, '/').map(String::from).collect::<Vec<_>>();

                    split
                        .get(1)
                        .cloned()
                        .ok_or_else(|| JujuError::UnknownModel(controller.to_string()))
                } else {
                    Err(JujuError::UnknownModel(controller.to_string()))
                }
            }
        }
    }
}

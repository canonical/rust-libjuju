pub mod operator;
pub mod reactive;
pub mod sidecar;

use std::collections::HashMap;
use std::path::PathBuf;

use crate::charm_url::CharmURL;
use crate::error::JujuError;

#[derive(Debug)]
pub enum CharmSource {
    Reactive(reactive::CharmSource),
    Operator(operator::CharmSource),
    Sidecar(sidecar::CharmSource),
}

impl CharmSource {
    /// Load a charm from its source directory
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<Self, JujuError> {
        let path = path.into();
        if let Ok(cs) = reactive::CharmSource::load(&path) {
            return Ok(CharmSource::Reactive(cs));
        }

        if let Ok(cs) = operator::CharmSource::load(&path) {
            return Ok(CharmSource::Operator(cs));
        }

        Ok(CharmSource::Sidecar(sidecar::CharmSource::load(&path)?))
    }

    /// Build the charm from its source directory
    ///
    /// `name` is used for `Reactive` charms, `destructive_mode` is used for `Operator`
    /// and `Sidecar` charms.
    pub fn build(&self, name: &str, destructive_mode: bool) -> Result<(), JujuError> {
        match self {
            CharmSource::Reactive(cs) => cs.build(name),
            CharmSource::Operator(cs) => cs.build(destructive_mode),
            CharmSource::Sidecar(cs) => cs.build(destructive_mode),
        }
    }

    /// Returns path of built artifact
    pub fn artifact_path(&self) -> CharmURL {
        match self {
            CharmSource::Reactive(cs) => cs.artifact_path(),
            CharmSource::Operator(cs) => cs.artifact_path(),
            CharmSource::Sidecar(cs) => cs.artifact_path(),
        }
    }

    pub fn upload_charm_store(
        &self,
        url: &str,
        resources: &HashMap<String, String>,
        to: &[String],
        destructive_mode: bool,
    ) -> Result<String, JujuError> {
        match self {
            CharmSource::Reactive(cs) => cs.upload_charm_store(url, resources, to),
            CharmSource::Operator(cs) => {
                cs.upload_charm_store(url, resources, to, destructive_mode)
            }
            CharmSource::Sidecar(cs) => cs.upload_charm_store(url, resources, to, destructive_mode),
        }
    }

    pub fn upload_charmhub(
        &self,
        url: &str,
        resources: &HashMap<String, String>,
        to: &[String],
        destructive_mode: bool,
    ) -> Result<String, JujuError> {
        match self {
            CharmSource::Reactive(cs) => cs.upload_charmhub(url, resources, to),
            CharmSource::Operator(cs) => cs.upload_charmhub(url, resources, to, destructive_mode),
            CharmSource::Sidecar(cs) => cs.upload_charmhub(url, resources, to, destructive_mode),
        }
    }

    /// Merge default resources with resources given in e.g. a bundle.yaml
    pub fn resources_with_defaults(
        &self,
        configured: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, JujuError> {
        match self {
            CharmSource::Reactive(cs) => cs.resources_with_defaults(configured),
            CharmSource::Operator(cs) => cs.resources_with_defaults(configured),
            CharmSource::Sidecar(cs) => cs.resources_with_defaults(configured),
        }
    }
}

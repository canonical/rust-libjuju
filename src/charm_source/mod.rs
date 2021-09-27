pub mod v1;
pub mod v2;

use std::collections::HashMap;
use std::path::PathBuf;

use crate::channel::Channel;
use crate::charm_url::CharmURL;
use crate::error::JujuError;

#[derive(Debug)]
pub enum CharmSource {
    V1(v1::CharmSource),
    V2(v2::CharmSource),
}

impl CharmSource {
    /// Load a charm from its source directory
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<Self, JujuError> {
        let path = path.into();
        match v1::CharmSource::load(&path) {
            Ok(cs) => Ok(CharmSource::V1(cs)),
            Err(_) => match v2::CharmSource::load(&path) {
                Ok(cs) => Ok(CharmSource::V2(cs)),
                Err(err) => Err(err),
            },
        }
    }

    /// Build the charm from its source directory
    ///
    /// `name` is used for V1 charms, `destructive_mode` is used for both V1 and V2
    /// charms, though for V1 Operator charms only, not Reactive charms.
    pub fn build(&self, name: &str, destructive_mode: bool) -> Result<(), JujuError> {
        match self {
            CharmSource::V1(cs) => cs.build(name, destructive_mode),
            CharmSource::V2(cs) => cs.build(destructive_mode),
        }
    }

    ///
    pub fn artifact_path(&self) -> CharmURL {
        match self {
            CharmSource::V1(cs) => cs.artifact_path(),
            CharmSource::V2(cs) => cs.artifact_path(),
        }
    }

    /// Push the charm to the charm store, and return the revision URL
    pub fn push(
        &self,
        cs_url: &str,
        resources: &HashMap<String, String>,
    ) -> Result<String, JujuError> {
        match self {
            CharmSource::V1(cs) => cs.push(cs_url, resources),
            CharmSource::V2(cs) => cs.push(cs_url, resources),
        }
    }

    /// Promote a charm from unpublished to the given channel
    pub fn promote(&self, rev_url: &str, to: Channel) -> Result<(), JujuError> {
        match self {
            CharmSource::V1(cs) => cs.promote(rev_url, to),
            CharmSource::V2(cs) => cs.promote(rev_url, to),
        }
    }

    /// Merge default resources with resources given in e.g. a bundle.yaml
    pub fn resources_with_defaults(
        &self,
        configured: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, JujuError> {
        match self {
            CharmSource::V1(cs) => cs.resources_with_defaults(configured),
            CharmSource::V2(cs) => cs.resources_with_defaults(configured),
        }
    }
}

use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

use super::container::Container;
use super::device::Device;
use super::relation::Relation;
use super::resource::Resource;
use super::storage::Storage;

/// A charm's metadata.yaml file
///
/// See https://juju.is/docs/sdk/metadata-reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Metadata {
    /// The name of the charm
    ///
    /// Determines URL in Charmhub and the name administrators will ultimately use to deploy the
    /// charm. E.g. `juju deploy <name>`
    pub name: String,

    /// A short, one-line description of the charm
    pub summary: String,

    /// A full description of the configuration layer
    pub description: String,

    /// A list of maintainers in the format "First Last <email>"
    #[serde(default)]
    pub maintainers: Vec<String>,

    /// A list of terms that any charm user must agree with
    #[serde(default)]
    pub terms: Vec<String>,

    /// True if the charm is meant to be deployed as a subordinate to a principal charm
    #[serde(default)]
    pub subordinate: bool,

    /// A map of containers to be created adjacent to the charm.
    ///
    /// This field is required when the charm is targeting Kubernetes, where each of the specified
    /// containers will be created as sidecars to the charm in the same pod.
    #[serde(default)]
    pub containers: HashMap<String, Container>,

    /// Additional resources that accompany the charm
    ///
    /// Each key represents the name of the resource
    #[serde(default)]
    pub resources: HashMap<String, Resource>,

    /// Map of relations provided by this charm
    ///
    /// Each key represents the name of the relation as known by this charm
    #[serde(default)]
    pub provides: HashMap<String, Relation>,

    /// Map of relations required by this charm
    ///
    /// Each key represents the name of the relation as known by this charm
    #[serde(default)]
    pub requires: HashMap<String, Relation>,

    /// Mutual relations between units/peers of this charm
    ///
    /// Each key represents the name of the relation as known by this charm
    #[serde(default)]
    pub peer: HashMap<String, Relation>,

    /// Storage requests for the charm
    ///
    /// Each key represents the name of the storage
    #[serde(default)]
    pub storage: HashMap<String, Storage>,

    /// Device requests for the charm, for example a GPU
    ///
    /// Each key represents the name of the device
    #[serde(default)]
    pub devices: HashMap<String, Device>,

    /// Extra bindings for the charm
    ///
    /// For example binding extra network interfaces. Key only map, value must be blank. Key
    /// represents the name
    #[serde(default)]
    pub extra_bindings: HashMap<String, ()>,

    /// If set, Juju magically determines that the charm is using v1 metadata
    #[serde(default)]
    pub series: Option<Vec<String>>,
}

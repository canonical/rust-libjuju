use serde_derive::{Deserialize, Serialize};

/// A Kubernetes container for a charm
///
/// Note: One of either resource or bases must be specified.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct ContainerBase {
    /// Name of the OS
    ///
    /// For example ubuntu/centos/windows/osx/opensuse
    pub name: String,

    /// Channel of the OS in format "track[/risk][/branch]" such as used by Snaps
    ///
    /// For example 20.04/stable or 18.04/stable/fips
    pub channel: String,

    /// List of architectures that this particular charm can run on
    pub architectures: Vec<String>,
}

/// A Kubernetes container for a charm
///
/// Note: One of either resource or bases must be specified.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct ContainerMount {
    /// Name of the storage to mount from the charm storage
    pub storage: String,

    /// In the case of filesystem storages, the location to mount the storage
    ///
    /// For multi-stores, the location acts as the parent directory for each mounted store.
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct ResourceContainer {
    /// Reference for an entry in the resources field
    ///
    /// Specifies the oci-image resource used to create the container.
    #[serde(default)]
    pub resource: String,

    /// List of mounted storages for this container
    #[serde(default)]
    pub mounts: Vec<ContainerMount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct BaseContainer {
    /// A list of bases in descending order of preference for use in resolving a container image
    ///
    /// These bases are listed as base (instead of name) and channel as in the Base definition, as
    /// an unnamed top-level object list
    #[serde(default)]
    pub bases: Vec<ContainerBase>,

    /// List of mounted storages for this container
    #[serde(default)]
    pub mounts: Vec<ContainerMount>,
}

/// A Kubernetes container for a charm
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case", untagged)]
pub enum Container {
    Resource(ResourceContainer),
    Base(BaseContainer),
}

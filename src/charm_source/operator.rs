//! Parsing for a charm's source directory

use std::collections::HashMap;
use std::env::current_dir;
use std::io::Read;
use std::path::{Path, PathBuf};

use ex::fs::{read, File};
use serde_derive::{Deserialize, Serialize};
use serde_yaml::{from_slice, Value};
use zip::ZipArchive;

use crate::channel::Channel;
use crate::charm_url::CharmURL;
use crate::cmd;
use crate::error::JujuError;

/// Helper for parsing storage ranges
mod storage_range {
    use serde::{self, Deserialize, Deserializer, Serializer};

    use super::Range;

    pub fn serialize<S>(range: &Range, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:?}", range))
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Range, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let split: Vec<_> = s.splitn(2, '-').collect();

        match split.len() {
            1 => Ok(Range::Count(split[0].parse().unwrap())),
            2 => Ok(Range::Range {
                min: split[0].parse().unwrap(),
                max: Some(split[1].parse().unwrap()),
            }),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum ResourceType {
    File,
    OciImage,
    Pypi,
    Url,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum RelationScope {
    Global,
    Container,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Resource {
    #[serde(rename = "type")]
    pub kind: ResourceType,
    pub description: String,
    #[serde(default)]
    pub auto_fetch: bool,
    pub upstream_source: Option<String>,
    pub filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Interface {
    pub interface: String,
    pub scope: Option<RelationScope>,
    pub schema: Option<String>,
    #[serde(default)]
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Range {
    Count(u32),
    Range { min: u32, max: Option<u32> },
}

impl Default for Range {
    fn default() -> Self {
        Range::Range { min: 0, max: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct StorageMultiple {
    #[serde(with = "storage_range")]
    range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum Storage {
    #[serde(rename_all = "kebab-case")]
    Filesystem {
        #[serde(default)]
        description: String,
        location: String,
        #[serde(default)]
        minimum_size: String,
        #[serde(default)]
        multiple: StorageMultiple,
        #[serde(default)]
        read_only: bool,
        #[serde(default)]
        shared: bool,
    },
    Block {
        #[serde(default)]
        multiple: StorageMultiple,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum DeploymentMode {
    Workload,
    Operator,
}

impl Default for DeploymentMode {
    fn default() -> Self {
        DeploymentMode::Workload
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Deployment {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub service: Option<String>,
    pub daemonset: Option<bool>,
    pub min_version: Option<String>,
    #[serde(default)]
    pub mode: DeploymentMode,
}

/// A charm's metadata.yaml file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Metadata {
    /// Machine-friendly name of the charm
    pub name: String,

    /// Human-friendly name of the charm
    pub display_name: Option<String>,

    /// Tweetable summary of the charm
    pub summary: String,

    /// Long-form description of the charm
    pub description: String,

    /// List of charm maintainers
    ///
    /// Expected format is `"Full Name <email@example.com>"`
    #[serde(default)]
    pub maintainers: Vec<String>,

    /// List of arbitrary topic tags for the charm
    pub tags: Vec<String>,

    /// Which OS this charm expects
    pub series: Vec<String>,

    /// Resources for the charm
    #[serde(default)]
    pub resources: HashMap<String, Resource>,

    /// Which other charms this charm requires a relation to in order to run
    #[serde(default)]
    pub requires: HashMap<String, Interface>,

    /// Which types of relations this charm provides
    #[serde(default)]
    pub provides: HashMap<String, Interface>,

    /// Storage configuration for the charm
    #[serde(default)]
    pub storage: HashMap<String, Storage>,

    /// Whether or not this charm is subordinate to another charm
    #[serde(default)]
    pub subordinate: bool,

    #[serde(default)]
    pub deployment: Option<Deployment>,

    /// Minimum Juju version supported by the charm
    pub min_juju_version: Option<String>,
}

/// Config option as defined in config.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum ConfigOption {
    /// String config option
    String {
        default: Option<String>,
        description: String,
    },

    /// Integer config option
    #[serde(rename = "int")]
    Integer { default: i64, description: String },

    /// Boolean config option
    Boolean { default: bool, description: String },
}

/// A charm's config.yaml file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    pub options: HashMap<String, ConfigOption>,
}

/// A charm's layers.yaml file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Layers {
    /// The charm's repository
    pub repo: String,

    /// A list of layers / interfaces to include
    ///
    /// Should be specified in the form of `"type:name"`, for example
    /// `"layer:status"`.
    pub includes: Vec<String>,

    // TODO: Make these better
    name: Option<String>,
    maintainer: Option<String>,
    description: Option<String>,

    proof: Option<Value>,

    #[serde(default)]
    options: HashMap<String, HashMap<String, bool>>,

    #[serde(default)]
    tactics: Vec<String>,

    is: Option<String>,
}

/// A charm, as represented by the source directory
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharmSource {
    /// The path to the charm's source code
    source: PathBuf,

    /// The charm's config.yaml file
    pub config: Option<Config>,

    /// The charm's layers.yaml file
    pub layers: Option<Layers>,

    /// The charm's metadata.yaml file
    pub metadata: Metadata,
}

impl CharmSource {
    fn load_dir(source: &Path) -> Result<Self, JujuError> {
        if source.join("reactive").exists() {
            return Err(JujuError::WrongCharmTypeError);
        }

        // Deserialize the layers.yaml and config.yaml files, if they exist.
        // Operator charms don't have layers.yaml, and charms with no config
        // don't need config.yaml. metadata.yaml is necessary, so we can assume
        // it exists and not jump through the extra hoop of `map`.
        let config: Option<Config> = read(source.join("config.yaml"))
            .map(|bytes| from_slice(&bytes))
            .unwrap_or(Ok(None))?;
        let layers: Option<Layers> = read(source.join("layer.yaml"))
            .map(|bytes| from_slice(&bytes))
            .unwrap_or(Ok(None))?;

        let metadata = from_slice(&read(source.join("metadata.yaml"))?)?;

        Ok(Self {
            source: source.into(),
            config,
            layers,
            metadata,
        })
    }

    fn load_zip(source: &Path) -> Result<Self, JujuError> {
        let mut archive = ZipArchive::new(File::open(source)?)?;

        if archive.by_name("reactive").is_ok() {
            return Err(JujuError::WrongCharmTypeError);
        }

        // Deserialize the layers.yaml and config.yaml files, if they exist.
        // Operator charms don't have layers.yaml, and charms with no config
        // don't need config.yaml. metadata.yaml is necessary, so we can assume
        // it exists and not jump through the extra hoop of `map`.
        let config: Option<Config> = archive
            .by_name("config.yaml")
            .map(|mut zf| -> Result<_, JujuError> {
                let mut buf = String::new();
                zf.read_to_string(&mut buf)?;
                Ok(from_slice(buf.as_bytes())?)
            })
            .unwrap_or(Ok(None))?;

        let layers: Option<Layers> = archive
            .by_name("layer.yaml")
            .map(|mut zf| -> Result<_, JujuError> {
                let mut buf = String::new();
                zf.read_to_string(&mut buf)?;
                Ok(from_slice(buf.as_bytes())?)
            })
            .unwrap_or(Ok(None))?;

        let metadata = {
            let mut zf = archive.by_name("metadata.yaml")?;
            let mut buf = String::new();
            zf.read_to_string(&mut buf)?;
            from_slice(buf.as_bytes())?
        };

        Ok(Self {
            source: source.into(),
            config,
            layers,
            metadata,
        })
    }

    /// Load a charm from its source directory
    pub fn load(source: &Path) -> Result<Self, JujuError> {
        if source.is_file() {
            Self::load_zip(source)
        } else {
            Self::load_dir(source)
        }
    }

    /// Build the charm from its source directory
    pub fn build(&self, destructive_mode: bool) -> Result<(), JujuError> {
        let source = self.source.to_string_lossy();
        let mut args = vec!["pack", "-p", &source];
        if destructive_mode {
            args.push("--destructive-mode")
        }
        cmd::run("charmcraft", &args)
    }

    pub fn artifact_path(&self) -> CharmURL {
        let mut path = current_dir().unwrap();
        path.push(&format!("{}_ubuntu-20.04-amd64.charm", self.metadata.name));
        CharmURL::from_path(path)
    }

    /// Push the charm to the charm store, and return the revision URL
    pub fn push(
        &self,
        _cs_url: &str,
        _resources: &HashMap<String, String>,
    ) -> Result<String, JujuError> {
        unimplemented!();
    }

    /// Promote a charm from unpublished to the given channel
    pub fn promote(&self, _rev_url: &str, _to: Channel) -> Result<(), JujuError> {
        unimplemented!();
    }

    /// Merge default resources with resources given in e.g. a bundle.yaml
    pub fn resources_with_defaults(
        &self,
        configured: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, JujuError> {
        self.metadata
            .resources
            .iter()
            .map(|(k, v)| -> Result<(String, String), JujuError> {
                match (configured.get(k), &v.upstream_source) {
                    (Some(val), _) | (_, Some(val)) => Ok((k.clone(), val.clone())),
                    (None, None) => Err(JujuError::ResourceNotFound(
                        k.clone(),
                        self.metadata.name.clone(),
                    )),
                }
            })
            .collect()
    }
}

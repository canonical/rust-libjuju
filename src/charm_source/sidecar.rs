use std::collections::HashMap;
use std::env::current_dir;
use std::io::Read;
use std::path::{Path, PathBuf};

use ex::fs::{read, File};
use serde_derive::{Deserialize, Serialize};
use serde_yaml::from_slice;
use zip::ZipArchive;

use crate::channel::Channel;
use crate::charm_url::CharmURL;
use crate::cmd;
use crate::error::JujuError;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Container {
    /// Back oci-image resource
    pub resource: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum ResourceType {
    File,
    OciImage,
    Pypi,
    Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Resource {
    #[serde(rename = "type")]
    pub kind: ResourceType,
    pub description: String,
    pub upstream_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum RelationScope {
    Global,
    Container,
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

/// A charm's metadata.yaml file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Metadata {
    /// Machine-friendly name of the charm
    pub name: String,

    /// Long-form description of the charm
    pub description: String,

    /// Tweetable summary of the charm
    pub summary: String,

    /// Containers for the charm
    #[serde(default)]
    pub containers: HashMap<String, Container>,

    /// Resources for the charm
    #[serde(default)]
    pub resources: HashMap<String, Resource>,

    /// Which other charms this charm requires a relation to in order to run
    #[serde(default)]
    pub requires: HashMap<String, Interface>,

    /// Which types of relations this charm provides
    #[serde(default)]
    pub provides: HashMap<String, Interface>,
}

/// A charm, as represented by the source directory
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharmSource {
    /// The path to the charm's source code
    source: PathBuf,

    /// The charm's config.yaml file
    pub config: Option<Config>,

    /// The charm's metadata.yaml file
    pub metadata: Metadata,
}

impl CharmSource {
    fn load_dir(source: &Path) -> Result<Self, JujuError> {
        let config: Option<Config> = read(source.join("config.yaml"))
            .map(|bytes| from_slice(&bytes))
            .unwrap_or(Ok(None))?;
        let metadata = from_slice(&read(source.join("metadata.yaml"))?)?;

        Ok(Self {
            source: source.into(),
            config,
            metadata,
        })
    }

    fn load_zip(source: &Path) -> Result<Self, JujuError> {
        let mut archive = ZipArchive::new(File::open(source)?)?;
        let config: Option<Config> = archive
            .by_name("config.yaml")
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

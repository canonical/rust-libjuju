//! Parsing for bundle.yaml files

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use ex::fs::{read, write};
use reqwest;
use serde_derive::{Deserialize, Serialize};
use serde_yaml::{from_slice, from_str, to_vec};

use crate::channel::Channel;
use crate::charm_url::CharmURL;
use crate::cmd;
use crate::error::JujuError;
use crate::series::Series;
use crate::store::Resource;

/// Represents a YAML value that doesn't have a pre-determined type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Integer(i64),
    Boolean(bool),
    None,
}

/// Arbitrary annotations for an application
///
/// TODO: These seem to be the only ones in use, are there any others?
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Annotations {
    pub gui_x: String,
    pub gui_y: String,
}

/// An application within the bundle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Application {
    /// Arbitrary annotations intepreted by things other than Juju itself
    #[serde(default)]
    pub annotations: Option<Annotations>,

    /// Charm source code location
    ///
    /// If the path starts with `.`, it's interpreted as being relative to
    /// the bundle itself. Otherwise, it's interpreted as being relative to
    /// `$CHARM_SOURCE_DIR`.
    pub source: Option<String>,

    /// URL of the charm
    ///
    /// Normally points to a charm store location in the form of `cs:~user/charm`
    /// If not set, `Application::source` will be used to build the charm. If both
    /// are set, this property is preferred, unless `--build` is passed. One or
    /// the other must be set.
    pub charm: Option<CharmURL>,

    /// Config options
    #[serde(default)]
    pub config: HashMap<String, Value>,

    /// Constraints such as `cores=2 mem=4G`
    ///
    /// See https://docs.jujucharms.com/2.5/en/reference-constraints for more info
    pub constraints: Option<String>,

    /// Whether to expose the application externally
    #[serde(default)]
    pub expose: bool,

    /// TODO: Is this an alias of `config`?
    #[serde(default)]
    pub options: HashMap<String, String>,

    /// Resources to make available to the application
    ///
    /// See https://docs.jujucharms.com/2.5/en/charms-resources for more info
    #[serde(default)]
    pub resources: HashMap<String, String>,

    /// How many units to use for the application
    #[serde(default, alias = "num_units")]
    pub scale: u32,
}

impl Application {
    pub fn release(&self, to: Channel) -> Result<(), JujuError> {
        match &self.charm {
            Some(charm) => {
                let url = format!(
                    "https://api.jujucharms.com/charmstore/v5/{}/meta/resources",
                    &charm.api_name()
                );

                let response: Vec<Resource> = reqwest::get(&url).unwrap().json().unwrap();

                let args = vec!["release", "--channel", to.into(), &charm.to_string()]
                    .into_iter()
                    .map(String::from)
                    .chain(
                        response
                            .iter()
                            .map(|res| format!("--resource={}-{}", res.name, res.revision)),
                    )
                    .collect::<Vec<_>>();

                cmd::run("charm", &args)
            }
            None => Err(JujuError::ApplicationNotFound("No charm URL set!".into())),
        }
    }
}

/// Represents a `bundle.yaml` file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    /// The applications in the bundle
    #[serde(alias = "services")]
    pub applications: HashMap<String, Application>,

    /// Which OS series to use for this bundle
    ///
    /// Either this or `series` must be set
    pub bundle: Option<Series>,

    /// Long-form description of the bundle
    pub description: Option<String>,

    /// Pairs of application names that require a relation between them
    #[serde(default)]
    pub relations: Vec<Vec<String>>,

    /// Which OS series to use for this bundle
    ///
    /// Either this or `bundle` must be set
    pub series: Option<Series>,
}

impl Bundle {
    /// Load a bundle from the given path
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<Self, JujuError> {
        Ok(from_slice(&read(path.into())?)?)
    }

    /// Load a bundle from the charm store
    ///
    /// TODO: Turn this into a more general charm store client
    pub fn load_from_store(name: &str, channel: Channel) -> Result<(u32, Self), JujuError> {
        let base_url = format!("https://api.jujucharms.com/charmstore/v5/bundle/{}", name);
        let rev_url = format!(
            "{}/meta/id-revision/?channel={}",
            base_url,
            channel.to_string()
        );

        let response: HashMap<String, u32> = reqwest::get(&rev_url).unwrap().json().unwrap();

        let revision = response["Revision"];

        let bundle_url = format!("{}-{}/archive/bundle.yaml", base_url, revision);
        let response = reqwest::get(&bundle_url)?.text().unwrap();

        Ok((revision, from_str(&response)?))
    }

    /// Save this bundle to the given path
    pub fn save<P: Into<PathBuf>>(&self, path: P) -> Result<(), JujuError> {
        write(path.into(), to_vec(self)?)?;
        Ok(())
    }

    /// Get a subset of the applications in this bundle
    ///
    /// Returns a copy of `self.applications` with only the given applications
    /// in it.
    pub fn app_subset(
        &self,
        names: Vec<String>,
    ) -> Result<HashMap<String, Application>, JujuError> {
        if names.is_empty() {
            return Ok(self.applications.clone());
        }

        let keys: HashSet<_> = self.applications.keys().cloned().collect();
        let names: HashSet<_> = names.into_iter().collect();
        let diff: Vec<String> = names.difference(&keys).cloned().collect();

        if diff.is_empty() {
            Ok(self
                .applications
                .iter()
                .filter_map(|(k, v)| {
                    if names.contains(k.as_str()) {
                        Some((k.clone(), v.clone()))
                    } else {
                        None
                    }
                })
                .collect())
        } else {
            Err(JujuError::ApplicationNotFound(diff.join(", ")))
        }
    }

    pub fn push(&self, bundle_path: &str, cs_url: &str) -> Result<String, JujuError> {
        let output: HashMap<String, String> =
            from_slice(&cmd::get_output("charm", &["push", bundle_path, cs_url])?)?;
        Ok(output["url"].clone())
    }

    pub fn release(&self, bundle_url: &str, to: Channel) -> Result<(), JujuError> {
        cmd::run("charm", &["release", "--channel", to.into(), bundle_url])
    }
}
//! Parsing for bundle.yaml files

use std::collections::{HashMap, HashSet};
use std::fs::{read, write};
use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};
use serde_yaml::{from_slice, to_vec};

use crate::channel::Channel;
use crate::cmd;
use crate::error::JujuError;

/// Represents a YAML value that doesn't have a pre-determined type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Integer(i64),
    Boolean(bool),
    None,
}

/// Type of bundle
///
/// Currently only `kubernetes` is allowed as a value, as any other
/// types should actually be put under `series` instead of `bundle`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum BundleType {
    Kubernetes,
}

/// Type of bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum Series {
    Oneiric,
    Precise,
    Quantal,
    Raring,
    Saucy,
    Trusty,
    Utopic,
    Vivid,
    Wily,
    Xenial,
    Yakkety,
    Zesty,
    Artful,
    Bionic,
    Cosmic,
    Disco,
    Win2012hvr2,
    Win2012hv,
    Win2012r2,
    Win2012,
    Win7,
    Win8,
    Win81,
    Win10,
    Win2016,
    Win2016hv,
    Win2016nano,
    Centos7,
}

/// Arbitrary annotations for an application
///
/// TODO: These seem to be the only ones in use, are there any others?
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Annotations {
    gui_x: String,
    gui_y: String,
}

/// An application within the bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub charm: Option<String>,

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

/// Represents a `bundle.yaml` file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    /// The applications in the bundle
    #[serde(alias = "services")]
    pub applications: HashMap<String, Application>,

    /// Which type of bundle this is
    ///
    /// Currently only `kubernetes` is accepted. For other bundle types,
    /// the value must be put under `series:`.
    pub bundle: BundleType,

    /// Long-form description of the bundle
    pub description: Option<String>,

    /// Pairs of application names that require a relation between them
    #[serde(default)]
    pub relations: Vec<Vec<String>>,

    /// Which OS series to use for this bundle
    ///
    /// Mutually exclusive with `bundle`
    pub series: Option<Series>,
}

impl Bundle {
    /// Load a bundle from the given path
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<Self, JujuError> {
        Ok(from_slice(&read(path.into())?)?)
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
        cmd::run(
            "charm",
            &["release", "--channel", &to.to_string(), bundle_url],
        )
    }
}

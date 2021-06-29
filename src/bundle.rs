//! Parsing for bundle.yaml files

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use ex::fs::{read, write};
use serde_derive::{Deserialize, Serialize};
use serde_yaml::{from_slice, from_str, to_vec};

use crate::channel::Channel;
use crate::charm_source::CharmSource;
use crate::charm_url::CharmURL;
use crate::cmd;
use crate::cmd::get_output;
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

/// An application within a bundle
///
/// See the `ApplicationSpec` defined [here][spec] for the canonical upstream definition
///
/// [spec]: https://github.com/juju/charm/blob/master/bundledata.go
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Application {
    /// Arbitrary annotations intepreted by things other than Juju itself
    #[serde(default)]
    pub annotations: Option<Annotations>,

    /// Preferred channel to use when deploying a remote charm
    pub channel: Option<String>,

    /// URL of the charm
    ///
    /// Normally points to a charm store location in the form of `cs:~user/charm`
    /// If not set, `Application::source` will be used to build the charm. If both
    /// are set, this property is preferred, unless `--build` is passed. One or
    /// the other must be set.
    pub charm: Option<CharmURL>,

    /// Used to set charm config at deployment time
    ///
    /// Duplicate of `options`, but Juju doesn't care if both are specified,
    /// which serde doesn't like. So, we copy it here as well.
    #[serde(default)]
    pub config: HashMap<String, Value>,

    /// Constraints such as `cores=2 mem=4G`
    ///
    /// See the [constraints documentation][constraints] for more info
    ///
    /// [constraints]: https://juju.is/docs/olm/constraints
    pub constraints: Option<String>,

    /// Constraints for devices to assign to units of the application
    #[serde(default)]
    pub devices: HashMap<String, String>,

    /// Maps how endpoints are bound to spaces
    #[serde(default)]
    pub endpoint_bindings: HashMap<String, String>,

    /// Whether to expose the application externally
    #[serde(default)]
    pub expose: bool,

    /// Used to set charm config at deployment time
    #[serde(default)]
    pub options: HashMap<String, Value>,

    /// Model selector/affinity expression for specifying pod placement
    ///
    /// Use for Kubernetes applications, not relevant for IaaS applications
    pub placement: Option<String>,

    /// Plan under which the application is to be deployed
    pub plan: Option<String>,

    /// Whether the application requires access to cloud credentials
    #[serde(default)]
    pub requires_trust: bool,

    /// Resources to make available to the application
    ///
    /// See https://docs.jujucharms.com/2.5/en/charms-resources for more info
    #[serde(default)]
    pub resources: HashMap<String, String>,

    /// How many units to use for the application
    #[serde(default, alias = "num_units")]
    pub scale: u32,

    /// Series to use when deploying a local charm
    pub series: Option<String>,

    /// Charm source code location
    ///
    /// If the path starts with `.`, it's interpreted as being relative to
    /// the bundle itself. Otherwise, it's interpreted as being relative to
    /// `$CHARM_SOURCE_DIR`.
    pub source: Option<String>,

    /// Constraints for storage to assign to units of the application
    #[serde(default)]
    pub storage: HashMap<String, String>,

    /// Which Node (Kubernetes) or Unit (IaaS) this charm should be assigned to
    #[serde(default)]
    pub to: Vec<String>,
}

impl Application {
    pub fn release(&self, to: Channel) -> Result<(), JujuError> {
        match &self.charm {
            Some(charm) => {
                let resources: Vec<Resource> = from_slice(&get_output(
                    "charm",
                    &["list-resources", "--format", "yaml", &charm.to_string()],
                )?)?;

                let resource_args = resources
                    .iter()
                    .map(|res| format!("--resource={}-{}", res.name, res.revision));

                let args = vec!["release", "--channel", to.into(), &charm.to_string()]
                    .into_iter()
                    .map(String::from)
                    .chain(resource_args)
                    .collect::<Vec<_>>();

                cmd::run("charm", &args)
            }
            None => Err(JujuError::ApplicationNotFound("No charm URL set!".into())),
        }
    }

    pub fn upgrade(&self, name: &str) -> Result<(), JujuError> {
        let source_dir = self
            .charm
            .as_ref()
            .map(ToString::to_string)
            .expect("Built charm directory can't be empty");
        let charm = CharmSource::load(source_dir.clone())?;
        let resources = charm.resources_with_defaults(&self.resources)?;

        let args = vec!["upgrade-charm", name, "--path", &source_dir]
            .into_iter()
            .map(String::from)
            .chain(
                resources
                    .iter()
                    .map(|(k, v)| format!("--resource={}={}", k, v)),
            )
            .collect::<Vec<_>>();

        cmd::run("juju", &args)
    }

    /// Calculates the path to the charm's source directory
    ///
    /// This can be either manually set with `source: ./foo` in `bundle.yaml`,
    /// or implicit in the existence of a `./charms/foo` directory, relative
    /// to `bundle.yaml`.
    pub fn source(&self, name: &str, bundle_path: &str) -> Option<String> {
        if self.source.is_some() {
            self.source.clone()
        } else {
            let root = PathBuf::from(bundle_path);
            let root = root.parent().unwrap();
            let paths = [
                root.join("./").join(name),
                root.join("./charms/").join(name),
            ];

            for path in &paths {
                if path.exists() {
                    return Some(path.to_string_lossy().to_string());
                }
            }

            None
        }
    }
}

/// Represents a `bundle.yaml` file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    /// Bundle name, used for uploading to charm store
    #[serde(default)]
    pub name: Option<String>,

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
        let parsed = CharmURL::parse(name).unwrap();
        let namespace = match parsed.namespace {
            Some(n) => n,
            None => return Err(JujuError::NamespaceRequired(name.into())),
        };

        let output: HashMap<String, HashMap<String, u32>> = from_slice(&cmd::get_output(
            "charm",
            &[
                "show",
                name,
                "--channel",
                &channel.to_string(),
                "--format=yaml",
                "id-revision",
            ],
        )?)?;

        let revision = output["id-revision"]["Revision"];

        let bundle_url = format!(
            "https://api.jujucharms.com/charmstore/v5/~{}/bundle/{}-{}/archive/bundle.yaml",
            namespace, parsed.name, revision
        );

        let response = reqwest::get(&bundle_url)?.text()?;
        let parsed: CharmStoreResponse = from_str(&response)?;

        match parsed {
            CharmStoreResponse::Bundle(b) => Ok((revision, b)),
            CharmStoreResponse::Error { message, .. } => Err(JujuError::MacaroonError(message)),
        }
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
        exceptions: Vec<String>,
    ) -> Result<HashMap<String, Application>, JujuError> {
        if names.is_empty() {
            return Ok(self.applications.clone());
        }

        let keys: HashSet<_> = self.applications.keys().cloned().collect();
        let names: HashSet<_> = names.into_iter().collect();
        let exceptions: HashSet<_> = exceptions.into_iter().collect();
        let diff: Vec<String> = names.difference(&keys).cloned().collect();

        if diff.is_empty() {
            Ok(self
                .applications
                .iter()
                .filter_map(|(k, v)| {
                    if names.contains(k.as_str()) && !exceptions.contains(k.as_str()) {
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

    pub fn upgrade_charms(&self) -> Result<(), JujuError> {
        for (name, app) in &self.applications {
            app.upgrade(name)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged, deny_unknown_fields)]
enum CharmStoreResponse {
    Bundle(Bundle),
    #[serde(rename_all = "PascalCase")]
    Error {
        code: String,
        message: String,
        info: serde_yaml::Value,
    },
}

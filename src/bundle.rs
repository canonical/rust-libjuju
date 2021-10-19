//! Parsing for bundle.yaml files

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;

use ex::fs::{read, write};
use rayon::prelude::*;
use serde_derive::{Deserialize, Serialize};
use serde_yaml::{from_slice, from_str, to_vec};

use crate::channel::Channel;
use crate::charm_source::CharmSource;
use crate::charm_url::CharmURL;
use crate::cmd;
use crate::cmd::get_output;
use crate::error::JujuError;
use crate::paths;
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
    pub trust: bool,

    /// Resources to make available to the application
    ///
    /// See <https://juju.is/docs/sdk/resources> for more info
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
        match &self.source {
            Some(s) => Some(s.clone()),
            None => {
                let root = PathBuf::from(bundle_path);
                let root = root.parent().unwrap();
                let paths = [
                    root.join("./").join(name),
                    root.join("./charms/").join(name),
                    root.join("./operators/").join(name),
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

    pub fn upload_charm_store(
        &self,
        name: &str,
        publish_namespace: Option<String>,
        bundle_path: &str,
        channels: &[String],
        destructive_mode: bool,
    ) -> Result<String, JujuError> {
        let name = self
            .charm
            .as_ref()
            .map(|c| c.name.as_ref())
            .unwrap_or(name)
            .to_string();
        let source = self.source(&name, bundle_path);

        let charm = self
            .charm
            .as_ref()
            .ok_or(JujuError::UnknownCharmURLError(name))?;

        let cs_url = charm
            .with_namespace(publish_namespace)
            .with_store(Some("cs".into()));

        match &source {
            Some(source) => {
                // If `source` starts with `.`, it's a relative path from the bundle we're
                // deploying. Otherwise, look in `CHARM_SOURCE_DIR` for it.
                let charm_path = if source.starts_with('.') {
                    PathBuf::from(bundle_path).parent().unwrap().join(source)
                } else {
                    paths::charm_source_dir().join(source)
                };

                let charm = CharmSource::load(&charm_path)?;

                let rev_url = charm.upload_charm_store(
                    &cs_url.to_string(),
                    &self.resources,
                    channels,
                    destructive_mode,
                )?;
                Ok(rev_url)
            }
            None => {
                let channel = self
                    .channel
                    .as_ref()
                    .map(|c| Channel::from_str(c))
                    .transpose()?
                    .unwrap_or(Channel::Stable);

                let revision = charm.show(channel)?.id_revision.revision;
                Ok(charm.with_revision(Some(revision)).to_string())
            }
        }
    }

    pub fn upload_charmhub(
        &self,
        name: &str,
        publish_namespace: Option<String>,
        bundle_path: &str,
        channels: &[String],
        destructive_mode: bool,
    ) -> Result<String, JujuError> {
        let name = self
            .charm
            .as_ref()
            .map(|c| c.name.as_ref())
            .unwrap_or(name)
            .to_string();
        let source = self.source(&name, bundle_path);

        let charm = self
            .charm
            .as_ref()
            .ok_or(JujuError::UnknownCharmURLError(name))?;

        let cs_url = charm
            .with_namespace(publish_namespace)
            .with_store(Some("cs".into()));

        match &source {
            Some(source) => {
                // If `source` starts with `.`, it's a relative path from the bundle we're
                // deploying. Otherwise, look in `CHARM_SOURCE_DIR` for it.
                let charm_path = if source.starts_with('.') {
                    PathBuf::from(bundle_path).parent().unwrap().join(source)
                } else {
                    paths::charm_source_dir().join(source)
                };

                let charm = CharmSource::load(&charm_path)?;

                let rev_url = charm.upload_charmhub(
                    &cs_url.to_string(),
                    &self.resources,
                    channels,
                    destructive_mode,
                )?;
                Ok(rev_url)
            }
            None => {
                let channel = self
                    .channel
                    .as_ref()
                    .map(|c| Channel::from_str(c))
                    .transpose()?
                    .unwrap_or(Channel::Stable);

                let revision = charm.show(channel)?.id_revision.revision;
                Ok(charm.with_revision(Some(revision)).to_string())
            }
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
    #[serde(skip_serializing_if = "Option::is_none")]
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

    /// Updates bundle to use subset of applications
    pub fn limit_apps(&mut self, names: &[String], exceptions: &[String]) -> Result<(), JujuError> {
        if names.is_empty() {
            return Ok(());
        }

        self.applications
            .retain(|k, _| names.contains(k) && !exceptions.contains(k));

        // Filter out relations that point to an application that was filtered out
        let apps: HashSet<_> = self.applications.keys().collect();
        self.relations.retain(|rels| {
            // Strip out interface name-style syntax before filtering,
            // e.g. `foo:bar` => `foo`.
            rels.iter()
                .map(|r| r.split(':').next().unwrap().to_string())
                .all(|r| apps.contains(&r))
        });

        Ok(())
    }

    pub fn upload_charm_store(
        &self,
        bundle_path: &str,
        cs_url: &str,
        channels: &[String],
    ) -> Result<(), JujuError> {
        let output: HashMap<String, String> =
            from_slice(&cmd::get_output("charm", &["push", bundle_path, cs_url])?)?;
        let bundle_url = output["url"].clone();

        for channel in channels.iter() {
            self.release(&bundle_url, channel)?;
        }

        Ok(())
    }

    pub fn upload_charmhub(
        &self,
        bundle_path: &str,
        cs_url: &CharmURL,
        channels: &[String],
    ) -> Result<(), JujuError> {
        cmd::run("charmcraft", &["pack", "-p", bundle_path])?;

        let zip = PathBuf::from(bundle_path)
            .join(&cs_url.name)
            .with_extension("zip");

        let args: Vec<_> = vec!["upload".into(), zip.to_string_lossy().to_string()]
            .into_iter()
            .chain(channels.iter().map(|ch| format!("--release={}", ch)))
            .collect();

        let output = cmd::get_output("charmcraft", &args)?;

        println!("\n// https://github.com/canonical/charmcraft/issues/478");
        println!("Output from charmcraft upload in case something broke:");
        println!("{}", String::from_utf8_lossy(&output));

        Ok(())
    }

    pub fn release(&self, bundle_url: &str, to: &str) -> Result<(), JujuError> {
        cmd::run("charm", &["release", "--channel", to, bundle_url])
    }

    pub fn upgrade_charms(&self) -> Result<(), JujuError> {
        for (name, app) in &self.applications {
            app.upgrade(name)?;
        }

        Ok(())
    }

    pub fn build(
        &mut self,
        path: &str,
        build_apps: Option<HashMap<String, Option<String>>>,
        destructive_mode: bool,
        parallel_build: bool,
    ) -> Result<(), JujuError> {
        let map = |(name, application): (&String, &Application)| {
            let mut new_application = application.clone();

            let source = match &build_apps {
                Some(app) => app
                    .get(name)
                    .cloned()
                    .and_then(|source| source.or_else(|| application.source(name, path))),
                None => application.source(name, path),
            };
            new_application.charm = match (&application.charm, source) {
                // Either `charm` or `source` must be set
                (None, None) => {
                    return Err(JujuError::MissingSourceError(name.into()));
                }

                // If the charm source was defined and either the `--build` flag was passed, or
                // if there's no `charm` property, build the charm
                (_, Some(source)) => {
                    println!("Building {}", name);

                    // If `source` starts with `.`, it's a relative path from the bundle we're
                    // deploying. Otherwise, look in `CHARM_SOURCE_DIR` for it.
                    let charm_path = if source.starts_with('.') {
                        PathBuf::from(path).parent().unwrap().join(source)
                    } else {
                        paths::charm_source_dir().join(source)
                    };

                    let charm = CharmSource::load(&charm_path)?;

                    charm.build(name, destructive_mode)?;

                    new_application.resources =
                        charm.resources_with_defaults(&new_application.resources)?;

                    Some(charm.artifact_path())
                }

                // If a charm URL was defined and charm source isn't available
                // locally, use the charm URL
                (Some(charm), None) => Some(charm.clone()),
            };

            Ok((name.clone(), new_application))
        };

        let mapped: Result<HashMap<String, Application>, JujuError> = if parallel_build {
            self.applications.par_iter().map(map).collect()
        } else {
            self.applications.iter().map(map).collect()
        };

        self.applications = mapped?;

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

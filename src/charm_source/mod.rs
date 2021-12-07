pub mod config;
pub mod container;
pub mod device;
pub mod metadata;
pub mod relation;
pub mod resource;
pub mod storage;

pub use config::{Config, ConfigOption};
pub use container::{BaseContainer, Container, ContainerBase, ContainerMount, ResourceContainer};
pub use metadata::Metadata;
pub use relation::{Relation, RelationScope};
pub use resource::Resource;
pub use storage::Storage;

use std::collections::HashMap;
use std::env::current_dir;
use std::io::Read;
use std::path::PathBuf;
use std::str::from_utf8;

use ex::fs::{read, File};
use serde_derive::{Deserialize, Serialize};
use serde_yaml::from_slice;
use zip::ZipArchive;

use crate::charm_url::CharmURL;
use crate::cmd;
use crate::error::JujuError;

/// A charm, as represented by the source directory
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    fn load_dir<P: Into<PathBuf>>(source: P) -> Result<Self, JujuError> {
        let source = source.into();
        let config: Option<Config> = read(source.join("config.yaml"))
            .map(|bytes| from_slice(&bytes))
            .unwrap_or(Ok(None))?;
        let metadata = from_slice(&read(source.join("metadata.yaml"))?)?;

        Ok(Self {
            source,
            config,
            metadata,
        })
    }

    fn load_zip<P: Into<PathBuf>>(source: P) -> Result<Self, JujuError> {
        let source = source.into();
        let mut archive = ZipArchive::new(File::open(&source)?)?;
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
            source,
            config,
            metadata,
        })
    }

    /// Load a charm from its source directory
    pub fn load<P: Into<PathBuf>>(source: P) -> Result<Self, JujuError> {
        let source = source.into();
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

    pub fn upload_charmhub(
        &self,
        resources: &HashMap<String, String>,
        to: &[String],
        destructive_mode: bool,
    ) -> Result<String, JujuError> {
        self.build(destructive_mode)?;

        let resources = self.resources_with_defaults(resources)?;

        let resources: Vec<_> = resources
            .iter()
            .filter_map(|(name, value)| {
                let res = self.metadata.resources.get(name).expect("Must exist!");

                match res {
                    Resource::OciImage { .. } => {
                        cmd::run(
                            "charmcraft",
                            &[
                                "upload-resource",
                                &self.metadata.name,
                                name,
                                "--image",
                                value,
                            ],
                        )
                        .unwrap();

                        let output = cmd::get_output(
                            "charmcraft",
                            &["resource-revisions", &self.metadata.name, name],
                        )
                        .unwrap();
                        let output = String::from_utf8_lossy(&output);
                        let revision = output.lines().nth(1).unwrap().split(' ').next().unwrap();

                        Some(format!("--resource={}:{}", name, revision))
                    }
                    Resource::File { .. } => None,
                }
            })
            .collect();

        let args: Vec<_> = vec![
            "upload".into(),
            "--quiet".into(),
            self.artifact_path().to_string(),
        ]
        .into_iter()
        .chain(to.iter().map(|ch| format!("--release={}", ch)))
        .chain(resources)
        .collect();

        let mut output = cmd::get_output("charmcraft", &args)?;
        output.drain(0..9);
        output.truncate(output.iter().position(|&x| x == 0x20).unwrap());
        let revision = from_utf8(&output).unwrap().parse::<u32>().unwrap();

        Ok(CharmURL::parse(&self.metadata.name)
            .unwrap()
            .with_revision(Some(revision))
            .to_string())
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
                if let Some(c) = configured.get(k) {
                    return Ok((k.clone(), c.clone()));
                }

                match v {
                    Resource::OciImage {
                        upstream_source: Some(us),
                        ..
                    } => Ok((k.clone(), us.clone())),
                    Resource::OciImage { .. } => Err(JujuError::ResourceNotFound(
                        k.clone(),
                        self.metadata.name.clone(),
                    )),
                    Resource::File { .. } => Err(JujuError::ResourceNotFound(
                        k.clone(),
                        self.metadata.name.clone(),
                    )),
                }
            })
            .collect()
    }
}

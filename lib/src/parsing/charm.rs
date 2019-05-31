//! Parsing for a charm's source directory

use std::collections::HashMap;
use std::fs::read;
use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};
use serde_yaml::from_slice;

use crate::channel::Channel;
use crate::cmd;
use crate::error::JujuError;
use crate::paths;

/// Helper for parsing storage ranges
mod storage_range {
    use super::Range;
    use serde::{self, Deserialize, Deserializer, Serializer};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Resource {
    #[serde(rename = "type")]
    pub kind: ResourceType,
    pub description: String,
    pub auto_fetch: bool,
    pub upstream_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Interface {
    pub interface: String,
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
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum StorageType {
    Filesystem,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub struct Storage {
    #[serde(default)]
    description: String,
    #[serde(rename = "type")]
    kind: StorageType,
    location: String,
    #[serde(default)]
    minimum_size: String,
    #[serde(default)]
    multiple: StorageMultiple,
    #[serde(default)]
    read_only: bool,
    #[serde(default)]
    shared: bool,
}

/// A charm's metadata.yaml file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Metadata {
    pub name: String,
    pub display_name: Option<String>,
    pub summary: String,
    pub maintainers: Vec<String>,
    pub description: String,
    pub tags: Vec<String>,
    pub series: Vec<String>,
    #[serde(default)]
    pub resources: HashMap<String, Resource>,
    #[serde(default)]
    pub requires: HashMap<String, Interface>,
    #[serde(default)]
    pub provides: HashMap<String, Interface>,
    #[serde(default)]
    pub storage: HashMap<String, Storage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum ConfigOption {
    String {
        default: String,
        description: String,
    },
    #[serde(rename = "int")]
    Integer {
        default: i64,
        description: String,
    },
    Boolean {
        default: bool,
        description: String,
    },
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
}

/// A charm, as represented by the source directory
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Charm {
    /// The charm's config.yaml file
    pub config: Config,

    /// The charm's layers.yaml file
    pub layers: Layers,

    /// The charm's metadata.yaml file
    pub metadata: Metadata,

    /// The path to the charm's source code
    source: PathBuf,
}

impl Charm {
    /// Load a charm from its source directory
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<Self, JujuError> {
        let p = path.into();
        let config = read(p.join("config.yaml"))?;
        let layers = read(p.join("layer.yaml"))?;
        let metadata = read(p.join("metadata.yaml"))?;

        Ok(Self {
            config: from_slice(&config)?,
            layers: from_slice(&layers)?,
            metadata: from_slice(&metadata)?,
            source: p,
        })
    }

    /// Build the charm from its source directory
    pub fn build(&self) -> Result<(), JujuError> {
        cmd::run(
            "charm",
            &[
                "build",
                &self.source.to_string_lossy(),
                "--cache-dir",
                &paths::charm_cache_dir(&self.metadata.name).to_string_lossy(),
            ],
        )
    }

    /// Push the charm to the charm store, and return the revision URL
    pub fn push(
        &self,
        cs_url: &str,
        resources: &HashMap<String, String>,
    ) -> Result<String, JujuError> {
        let build_dir = paths::charm_build_dir()
            .join(&self.metadata.name)
            .to_string_lossy()
            .to_string();

        let resources = self.resources_with_defaults(resources)?;

        let mut args = vec!["push".to_string(), build_dir, cs_url.to_string()];
        args.extend(
            resources
                .iter()
                .flat_map(|(k, v)| vec![String::from("--resource"), format!("{}={}", k, v)]),
        );

        for (name, value) in resources {
            let res = self.metadata.resources.get(&name).expect("Must exist!");

            if res.kind != ResourceType::OciImage {
                continue;
            }

            cmd::run("docker", &["pull", &value])?;
        }

        let mut output = cmd::get_output(
            "charm",
            args.iter()
                .map(std::ops::Deref::deref)
                .collect::<Vec<_>>()
                .as_ref(),
        )?;

        // The command output is valid YAML that includes the URL that we care about, but
        // also includes output from `docker push`, so just chop out the first line that's
        // valid YAML.
        output.truncate(output.iter().position(|&x| x == 0x0a).unwrap());
        let push_metadata: HashMap<String, String> = from_slice(&output)?;
        let rev_url = push_metadata["url"].clone();

        // Attempt to tag the revision with the git commit, but ignore any failures
        // getting the commit.
        match cmd::get_output("git", &["rev-parse", "HEAD"]) {
            Ok(rev_output) => {
                let revision = String::from_utf8_lossy(&rev_output);
                cmd::run("charm", &["set", &rev_url, &format!("commit={}", revision)])?;
            }
            Err(err) => {
                println!(
                    "Error while getting git revision for {}, not tagging: `{}`",
                    self.metadata.name, err
                );
            }
        }

        Ok(rev_url)
    }

    /// Promote a charm from unpublished to the given channel
    pub fn promote(&self, rev_url: &str, to: Channel) -> Result<(), JujuError> {
        let resources: Vec<HashMap<String, String>> = from_slice(&cmd::get_output(
            "charm",
            &["list-resources", rev_url, "--format", "yaml"],
        )?)?;

        let mut release_args = vec![
            "release".to_string(),
            rev_url.to_string(),
            "--channel".to_string(),
            to.to_string(),
        ];

        release_args.extend(resources.iter().flat_map(|r| {
            vec![
                String::from("--resource"),
                format!("{}-{}", r["name"], r["revision"]),
            ]
        }));

        cmd::run(
            "charm",
            release_args
                .iter()
                .map(std::ops::Deref::deref)
                .collect::<Vec<_>>()
                .as_ref(),
        )
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

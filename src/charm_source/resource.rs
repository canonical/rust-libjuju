use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", tag = "type")]
pub enum Resource {
    #[serde(rename_all = "kebab-case")]
    File {
        /// Description of the resource and its purpose
        description: Option<String>,

        /// The filename of the resource as it should appear in the filesystem
        filename: String,
    },
    #[serde(rename_all = "kebab-case")]
    OciImage {
        /// Description of the resource and its purpose
        description: Option<String>,

        /// Default used by many tools
        ///
        /// Juju is weird about this though for reasons, see
        /// https://bugs.launchpad.net/juju/+bug/1946121
        upstream_source: Option<String>,
    },
}

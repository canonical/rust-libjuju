use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Device {
    /// The interface schema that this relation conforms to
    #[serde(rename = "type")]
    pub kind: String,

    /// Description of the requested device
    #[serde(default)]
    pub description: Option<String>,

    /// Minimum number of devices required
    #[serde(default)]
    pub countmin: Option<u32>,

    /// Maximum number of devices required
    #[serde(default)]
    pub countmax: Option<u32>,
}

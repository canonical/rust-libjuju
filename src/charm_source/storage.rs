use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", tag = "type")]
pub enum Storage {
    Filesystem {
        /// Description of the storage requested
        #[serde(default)]
        description: Option<String>,

        /// The mount location for filesystem stores
        ///
        /// For multi-stores the location acts as the parent directory for each mounted store.
        #[serde(default)]
        location: Option<String>,

        /// Indicates if all units of the application share the storage
        #[serde(default)]
        shared: bool,

        /// Indicates if the storage should be made read-only (where possible)
        #[serde(default)]
        read_only: bool,

        /// The number of storage instances to be requested
        #[serde(default)]
        multiple: Option<String>,

        /// Minimum size of requested storage in forms G, GiB, GB
        ///
        /// Size multipliers are M, G, T, P, E, Z or Y. With no multiplier supplied, M is implied.
        #[serde(default)]
        minimum_size: Option<String>,

        /// List of properties, only supported value is "transient"
        #[serde(default)]
        properties: Vec<String>,
    },
    Block {
        /// Description of the storage requested
        #[serde(default)]
        description: Option<String>,

        /// The mount location for filesystem stores
        ///
        /// For multi-stores the location acts as the parent directory for each mounted store.
        #[serde(default)]
        location: Option<String>,

        /// Indicates if all units of the application share the storage
        #[serde(default)]
        shared: bool,

        /// Indicates if the storage should be made read-only (where possible)
        #[serde(default)]
        read_only: bool,

        /// The number of storage instances to be requested
        #[serde(default)]
        multiple: Option<String>,

        /// Minimum size of requested storage in forms G, GiB, GB
        ///
        /// Size multipliers are M, G, T, P, E, Z or Y. With no multiplier supplied, M is implied.
        #[serde(default)]
        minimum_size: Option<String>,

        /// List of properties, only supported value is "transient"
        #[serde(default)]
        properties: Vec<String>,
    },
}

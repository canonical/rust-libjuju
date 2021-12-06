use serde_derive::{Deserialize, Serialize};

/// Scope of a given relation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum RelationScope {
    Global,
    Container,
}

impl Default for RelationScope {
    fn default() -> Self {
        Self::Global
    }
}

/// Relation between charms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Relation {
    /// The interface schema that this relation conforms to
    pub interface: String,

    /// Maximum number of supported connections to this relation endpoint
    pub limit: Option<u32>,

    /// Defines if the relation is required
    ///
    /// Informational only.
    #[serde(default)]
    pub optional: bool,

    /// The scope of the relation. Defaults to "global"
    pub scope: Option<RelationScope>,

    /// Which schema this relation conforms to
    ///
    /// Distinct from `interface`, which doesn't actually have any notion of schemas because 🤷. For
    /// more information, see:
    ///
    /// https://github.com/canonical/operator-schemas
    /// https://github.com/canonical/serialized-data-interface
    pub schema: Option<String>,

    /// Which schema versions this charm accepts
    ///
    /// For more information, see:
    ///
    /// https://github.com/canonical/operator-schemas
    /// https://github.com/canonical/serialized-data-interface
    #[serde(default)]
    pub versions: Vec<String>,
}

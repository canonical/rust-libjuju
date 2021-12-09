use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

//use serde_yaml::from_str;

// WIP

//struct ArchiveSize {}
//struct ArchiveUploadTime {}
//struct BundleMachineCount {}
//struct BundleMetadata {}
//struct BundleUnitCount {}
//struct BundlesContaining {}
//struct CanIngest {}
//struct CanWrite {}
//struct CharmActions {}
//struct CharmConfig {}
//struct CharmMetadata {}
//struct CharmMetrics {}
//struct CharmRelated {}
//struct CommonInfo {}
//struct ExtraInfo {}
//struct Hash {}
//struct Hash256 {}
//struct IdName {}
//struct IdRevision {}
//struct IdSeries {}
//struct IdUser {}
//struct Manifest {}
//struct Owner {}
//struct Perm {}
//struct Promulgated {}
//struct PromulgatedId {}
//struct Published {}
//struct RevisionInfo {}
//struct Stats {}
//struct SupportedSeries {}
//
//#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
//#[serde(rename_all = "PascalCase")]
//enum MetaField {
//    Id {
//        Foo: u32,
//    },
//    Tags(Vec<String>),
//    Terms,
//    UnpromulgatedId,
//}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceType {
    OciImage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Resource {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ResourceType,
    pub path: String,
    pub description: String,
    pub revision: u32,
    pub fingerprint: Option<String>,
    pub size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct ShowMetadata {
    #[serde(default)]
    pub deployment: HashMap<String, String>,
    pub description: String,
    pub name: String,
    #[serde(default)]
    pub provides: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub requires: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub peers: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub resources: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub storage: HashMap<String, HashMap<String, String>>,
    pub subordinate: bool,
    pub summary: String,
    pub supported_series: Vec<String>,
    pub tags: Vec<String>,
    #[serde(rename = "min-juju-version")]
    pub min_juju_version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct IdName {
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct IdRevision {
    pub revision: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct OwnerUser {
    pub user: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct Perm {
    pub read: Vec<String>,
    pub write: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct Promulgated {
    pub promulgated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct Published {
    pub channel: String,
    pub current: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct SupportedSeries {
    pub supported_series: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Show {
    pub bugs_url: String,
    pub charm_metadata: ShowMetadata,
    pub homepage: String,
    pub id_name: IdName,
    pub id_revision: IdRevision,
    pub owner: OwnerUser,
    pub perm: Perm,
    pub promulgated: Promulgated,
    pub published: HashMap<String, Vec<Published>>,
    pub supported_series: SupportedSeries,
    pub terms: Vec<String>,
}

//#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
//pub struct Meta {
//    id: MetaField,
//    tags: MetaField,
//}
//
//#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
//#[serde(rename_all = "PascalCase")]
//pub struct MetaResponse {
//    id: String,
//    meta: Meta,
//}
//
//
//#[test]
//fn parse_meta() {
//    let string = r#"{"Tags":["file-servers","storage"]}"#;
//    let parsed: MetaField = from_str(string).unwrap();
//    println!("DEBUG: {:?}", parsed);
//
//    let string = r#"{"Id":"cs:~kubeflow-charmers/minio-4","Meta":{"id":{"Id":"cs:~kubeflow-charmers/minio-4","User":"kubeflow-charmers","Name":"minio","Revision":4}}}"#;
//    let parsed: MetaResponse = from_str(string).unwrap();
//    println!("DEBUG: {:?}", parsed);
//}

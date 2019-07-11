use serde_derive::{Deserialize, Serialize};

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
//#[serde(deny_unknown_fields, rename_all = "PascalCase")]
//enum MetaField {
//    Id {
//        Foo: u32,
//    },
//    Tags(Vec<String>),
//    Terms,
//    UnpromulgatedId,
//}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum ResourceType {
    OciImage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
pub struct Resource {
    pub name: String,
    #[serde(rename = "Type")]
    pub kind: ResourceType,
    pub path: String,
    pub description: String,
    pub revision: u32,
    pub fingerprint: Option<String>,
    pub size: u32,
}

//#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
//#[serde(deny_unknown_fields)]
//pub struct Meta {
//    id: MetaField,
//    tags: MetaField,
//}
//
//#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
//#[serde(deny_unknown_fields, rename_all = "PascalCase")]
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

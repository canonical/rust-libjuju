use serde_derive::{Deserialize, Serialize};

/// Type of charm and/or bundle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum Series {
    // If the charm/bundle runs on Kubernetes instead of a specific OS
    Kubernetes,

    // Ubuntu
    Oneiric,
    Precise,
    Quantal,
    Raring,
    Saucy,
    Trusty,
    Utopic,
    Vivid,
    Wily,
    Xenial,
    Yakkety,
    Zesty,
    Artful,
    Bionic,
    Cosmic,
    Disco,
    Eoan,

    // Windows
    Win2012hvr2,
    Win2012hv,
    Win2012r2,
    Win2012,
    Win7,
    Win8,
    Win81,
    Win10,
    Win2016,
    Win2016hv,
    Win2016nano,

    // CentOS
    Centos7,
}

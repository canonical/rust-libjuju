use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum Channel {
    Unpublished,
    Edge,
    Beta,
    Candidate,
    Stable,
}

impl ToString for Channel {
    fn to_string(&self) -> String {
        match self {
            Channel::Unpublished => "unpublished",
            Channel::Edge => "edge",
            Channel::Beta => "beta",
            Channel::Candidate => "candidate",
            Channel::Stable => "stable",
        }
        .into()
    }
}

impl From<Channel> for &str {
    fn from(ch: Channel) -> &'static str {
        match ch {
            Channel::Unpublished => "unpublished",
            Channel::Edge => "edge",
            Channel::Beta => "beta",
            Channel::Candidate => "candidate",
            Channel::Stable => "stable",
        }
    }
}

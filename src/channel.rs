use std::str::FromStr;

use serde_derive::{Deserialize, Serialize};
use serde_yaml::{from_slice, Error};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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

impl FromStr for Channel {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        from_slice(s.as_bytes())
    }
}
